//! Phone remote: an `axum` server inside the same process, bound to the LAN
//! only while the operator has the remote enabled (FR-52, PRD §15.8).
//!
//! Planned files:
//!   server.rs   axum app serving the remote bundle and the WebSocket (M7)
//!   pairing.rs  QR + 6-digit code, short-lived tokens, expiry at End Service (FR-53)
//!   ws_hub.rs   keeps remote devices and the operator UI in sync (FR-54)
//!   devices.rs  up to 5 concurrent devices, listed and revocable (FR-55)
//!
//! Security: off by default, LAN only, token-based, revocable, and the port
//! must be closed when the remote is disabled — a port scan is part of the M7
//! definition of done (PRD §17).

pub const REMOTE_PORT: u16 = 8002;

/// Concurrent paired devices allowed at once (FR-55).
pub const MAX_DEVICES: usize = 5;
