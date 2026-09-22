//! Audio device commands (M1 deliverable 1, PRD §8.1 FR-01, FR-02, FR-05).

use tauri::{AppHandle, Emitter, State};

use crate::audio::capture;
use crate::audio::devices::{self, AudioDevice};
use crate::error::Result;
use crate::state::AppState;

/// Every audio source the operator can pick (FR-01).
///
/// The frontend calls this on mount and again whenever the operator asks, so
/// that plugging in an interface mid-setup does not need a restart. Enumeration
/// is cheap enough to do on demand — it is a CoreAudio or WASAPI endpoint walk,
/// not a device open.
#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<AudioDevice>> {
    devices::list_devices()
}

/// Check that a device is still there before committing to it (FR-02).
///
/// The operator's choice is a name, and a name can go stale between the picker
/// opening and Start being pressed. This gives the UI a way to say so while
/// there is still time to choose again, rather than at the moment capture was
/// supposed to begin.
#[tauri::command]
pub fn check_audio_device(name: String) -> Result<()> {
    devices::find_device(&name).map(|_| ())
}

/// Payload of the `transcript:level` event (FR-04).
///
/// Mirrored by the `transcript:level` entry in `src/lib/ipc.ts`.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelPayload {
    /// Peak since the last frame, in dBFS. 0 is full scale, -60 is the floor.
    pub peak_dbfs: f32,
}

/// Payload of the `audio:error` event.
#[derive(Clone, serde::Serialize)]
pub struct AudioErrorPayload {
    pub message: String,
}

/// Start listening to a device so its level shows in the top bar (FR-04).
///
/// This is level monitoring, not a service: it opens the device and drives the
/// meter so an operator can confirm the input is live before anything depends
/// on it. Full transport — pause and resume without reopening — is FR-06.
///
/// Starting again replaces any running capture. Assigning over the handle
/// drops the previous one, which stops that device and joins its thread, so
/// two inputs are never open at once.
#[tauri::command]
pub fn start_level_monitor(
    app: AppHandle,
    state: State<'_, AppState>,
    device_name: String,
) -> Result<()> {
    // Dropped before the new one opens, not after: some interfaces allow only
    // one capture client, and holding both would fail to open the new device
    // while having already told the operator it was switching.
    stop_level_monitor(state.clone())?;

    let level_app = app.clone();
    let error_app = app;

    let handle = capture::spawn(
        device_name,
        // Chunks are discarded while monitoring. They exist for the
        // transcriber, which is not listening yet (FR-07, M1 deliverable 5);
        // conversion still runs so the meter reflects a real capture path.
        Box::new(|_chunk| {}),
        Box::new(move |peak_dbfs| {
            // A failed emit means the window has gone. Not worth logging on
            // every frame at 30 fps.
            let _ = level_app.emit("transcript:level", LevelPayload { peak_dbfs });
        }),
        Box::new(move |err| {
            tracing::error!(%err, "capture failed");
            let _ = error_app.emit(
                "audio:error",
                AudioErrorPayload {
                    message: err.to_string(),
                },
            );
        }),
    )?;

    *state.capture.lock().expect("capture lock") = Some(handle);
    Ok(())
}

/// Stop monitoring and release the device.
#[tauri::command]
pub fn stop_level_monitor(state: State<'_, AppState>) -> Result<()> {
    // Taken out of the lock before dropping: the drop joins the capture
    // thread, and holding the mutex across that would block any command that
    // touches capture until the thread finishes.
    let running = state.capture.lock().expect("capture lock").take();
    drop(running);
    Ok(())
}
