use std::sync::atomic::Ordering;

use serde_json::json;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::db;
use crate::state::AppState;

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let monitoring = app
        .state::<AppState>()
        .monitoring
        .load(Ordering::Relaxed);
    let menu = build_menu(app, monitoring)?;
    let icon = app.default_window_icon().cloned();
    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip(tooltip(monitoring))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "settings" => {
                show_main(app);
                let _ = app.emit("open-settings", ());
            }
            "toggle" => toggle_monitoring(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                // The tray icon is the quick way to reach your clips: show the
                // compact picker, not the full library window. Right-click still
                // opens the menu with Open ClipVault, Settings and Quit.
                crate::shortcut::toggle_picker(tray.app_handle());
            }
        });
    if let Some(icon) = icon {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

fn build_menu(app: &AppHandle, monitoring: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", "Open ClipVault", true, None::<&str>)?;
    let toggle = MenuItem::with_id(
        app,
        "toggle",
        if monitoring {
            "Pause monitoring"
        } else {
            "Resume monitoring"
        },
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit ClipVault", true, None::<&str>)?;
    Menu::with_items(app, &[&show, &toggle, &settings, &separator, &quit])
}

fn tooltip(monitoring: bool) -> &'static str {
    if monitoring {
        "ClipVault — monitoring the clipboard"
    } else {
        "ClipVault — monitoring paused"
    }
}

pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main-tray") else {
        return;
    };
    let monitoring = app
        .state::<AppState>()
        .monitoring
        .load(Ordering::Relaxed);
    if let Ok(menu) = build_menu(app, monitoring) {
        let _ = tray.set_menu(Some(menu));
    }
    let _ = tray.set_tooltip(Some(tooltip(monitoring)));
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn toggle_monitoring(app: &AppHandle) {
    let state = app.state::<AppState>();
    let enabled = !state.monitoring.load(Ordering::Relaxed);
    let next = {
        let mut settings = match state.settings.lock() {
            Ok(settings) => settings,
            Err(_) => return,
        };
        settings.monitoring_enabled = enabled;
        settings.clone()
    };
    let _ = state.with_db(|conn| db::save_settings(conn, &next));
    state.monitoring.store(enabled, Ordering::Relaxed);
    let _ = app.emit("monitoring-changed", json!({ "enabled": enabled }));
    let _ = app.emit("settings-changed", ());
    refresh(app);
}
