//! Staging and outputs. Nothing reaches an output without passing through the
//! staging slot first, unless the operator has opted into auto-live
//! (FR-50, PRD §10.2).
//!
//! Planned files:
//!   staging.rs      the staged/live pair and the Go Live transition (M3)
//!   obs_overlay.rs  transparent overlay on localhost:8001 (FR-28, M7)
//!   ndi.rs          "SermonAI Scripture" source with alpha, optional pack (FR-27, M7)
//!   watermark.rs    free-tier watermark logic (FR-49, M7)

/// Loopback port for the OBS Browser Source overlay (PRD §15.7).
pub const OBS_OVERLAY_PORT: u16 = 8001;

/// Default projector transition length (FR-25, Branding §7.2 motion.verse-in).
pub const VERSE_TRANSITION_MS: u32 = 400;
