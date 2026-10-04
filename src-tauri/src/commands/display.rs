//! Display selection commands (M0 deliverable 3, PRD §8.5 FR-23, FR-24).

use tauri::{AppHandle, State};

use crate::error::Result;
use crate::state::AppState;
use crate::windows::{self, MonitorInfo, OutputAssignments, ALTERNATE_LABEL, PROJECTOR_LABEL};

/// The operator window has painted its first frame.
///
/// Called once by the operator window's entry script. The elapsed time since
/// `run()` began is the cold start PRD §9.1 budgets at one second, measured to
/// the operator seeing the app rather than to the end of our setup, which
/// finishes well before WebView2 has rendered anything. The same log line is
/// what `scripts/measure-cold-start.ps1` and the CI smoke job read.
#[tauri::command]
pub fn operator_window_ready(state: State<'_, AppState>) {
    use std::sync::atomic::Ordering;
    if state
        .operator_ready_logged
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        tracing::info!(
            elapsed_ms = state.started.elapsed().as_millis() as u64,
            "operator window ready"
        );
    }
}

#[tauri::command]
pub fn list_monitors(app: AppHandle) -> Result<Vec<MonitorInfo>> {
    windows::list_monitors(&app)
}

/// Which display each output is currently on.
///
/// The operator panel calls this on mount: it unmounts whenever the operator
/// switches tabs, so it cannot keep the assignment in component state.
#[tauri::command]
pub fn get_output_assignments(state: State<'_, AppState>) -> OutputAssignments {
    state.outputs.lock().expect("outputs lock").clone()
}

/// Send the projector output to a display. `None` hides it again.
#[tauri::command]
pub fn set_projector_monitor(
    app: AppHandle,
    state: State<'_, AppState>,
    monitor_name: Option<String>,
) -> Result<OutputAssignments> {
    match &monitor_name {
        Some(name) => windows::place_on_monitor(&app, PROJECTOR_LABEL, Some(name))?,
        None => windows::hide_output(&app, PROJECTOR_LABEL)?,
    }

    let mut outputs = state.outputs.lock().expect("outputs lock");
    outputs.projector = monitor_name;
    Ok(outputs.clone())
}

/// Send the stage confidence monitor to a display. `None` hides it again.
#[tauri::command]
pub fn set_alternate_monitor(
    app: AppHandle,
    state: State<'_, AppState>,
    monitor_name: Option<String>,
) -> Result<OutputAssignments> {
    match &monitor_name {
        Some(name) => windows::place_on_monitor(&app, ALTERNATE_LABEL, Some(name))?,
        None => windows::hide_output(&app, ALTERNATE_LABEL)?,
    }

    let mut outputs = state.outputs.lock().expect("outputs lock");
    outputs.alternate = monitor_name;
    Ok(outputs.clone())
}
