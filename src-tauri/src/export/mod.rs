//! Exports. Every artifact written to disk is also recorded in the `exports`
//! table with a version, so an older PDF stays downloadable after a regenerate
//! (FR-44).
//!
//! Planned files:
//!   transcript_pdf.rs  single sermon transcript as PDF (FR-36, M9)
//!   docx.rs            transcript and derivatives via `docx-rs` (FR-37, M9)
//!   book.rs            series-to-book compiler via `typst` (FR-39, M10)
//!   slides.rs          PPTX from an outline or summary (FR-58, M10)
//!
//! Default export folder is `Documents/SermonAI/`, with an optional auto-save
//! folder for summary PDFs (PRD §14.3).
