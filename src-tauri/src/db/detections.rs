//! Every detection and every operator decision, written to
//! `detected_scriptures` (M2 deliverable 8, PRD §14.2).
//!
//! A row is written when a candidate is **confirmed** — provisional cards are
//! revisions in flight (PRD §18.1) and a withdrawn one was never a detection.
//! The operator's Accept or Reject updates the same row; an Edit writes a new
//! row with source `manual`, since the operator has named a different verse.
//! `went_live` and `displayed_at` belong to staging (M3) and stay unset here.

use rusqlite::{params, Connection};

use crate::bible::reference;
use crate::db::models::{DetectedScripture, DetectionSource};
use crate::detection::pipeline::Candidate;
use crate::error::{Error, Result};

fn source_text(source: DetectionSource) -> &'static str {
    match source {
        DetectionSource::Regex => "regex",
        DetectionSource::Vector => "vector",
        DetectionSource::Llm => "llm",
        DetectionSource::Manual => "manual",
    }
}

fn source_from(text: &str) -> DetectionSource {
    match text {
        "regex" => DetectionSource::Regex,
        "vector" => DetectionSource::Vector,
        "llm" => DetectionSource::Llm,
        _ => DetectionSource::Manual,
    }
}

/// Write a confirmed candidate. Returns the row id, which the caller keeps
/// against the candidate's id so a later decision finds the row.
pub fn record(
    conn: &Connection,
    sermon_id: i64,
    candidate: &Candidate,
    translation: &str,
) -> Result<i64> {
    let reference =
        reference::parse(&candidate.passage_id).map_err(|e| Error::Detection(e.to_string()))?;
    conn.execute(
        "INSERT INTO detected_scriptures
           (sermon_id, book, chapter, verse, end_verse, translation, confidence, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            sermon_id,
            reference.code(),
            reference.chapter,
            reference.verse,
            reference.end_verse,
            translation,
            f64::from(candidate.confidence),
            source_text(candidate.source),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// The operator accepted (`true`) or rejected (`false`) the detection.
pub fn mark_accepted(conn: &Connection, row_id: i64, accepted: bool) -> Result<()> {
    conn.execute(
        "UPDATE detected_scriptures SET accepted_by_operator = ?2 WHERE id = ?1",
        params![row_id, accepted],
    )?;
    Ok(())
}

/// Everything recorded for a service, in detection order.
pub fn list(conn: &Connection, sermon_id: i64) -> Result<Vec<DetectedScripture>> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, sermon_id, segment_id, book, chapter, verse, end_verse, translation,
                confidence, source, COALESCE(accepted_by_operator, 0), COALESCE(went_live, 0)
         FROM detected_scriptures WHERE sermon_id = ?1 ORDER BY id",
    )?;
    let rows = stmt
        .query_map(params![sermon_id], |row| {
            let source: Option<String> = row.get(9)?;
            Ok(DetectedScripture {
                id: row.get(0)?,
                sermon_id: row.get(1)?,
                segment_id: row.get(2)?,
                book: row.get(3)?,
                chapter: row.get(4)?,
                verse: row.get(5)?,
                end_verse: row.get(6)?,
                translation: row.get(7)?,
                confidence: row.get(8)?,
                source: source_from(source.as_deref().unwrap_or("manual")),
                accepted_by_operator: row.get(10)?,
                went_live: row.get(11)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}
