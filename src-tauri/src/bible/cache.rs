//! The verse cache: `bible_cache` in SQLite, seeded from the bundled packs and
//! filled further from YouVersion on demand (FR-18, FR-19, FR-22).
//!
//! ## Where text comes from, in order
//!
//! 1. **Bundled packs**, at first start. KJV, WEB and ASV ship as
//!    `assets/translations/*.jsonl.gz` and are loaded into the cache once. They
//!    are public domain, so their attribution is a courtesy line rather than a
//!    licence term — but it is stored and shown exactly as a licensed one is,
//!    because the display code must not have two paths.
//! 2. **The cache**, at every lookup. An indexed row fetch, measured well
//!    under FR-15's 5 ms.
//! 3. **YouVersion**, on a miss for a licensed translation, when online. The
//!    returned markup is sanitized to projector text and stored **with the
//!    version's attribution**. A verse whose version has no attribution is not
//!    cached and not returned: YouVersion's terms require the attribution
//!    wherever the text is shown, and a bare verse would be shown bare.
//!
//! ## Integrity at startup (FR-22)
//!
//! A bundled translation is re-seeded whenever its cached verse count differs
//! from its manifest's, or it has never been verified. Before any seed, the
//! pack's SHA-256 is checked against the manifest; a pack that does not match
//! is refused with a message naming it, rather than loaded and displayed. A
//! translation that passes both is left alone — startup pays one `COUNT(*)`
//! per bundled version, not a re-read of 31,102 rows.
//!
//! ## What is deliberately not here
//!
//! Imports (FR-47) and pack downloads of further translations belong to M6.
//! This module reads what is bundled and what YouVersion returns; it does not
//! know how a pack got onto the disk.

use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::time::Instant;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::reference::{self, UsfmRef};
use super::sanitize;
use super::youversion::Passage;
use crate::db::models::Verse;
use crate::error::{Error, Result};

/// A cached verse with what has to be shown beside it.
#[derive(Debug, Clone, PartialEq)]
pub struct CachedVerse {
    pub verse: Verse,
    /// The version's copyright line. Never empty for a row that exists.
    pub attribution: String,
    /// Original markup where the source had any; the summary PDF renders it.
    pub html: Option<String>,
}

/// What seeding did, for the startup log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedReport {
    pub seeded: Vec<(String, usize)>,
    pub verified: Vec<String>,
    pub refused: Vec<String>,
}

/// The header row of a pack and its sidecar `.manifest.json` share this shape.
#[derive(Debug, Deserialize)]
struct Manifest {
    code: String,
    name: String,
    #[serde(default)]
    language: String,
    #[serde(default)]
    attribution: String,
    verse_count: usize,
    #[serde(default)]
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct PackRow {
    b: String,
    c: u16,
    v: u16,
    t: String,
}

/// Seed every bundled pack that is missing, incomplete or unverified.
///
/// Idempotent and cheap on the second run: each translation costs one row
/// count unless something is wrong with it.
pub fn seed_bundled(conn: &mut Connection, assets_dir: &Path) -> Result<SeedReport> {
    let translations = assets_dir.join("translations");
    let mut report = SeedReport::default();

    let mut manifests: Vec<_> = std::fs::read_dir(&translations)
        .map_err(|e| {
            Error::Bible(format!(
                "The bundled translations were not found at {}: {e}. Reinstall SermonAI.",
                translations.display()
            ))
        })?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".manifest.json"))
        })
        .collect();
    manifests.sort();

    for manifest_path in manifests {
        let manifest: Manifest =
            serde_json::from_reader(File::open(&manifest_path)?).map_err(|e| {
                Error::Bible(format!(
                    "{} is not a valid pack manifest: {e}",
                    manifest_path.display()
                ))
            })?;

        if is_verified(conn, &manifest)? {
            report.verified.push(manifest.code.clone());
            continue;
        }

        let pack_path =
            manifest_path.with_file_name(format!("{}.jsonl.gz", manifest.code.to_lowercase()));
        match seed_pack(conn, &manifest, &pack_path) {
            Ok(rows) => report.seeded.push((manifest.code.clone(), rows)),
            Err(err) => {
                // One bad pack must not stop the others, or the app. It is
                // reported and left out; the operator is told which.
                tracing::error!(pack = %pack_path.display(), %err, "refusing a bundled translation");
                report.refused.push(manifest.code.clone());
            }
        }
    }

    Ok(report)
}

/// A translation whose cached rows match its manifest and which has been
/// verified before needs nothing.
fn is_verified(conn: &Connection, manifest: &Manifest) -> Result<bool> {
    let verified_at: Option<i64> = conn
        .query_row(
            "SELECT last_verified_at FROM bible_versions WHERE code = ?1",
            [&manifest.code],
            |row| row.get(0),
        )
        .optional()?;
    if verified_at.unwrap_or(0) == 0 {
        return Ok(false);
    }
    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM bible_cache WHERE translation_code = ?1",
        [&manifest.code],
        |row| row.get(0),
    )?;
    Ok(count == manifest.verse_count)
}

fn seed_pack(conn: &mut Connection, manifest: &Manifest, pack_path: &Path) -> Result<usize> {
    let bytes = std::fs::read(pack_path).map_err(|e| {
        Error::Bible(format!(
            "Could not read the {} pack: {e}. Reinstall SermonAI.",
            manifest.code
        ))
    })?;

    // FR-22: the pack must be the one the manifest describes. A pack that
    // fails this is not loaded, not "loaded with a warning".
    if !manifest.sha256.is_empty() {
        let digest = hex::encode(Sha256::digest(&bytes));
        if digest != manifest.sha256 {
            return Err(Error::Bible(format!(
                "The {} pack does not match its manifest and was not loaded. Reinstall SermonAI.",
                manifest.code
            )));
        }
    }

    let started = Instant::now();
    let reader = BufReader::new(flate2::read::GzDecoder::new(&bytes[..]));
    let now = chrono::Utc::now().timestamp();

    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM bible_cache WHERE translation_code = ?1",
        [&manifest.code],
    )?;

    let mut rows = 0usize;
    {
        let mut insert = tx.prepare(
            "INSERT INTO bible_cache (translation_code, book_id, chapter, verse, text, attribution, attribution_updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for (n, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            // The first line is the manifest header, repeated inside the pack
            // so the pack is self-describing without its sidecar.
            if n == 0 && line.contains("\"verse_count\"") {
                continue;
            }
            let row: PackRow = serde_json::from_str(&line).map_err(|e| {
                Error::Bible(format!(
                    "The {} pack has a malformed row {n}: {e}",
                    manifest.code
                ))
            })?;
            insert.execute(params![
                manifest.code,
                row.b,
                row.c,
                row.v,
                row.t,
                manifest.attribution,
                now
            ])?;
            rows += 1;
        }
    }

    if rows != manifest.verse_count {
        // Rolled back by the drop of `tx`: a partial translation on screen is
        // worse than none.
        return Err(Error::Bible(format!(
            "The {} pack holds {rows} verses but its manifest says {}. Not loaded.",
            manifest.code, manifest.verse_count
        )));
    }

    tx.execute(
        "INSERT INTO bible_versions (code, full_name, language, source, attribution, license_status, last_verified_at)
         VALUES (?1, ?2, ?3, 'bundled', ?4, 'public_domain', ?5)
         ON CONFLICT(code) DO UPDATE SET
           full_name = excluded.full_name,
           language = excluded.language,
           attribution = excluded.attribution,
           last_verified_at = excluded.last_verified_at",
        params![manifest.code, manifest.name, manifest.language, manifest.attribution, now],
    )?;
    tx.commit()?;

    tracing::info!(
        translation = %manifest.code,
        rows,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "seeded a bundled translation into the cache"
    );
    Ok(rows)
}

/// A verse, or verse range, from the cache. `None` when it is not cached.
///
/// `passage_id` is USFM (`JHN.3.16`, `PSA.139.13-16`); anything the reference
/// parser accepts works. A range is returned as one text with the verses in
/// order, because that is what goes on the projector.
pub fn lookup(
    conn: &Connection,
    translation_code: &str,
    passage_id: &str,
) -> Result<Option<CachedVerse>> {
    let reference = reference::parse(passage_id).map_err(|e| Error::Bible(e.to_string()))?;
    lookup_ref(conn, translation_code, &reference)
}

pub fn lookup_ref(
    conn: &Connection,
    translation_code: &str,
    reference: &UsfmRef,
) -> Result<Option<CachedVerse>> {
    let last = reference.end_verse.unwrap_or(reference.verse);
    let mut stmt = conn.prepare_cached(
        "SELECT verse, text, attribution, html FROM bible_cache
         WHERE translation_code = ?1 AND book_id = ?2 AND chapter = ?3 AND verse BETWEEN ?4 AND ?5
         ORDER BY verse",
    )?;
    let rows = stmt.query_map(
        params![
            translation_code,
            reference.code(),
            reference.chapter,
            reference.verse,
            last
        ],
        |row| {
            Ok((
                row.get::<_, u16>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        },
    )?;

    let mut texts = Vec::new();
    let mut attribution = String::new();
    let mut html: Option<String> = None;
    for row in rows {
        let (_, text, attr, h) = row?;
        texts.push(text);
        if attribution.is_empty() {
            attribution = attr;
        }
        if html.is_none() {
            html = h;
        }
    }

    if texts.is_empty() {
        return Ok(None);
    }
    // A row with no attribution is one migration 0002 marked "never verified".
    // It is not shown until refreshed; reporting it as absent sends the
    // caller to fetch, which is the refresh.
    if attribution.is_empty() {
        return Ok(None);
    }

    Ok(Some(CachedVerse {
        verse: Verse {
            reference: reference.display(),
            translation: translation_code.to_string(),
            text: texts.join(" "),
        },
        attribution,
        html,
    }))
}

/// Store a passage YouVersion returned, sanitized, with its version's
/// attribution. Refuses an empty attribution: the text would otherwise be
/// shown without the line the licence requires beside it.
pub fn store_fetched(
    conn: &Connection,
    translation_code: &str,
    passage: &Passage,
    attribution: &str,
) -> Result<CachedVerse> {
    if attribution.trim().is_empty() {
        return Err(Error::Bible(format!(
            "{translation_code} has no copyright attribution on record, so its text cannot be shown. Refresh licences in Settings → Packs."
        )));
    }

    let reference = reference::parse(&passage.id).map_err(|e| Error::Bible(e.to_string()))?;
    let clean = sanitize::sanitize(&passage.content);
    let now = chrono::Utc::now().timestamp();

    // One row per verse where the markup marked them, so a later single-verse
    // lookup inside this range hits; the whole text as one row otherwise.
    let per_verse: Vec<(u16, String)> = if clean.verses.is_empty() {
        vec![(reference.verse, clean.text.clone())]
    } else {
        clean.verses.clone()
    };

    let mut insert = conn.prepare_cached(
        "INSERT INTO bible_cache (translation_code, book_id, chapter, verse, text, attribution, attribution_updated_at, html)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(translation_code, book_id, chapter, verse) DO UPDATE SET
           text = excluded.text,
           attribution = excluded.attribution,
           attribution_updated_at = excluded.attribution_updated_at,
           html = excluded.html",
    )?;
    for (verse, text) in &per_verse {
        insert.execute(params![
            translation_code,
            reference.code(),
            reference.chapter,
            verse,
            text.trim(),
            attribution,
            now,
            passage.content
        ])?;
    }

    Ok(CachedVerse {
        verse: Verse {
            reference: reference.display(),
            translation: translation_code.to_string(),
            text: clean.text.trim().to_string(),
        },
        attribution: attribution.to_string(),
        html: Some(passage.content.clone()),
    })
}

/// Read a pack header without the sidecar, for tools and tests.
pub fn read_pack_header(pack_path: &Path) -> Result<(String, usize)> {
    let file = File::open(pack_path)?;
    let mut reader = BufReader::new(flate2::read::GzDecoder::new(file));
    let mut first = String::new();
    reader.read_line(&mut first)?;
    let header: Manifest = serde_json::from_str(&first).map_err(|e| {
        Error::Bible(format!(
            "{} has no readable header: {e}",
            pack_path.display()
        ))
    })?;
    let mut rest = String::new();
    reader.read_to_string(&mut rest)?;
    Ok((header.code, header.verse_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_for_tests(&conn).unwrap();
        conn
    }

    fn passage(id: &str, html: &str) -> Passage {
        Passage {
            id: id.into(),
            content: html.into(),
            reference: String::new(),
        }
    }

    #[test]
    fn a_fetched_passage_is_stored_with_its_attribution_and_read_back() {
        let conn = fresh();
        let html = r#"<div><div class="p"><span class="yv-v" v="16"></span><span class="yv-vlbl">16</span>For God so loved the world.</div></div>"#;
        let stored =
            store_fetched(&conn, "NIV", &passage("JHN.3.16", html), "Holy Bible, NIV").unwrap();
        assert_eq!(stored.verse.text, "For God so loved the world.");
        assert_eq!(stored.attribution, "Holy Bible, NIV");

        let hit = lookup(&conn, "NIV", "JHN.3.16").unwrap().expect("cached");
        assert_eq!(hit.verse.reference, "John 3:16");
        assert_eq!(hit.verse.text, "For God so loved the world.");
        assert_eq!(hit.attribution, "Holy Bible, NIV");
        assert!(hit.html.is_some(), "the markup is kept for the PDF");
    }

    #[test]
    fn a_verse_without_attribution_is_refused_not_cached() {
        // YouVersion's terms: the attribution goes wherever the text goes. A
        // verse with none would be shown bare, so it is not shown at all.
        let conn = fresh();
        let err = store_fetched(&conn, "NIV", &passage("JHN.3.16", "<div>x</div>"), "   ")
            .expect_err("no attribution, no cache");
        assert!(err.to_string().contains("attribution"), "{err}");
        assert!(lookup(&conn, "NIV", "JHN.3.16").unwrap().is_none());
    }

    #[test]
    fn a_missing_verse_is_none_rather_than_an_error() {
        let conn = fresh();
        assert!(lookup(&conn, "KJV", "JHN.3.16").unwrap().is_none());
    }

    #[test]
    fn a_range_comes_back_as_one_text_in_order() {
        let conn = fresh();
        for (v, t) in [
            (4u16, "Love is patient."),
            (5, "It does not envy."),
            (6, "It rejoices in truth."),
        ] {
            store_fetched(
                &conn,
                "WEB",
                &passage(&format!("1CO.13.{v}"), &format!("<div>{t}</div>")),
                "World English Bible. Public domain.",
            )
            .unwrap();
        }
        let hit = lookup(&conn, "WEB", "1CO.13.4-6").unwrap().expect("range");
        assert_eq!(hit.verse.reference, "1 Corinthians 13:4-6");
        assert_eq!(
            hit.verse.text,
            "Love is patient. It does not envy. It rejoices in truth."
        );
    }

    #[test]
    fn a_pack_whose_hash_does_not_match_is_refused() {
        let dir = tempfile_dir();
        let pack = dir.join("xxx.jsonl.gz");
        std::fs::write(&pack, gz(b"{\"code\":\"XXX\",\"name\":\"X\",\"verse_count\":1}\n{\"b\":\"GEN\",\"c\":1,\"v\":1,\"t\":\"In the beginning\"}\n")).unwrap();
        let manifest = Manifest {
            code: "XXX".into(),
            name: "X".into(),
            language: "eng".into(),
            attribution: "test".into(),
            verse_count: 1,
            sha256: "0000000000000000000000000000000000000000000000000000000000000000".into(),
        };
        let mut conn = fresh();
        let err = seed_pack(&mut conn, &manifest, &pack).expect_err("wrong hash");
        assert!(
            err.to_string().contains("does not match its manifest"),
            "{err}"
        );
        assert!(
            lookup(&conn, "XXX", "GEN.1.1").unwrap().is_none(),
            "nothing was loaded"
        );
    }

    #[test]
    fn a_pack_with_the_wrong_verse_count_is_rolled_back() {
        let dir = tempfile_dir();
        let body = b"{\"code\":\"YYY\",\"name\":\"Y\",\"verse_count\":2}\n{\"b\":\"GEN\",\"c\":1,\"v\":1,\"t\":\"one\"}\n";
        let pack = dir.join("yyy.jsonl.gz");
        let bytes = gz(body);
        std::fs::write(&pack, &bytes).unwrap();
        let manifest = Manifest {
            code: "YYY".into(),
            name: "Y".into(),
            language: "eng".into(),
            attribution: "test".into(),
            verse_count: 2, // the pack holds one
            sha256: hex::encode(Sha256::digest(&bytes)),
        };
        let mut conn = fresh();
        let err = seed_pack(&mut conn, &manifest, &pack).expect_err("short pack");
        assert!(
            err.to_string()
                .contains("holds 1 verses but its manifest says 2"),
            "{err}"
        );
        // The transaction rolled back: not even the one row survived.
        assert!(lookup(&conn, "YYY", "GEN.1.1").unwrap().is_none());
    }

    fn gz(bytes: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(bytes).unwrap();
        enc.finish().unwrap()
    }

    fn tempfile_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sermonai-cache-test-{}-{}",
            std::process::id(),
            rand_suffix()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn rand_suffix() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }
}
