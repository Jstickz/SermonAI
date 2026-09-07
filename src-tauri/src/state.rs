//! Process-wide state, registered with Tauri's manager at setup and reachable
//! from any command via `tauri::State<AppState>`.

use std::sync::Mutex;

use rusqlite::Connection;

/// The single SQLite connection for the app data directory.
///
/// One connection behind a mutex is deliberate: SQLite in WAL mode serializes
/// writers anyway, and a service produces a modest write rate (a transcript
/// segment every few hundred ms). If read contention ever shows up during a
/// service, this is the place to introduce a pool.
pub struct AppState {
    pub db: Mutex<Connection>,
}

impl AppState {
    pub fn new(db: Connection) -> Self {
        Self { db: Mutex::new(db) }
    }
}
