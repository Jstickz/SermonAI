//! Speech-to-text. Deepgram Nova-3 over WebSocket online (FR-07); whisper.cpp
//! via `whisper-rs` offline once the Offline Speech Pack is installed (FR-08).
//!
//! Planned files:
//!   deepgram.rs  streaming client, custom vocabulary, reconnect backoff (M1)
//!   whisper.rs   local model loaded from the speech pack (M5)
//!   router.rs    switches to local after three Deepgram failures (M5)
//!   buffer.rs    rolling 60-second transcript buffer for paraphrase analysis (FR-11)
//!
//! The 66 Bible book names and common archaic preaching terms are sent as
//! custom vocabulary on every stream (FR-09).

/// How long the rolling buffer holds transcript for the paraphrase stage.
pub const ROLLING_BUFFER_SECS: u64 = 60;
