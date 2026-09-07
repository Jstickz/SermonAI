//! Bible text: a local `rusqlite` cache seeded from bundled packs, API.Bible
//! downloads, and church-supplied imports (PRD §8.4).
//!
//! Planned files:
//!   cache.rs      verse lookup and pre-caching into SQLite (FR-19, M2)
//!   api_bible.rs  API.Bible client for the licensed catalog (FR-18, M2/M6)
//!   importer.rs   USFM, OSIS, JSON and CSV parsers (FR-47, M6)
//!   integrity.rs  startup checksum of every installed translation (FR-22)
//!
//! KJV, WEB and ASV ship inside the binary; everything else is a pack.

/// Translations bundled in the base installer (FR-59).
pub const BUNDLED_TRANSLATIONS: [&str; 3] = ["KJV", "WEB", "ASV"];
