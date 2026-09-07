//! Local SQLite store: one file, WAL mode, FTS5 enabled (PRD §14.1).
//! Everything stays on the machine; there is no cloud in v1.0.

pub mod models;

use std::path::Path;

use rusqlite::Connection;

use crate::error::Result;

const MIGRATIONS: &[(&str, &str)] =
    &[("0001_init", include_str!("../../migrations/0001_init.sql"))];

/// Open (creating if needed) the database in the app data directory and apply
/// any migrations that have not run yet.
pub fn init(data_dir: &Path) -> Result<Connection> {
    let db_dir = data_dir.join("db");
    std::fs::create_dir_all(&db_dir)?;

    let conn = Connection::open(db_dir.join("sermonai.sqlite"))?;

    // WAL keeps transcript writes off the read path during a service (PRD §9.2).
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            name       TEXT PRIMARY KEY,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    for (name, sql) in MIGRATIONS {
        let already: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE name = ?1)",
            [name],
            |row| row.get(0),
        )?;
        if already {
            continue;
        }

        tracing::info!(migration = name, "applying migration");
        conn.execute_batch(sql)?;
        conn.execute("INSERT INTO schema_migrations (name) VALUES (?1)", [name])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_once_and_are_idempotent() {
        let dir = std::env::temp_dir().join(format!("sermonai-test-{}", std::process::id()));
        let conn = init(&dir).expect("first init");
        drop(conn);

        // A second launch must not re-run migrations or fail.
        let conn = init(&dir).expect("second init");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, MIGRATIONS.len() as i64);

        std::fs::remove_dir_all(&dir).ok();
    }
}
