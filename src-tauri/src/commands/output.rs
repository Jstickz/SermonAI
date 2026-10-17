//! Staging and output commands (FR-50, FR-33, PRD §13.5).
//!
//! Thin: each one changes [`Staging`] under its lock and broadcasts the
//! resulting state as `output:changed` to every window. The projector and
//! the stage monitor render from that event; the operator's panel mirrors
//! it. Nothing here decides what may be staged — a provisional card has no
//! Stage button (§13.5), so by the time a verse arrives here the operator
//! has chosen it. The one verse that arrives without the operator is
//! [`auto_live`]'s, and `staging::auto_live_qualifies` decided that.

use std::time::Instant;

use tauri::{AppHandle, Emitter, State};

use crate::bible::{cache, reference};
use crate::db::{models::Verse, settings};
use crate::error::{Error, Result};
use crate::output::staging::{OutputState, ProjectorLayout, ProjectorTransition};
use crate::state::AppState;

const EVENT: &str = "output:changed";
/// The `settings` key the chosen layout is kept under.
pub const LAYOUT_SETTING: &str = "projector_layout";
/// The `settings` key auto-live is kept under: "true" or "false".
pub const AUTO_LIVE_SETTING: &str = "auto_live";
/// The `settings` keys the transition and its length are kept under.
pub const TRANSITION_SETTING: &str = "projector_transition";
pub const TRANSITION_MS_SETTING: &str = "projector_transition_ms";

/// Wall-clock milliseconds, the same clock the projector's `Date.now()` reads.
fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

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
        .stage(verse)?
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
        .go_live(unix_ms())?
        .clone();
    let locked = started.elapsed();
    broadcast(&app, &next);
    // Split so a slow line names its part: the lock wait is ours, the
    // broadcast is the windows' (17 Oct: one auto-live broadcast took 1.5 s
    // with the state set in 4 ms, and this line could not say which).
    tracing::info!(
        reference = %next.live.as_ref().map(|v| v.reference.as_str()).unwrap_or(""),
        staged_to_broadcast_us = started.elapsed().as_micros() as u64,
        lock_us = locked.as_micros() as u64,
        broadcast_us = (started.elapsed() - locked).as_micros() as u64,
        "went live"
    );
    Ok(next)
}

/// Next (`delta` 1) or previous (`delta` -1) verse of the passage on the
/// projector (FR-51, M3 deliverable 6). The neighbour comes from the cache
/// in the live verse's own translation; the live slot is replaced directly
/// (see `Staging::step_live`). Errors name the verse and the end it hit.
#[tauri::command]
pub fn step_verse(app: AppHandle, state: State<'_, AppState>, delta: i32) -> Result<OutputState> {
    if delta != 1 && delta != -1 {
        return Err(Error::Output(format!(
            "A step is one verse at a time; {delta} is not a step."
        )));
    }
    let live = state
        .staging
        .lock()
        .expect("staging lock")
        .state()
        .live
        .clone()
        .ok_or_else(|| Error::Output("Nothing is live. Send a verse live first.".to_string()))?;
    let parsed = reference::parse(&live.reference).map_err(|e| {
        Error::Output(format!(
            "{} cannot be stepped from: {e}. Stage the verse you want instead.",
            live.reference
        ))
    })?;
    let next = {
        let db = state.db.lock().expect("db lock");
        let Some(next_ref) = cache::neighbour(&db, &live.translation, &parsed, delta)? else {
            let end = if delta > 0 { "last" } else { "first" };
            return Err(Error::Output(format!(
                "{} is the {end} verse of {} in the {}. Stage another verse to continue.",
                live.reference,
                parsed.book_name(),
                live.translation
            )));
        };
        cache::lookup_ref(&db, &live.translation, &next_ref)?
            .map(|c| c.verse)
            .ok_or_else(|| {
                Error::Output(format!(
                    "{} is not cached in the {}. Stage it from the picker to fetch it.",
                    next_ref.display(),
                    live.translation
                ))
            })?
    };
    let started = Instant::now();
    let state_after = state
        .staging
        .lock()
        .expect("staging lock")
        .step_live(next, unix_ms())?
        .clone();
    broadcast(&app, &state_after);
    tracing::info!(
        reference = %state_after.live.as_ref().map(|v| v.reference.as_str()).unwrap_or(""),
        delta,
        staged_to_broadcast_us = started.elapsed().as_micros() as u64,
        "stepped the live verse"
    );
    Ok(state_after)
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

/// Choose what the projector shows beside the verse (PRD §13.6, FR-26 basic).
/// Persisted, and applied to the verse that is live now.
#[tauri::command]
pub fn set_projector_layout(
    app: AppHandle,
    state: State<'_, AppState>,
    layout: String,
) -> Result<OutputState> {
    let parsed = ProjectorLayout::parse(&layout).ok_or_else(|| {
        Error::Output(format!(
            "{layout} is not a projector layout. Choose verse only, verse and reference, or verse, reference and translation."
        ))
    })?;
    {
        let db = state.db.lock().expect("db lock");
        settings::set(&db, LAYOUT_SETTING, parsed.as_str())?;
    }
    let next = state
        .staging
        .lock()
        .expect("staging lock")
        .set_layout(parsed)
        .clone();
    tracing::info!(layout = parsed.as_str(), "projector layout set");
    broadcast(&app, &next);
    Ok(next)
}

/// Choose how verses change on the projector and how long it takes (FR-25).
/// Persisted; the next go-live uses it.
#[tauri::command]
pub fn set_projector_transition(
    app: AppHandle,
    state: State<'_, AppState>,
    transition: String,
    duration_ms: u32,
) -> Result<OutputState> {
    let parsed = ProjectorTransition::parse(&transition).ok_or_else(|| {
        Error::Output(format!(
            "{transition} is not a transition. Choose fade, slide or dissolve."
        ))
    })?;
    let next = state
        .staging
        .lock()
        .expect("staging lock")
        .set_transition(parsed, duration_ms)?
        .clone();
    {
        let db = state.db.lock().expect("db lock");
        settings::set(&db, TRANSITION_SETTING, parsed.as_str())?;
        settings::set(&db, TRANSITION_MS_SETTING, &duration_ms.to_string())?;
    }
    tracing::info!(
        transition = parsed.as_str(),
        duration_ms,
        "projector transition set"
    );
    broadcast(&app, &next);
    Ok(next)
}

/// Turn auto-live on or off (PRD §13.5, M3 deliverable 2). Persisted, and
/// broadcast at once so the indicator in every window changes with it.
#[tauri::command]
pub fn set_auto_live(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<OutputState> {
    {
        let db = state.db.lock().expect("db lock");
        settings::set(
            &db,
            AUTO_LIVE_SETTING,
            if enabled { "true" } else { "false" },
        )?;
    }
    let next = state
        .staging
        .lock()
        .expect("staging lock")
        .set_auto_live(enabled)
        .clone();
    tracing::info!(enabled, "auto-live set");
    broadcast(&app, &next);
    Ok(next)
}

/// Auto-live: a confirmed card above the line goes straight to the
/// projector. Called by the card emitter, which has already asked
/// `auto_live_qualifies`; this only checks the switch, under the same lock
/// the operator's commands take, so a card and a Go Live cannot interleave.
/// Returns whether it went live. Timed to the broadcast like `go_live`; the
/// projector's paint report covers the rest.
pub fn auto_live(app: &AppHandle, state: &AppState, verse: Verse) -> bool {
    let started = Instant::now();
    let mut staging = state.staging.lock().expect("staging lock");
    if !staging.auto_live() {
        return false;
    }
    let next = match staging.auto_live_verse(verse, unix_ms()) {
        Ok(next) => next.clone(),
        Err(err) => {
            tracing::warn!(%err, "auto-live refused");
            return false;
        }
    };
    drop(staging);
    let locked = started.elapsed();
    broadcast(app, &next);
    tracing::info!(
        reference = %next.live.as_ref().map(|v| v.reference.as_str()).unwrap_or(""),
        staged_to_broadcast_us = started.elapsed().as_micros() as u64,
        lock_us = locked.as_micros() as u64,
        broadcast_us = (started.elapsed() - locked).as_micros() as u64,
        "went live by auto-live"
    );
    true
}

/// The projector window reports when the live verse was painted, against
/// the time it went live, so staged → live is measured to the thing itself
/// (M3 DoD: under 100 ms) rather than to our broadcast.
#[tauri::command]
pub fn projector_painted(live_since_ms: u64, painted_at_ms: u64, settle_ms: u32) {
    tracing::info!(
        staged_to_paint_ms = painted_at_ms.saturating_sub(live_since_ms),
        budget_ms = 100,
        transition_settles_after_ms = settle_ms,
        "projector painted the live verse"
    );
}
