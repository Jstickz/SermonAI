//! Staging and output commands (FR-50, FR-33, PRD §13.5).
//!
//! Thin: each one changes [`Staging`] under its lock and broadcasts the
//! resulting state as `output:changed` to every window. The projector and
//! the stage monitor render from that event; the operator's panel mirrors
//! it. Nothing here decides what may be staged — a provisional card has no
//! Stage button (§13.5), so by the time a verse arrives here the operator
//! has chosen it.

use std::time::Instant;

use tauri::{AppHandle, Emitter, State};

use crate::db::models::Verse;
use crate::error::Result;
use crate::output::staging::OutputState;
use crate::state::AppState;

const EVENT: &str = "output:changed";

fn broadcast(app: &AppHandle, state: &OutputState) {
    if let Err(err) = app.emit(EVENT, state) {
        tracing::warn!(%err, "could not broadcast the output state");
    }
}

/// What is staged and live now. The operator window asks on mount, because
/// its panel unmounts on a tab switch and the backend owns the truth.
#[tauri::command]
pub fn get_output_state(state: State<'_, AppState>) -> OutputState {
    state.staging.lock().expect("staging lock").state().clone()
}

/// Put a verse in the staged slot (FR-50).
#[tauri::command]
pub fn stage_verse(
    app: AppHandle,
    state: State<'_, AppState>,
    verse: Verse,
) -> Result<OutputState> {
    let next = state
        .staging
        .lock()
        .expect("staging lock")
        .stage(verse)
        .clone();
    tracing::info!(reference = %next.staged.as_ref().map(|v| v.reference.as_str()).unwrap_or(""), "staged");
    broadcast(&app, &next);
    Ok(next)
}

/// Promote staged to live (FR-50). Timed from the command to the broadcast:
/// the M3 DoD budgets staged → live under 100 ms, and this is our share of
/// it; the projector reports its own paint once deliverable 3 lands.
#[tauri::command]
pub fn go_live(app: AppHandle, state: State<'_, AppState>) -> Result<OutputState> {
    let started = Instant::now();
    let next = state
        .staging
        .lock()
        .expect("staging lock")
        .go_live()?
        .clone();
    broadcast(&app, &next);
    tracing::info!(
        reference = %next.live.as_ref().map(|v| v.reference.as_str()).unwrap_or(""),
        staged_to_broadcast_us = started.elapsed().as_micros() as u64,
        "went live"
    );
    Ok(next)
}

/// Escape: empty the staged slot.
#[tauri::command]
pub fn clear_staged(app: AppHandle, state: State<'_, AppState>) -> Result<OutputState> {
    let next = state
        .staging
        .lock()
        .expect("staging lock")
        .clear_staged()
        .clone();
    broadcast(&app, &next);
    Ok(next)
}

/// Blank or unblank the projector (FR-33). The live verse is kept.
#[tauri::command]
pub fn set_blanked(
    app: AppHandle,
    state: State<'_, AppState>,
    blanked: bool,
) -> Result<OutputState> {
    let next = state
        .staging
        .lock()
        .expect("staging lock")
        .set_blanked(blanked)
        .clone();
    tracing::info!(blanked, "projector blank state changed");
    broadcast(&app, &next);
    Ok(next)
}
