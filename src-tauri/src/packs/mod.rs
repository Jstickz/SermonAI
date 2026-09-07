//! On-demand packs. The base installer stays under 40 MB by shipping only the
//! app, KJV/WEB/ASV, the verse index and the default theme; everything heavier
//! is downloaded from the Pack CDN on request (FR-59 to FR-61).
//!
//! Planned files:
//!   manifest.rs    fetch and verify the signed packs-manifest.json (M0)
//!   downloader.rs  ranged, resumable downloads with SHA-256 verification (M0)
//!   registry.rs    what is installed, what it cost in disk, removal (M0)
//!
//! A download interrupted by a crash or a network drop resumes from the last
//! verified chunk on next launch, and a partial pack is never activated
//! (PRD §10.6).

/// Size of each ranged request; also the resume granularity.
pub const CHUNK_BYTES: u64 = 4 * 1024 * 1024;

/// CI fails the build if either installer exceeds this (FR-59, PRD §19).
pub const MAX_INSTALLER_BYTES: u64 = 40 * 1024 * 1024;
