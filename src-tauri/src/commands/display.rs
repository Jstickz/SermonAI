//! Display selection commands (M0 deliverable 3, PRD §8.5 FR-23, FR-24).

use tauri::AppHandle;

use crate::error::Result;
use crate::windows::{self, MonitorInfo, ALTERNATE_LABEL, PROJECTOR_LABEL};

#[tauri::command]
pub fn list_monitors(app: AppHandle) -> Result<Vec<MonitorInfo>> {
    windows::list_monitors(&app)
}

/// Send the projector output to a display. `None` hides it again.
#[tauri::command]
pub fn set_projector_monitor(app: AppHandle, monitor_name: Option<String>) -> Result<()> {
    match monitor_name {
        Some(name) => windows::place_on_monitor(&app, PROJECTOR_LABEL, Some(&name)),
        None => windows::hide_output(&app, PROJECTOR_LABEL),
    }
}

/// Send the stage confidence monitor to a display. `None` hides it again.
#[tauri::command]
pub fn set_alternate_monitor(app: AppHandle, monitor_name: Option<String>) -> Result<()> {
    match monitor_name {
        Some(name) => windows::place_on_monitor(&app, ALTERNATE_LABEL, Some(&name)),
        None => windows::hide_output(&app, ALTERNATE_LABEL),
    }
}
