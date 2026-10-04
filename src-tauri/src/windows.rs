//! Creation and monitor placement for the projector and alternate windows.
//!
//! Both output windows are created hidden and frameless, so nothing ever
//! flashes on the congregation's screen before the operator sends it there.
//! They are shown only when assigned to a monitor (M0 deliverable, PRD §10.3).
//!
//! They are also created only when first assigned (4 Oct 2026). Creating
//! them at startup cost 1.4 s of a 2.6 s cold start on the release build —
//! two WebView2 instances brought up while the operator window was still
//! painting — against PRD §9.1's one-second budget. The booth assigns
//! displays before the service, so the cost moves to a moment nobody is
//! waiting on, and a laptop that never projects never pays it at all.

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::error::{Error, Result};

pub const OPERATOR_LABEL: &str = "operator";
pub const PROJECTOR_LABEL: &str = "projector";
pub const ALTERNATE_LABEL: &str = "alternate";

/// A display the operator can send an output window to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    /// Stable-ish OS name, used to remember the choice across restarts.
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

/// The output window with this label, created hidden on first use.
///
/// Idempotent: a second call finds the existing window. The page loads
/// asynchronously after `build()` returns; the window's background colour is
/// the projector's own dark, so a show that races the load paints dark, not
/// white.
fn ensure_output_window(app: &AppHandle, label: &str) -> Result<WebviewWindow> {
    if let Some(existing) = app.get_webview_window(label) {
        return Ok(existing);
    }
    let (url, title) = match label {
        PROJECTOR_LABEL => ("projector.html", "SermonAI — Projector"),
        ALTERNATE_LABEL => ("alternate.html", "SermonAI — Confidence Monitor"),
        other => return Err(Error::Window(format!("'{other}' is not an output window"))),
    };
    let started = std::time::Instant::now();
    let dark = tauri::window::Color(0x0F, 0x0F, 0x10, 0xFF);
    let created = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
        .decorations(false)
        .visible(false)
        .background_color(dark)
        .build()?;
    tracing::info!(
        label,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "output window created on first assignment"
    );
    Ok(created)
}

fn window(app: &AppHandle, label: &str) -> Result<WebviewWindow> {
    app.get_webview_window(label)
        .ok_or_else(|| Error::Window(format!("window '{label}' does not exist")))
}

/// Every display currently attached, for the operator's display picker.
pub fn list_monitors(app: &AppHandle) -> Result<Vec<MonitorInfo>> {
    let operator = window(app, OPERATOR_LABEL)?;
    let primary_name = operator
        .primary_monitor()?
        .and_then(|m| m.name().cloned())
        .unwrap_or_default();

    let monitors = operator
        .available_monitors()?
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| {
            let name = monitor
                .name()
                .cloned()
                .unwrap_or_else(|| format!("Display {}", index + 1));
            let size = monitor.size();
            let position = monitor.position();
            MonitorInfo {
                is_primary: name == primary_name,
                name,
                width: size.width,
                height: size.height,
                x: position.x,
                y: position.y,
                scale_factor: monitor.scale_factor(),
            }
        })
        .collect();

    Ok(monitors)
}

/// Move an output window onto the named monitor, go fullscreen there, and show it.
///
/// A monitor that has been unplugged since the church last chose it must not
/// take the app down mid-service, so an unknown name falls back to the primary
/// display rather than failing (M0 DoD: "unplug: app does not crash").
pub fn place_on_monitor(app: &AppHandle, label: &str, monitor_name: Option<&str>) -> Result<()> {
    let target = ensure_output_window(app, label)?;
    let monitors = target.available_monitors()?;

    let chosen = monitor_name
        .and_then(|wanted| {
            monitors
                .iter()
                .find(|m| m.name().map(|n| n.as_str()) == Some(wanted))
        })
        .or_else(|| {
            target
                .primary_monitor()
                .ok()
                .flatten()
                .as_ref()
                .and_then(|primary| {
                    let primary_name = primary.name().cloned();
                    monitors.iter().find(|m| m.name().cloned() == primary_name)
                })
        })
        .or_else(|| monitors.first())
        .ok_or_else(|| Error::Window("no displays are attached".into()))?;

    // Position before fullscreen: the OS makes a window fullscreen on whichever
    // monitor it currently sits on.
    target.set_fullscreen(false)?;
    target.set_position(PhysicalPosition::new(
        chosen.position().x,
        chosen.position().y,
    ))?;
    target.set_fullscreen(true)?;
    target.show()?;

    Ok(())
}

/// Hide an output window without destroying it, so its webview state survives.
///
/// A window that was never assigned was never created; hiding it is a no-op
/// rather than an error, so clearing an assignment always succeeds.
pub fn hide_output(app: &AppHandle, label: &str) -> Result<()> {
    let Some(target) = app.get_webview_window(label) else {
        return Ok(());
    };
    target.set_fullscreen(false)?;
    target.hide()?;
    Ok(())
}

/// Which monitor each output window was last sent to.
///
/// `None` means the window is hidden. A name here is only meaningful while
/// that display is still attached; the operator UI reconciles against the live
/// monitor list so an unplugged screen stops showing as assigned.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputAssignments {
    pub projector: Option<String>,
    pub alternate: Option<String>,
}
