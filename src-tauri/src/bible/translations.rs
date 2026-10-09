//! The translation picker's two questions (FR-32, PRD §13.4, M2 deliverable 7):
//! which translations can a card's text be shown in, and which one is the
//! default for this service.
//!
//! The list is what the cache knows — `bible_versions`, seeded from the
//! bundled packs and added to as YouVersion versions are licensed — so the
//! picker never offers a version the app cannot fetch. The default is one
//! row in `settings`, read at startup into `AppState::default_translation`
//! and written here when the operator picks; the picker switches which
//! translation the *next* card is fetched in and leaves the cards already on
//! screen as they are (FR-32: "without disrupting the current display").

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The key in `settings` the default lives under.
const SETTING_KEY: &str = "default_translation";

/// The translation shipped as the default until the operator picks another.
/// KJV because it is bundled, public domain and the index's own text; the
/// Parked note about NIV as default stands until YouVersion access exists.
pub const BUILT_IN_DEFAULT: &str = "KJV";

/// One entry in the picker. Mirrors `TranslationChoice` in `src/lib/types.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationChoice {
    /// The code the cache keys on, e.g. `KJV`, `NIV`.
    pub code: String,
    pub name: String,
    pub language: String,
    /// `true` when the whole text is on disk (a bundled pack). A version
    /// fetched verse by verse from YouVersion is listed but not cached, and
    /// the picker marks it so the operator knows a lookup goes online.
    pub cached: bool,
    /// `bundled` or `youversion`.
    pub source: String,
}

/// Every translation the cache can serve, bundled first, then by name.
pub fn list(conn: &Connection) -> Result<Vec<TranslationChoice>> {
    let mut stmt = conn.prepare_cached(
        "SELECT code, full_name, language, source FROM bible_versions
         WHERE license_status IN ('public_domain', 'approved')
         ORDER BY CASE source WHEN 'bundled' THEN 0 ELSE 1 END, full_name",
    )?;
    let rows = stmt
        .query_map([], |row| {
            let source: String = row.get(3)?;
            Ok(TranslationChoice {
                code: row.get(0)?,
                name: row.get(1)?,
                language: row.get(2)?,
                cached: source == "bundled",
                source,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// The persisted default, if the operator has ever picked one.
pub fn default(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![SETTING_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

/// Persist the default. Refuses a code the cache does not list, because a
/// default that cannot be fetched would make every card show "no cached
/// text" with nothing to say why.
pub fn set_default(conn: &Connection, code: &str) -> Result<()> {
    let known = list(conn)?.into_iter().any(|t| t.code == code);
    if !known {
        return Err(Error::Bible(format!(
            "{code} is not an available translation. Pick one from the list."
        )));
    }
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![SETTING_KEY, code],
    )?;
    Ok(())
}
