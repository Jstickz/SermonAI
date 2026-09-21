//! Local SQLite store: one file, WAL mode, FTS5 enabled (PRD §14.1).
//! Everything stays on the machine; there is no cloud in v1.0.

pub mod models;

use std::path::Path;

use rusqlite::Connection;

use crate::error::Result;

const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("../../migrations/0001_init.sql")),
    (
        "0002_youversion",
        include_str!("../../migrations/0002_youversion.sql"),
    ),
];

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

    /// The v2.2 rename and the attribution columns.
    ///
    /// Attribution is a licensing obligation, not a nicety: YouVersion's terms
    /// require it wherever the text is shown, so the columns must exist and
    /// existing rows must be marked stale rather than silently treated as
    /// attributed.
    #[test]
    fn youversion_migration_renames_tables_and_adds_attribution() {
        let dir = std::env::temp_dir().join(format!("sermonai-schema-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let conn = init(&dir).expect("init");

        let has_table = |name: &str| -> bool {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [name],
                |row| row.get::<_, bool>(0),
            )
            .unwrap()
        };

        assert!(has_table("bible_cache"), "bible_verses should be renamed");
        assert!(
            has_table("bible_versions"),
            "translations should be renamed"
        );
        assert!(!has_table("bible_verses"), "old name should be gone");
        assert!(!has_table("translations"), "old name should be gone");

        let columns = |table: &str| -> Vec<String> {
            let mut stmt = conn
                .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
                .unwrap();
            let names = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap();
            names
        };

        let cache = columns("bible_cache");
        for column in ["attribution", "attribution_updated_at", "html"] {
            assert!(
                cache.contains(&column.to_string()),
                "bible_cache missing {column}"
            );
        }

        let versions = columns("bible_versions");
        for column in [
            "yvp_version_id",
            "attribution",
            "license_status",
            "last_verified_at",
        ] {
            assert!(
                versions.contains(&column.to_string()),
                "bible_versions missing {column}"
            );
        }
        assert!(
            !versions.contains(&"api_bible_id".to_string()),
            "api_bible_id should be dropped with the vendor"
        );

        // A row cached before the migration must read as needing a refresh.
        conn.execute(
            "INSERT INTO bible_cache (translation_code, book_id, chapter, verse, text)
             VALUES ('KJV', 'JHN', 3, 16, 'For God so loved the world')",
            [],
        )
        .unwrap();
        let (attribution, updated): (String, i64) = conn
            .query_row(
                "SELECT attribution, attribution_updated_at FROM bible_cache LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            attribution, "",
            "no attribution until YouVersion supplies one"
        );
        assert_eq!(updated, 0, "zero marks the row as needing a refresh");

        std::fs::remove_dir_all(&dir).ok();
    }
}
