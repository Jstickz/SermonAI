//! Settings → Packs commands (FR-60, FR-61).

use tauri::{AppHandle, Emitter, State};

use crate::error::Result;
use crate::packs::registry::{PackInfo, PackStatus};
use crate::state::AppState;

/// Re-read the catalog from the Pack CDN, then report what is available.
#[tauri::command]
pub async fn refresh_pack_catalog(state: State<'_, AppState>) -> Result<Vec<PackInfo>> {
    state.packs.refresh_manifest().await?;
    state.packs.list()
}

#[tauri::command]
pub fn list_packs(state: State<'_, AppState>) -> Result<Vec<PackInfo>> {
    state.packs.list()
}

/// Download, verify and install a pack, emitting `pack:progress` as it goes.
#[tauri::command]
pub async fn download_pack(
    app: AppHandle,
    state: State<'_, AppState>,
    pack_id: String,
) -> Result<PackStatus> {
    state
        .packs
        .download(&pack_id, |progress| {
            // A dropped event is not worth failing a download over; the UI
            // reconciles from list_packs on the next render anyway.
            if let Err(error) = app.emit("pack:progress", progress) {
                tracing::warn!(%error, "could not emit pack progress");
            }
        })
        .await
}

#[tauri::command]
pub fn pause_pack_download(state: State<'_, AppState>, pack_id: String) {
    state.packs.pause(&pack_id);
}

#[tauri::command]
pub fn remove_pack(state: State<'_, AppState>, pack_id: String) -> Result<()> {
    state.packs.remove(&pack_id)
}
