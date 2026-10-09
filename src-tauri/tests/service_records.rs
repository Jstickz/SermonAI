//! What a service leaves in the database (M2 deliverables 7 and 8): the
//! translation default, the `sermons` row, and every confirmed detection
//! with the operator's decision on it.

use std::path::PathBuf;

use sermonai_lib::bible::translations;
use sermonai_lib::db::{self, detections, models::DetectionSource, sermons};
use sermonai_lib::detection::pipeline::Candidate;

fn temp_data_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sermonai-records-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A version row without seeding 93,000 verses: the picker lists versions,
/// not text.
fn add_version(conn: &rusqlite::Connection, code: &str, name: &str, source: &str, status: &str) {
    conn.execute(
        "INSERT INTO bible_versions (code, full_name, language, source, attribution, license_status, last_verified_at)
         VALUES (?1, ?2, 'eng', ?3, 'test', ?4, 0)",
        rusqlite::params![code, name, source, status],
    )
    .unwrap();
}

fn candidate(id: u64, passage_id: &str, source: DetectionSource, confidence: f32) -> Candidate {
    Candidate {
        id,
        passage_id: passage_id.to_string(),
        reference: passage_id.to_string(),
        source,
        confidence,
        provisional: false,
        evidence: None,
        detected_at_ms: 0,
        verse: None,
    }
}

#[test]
fn the_picker_lists_usable_versions_bundled_first_and_keeps_the_default() {
    let conn = db::init(&temp_data_dir("picker")).unwrap();
    add_version(
        &conn,
        "NIV",
        "New International Version",
        "youversion",
        "approved",
    );
    add_version(
        &conn,
        "KJV",
        "King James Version",
        "bundled",
        "public_domain",
    );
    add_version(&conn, "MSG", "The Message", "youversion", "pending");

    let listed = translations::list(&conn).unwrap();
    let codes: Vec<&str> = listed.iter().map(|t| t.code.as_str()).collect();
    assert_eq!(
        codes,
        ["KJV", "NIV"],
        "bundled first, pending licences left out"
    );
    assert!(listed[0].cached, "a bundled pack is on disk");
    assert!(
        !listed[1].cached,
        "a YouVersion version is fetched verse by verse"
    );

    assert_eq!(
        translations::default(&conn).unwrap(),
        None,
        "nothing picked yet"
    );
    translations::set_default(&conn, "NIV").unwrap();
    assert_eq!(
        translations::default(&conn).unwrap().as_deref(),
        Some("NIV")
    );
    translations::set_default(&conn, "KJV").unwrap();
    assert_eq!(
        translations::default(&conn).unwrap().as_deref(),
        Some("KJV"),
        "overwritten, not duplicated"
    );

    let err = translations::set_default(&conn, "ESV")
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("ESV"),
        "a version the cache cannot serve is refused: {err}"
    );
    assert_eq!(
        translations::default(&conn).unwrap().as_deref(),
        Some("KJV")
    );
}

#[test]
fn a_service_records_its_detections_and_the_operators_decisions() {
    let conn = db::init(&temp_data_dir("detections")).unwrap();

    let sermon_id = sermons::start(&conn, "KJV").unwrap();
    assert!(detections::list(&conn, sermon_id).unwrap().is_empty());

    let direct = detections::record(
        &conn,
        sermon_id,
        &candidate(1, "JER.29.11", DetectionSource::Regex, 0.97),
        "KJV",
    )
    .unwrap();
    let range = detections::record(
        &conn,
        sermon_id,
        &candidate(2, "PSA.23.1-2", DetectionSource::Vector, 0.81),
        "KJV",
    )
    .unwrap();
    detections::mark_accepted(&conn, direct, true).unwrap();
    detections::mark_accepted(&conn, range, false).unwrap();
    // The operator corrected card 2: a new manual row, the original kept.
    detections::record(
        &conn,
        sermon_id,
        &candidate(2, "JHN.3.16", DetectionSource::Manual, 1.0),
        "WEB",
    )
    .unwrap();

    let rows = detections::list(&conn, sermon_id).unwrap();
    assert_eq!(rows.len(), 3);

    assert_eq!(
        (
            rows[0].book.as_str(),
            rows[0].chapter,
            rows[0].verse,
            rows[0].end_verse
        ),
        ("JER", 29, 11, None)
    );
    assert_eq!(rows[0].source, DetectionSource::Regex);
    assert!(rows[0].accepted_by_operator);
    assert!(!rows[0].went_live, "staging is M3");

    assert_eq!(
        (
            rows[1].book.as_str(),
            rows[1].chapter,
            rows[1].verse,
            rows[1].end_verse
        ),
        ("PSA", 23, 1, Some(2))
    );
    assert_eq!(rows[1].source, DetectionSource::Vector);
    assert!(!rows[1].accepted_by_operator, "rejected");
    assert!((rows[1].confidence.unwrap() - 0.81).abs() < 1e-6);

    assert_eq!(rows[2].source, DetectionSource::Manual);
    assert_eq!(
        rows[2].translation, "WEB",
        "recorded in the translation it was shown in"
    );

    sermons::finish(&conn, sermon_id, 2712).unwrap();
    let (status, duration): (String, i64) = conn
        .query_row(
            "SELECT status, duration_seconds FROM sermons WHERE id = ?1",
            rusqlite::params![sermon_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((status.as_str(), duration), ("ready", 2712));
}

#[test]
fn a_candidate_whose_reference_does_not_parse_is_refused_not_written() {
    let conn = db::init(&temp_data_dir("badref")).unwrap();
    let sermon_id = sermons::start(&conn, "KJV").unwrap();
    let err = detections::record(
        &conn,
        sermon_id,
        &candidate(9, "NOT.A.REF", DetectionSource::Llm, 0.5),
        "KJV",
    );
    assert!(err.is_err());
    assert!(detections::list(&conn, sermon_id).unwrap().is_empty());
}
