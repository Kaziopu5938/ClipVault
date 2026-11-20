use std::str::FromStr;

use tauri::{AppHandle, Emitter, Manager};
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
    let Some(window) = app.get_webview_window("picker") else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    if visible {
        let _ = window.hide();
        return;
    }
    let _ = window.center();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("picker-open", ());
}

pub fn hide_picker(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("picker") {
        let _ = window.hide();
    }
}
