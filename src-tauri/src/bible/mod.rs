//! Bible text: a local `rusqlite` cache seeded from bundled packs, API.Bible
//! downloads, and church-supplied imports (PRD §8.4).
//!
//! Files:
//!   cache.rs      the verse cache: seeded from bundled packs at first start
//!                 with a SHA-256 and verse-count check (FR-19, FR-22), looked
//!                 up by USFM, filled from YouVersion on a miss (FR-18). M2.
//!   youversion.rs the licensed source (FR-18)
//!   reference.rs  spoken or written references to USFM ids
//!   sanitize.rs   vendor markup to projector text
//!   books.rs      the 66-book canon and chapter counts
//! Planned:
//!   importer.rs   USFM, OSIS, JSON and CSV parsers (FR-47, M6)
//!
//! KJV, WEB and ASV ship inside the binary; everything else is a pack.

/// Translations bundled in the base installer (FR-59).
pub const BUNDLED_TRANSLATIONS: [&str; 3] = ["KJV", "WEB", "ASV"];

pub mod books;
pub mod cache;

pub mod reference;

pub mod sanitize;

pub mod youversion;
