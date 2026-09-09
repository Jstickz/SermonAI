//! Three-stage scripture detection, run in order (PRD §10.2, §18.1):
//!
//!   1. regex.rs    direct references — standard, spoken, shorthand (FR-12, FR-13), under 5 ms
//!   2. vector.rs   brute-force cosine over the int8 verse index (FR-15), under 5 ms
//!   3. llm.rs      Claude paraphrase pass, online only, after 10 s with no regex hit (FR-14), 1 to 2 s
//!
//! pipeline.rs sequences them, scores confidence (FR-16) and queues results so
//! that rapid-fire references are never dropped (FR-17).
//!
//! Milestone: M2, except the vector stage's assets and search, built in M0.

pub mod vector;

/// Fire the LLM stage only after this long without a direct hit (FR-14).
pub const PARAPHRASE_IDLE_SECS: u64 = 10;

/// Verses in the protestant canon — the row count of the bundled index.
pub const VERSE_COUNT: usize = 31_102;

/// Dimensions of the bundled static encoder, quantized to int8.
///
/// PRD §15.4 said 384; the model chosen in M0 is 256, which is why the index
/// came in at 7.71 MB rather than the 12 MB the PRD budgeted.
pub const EMBEDDING_DIMS: usize = 256;
