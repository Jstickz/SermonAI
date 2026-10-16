//! Bible and licensing commands (PRD v2.2 §15.2).

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::bible::translations::{self, TranslationChoice};
use crate::bible::youversion::{BibleVersion, PORTAL_URL};
use crate::bible::{cache, reference};
use crate::db::models::Verse;
use crate::error::{Error, Result};
use crate::state::AppState;

/// A typed reference as a verse in the current translation (FR-31, PRD
/// §13.9; M3 deliverable 7). "john 3 16", "John 3:16" and "1 jn 4 8" all
/// resolve; the parser owns the spellings. A verse the cache does not hold
/// comes back with empty text, as a card does, so the palette can say so
/// rather than wait on a network it may not have.
#[tauri::command]
pub fn resolve_reference(state: State<'_, AppState>, reference: String) -> Result<Verse> {
    let parsed = reference::parse(&reference).map_err(|e| {
        Error::Bible(format!(
            "\"{}\" is not a reference: {e}. Try the form \"john 3 16\".",
            reference.trim()
        ))
    })?;
    let translation = state
        .default_translation
        .lock()
        .expect("translation lock")
        .clone();
    let db = state.db.lock().expect("db lock");
    let cached = cache::lookup_ref(&db, &translation, &parsed)?;
    Ok(cached.map(|c| c.verse).unwrap_or_else(|| Verse {
        reference: parsed.display(),
        translation,
        text: String::new(),
    }))
}

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
