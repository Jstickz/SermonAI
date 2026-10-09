//! Detection cards: what reaches the window, and what the operator does to
//! them (PRD §13.3, §13.5).
//!
//! Thin, like every handler here. The pipeline decides what is a candidate;
//! these send candidates to the window with their verse text, write each
//! confirmed one to `detected_scriptures`, and record the operator's Accept,
//! Reject and Edit against that row (M2 deliverable 8).

use tauri::{AppHandle, Emitter, Manager};

use crate::bible::{cache, reference};
use crate::db::{detections, models::DetectionSource};
use crate::detection::pipeline::Candidate;
use crate::error::{Error, Result};
use crate::state::AppState;

/// Write a confirmed card under the running service and remember its row.
/// Without a service (a Settings device test, say) there is nothing to write
/// under, and that is not an error.
fn persist(state: &AppState, card: &Candidate, translation: &str) {
    let Some((sermon_id, _)) = *state.current_sermon.lock().expect("sermon lock") else {
        return;
    };
    let db = state.db.lock().expect("db lock");
    match detections::record(&db, sermon_id, card, translation) {
        Ok(row_id) => {
            state
                .detection_rows
                .lock()
                .expect("rows lock")
                .insert(card.id, row_id);
        }
        Err(err) => tracing::warn!(%err, id = card.id, "could not record a detection"),
    }
}

/// Apply an operator decision to the card's row, if it has one.
fn decide(state: &AppState, id: u64, accepted: bool) {
    let row = state
        .detection_rows
        .lock()
        .expect("rows lock")
        .get(&id)
        .copied();
    let Some(row_id) = row else {
        return;
    };
    let db = state.db.lock().expect("db lock");
    if let Err(err) = detections::mark_accepted(&db, row_id, accepted) {
        tracing::warn!(%err, id, "could not record the operator's decision");
    }
}

/// Milliseconds since the Unix epoch, for `detectedAtMs`.
pub fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Send cards to the window, filling in verse text for confirmed ones.
///
/// A provisional card gets no text: it is not projectable, so fetching for it
/// would be spend without display, and the reference is enough to warn the
/// operator something is coming. A confirmed card whose verse is not cached
/// still goes, with `verse: None`; the card says so rather than waiting.
pub fn emit_cards(app: &AppHandle, cards: Vec<Candidate>) {
    if cards.is_empty() {
        return;
    }
    let state = app.state::<AppState>();
    let translation = state
        .default_translation
        .lock()
        .expect("translation lock")
        .clone();

    for mut card in cards {
        if !card.provisional && card.verse.is_none() {
            let db = state.db.lock().expect("db lock");
            match cache::lookup(&db, &translation, &card.passage_id) {
                Ok(Some(cached)) => card.verse = Some(cached.verse),
                Ok(None) => tracing::info!(
                    passage = %card.passage_id,
                    %translation,
                    "no cached text for a confirmed candidate; the card shows the reference only"
                ),
                Err(err) => tracing::warn!(%err, passage = %card.passage_id, "verse lookup failed"),
            }
        }
        if !card.provisional {
            persist(&state, &card, &translation);
        }
        let _ = app.emit("detection:new", &card);
    }
}

/// Tell the window which provisional cards the settled text did not support.
pub fn emit_withdrawals(app: &AppHandle, ids: &[u64]) {
    for id in ids {
        let _ = app.emit("detection:withdraw", serde_json::json!({ "id": id }));
    }
}

/// The operator accepted a card. Staging it is M3; here it is recorded.
#[tauri::command]
pub fn accept_detection(
    state: tauri::State<'_, AppState>,
    id: u64,
    passage_id: String,
) -> Result<()> {
    tracing::info!(id, passage = %passage_id, "operator accepted a detection");
    decide(&state, id, true);
    Ok(())
}

#[tauri::command]
pub fn reject_detection(
    state: tauri::State<'_, AppState>,
    id: u64,
    passage_id: String,
) -> Result<()> {
    tracing::info!(id, passage = %passage_id, "operator rejected a detection");
    decide(&state, id, false);
    Ok(())
}

/// The operator corrected a card's reference. The card keeps its id and comes
/// back as a manual detection with full confidence and the new verse text.
#[tauri::command]
pub fn edit_detection(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: u64,
    reference: String,
) -> Result<Candidate> {
    let parsed = reference::parse(&reference).map_err(|e| Error::Detection(e.to_string()))?;
    let translation = state
        .default_translation
        .lock()
        .expect("translation lock")
        .clone();
    let verse = {
        let db = state.db.lock().expect("db lock");
        cache::lookup_ref(&db, &translation, &parsed)?.map(|c| c.verse)
    };

    let card = Candidate {
        id,
        passage_id: parsed.passage_id(),
        reference: parsed.display(),
        source: DetectionSource::Manual,
        confidence: 1.0,
        provisional: false,
        evidence: None,
        detected_at_ms: unix_ms(),
        verse,
    };
    tracing::info!(id, passage = %card.passage_id, "operator edited a detection");
    // A new row, not an update: the operator named a different verse, and
    // the one the stage proposed stays on record as what it proposed.
    persist(&state, &card, &translation);
    decide(&state, id, true);
    let _ = app.emit("detection:new", &card);
    Ok(card)
}
