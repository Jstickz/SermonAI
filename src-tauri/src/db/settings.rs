//! The `settings` table: one string per key (PRD §14.2). Small operator
//! choices that must survive a restart — the projector layout, the default
//! translation — and nothing secret: credentials live in the keychain
//! (PRD §17), never here.

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::Result;

pub fn get(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
