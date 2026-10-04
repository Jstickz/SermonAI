//! SermonAI backend. One process owns audio, STT, detection, storage, summary
//! generation, export and the LAN servers; the frontend is three webview
//! windows plus the remote web app (PRD §10.1).

pub mod audio;
pub mod bible;
pub mod commands;
pub mod credentials;
pub mod db;
pub mod detection;
pub mod error;
pub mod export;
pub mod intelligence;
pub mod llm;
pub mod output;
pub mod packs;
pub mod remote;
pub mod state;
pub mod stt;
pub mod windows;

use tauri::Manager;

/// Load `.env` into the environment. **Development builds only.**
///
/// Note this only populates the environment. What *reads* it is
/// `credentials::local`, and only when the OS credential store has nothing for
/// that service — so a key pasted into Settings always wins.
///
/// A developer's `.env` holds working vendor keys, so this has to be certain
/// never to run in a shipped app. The whole function body is behind
/// `debug_assertions`, which is off for `--release` and therefore off for every
/// bundled installer: in a release build this compiles to nothing at all, not
/// to a read that fails to find a file. That is the difference that matters —
/// a release binary has no code path that reads a `.env`, so dropping one next
/// to the executable on a church machine does nothing.
///
/// Three further things keep keys out of the installer, none of which this
/// function is responsible for but all of which it depends on:
/// `.env` is gitignored, the Tauri bundle ships only `assets/**/*` (PRD §10.7),
/// and CI passes secrets as environment variables rather than writing a file.
///
/// How keys are meant to reach an installed app is PRD §17: provisioned
/// through SermonAI licensing and held in the OS keychain, never handled by
/// the church. None of that exists yet — see the Parked entry in
/// `docs/MILESTONES.md`, which is a decision owed before M4.
fn load_dev_env() {
    #[cfg(debug_assertions)]
    {
        match dotenvy::dotenv() {
            Ok(path) => eprintln!("loaded development environment from {}", path.display()),
            // Absent is the normal case for anyone who has not set one up, and
            // the app runs without it: the online features disable themselves
            // and say so. Nothing to report.
            Err(err) if err.not_found() => {}
            Err(err) => eprintln!("could not read .env: {err}"),
        }
    }
}

/// A log file the app writes itself, plus stdout where there is one.
///
/// **A release build has no console.** `main.rs` sets
/// `windows_subsystem = "windows"` so launching SermonAI does not open a black
/// window behind it, and the consequence is that `sermonai.exe > log.txt`
/// produces an **empty file**: there is no stdout to redirect. Every diagnostic
/// from a normally-launched run was therefore going nowhere, which is why a
/// 60-minute DoD run could not afterwards answer "were there any reconnects".
///
/// Appended rather than truncated, so a crash and a relaunch do not erase the
/// evidence of what happened before it. Nothing rotates it yet; a service
/// produces a few hundred kilobytes.
///
/// Returns the path so startup can log where it is — an operator asked for
/// diagnostics should not have to be told a directory over the phone.
fn init_logging() -> Option<std::path::PathBuf> {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    /// `MakeWriter` needs something cloneable that writes; a bare `File` is
    /// not, and cloning the handle per event would interleave lines from
    /// different threads mid-message.
    struct Shared(Arc<Mutex<std::fs::File>>);

    impl Write for Shared {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("log file lock").write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0.lock().expect("log file lock").flush()
        }
    }

    // Default to info for our own crate. Without this a release build honours
    // an unset RUST_LOG and records nothing at all, which is the same failure
    // in a different disguise.
    let filter = || {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("sermonai_lib=info"))
    };

    let path = log_file_path();
    let file = path.as_ref().and_then(|path| {
        std::fs::create_dir_all(path.parent()?).ok()?;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
    });

    match file {
        Some(file) => {
            use tracing_subscriber::layer::{Layer, SubscriberExt};
            use tracing_subscriber::util::SubscriberInitExt;

            let shared = Arc::new(Mutex::new(file));
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_ansi(false)
                        .with_writer(move || Shared(Arc::clone(&shared)))
                        .with_filter(filter()),
                )
                .with(tracing_subscriber::fmt::layer().with_filter(filter()))
                .init();
            path
        }
        None => {
            // A read-only or missing app-data directory must not stop a
            // service starting. Console only, and say so once it is up.
            tracing_subscriber::fmt().with_env_filter(filter()).init();
            None
        }
    }
}

/// `%LOCALAPPDATA%\SermonAI\logs\sermonai.log` on Windows,
/// `~/Library/Logs/SermonAI/sermonai.log` on macOS.
///
/// Resolved from the environment rather than from Tauri's path API because
/// logging has to be running before the app is built — a failure during
/// `setup()` is exactly the one worth having on disk.
fn log_file_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::path::PathBuf::from(std::env::var("LOCALAPPDATA").ok()?).join("SermonAI");

    #[cfg(target_os = "macos")]
    let base = std::path::PathBuf::from(std::env::var("HOME").ok()?)
        .join("Library")
        .join("Logs")
        .join("SermonAI");

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::temp_dir().join("SermonAI");

    Some(base.join("logs").join("sermonai.log"))
}

/// Where the bundled assets live for this build.
///
/// A release build ships `assets/**/*` as Tauri resources and finds them under
/// the resource directory. `tauri dev` does not copy resources, so a debug
/// build falls back to the source tree — compiled in under `debug_assertions`
/// only, so no developer path is embedded in a shipped binary.
fn resolve_assets_dir(app: &tauri::App) -> std::path::PathBuf {
    if let Ok(resources) = app.path().resource_dir() {
        let candidate = resources.join("assets");
        if candidate.join("translations").is_dir() {
            return candidate;
        }
    }

    #[cfg(debug_assertions)]
    {
        let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
        if dev.join("translations").is_dir() {
            tracing::info!(path = %dev.display(), "using assets from the source tree (dev build)");
            return dev;
        }
    }

    // Nothing found: return where they should have been, so the error that
    // follows names the path.
    app.path()
        .resource_dir()
        .map(|r| r.join("assets"))
        .unwrap_or_else(|_| std::path::PathBuf::from("assets"))
}

pub fn run() {
    // Taken before anything else so the cold-start figure covers the whole of
    // our startup, not just the part after logging is up.
    let started = std::time::Instant::now();

    load_dev_env();
    let log_path = init_logging();
    // Everything between here and "setup entered" is Tauri's: plugin init
    // and the operator window, which it creates before setup runs.
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis() as u64,
        "logging ready"
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            // Everything before this line is Tauri's: plugin init and the
            // operator window from tauri.conf.json, which Tauri creates
            // before setup runs. Logged so the cold-start figure separates
            // what is ours from what is the framework's.
            tracing::info!(
                elapsed_ms = started.elapsed().as_millis() as u64,
                "setup entered"
            );

            // Migrations run before any window can issue a command (M0 deliverable).
            let data_dir = app.path().app_data_dir()?;
            let mut conn = db::init(&data_dir)?;
            // Startup is attributed stage by stage so a missed cold-start
            // budget names its cause. Same clock as "startup complete".
            tracing::info!(
                elapsed_ms = started.elapsed().as_millis() as u64,
                "database ready"
            );

            // Bundled translations into the verse cache (FR-19), checked
            // against their manifests (FR-22). One COUNT(*) per translation
            // once seeded; a few seconds the first time. Done before the
            // window opens so the first detection card has text to show.
            let assets_dir = resolve_assets_dir(app);
            match bible::cache::seed_bundled(&mut conn, &assets_dir) {
                Ok(report) => {
                    for (code, rows) in &report.seeded {
                        tracing::info!(translation = %code, rows, "seeded bundled translation");
                    }
                    if !report.verified.is_empty() {
                        tracing::info!(
                            verified = ?report.verified,
                            elapsed_ms = started.elapsed().as_millis() as u64,
                            "bundled translations already cached and intact"
                        );
                    }
                    for code in &report.refused {
                        tracing::error!(translation = %code, "a bundled translation failed its integrity check and was not loaded");
                    }
                }
                // Startup continues: the app is usable without cached verses,
                // and the operator is better served by a window that says so.
                Err(err) => tracing::error!(%err, "could not seed the verse cache"),
            }

            // Pack catalog location is overridable for testing against a local
            // bucket; production points at the Pack CDN (PRD §15.5).
            let cdn = std::env::var("PACK_CDN_BASE_URL")
                .unwrap_or_else(|_| "https://packs.sermonai.app".to_string());
            let manifest_url = format!("{}/packs-manifest.json", cdn.trim_end_matches('/'));
            let packs = packs::PackManager::new(&data_dir, manifest_url);

            // Credentials first: every service client is built from them
            // (PRD §17). Today that is BYOK — a church's own keys in the OS
            // credential store, with a developer's .env behind it — and the
            // gateway provider replaces it in Phase 3 without any service
            // client changing.
            let credentials =
                credentials::Credentials::new(Box::new(credentials::local::LocalProvider::new()));

            // Online Bible access. Logs once and disables itself if no
            // credential is available, rather than failing startup.
            let bible = bible::youversion::YouVersionClient::from_credentials(&credentials);

            // The pipeline starts with the regex stage only. The vector
            // index and encoder — 7.7 MB plus 8 MB, 400–740 ms from disk —
            // load on a thread below and join once ready, so the window opens
            // inside PRD §9.1's one-second cold start. Until they join, a
            // final with no direct reference gets regex alone, and the stop
            // log counts how many that was.
            let llm = llm::LlmConfig::load();
            let pipeline = detection::pipeline::Pipeline::new(None, llm.gate.clone());
            let vector_assets = assets_dir.clone();

            app.manage(state::AppState::new(
                conn, packs, bible, credentials, assets_dir, pipeline, llm, started,
            ));

            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("sermonai-vector-load".to_string())
                .spawn(move || {
                    let started = std::time::Instant::now();
                    match detection::vector::SemanticSearch::load(&vector_assets) {
                        Ok(search) => {
                            let verses = search.index().len();
                            let dims = search.index().dims();
                            handle
                                .state::<state::AppState>()
                                .pipeline
                                .lock()
                                .expect("pipeline lock")
                                .attach_vector(search);
                            tracing::info!(
                                verses,
                                dims,
                                elapsed_ms = started.elapsed().as_millis() as u64,
                                "vector stage joined the pipeline"
                            );
                        }
                        // Regex still works without it; semantic detection
                        // does not, and the operator is better told than
                        // surprised.
                        Err(err) => tracing::error!(
                            %err,
                            "semantic detection is off: the verse index did not load"
                        ),
                    }
                })
                .map_err(|e| std::io::Error::new(e.kind(), format!("could not start the index loader: {e}")))?;

            // The projector and alternate windows are created from Rust so we
            // can place them on the operator's chosen monitors (PRD §10.3) —
            // but not here. Creating two WebView2 windows at startup cost
            // 1.4 s of a 2.6 s cold start on the release build (4 Oct 2026);
            // `windows::place_on_monitor` creates each on first assignment.

            // Cold start marker. PRD §9.1 budgets one second from launch to
            // ready, and M0's DoD line measures it. Emitting it here rather
            // than timing with a stopwatch means CI can check the budget on
            // every build, and means the number covers the same work on both
            // platforms: migrations, pack manager, Bible client, operator
            // window. The output windows are created on first assignment.
            //
            // Needs RUST_LOG to be set, since the filter comes from the
            // environment and is empty by default.
            match &log_path {
                Some(path) => tracing::info!(path = %path.display(), "writing this log to disk"),
                None => tracing::warn!(
                    "no log file could be opened; this run leaves no diagnostics behind"
                ),
            }

            tracing::info!(
                elapsed_ms = started.elapsed().as_millis() as u64,
                "startup complete"
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::audio::list_audio_devices,
            commands::audio::check_audio_device,
            commands::audio::start_capture,
            commands::audio::stop_capture,
            commands::audio::switch_capture_device,
            commands::audio::pause_capture,
            commands::audio::resume_capture,
            commands::audio::capture_state,
            commands::audio::transcript_snapshot,
            commands::audio::transcript_rolling,
            commands::audio::transcript_latency,
            commands::display::list_monitors,
            commands::display::set_projector_monitor,
            commands::display::set_alternate_monitor,
            commands::display::get_output_assignments,
            commands::display::operator_window_ready,
            commands::packs::refresh_pack_catalog,
            commands::packs::list_packs,
            commands::packs::download_pack,
            commands::packs::pause_pack_download,
            commands::packs::remove_pack,
            commands::bible::open_license_portal,
            commands::bible::refresh_bible_licenses,
            commands::bible::is_bible_online,
            commands::credentials::list_service_credentials,
            commands::credentials::set_service_key,
            commands::credentials::remove_service_key,
            commands::credentials::test_service_key,
            commands::detection::accept_detection,
            commands::detection::reject_detection,
            commands::detection::edit_detection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SermonAI");
}
