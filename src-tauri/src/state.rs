//! Process-wide state, registered with Tauri's manager at setup and reachable
//! from any command via `tauri::State<AppState>`.

use std::sync::Mutex;

use rusqlite::Connection;

use crate::audio::capture::CaptureHandle;
use crate::bible::youversion::YouVersionClient;
use crate::packs::PackManager;
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
    /// Online scripture. Disabled when YVP_APP_KEY is absent; the cache still
    /// serves, so a missing key costs new lookups rather than the service.
    pub bible: YouVersionClient,
    /// The running capture, if any. Holding it here is what keeps it alive:
    /// dropping the handle stops the device and joins its thread, so replacing
    /// this releases the old input before the new one is opened.
    pub capture: Mutex<Option<CaptureHandle>>,
}

impl AppState {
    pub fn new(db: Connection, packs: PackManager, bible: YouVersionClient) -> Self {
        Self {
            db: Mutex::new(db),
            packs,
            outputs: Mutex::new(OutputAssignments::default()),
            bible,
            capture: Mutex::new(None),
        }
    }
}
