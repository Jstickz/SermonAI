//! Bible and licensing commands (PRD v2.2 §15.2).

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::bible::translations::{self, TranslationChoice};
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
            "Licences cannot be checked: online verse lookup comes with SermonAI activation, which is not built yet."
                .into(),
        ));
    }

    // English only for now; PRD §26.2's catalog is English, and M6 widens it
    // when local-language translations arrive.
    state.bible.list_bibles("eng").await
}

/// Whether online lookups are possible at all. The Packs screen uses this to
/// explain why licence states cannot be checked rather than showing an error.
#[tauri::command]
pub fn is_bible_online(state: State<'_, AppState>) -> bool {
    state.bible.is_online_enabled()
}

/// Every translation a card can be shown in (FR-32, PRD §13.4).
#[tauri::command]
pub fn list_translations(state: State<'_, AppState>) -> Result<Vec<TranslationChoice>> {
    let db = state.db.lock().expect("db lock");
    translations::list(&db)
}

#[tauri::command]
pub fn get_default_translation(state: State<'_, AppState>) -> String {
    state
        .default_translation
        .lock()
        .expect("translation lock")
        .clone()
}

/// Persist the operator's pick and use it for the next card. Cards already
/// on screen keep the translation they were fetched in (FR-32).
#[tauri::command]
pub fn set_default_translation(state: State<'_, AppState>, code: String) -> Result<()> {
    {
        let db = state.db.lock().expect("db lock");
        translations::set_default(&db, &code)?;
    }
    *state.default_translation.lock().expect("translation lock") = code.clone();
    tracing::info!(translation = %code, "default translation set");
    Ok(())
}
