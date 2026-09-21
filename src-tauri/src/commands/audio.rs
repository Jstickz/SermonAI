//! Audio device commands (M1 deliverable 1, PRD §8.1 FR-01, FR-02, FR-05).

use crate::audio::devices::{self, AudioDevice};
use crate::error::Result;

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
