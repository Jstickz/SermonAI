//! Bible and licensing commands (PRD v2.2 §15.2).

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::bible::youversion::{BibleVersion, PORTAL_URL};
use crate::error::{Error, Result};
use crate::state::AppState;

/// Open the YouVersion portal so the operator can accept a version's terms.
///
/// Licences are granted to our app key, not to the app, so this is the only
/// route to approval — the operator has to accept them on YouVersion's site.
#[tauri::command]
pub fn open_license_portal(app: AppHandle) -> Result<()> {
    app.opener()
        .open_url(PORTAL_URL, None::<&str>)
        .map_err(|e| Error::Bible(format!("could not open the YouVersion portal: {e}")))
}

/// Re-read which versions this app key may use.
///
/// Called when the operator returns from the portal, so a licence approved
/// there shows up without restarting the app.
#[tauri::command]
pub async fn refresh_bible_licenses(state: State<'_, AppState>) -> Result<Vec<BibleVersion>> {
    if !state.bible.is_online_enabled() {
        return Err(Error::Bible(
            "online Bible access is off because no YouVersion app key is set. Add YVP_APP_KEY to .env and restart"
                .into(),
        ));
    }

    state.bible.list_bibles().await
}

/// Whether online lookups are possible at all. The Packs screen uses this to
/// explain why licence states cannot be checked rather than showing an error.
#[tauri::command]
pub fn is_bible_online(state: State<'_, AppState>) -> bool {
    state.bible.is_online_enabled()
}
