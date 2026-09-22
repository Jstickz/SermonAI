//! Audio device commands (M1 deliverable 1, PRD §8.1 FR-01, FR-02, FR-05).

use tauri::{AppHandle, Emitter, State};

use crate::audio::capture::{self, CaptureHandle, CaptureState};
use crate::audio::devices::{self, AudioDevice};
use crate::error::{Error, Result};
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

/// Start capturing from a device (FR-06).
///
/// Starting again replaces any running capture. The old handle is dropped
/// *before* the new device opens, not after: some interfaces allow only one
/// capture client, so holding both would fail to open the new one having
/// already told the operator it was switching.
#[tauri::command]
pub fn start_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    device_name: String,
) -> Result<CaptureState> {
    stop_capture(state.clone())?;

    let level_app = app.clone();
    let error_app = app;

    let handle = capture::spawn(
        device_name,
        // Chunks are discarded until the transcriber is listening (FR-07, M1
        // deliverable 5). Conversion still runs, so what is exercised here is
        // the real capture path rather than a metering shortcut.
        Box::new(|_chunk| {}),
        Box::new(move |peak_dbfs| {
            // A failed emit means the window has gone. Not worth logging at
            // 30 fps.
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

    let reported = handle.state();
    *state.capture.lock().expect("capture lock") = Some(handle);
    Ok(reported)
}

/// Stop capture and release the device.
///
/// Dropping the handle stops the thread, which flushes the converter's tail
/// before the stream goes — so the last partial chunk of a service is
/// delivered rather than lost.
#[tauri::command]
pub fn stop_capture(state: State<'_, AppState>) -> Result<CaptureState> {
    // Taken out of the lock before dropping: the drop joins the capture
    // thread, and holding the mutex across that would block every other
    // command that touches capture until the thread finishes.
    let running = state.capture.lock().expect("capture lock").take();
    drop(running);
    Ok(CaptureState::Stopped)
}

/// Stop delivering audio without releasing the device (FR-06).
#[tauri::command]
pub fn pause_capture(state: State<'_, AppState>) -> Result<CaptureState> {
    with_capture(&state, |handle| {
        handle.pause()?;
        Ok(handle.state())
    })
}

#[tauri::command]
pub fn resume_capture(state: State<'_, AppState>) -> Result<CaptureState> {
    with_capture(&state, |handle| {
        handle.resume()?;
        Ok(handle.state())
    })
}

/// What capture is doing, for a panel that has just mounted.
#[tauri::command]
pub fn capture_state(state: State<'_, AppState>) -> CaptureState {
    state
        .capture
        .lock()
        .expect("capture lock")
        .as_ref()
        .map_or(CaptureState::Stopped, |handle| handle.state())
}

fn with_capture(
    state: &State<'_, AppState>,
    f: impl FnOnce(&CaptureHandle) -> Result<CaptureState>,
) -> Result<CaptureState> {
    let guard = state.capture.lock().expect("capture lock");
    match guard.as_ref() {
        Some(handle) => f(handle),
        None => Err(Error::Audio(
            "Capture is not running. Start it in Settings.".to_string(),
        )),
    }
}
