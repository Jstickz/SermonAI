//! Audio capture via `cpal`: WASAPI on Windows, CoreAudio on macOS.
//! 250 ms chunks of 16-bit PCM mono at 16 kHz (FR-03, PRD §10.2).
//!
//! Milestone: M1. Planned files:
//!   devices.rs   enumeration incl. HDMI capture cards and loopback (FR-01, FR-05)
//!   capture.rs   stream lifecycle: start/stop/pause/resume without restart (FR-06)
//!   meter.rs     30 fps dBFS level meter emitted as `transcript:level` (FR-04)
//!
//! A device disconnect mid-service must pause and let the operator pick
//! another, never crash (PRD §10.6).

/// Capture format. Deepgram and whisper.cpp both take 16 kHz mono PCM, so
/// nothing downstream resamples.
pub const SAMPLE_RATE_HZ: u32 = 16_000;
pub const CHANNELS: u16 = 1;
pub const CHUNK_MS: u32 = 250;
