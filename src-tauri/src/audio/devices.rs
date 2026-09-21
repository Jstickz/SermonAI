//! Audio input enumeration and selection (FR-01, FR-02, FR-05, PRD §8.1).
//!
//! Three kinds of thing can feed a service and the operator should not have to
//! know which is which:
//!
//! - a **microphone or interface**, including HDMI capture cards, which is an
//!   ordinary capture endpoint;
//! - a **system output captured back**, which is how a church takes the sound
//!   desk feed already going to the speakers rather than a room mic;
//! - a **virtual cable** such as BlackHole or VB-Audio, which is how the same
//!   job is done where the OS offers nothing native.
//!
//! The second is where the platforms diverge, and it shapes this module.
//!
//! **Windows** has real loopback. cpal 0.15 enables it transparently: build an
//! *input* stream on an *output* device and it sets
//! `AUDCLNT_STREAMFLAGS_LOOPBACK` for you. But `supported_input_configs()`
//! returns an empty list for an output device — cpal asks the endpoint what it
//! supports *as an input*, and an output endpoint answers nothing. So a
//! loopback device's format has to be read from the *output* config, which is
//! why `describe` branches rather than calling one method for everything. Miss
//! that and every loopback device looks unusable.
//!
//! **macOS** has no equivalent. CoreAudio cannot capture a render endpoint, so
//! output devices are not offered at all there; a Mac church installs BlackHole
//! or similar, which then appears as an ordinary input. Those are recognised by
//! name so the UI can explain what they are, but the recognition is cosmetic —
//! an unrecognised virtual cable still works, it is just labelled as an input.

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// What kind of source this is, so the operator UI can group and explain them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    /// A capture endpoint: microphone, USB interface, HDMI capture card.
    Input,
    /// A system output captured back through WASAPI loopback. Windows only.
    Loopback,
    /// A virtual cable presenting itself as an ordinary input.
    VirtualInput,
}

/// One selectable audio source.
///
/// Mirrored by `AudioDevice` in `src/lib/types.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    /// The OS name, and the handle used to select the device again (FR-02).
    pub name: String,
    pub kind: DeviceKind,
    /// The host's default capture device, pre-selected on first run.
    pub is_default: bool,
    /// The rate the device prefers. Capture at 16 kHz (FR-03) resamples from
    /// this where it differs, which is most of the time — 44.1 and 48 kHz are
    /// the common answers.
    pub default_sample_rate: u32,
    pub channels: u16,
    /// Set when the device was found but could not be interrogated. It is still
    /// listed: a device that refuses to describe itself often still captures,
    /// and hiding it would leave the operator with an empty list and no reason.
    pub warning: Option<String>,
}

/// Substrings identifying virtual audio cables, matched case-insensitively.
///
/// Cosmetic only. An unrecognised cable still works and is simply labelled as
/// an input, so a name missing here costs a label and not a capture path.
const VIRTUAL_DEVICE_MARKERS: &[&str] = &[
    "blackhole",      // macOS, the common choice
    "soundflower",    // macOS, older
    "loopback audio", // Rogue Amoeba Loopback
    "ishowu",
    "vb-audio",
    "vb audio",
    "cable output", // VB-Audio Virtual Cable
    "voicemeeter",
    "virtual audio",
    "aggregate device", // CoreAudio combination device
];

fn classify(name: &str, is_output_endpoint: bool) -> DeviceKind {
    if is_output_endpoint {
        return DeviceKind::Loopback;
    }

    let lower = name.to_lowercase();
    if VIRTUAL_DEVICE_MARKERS.iter().any(|m| lower.contains(m)) {
        DeviceKind::VirtualInput
    } else {
        DeviceKind::Input
    }
}

/// Read a device's format, from the input or output side as appropriate.
///
/// `is_output_endpoint` is the whole point: see the module note on WASAPI
/// returning no input configs for an output device.
fn describe(device: &cpal::Device, is_output_endpoint: bool, is_default: bool) -> AudioDevice {
    let name = device
        .name()
        .unwrap_or_else(|_| "Unnamed device".to_string());
    let kind = classify(&name, is_output_endpoint);

    let config = if is_output_endpoint {
        device.default_output_config()
    } else {
        device.default_input_config()
    };

    match config {
        Ok(config) => AudioDevice {
            name,
            kind,
            is_default,
            default_sample_rate: config.sample_rate().0,
            channels: config.channels(),
            warning: None,
        },
        // Listed anyway, with the reason attached. A device that will not
        // describe itself is usually one that is busy or asleep, and both
        // resolve by the time someone selects it.
        Err(err) => AudioDevice {
            name,
            kind,
            is_default,
            default_sample_rate: 0,
            channels: 0,
            warning: Some(format!("could not read its format: {err}")),
        },
    }
}

/// Every source the operator can pick (FR-01, FR-05).
///
/// Called on launch and again on demand, because devices appear and disappear
/// while the app is running — a USB interface gets plugged in, an HDMI capture
/// card wakes up. Enumeration never fails on one bad device: a device that
/// cannot be read is listed with a warning rather than dropped, so the operator
/// sees something to act on instead of a short list with no explanation.
pub fn list_devices() -> Result<Vec<AudioDevice>> {
    let host = cpal::default_host();

    let default_input_name = host.default_input_device().and_then(|d| d.name().ok());

    let mut devices = Vec::new();

    let inputs = host
        .input_devices()
        .map_err(|e| Error::Audio(format!("could not list input devices: {e}")))?;

    for device in inputs {
        let is_default = device
            .name()
            .ok()
            .zip(default_input_name.as_ref())
            .is_some_and(|(name, default)| &name == default);
        devices.push(describe(&device, false, is_default));
    }

    // Windows only: an output endpoint is a valid capture source through
    // loopback. CoreAudio has no equivalent, and listing outputs on macOS would
    // offer the operator a device that cannot possibly record.
    #[cfg(target_os = "windows")]
    {
        match host.output_devices() {
            Ok(outputs) => {
                for device in outputs {
                    devices.push(describe(&device, true, false));
                }
            }
            // Loopback is a convenience, not the main path. Losing it should
            // not cost the operator their microphones too.
            Err(err) => {
                tracing::warn!(%err, "could not list output devices for loopback capture");
            }
        }
    }

    Ok(devices)
}

/// Resolve a stored device name back to a device (FR-02).
///
/// Returns whether the match was an output endpoint alongside the device, since
/// every later call has to know which side to read its config from.
///
/// Selection is by name because a name is what survives a restart — cpal
/// exposes no stable device id, and index order changes as devices come and go.
/// The cost is that two identically named devices are indistinguishable, which
/// happens with paired capture cards; the first match wins and a warning is
/// logged. Resolving that needs a platform-specific endpoint id and is not
/// worth it until someone hits it.
pub fn find_device(name: &str) -> Result<(cpal::Device, bool)> {
    let host = cpal::default_host();
    let mut matches = Vec::new();

    let inputs = host
        .input_devices()
        .map_err(|e| Error::Audio(format!("could not list input devices: {e}")))?;
    for device in inputs {
        if device.name().is_ok_and(|n| n == name) {
            matches.push((device, false));
        }
    }

    #[cfg(target_os = "windows")]
    if let Ok(outputs) = host.output_devices() {
        for device in outputs {
            if device.name().is_ok_and(|n| n == name) {
                matches.push((device, true));
            }
        }
    }

    if matches.len() > 1 {
        tracing::warn!(
            device = name,
            count = matches.len(),
            "several devices share this name; using the first"
        );
    }

    matches.into_iter().next().ok_or_else(|| {
        Error::Audio(format!(
            "\"{name}\" is no longer available. Choose another input in Settings."
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_output_endpoint_is_always_loopback_whatever_it_is_called() {
        assert_eq!(classify("Speakers (Realtek)", true), DeviceKind::Loopback);
        // Even a name on the virtual list: how it is captured decides the kind,
        // not what it is called.
        assert_eq!(classify("VB-Audio Point", true), DeviceKind::Loopback);
    }

    #[test]
    fn virtual_cables_are_recognised_case_insensitively() {
        for name in [
            "BlackHole 2ch",
            "blackhole 16ch",
            "VB-Audio Virtual Cable",
            "CABLE Output (VB-Audio Virtual Cable)",
            "Voicemeeter Out B1",
            "Loopback Audio",
            "Aggregate Device",
        ] {
            assert_eq!(
                classify(name, false),
                DeviceKind::VirtualInput,
                "{name} should be recognised as a virtual input"
            );
        }
    }

    #[test]
    fn real_hardware_is_not_mistaken_for_a_virtual_cable() {
        for name in [
            "Microphone (USB Audio Device)",
            "MacBook Pro Microphone",
            "Scarlett 2i2 USB",
            "Elgato HD60 X",
            "Shure MV7",
        ] {
            assert_eq!(
                classify(name, false),
                DeviceKind::Input,
                "{name} should be a plain input"
            );
        }
    }

    /// Runs against whatever hardware the machine has, including none. The
    /// assertion is that enumeration is total: a CI runner with no sound card
    /// must get an empty list rather than an error, because the operator UI
    /// distinguishes "no devices" from "enumeration failed" and only the first
    /// is a state a church can act on.
    #[test]
    fn enumeration_survives_whatever_hardware_is_present() {
        let devices = list_devices().expect("enumeration should not fail");

        for device in &devices {
            assert!(
                !device.name.is_empty(),
                "every device needs a name to select it"
            );
            if device.warning.is_none() {
                assert!(
                    device.default_sample_rate > 0,
                    "{} reported no sample rate but no warning",
                    device.name
                );
            }
        }

        assert!(
            devices.iter().filter(|d| d.is_default).count() <= 1,
            "at most one device can be the default"
        );
    }

    #[test]
    fn selecting_a_device_that_has_gone_away_names_it_and_offers_a_way_out() {
        // .err(), not .unwrap_err(): cpal::Device has no Debug impl, so the
        // Ok side cannot be formatted for a panic message.
        let err = find_device("Nonexistent Interface 9000")
            .err()
            .expect("a device that does not exist should not resolve");
        let message = err.to_string();
        assert!(message.contains("Nonexistent Interface 9000"), "{message}");
        assert!(message.contains("Settings"), "{message}");
    }
}
