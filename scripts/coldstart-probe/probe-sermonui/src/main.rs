#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Cold-start probe: a Tauri 2 window and nothing else, timed exactly as
//! SermonAI times itself, so the platform's share of the cold start can be
//! measured on its own. See docs/testing/cold-start.md.
//!
//! Lines are written in the same format SermonAI's log uses, so
//! scripts/measure-cold-start.ps1 reads them unchanged.

use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;

static STARTED: Mutex<Option<Instant>> = Mutex::new(None);
static LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);

fn log(msg: &str) {
    let elapsed = STARTED.lock().unwrap().unwrap().elapsed().as_millis();
    if let Some(f) = LOG.lock().unwrap().as_mut() {
        let _ = writeln!(f, "{}  INFO probe: {} elapsed_ms={}", chrono_free_now(), msg, elapsed);
        let _ = f.flush();
    }
}

/// A timestamp without pulling in a date crate: seconds since the epoch.
fn chrono_free_now() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:06}Z", d.as_secs(), d.subsec_micros())
}

#[tauri::command]
fn operator_window_ready() {
    log("operator window ready");
}

fn main() {
    *STARTED.lock().unwrap() = Some(Instant::now());
    let dir = std::path::PathBuf::from(std::env::var("LOCALAPPDATA").unwrap()).join("SermonAI").join("logs");
    let _ = std::fs::create_dir_all(&dir);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(concat!(env!("CARGO_PKG_NAME"), ".log")))
        .expect("log file");
    *LOG.lock().unwrap() = Some(file);
    log("logging ready");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            log("setup entered");
            log("startup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![operator_window_ready])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
