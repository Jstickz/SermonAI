//! Creation and monitor placement for the projector and alternate windows.
//!
//! Both output windows are created hidden and frameless, so nothing ever
//! flashes on the congregation's screen before the operator sends it there.
//! They are shown only when assigned to a monitor (M0 deliverable, PRD §10.3).

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

pub fn create_output_windows(app: &AppHandle) -> Result<()> {
    let dark = tauri::window::Color(0x0F, 0x0F, 0x10, 0xFF);

    WebviewWindowBuilder::new(
        app,
        PROJECTOR_LABEL,
        WebviewUrl::App("projector.html".into()),
    )
    .title("SermonAI — Projector")
    .decorations(false)
    .visible(false)
    .background_color(dark)
    .build()?;

    WebviewWindowBuilder::new(
        app,
        ALTERNATE_LABEL,
        WebviewUrl::App("alternate.html".into()),
    )
    .title("SermonAI — Confidence Monitor")
    .decorations(false)
    .visible(false)
    .background_color(dark)
    .build()?;

    Ok(())
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
    let target = window(app, label)?;
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
pub fn hide_output(app: &AppHandle, label: &str) -> Result<()> {
    let target = window(app, label)?;
    target.set_fullscreen(false)?;
    target.hide()?;
    Ok(())
}
