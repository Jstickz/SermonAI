//! Display selection commands (M0 deliverable 3, PRD §8.5 FR-23, FR-24).

use tauri::{AppHandle, State};

use crate::error::Result;
use crate::state::AppState;
use crate::windows::{self, MonitorInfo, OutputAssignments, ALTERNATE_LABEL, PROJECTOR_LABEL};

/// The operator window has painted its first frame.
///
/// Called once by the operator window's entry script, which passes the wall
/// clock reading it took at that frame. The time from `run()` to that reading
/// is the cold start PRD §9.1 budgets at one second, measured to the operator
/// seeing the app rather than to the end of our setup, which finishes well
/// before WebView2 has rendered anything. The reading is taken in the window
/// rather than here because synchronous commands queue on the main thread:
/// a device enumeration ahead of this call would otherwise be charged to the
/// paint. `reported_after_ms` is when the call actually arrived; the gap
/// between the two is that queue. `scripts/measure-cold-start.ps1` and the CI
/// smoke job read the `elapsed_ms` of this line.
///
/// Async so it runs off the main thread and is not itself queued.
#[tauri::command]
pub async fn operator_window_ready(state: State<'_, AppState>, painted_at_ms: f64) -> Result<()> {
    use std::sync::atomic::Ordering;
    if state
        .operator_ready_logged
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        let reported_after_ms = state.started.elapsed().as_millis() as u64;
        let started_wall_ms = state
            .started_wall
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(painted_at_ms);
        // Two clocks: the window's Date.now() and our SystemTime, both wall
        // time, both on this machine. Clamp a negative reading to zero rather
        // than report nonsense if the clock steps between the two.
        let elapsed_ms = (painted_at_ms - started_wall_ms).max(0.0).round() as u64;
        tracing::info!(elapsed_ms, reported_after_ms, "operator window ready");
    }
    Ok(())
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
