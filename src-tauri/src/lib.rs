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
    // Taken before anything else so the cold-start figure covers the whole of
    // our startup, not just the part after logging is up.
    let started = std::time::Instant::now();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            // Migrations run before any window can issue a command (M0 deliverable).
            let data_dir = app.path().app_data_dir()?;
            let conn = db::init(&data_dir)?;

            // Pack catalog location is overridable for testing against a local
            // bucket; production points at the Pack CDN (PRD §15.5).
            let cdn = std::env::var("PACK_CDN_BASE_URL")
                .unwrap_or_else(|_| "https://packs.sermonai.app".to_string());
            let manifest_url = format!("{}/packs-manifest.json", cdn.trim_end_matches('/'));
            let packs = packs::PackManager::new(&data_dir, manifest_url);

            // Online Bible access. from_env logs once and disables itself if
            // YVP_APP_KEY is missing rather than failing startup.
            let bible = bible::youversion::YouVersionClient::from_env();

            app.manage(state::AppState::new(conn, packs, bible));

            // The projector and alternate windows are created from Rust so we can
            // place them on the operator's chosen monitors (PRD §10.3).
            windows::create_output_windows(app.handle())?;

            // Cold start marker. PRD §9.1 budgets one second from launch to
            // ready, and M0's DoD line measures it. Emitting it here rather
            // than timing with a stopwatch means CI can check the budget on
            // every build, and means the number covers the same work on both
            // platforms: migrations, pack manager, Bible client, windows.
            //
            // Needs RUST_LOG to be set, since the filter comes from the
            // environment and is empty by default.
            tracing::info!(
                elapsed_ms = started.elapsed().as_millis() as u64,
                "startup complete"
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::audio::list_audio_devices,
            commands::audio::check_audio_device,
            commands::display::list_monitors,
            commands::display::set_projector_monitor,
            commands::display::set_alternate_monitor,
            commands::display::get_output_assignments,
            commands::packs::refresh_pack_catalog,
            commands::packs::list_packs,
            commands::packs::download_pack,
            commands::packs::pause_pack_download,
            commands::packs::remove_pack,
            commands::bible::open_license_portal,
            commands::bible::refresh_bible_licenses,
            commands::bible::is_bible_online,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SermonAI");
}
