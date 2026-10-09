//! The `sermons` row a live service writes under (PRD §14.2).
//!
//! Opened when transcription starts and closed when it stops, because
//! `detected_scriptures.sermon_id` is NOT NULL: a detection cannot be
//! persisted (M2 deliverable 8) without a service to belong to. This is the
//! minimum M2 needs. Title, preacher and series come with the service
//! lifecycle in M4 (FR-34), which is also where `status` gains its
//! `processing` and `needs_attention` states; until then a finished service
//! is `ready`.

use rusqlite::{params, Connection};

use crate::error::Result;

/// Open a service. Returns the new `sermons.id`.
pub fn start(conn: &Connection, default_translation: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO sermons (date, default_translation, status)
         VALUES (CURRENT_TIMESTAMP, ?1, 'live')",
        params![default_translation],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Close a service with how long it ran.
pub fn finish(conn: &Connection, sermon_id: i64, duration_seconds: u64) -> Result<()> {
    conn.execute(
        "UPDATE sermons SET duration_seconds = ?2, status = 'ready' WHERE id = ?1",
        params![sermon_id, duration_seconds as i64],
    )?;
    Ok(())
}
