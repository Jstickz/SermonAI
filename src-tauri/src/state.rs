//! Process-wide state, registered with Tauri's manager at setup and reachable
//! from any command via `tauri::State<AppState>`.

use std::sync::Mutex;

use rusqlite::Connection;

use crate::audio::capture::CaptureHandle;
use crate::bible::youversion::YouVersionClient;
use crate::credentials::Credentials;
use crate::packs::PackManager;
use crate::stt::deepgram::DeepgramSession;
use crate::windows::OutputAssignments;

/// Long-lived services shared by every command.
pub struct AppState {
    /// The single SQLite connection for the app data directory.
    ///
    /// One connection behind a mutex is deliberate: SQLite in WAL mode
    /// serializes writers anyway, and a service produces a modest write rate (a
    /// transcript segment every few hundred ms). If read contention shows up
    /// during a service, this is the place to introduce a pool.
    pub db: Mutex<Connection>,
    pub packs: PackManager,
    /// Which display each output window is on. The backend owns this because
    /// the operator UI unmounts when the operator switches tabs, and a
    /// remounted panel must be able to ask what is actually on screen rather
    /// than guess.
    pub outputs: Mutex<OutputAssignments>,
    /// Online scripture. Disabled when no credential is available; the cache
    /// still serves, so that costs new lookups rather than the service.
    pub bible: YouVersionClient,
    /// The one source of vendor credentials (PRD §17). Service clients ask
    /// this rather than the environment, so the gateway swaps in behind them.
    pub credentials: Credentials,
    /// The running capture, if any. Holding it here is what keeps it alive:
    /// dropping the handle stops the device and joins its thread, so replacing
    /// this releases the old input before the new one is opened.
    pub capture: Mutex<Option<CaptureHandle>>,
    /// The live transcription stream, when capture was started with
    /// transcription on. Owned rather than shared: stopping consumes it to
    /// send `CloseStream` and collect the final results.
    pub transcript: Mutex<Option<DeepgramSession>>,
}

impl AppState {
    pub fn new(
        db: Connection,
        packs: PackManager,
        bible: YouVersionClient,
        credentials: Credentials,
    ) -> Self {
        Self {
            db: Mutex::new(db),
            packs,
            outputs: Mutex::new(OutputAssignments::default()),
            bible,
            credentials,
            capture: Mutex::new(None),
            transcript: Mutex::new(None),
        }
    }
}
