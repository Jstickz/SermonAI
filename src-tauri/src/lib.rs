//! SermonAI backend. One process owns audio, STT, detection, storage, summary
//! generation, export and the LAN servers; the frontend is three webview
//! windows plus the remote web app (PRD §10.1).

pub mod audio;
pub mod bible;
pub mod commands;
pub mod db;
pub mod detection;
pub mod error;
pub mod export;
pub mod intelligence;
pub mod output;
pub mod packs;
pub mod remote;
pub mod state;
pub mod stt;
pub mod windows;

use tauri::Manager;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Migrations run before any window can issue a command (M0 deliverable).
            let data_dir = app.path().app_data_dir()?;
            let conn = db::init(&data_dir)?;

            // Pack catalog location is overridable for testing against a local
            // bucket; production points at the Pack CDN (PRD §15.5).
            let cdn = std::env::var("PACK_CDN_BASE_URL")
                .unwrap_or_else(|_| "https://packs.sermonai.app".to_string());
            let manifest_url = format!("{}/packs-manifest.json", cdn.trim_end_matches('/'));
            let packs = packs::PackManager::new(&data_dir, manifest_url);

            app.manage(state::AppState::new(conn, packs));

            // The projector and alternate windows are created from Rust so we can
            // place them on the operator's chosen monitors (PRD §10.3).
            windows::create_output_windows(app.handle())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::display::list_monitors,
            commands::display::set_projector_monitor,
            commands::display::set_alternate_monitor,
            commands::display::get_output_assignments,
            commands::packs::refresh_pack_catalog,
            commands::packs::list_packs,
            commands::packs::download_pack,
            commands::packs::pause_pack_download,
            commands::packs::remove_pack,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SermonAI");
}
