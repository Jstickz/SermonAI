//! Process-wide state, registered with Tauri's manager at setup and reachable
//! from any command via `tauri::State<AppState>`.

use std::sync::Mutex;

use rusqlite::Connection;

use crate::packs::PackManager;

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
}

impl AppState {
    pub fn new(db: Connection, packs: PackManager) -> Self {
        Self {
            db: Mutex::new(db),
            packs,
        }
    }
}
