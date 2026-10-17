//! The verse cache against the real bundled packs (M2 deliverable 5).
//!
//! Seeds KJV, WEB and ASV from `assets/translations` into a temporary
//! database exactly as startup does, then checks the three things the
//! deliverable promises: the text is there and correct, a second start
//! verifies rather than re-seeds (FR-22), and a lookup is inside the 5 ms the
//! M2 Definition of Done gives it.

use std::path::PathBuf;
use std::time::Instant;

use sermonai_lib::bible::cache::{lookup, seed_bundled};
use sermonai_lib::db;

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn temp_data_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sermonai-cache-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_bundled_translations_seed_and_verify() {
    let data_dir = temp_data_dir("seed");
    let mut conn = db::init(&data_dir).expect("db");

    let started = Instant::now();
    let first = seed_bundled(&mut conn, &assets_dir()).expect("seed");
    let seed_time = started.elapsed();

    let mut codes: Vec<&str> = first.seeded.iter().map(|(c, _)| c.as_str()).collect();
    codes.sort();
    assert_eq!(
        codes,
        ["ASV", "KJV", "WEB"],
        "all three bundled packs seed: {first:?}"
    );
    assert!(
        first.refused.is_empty(),
        "no pack failed integrity: {:?}",
        first.refused
    );
    // KJV is the full 31,102; ASV omits 16 verses its revisers moved to the
    // margin, and the manifest says so.
    let kjv = first.seeded.iter().find(|(c, _)| c == "KJV").unwrap().1;
    assert_eq!(kjv, 31_102);
    eprintln!("seeded three translations in {seed_time:.2?}");

    // A second start: every translation verifies by count and none re-seeds.
    let started = Instant::now();
    let second = seed_bundled(&mut conn, &assets_dir()).expect("verify");
    let verify_time = started.elapsed();
    assert!(
        second.seeded.is_empty(),
        "nothing should re-seed: {second:?}"
    );
    let mut verified = second.verified.clone();
    verified.sort();
    assert_eq!(verified, ["ASV", "KJV", "WEB"]);
    eprintln!("verified three translations in {verify_time:.2?}");
    assert!(
        verify_time.as_millis() < 500,
        "FR-22 verification should be a count per translation, not a re-read: {verify_time:?}"
    );
}

#[test]
fn the_text_is_the_text() {
    let data_dir = temp_data_dir("text");
    let mut conn = db::init(&data_dir).expect("db");
    seed_bundled(&mut conn, &assets_dir()).expect("seed");

    let hit = lookup(&conn, "KJV", "JHN.3.16")
        .unwrap()
        .expect("John 3:16 in the KJV");
    assert!(
        hit.verse.text.starts_with("For God so loved the world"),
        "{}",
        hit.verse.text
    );
    assert_eq!(hit.verse.reference, "John 3:16");
    assert_eq!(hit.verse.translation, "KJV");
    assert!(
        hit.attribution.contains("Public domain"),
        "{}",
        hit.attribution
    );

    // The same verse in a different bundled translation is different text.
    let web = lookup(&conn, "WEB", "JHN.3.16").unwrap().expect("WEB");
    assert!(web.verse.text.starts_with("For God so loved the world"));
    assert_ne!(
        web.verse.text, hit.verse.text,
        "WEB and KJV should not be identical"
    );

    // A range, and a single-chapter book, through the same path.
    let range = lookup(&conn, "KJV", "PSA.23.1-2").unwrap().expect("range");
    assert!(range.verse.text.contains("shepherd") && range.verse.text.contains("green pastures"));
    let jude = lookup(&conn, "KJV", "JUD.1.3").unwrap().expect("Jude 3");
    assert!(
        jude.verse.text.contains("common salvation"),
        "{}",
        jude.verse.text
    );

    // Something the canon does not have is refused before the cache is asked
    // (M3 polish item 1, 17 Oct): an error that names the count, never an
    // empty result that a caller could show as a reference-only verse.
    let err = lookup(&conn, "KJV", "JHN.3.200").unwrap_err().to_string();
    assert!(err.contains("John 3 has 36 verses, not 200"), "{err}");
}

#[test]
fn a_lookup_is_inside_the_five_millisecond_budget() {
    let data_dir = temp_data_dir("speed");
    let mut conn = db::init(&data_dir).expect("db");
    seed_bundled(&mut conn, &assets_dir()).expect("seed");

    // Warm the statement cache and the page cache.
    for _ in 0..10 {
        lookup(&conn, "KJV", "ROM.8.28").unwrap();
    }

    let refs = [
        "JHN.3.16",
        "ROM.8.28",
        "JER.29.11",
        "PSA.23.1",
        "GEN.1.1",
        "REV.22.21",
        "1CO.13.4-7",
        "HEB.11.1",
    ];
    let runs = 200;
    let started = Instant::now();
    for i in 0..runs {
        let r = refs[i % refs.len()];
        lookup(&conn, "KJV", r).unwrap().expect(r);
    }
    let per_lookup = started.elapsed() / runs as u32;
    eprintln!(
        "cache lookup: {per_lookup:.3?} per lookup ({} profile)",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    assert!(
        per_lookup.as_micros() < 5_000,
        "M2 DoD: cache lookup under 5 ms, measured {per_lookup:?}"
    );
}

/// FR-51: stepping through a passage follows the cache, crosses chapters,
/// steps off a range from its ends, and stops at the ends of a book.
#[test]
fn a_verse_has_a_next_and_a_previous_in_its_book() {
    use sermonai_lib::bible::cache::neighbour;
    use sermonai_lib::bible::reference::parse;

    let data_dir = temp_data_dir("neighbour");
    let mut conn = db::init(&data_dir).expect("db");
    seed_bundled(&mut conn, &assets_dir()).expect("seed");

    let step = |reference: &str, delta: i32| {
        neighbour(&conn, "KJV", &parse(reference).unwrap(), delta)
            .unwrap()
            .map(|r| r.passage_id())
    };

    assert_eq!(step("John 3:16", 1).as_deref(), Some("JHN.3.17"));
    assert_eq!(step("John 3:16", -1).as_deref(), Some("JHN.3.15"));
    // Chapter boundaries are crossed: John 3 has 36 verses.
    assert_eq!(step("John 3:36", 1).as_deref(), Some("JHN.4.1"));
    assert_eq!(step("John 4:1", -1).as_deref(), Some("JHN.3.36"));
    // A range steps from its last verse forward and its first verse back.
    assert_eq!(step("Psalm 139:13-16", 1).as_deref(), Some("PSA.139.17"));
    assert_eq!(step("Psalm 139:13-16", -1).as_deref(), Some("PSA.139.12"));
    // The ends of a book are the ends.
    assert_eq!(step("Genesis 1:1", -1), None);
    assert_eq!(step("Revelation 22:21", 1), None);
    assert_eq!(step("Jude 1:25", 1), None, "a one-chapter book ends too");
}
