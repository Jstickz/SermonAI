//! Sermon intelligence — the product, not a bonus (PRD §5.3).
//!
//! On End Service the transcript is sealed and a detailed summary PDF is
//! generated with no further user action, ready in under 90 seconds
//! (FR-40 to FR-46, pipeline in PRD §10.5).
//!
//! Planned files:
//!   summarizer.rs         seal, chunk, prompt, Claude structured JSON (M4)
//!   template_renderer.rs  the offline default: a full PDF with no model (FR-45, M5)
//!   local_model.rs        optional Offline Intelligence Pack path (M5)
//!   validator.rs          drops any scripture not in the accepted log or the
//!                         transcript, and any quote that is not a transcript
//!                         substring (FR-46) — the anti-hallucination gate
//!   pdf.rs                HTML template printed to PDF in a hidden webview (M4)
//!   search.rs             FTS5 archive search (FR-56, M8)
//!
//! Generation is retried three times with backoff; a failure marks the sermon
//! `needs_attention` and never loses the transcript (PRD §10.5 step 8).

/// p95 budget from End Service to a downloadable PDF (FR-42).
pub const SUMMARY_DEADLINE_SECS: u64 = 90;

pub const MAX_SUMMARY_RETRIES: u32 = 3;

/// Transcript window size for chunked summarization, with overlap (PRD §10.5).
pub const CHUNK_MINUTES: u64 = 8;
