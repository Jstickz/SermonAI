//! Settings → Services and keys (PRD §17, key strategy Phase 5).
//!
//! Thin, like every handler here: validate, delegate to `crate::credentials`,
//! map errors.
//!
//! ## The rule these handlers exist to keep
//!
//! **No raw key crosses this boundary in either direction, except the one the
//! operator is typing.** `set_service_key` takes a key inbound because there is
//! no other way for a paste to reach the store; nothing returns one. The
//! listing carries a mask and a status, so a frontend bug, a devtools session
//! or a logged IPC payload cannot leak a credential that was never sent.

use tauri::State;

use crate::credentials::verify::{self, TestOutcome};
use crate::credentials::{Service, ServiceCredential};
use crate::error::Result;
use crate::state::AppState;

/// Every service with its current state. Safe to call on every panel mount.
#[tauri::command]
pub fn list_service_credentials(state: State<'_, AppState>) -> Vec<ServiceCredential> {
    state.credentials.summary()
}

/// Store a key the operator pasted.
///
/// Returns the refreshed listing rather than nothing, so the panel cannot show
/// a stale row after a save — and so a save that lands but reads back as
/// missing, which is what a failing credential store looks like, is visible
/// immediately instead of on the next restart.
#[tauri::command]
pub fn set_service_key(
    state: State<'_, AppState>,
    service: Service,
    key: String,
) -> Result<Vec<ServiceCredential>> {
    state.credentials.set_byok(service, &key)?;
    tracing::info!(service = service.slug(), "stored a key for this service");
    Ok(state.credentials.summary())
}

/// Remove a stored key.
#[tauri::command]
pub fn remove_service_key(
    state: State<'_, AppState>,
    service: Service,
) -> Result<Vec<ServiceCredential>> {
    state.credentials.remove_byok(service)?;
    tracing::info!(service = service.slug(), "removed the key for this service");
    Ok(state.credentials.summary())
}

/// Make the cheapest real call this service offers and report what happened.
#[tauri::command]
pub async fn test_service_key(state: State<'_, AppState>, service: Service) -> Result<TestOutcome> {
    verify::test(&state.credentials, service).await
}
