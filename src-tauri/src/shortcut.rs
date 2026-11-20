use std::str::FromStr;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::state::AppState;

pub fn register(app: &AppHandle) -> Result<(), String> {
    let hotkey = app
        .state::<AppState>()
        .settings_snapshot()
        .hotkey;
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();
    let shortcut = Shortcut::from_str(&hotkey).map_err(|err| format!("Invalid shortcut “{hotkey}”: {err}"))?;
    manager
        .on_shortcut(shortcut, |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                toggle_picker(app);
            }
        })
        .map_err(|err| format!("Could not register {hotkey}: {err}"))
}

pub fn toggle_picker(app: &AppHandle) {
    show_picker(app, false);
}

/// Tray clicks should feel like a system flyout: anchored just above the
/// system tray at the bottom-right of the working area, not floating in the
/// centre of the screen.
pub fn toggle_picker_tray(app: &AppHandle) {
    show_picker(app, true);
}

fn show_picker(app: &AppHandle, anchor_to_tray: bool) {
    let Some(window) = app.get_webview_window("picker") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    if anchor_to_tray {
        if let Ok(Some(monitor)) = window.current_monitor() {
            let work = monitor.work_area();
            if let Ok(size) = window.outer_size() {
                let x = work.position.x
                    + work.size.width.saturating_sub(size.width) as i32
                    - 16;
                let y = work.position.y
                    + work.size.height.saturating_sub(size.height) as i32
                    - 16;
                let _ = window.set_position(Position::Physical(PhysicalPosition {
                    x: x.max(work.position.x + 8),
                    y: y.max(work.position.y + 8),
                }));
            }
        }
    } else {
        let _ = window.center();
    }
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("picker-open", ());
}

pub fn hide_picker(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("picker") {
        let _ = window.hide();
    }
}
