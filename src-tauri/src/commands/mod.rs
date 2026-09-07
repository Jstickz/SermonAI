//! `#[tauri::command]` handlers, grouped by domain and registered in
//! `lib.rs::run()`. Handlers stay thin: validate, delegate to the domain
//! module, map errors. Every command here has a typed wrapper in
//! `src/lib/ipc.ts` — add both in the same commit.
//!
//! Planned files:
//!   audio.rs        list/select devices, start/stop capture (M1)
//!   service.rs      start_service, end_service (M1, M4)
//!   output.rs       stage_verse, go_live, blank, step_verse (M3)
//!   bible.rs        lookup_verse, search_verses, translations (M2, M6)
//!   library.rs      list_sermons, download_summary_pdf (M4)
//!   packs.rs        list/download/pause/remove packs (M0)
//!   remote.rs       enable/disable remote, pairing, revoke devices (M7)

pub mod display;
