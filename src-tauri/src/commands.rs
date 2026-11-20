use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::backup;
use crate::clipboard;
use crate::db;
use crate::error::{AppError, Result};
use crate::models::{
    AppInfo, BackupInfo, ClipDetail, ClipPage, ClipQuery, ClipSummary, IntegrityReport, Settings,
    Stats,
};
use crate::state::AppState;
use crate::storage::remove_dir_contents;

#[tauri::command]
pub fn list_clips(state: State<'_, AppState>, query: ClipQuery) -> Result<ClipPage> {
    state.with_db(|conn| db::list_clips(conn, &query, &state.paths))
}

#[tauri::command]
pub fn get_clip(state: State<'_, AppState>, id: i64) -> Result<ClipDetail> {
    state.with_db(|conn| db::get_detail(conn, &state.paths, id))
}

#[tauri::command]
pub fn copy_clip(app: AppHandle, id: i64) -> Result<i64> {
    clipboard::copy_clip(&app, id, false)?;
    Ok(id)
}

#[tauri::command]
pub fn copy_clip_plain(app: AppHandle, id: i64) -> Result<i64> {
    clipboard::copy_clip(&app, id, true)?;
    Ok(id)
}

#[tauri::command]
pub fn toggle_pin(state: State<'_, AppState>, id: i64) -> Result<bool> {
    state.with_db(|conn| db::toggle_pin(conn, id))
}

#[tauri::command]
pub fn toggle_favorite(state: State<'_, AppState>, id: i64) -> Result<bool> {
    state.with_db(|conn| db::toggle_favorite(conn, id))
}

#[tauri::command]
pub fn bulk_set_flag(
    state: State<'_, AppState>,
    ids: Vec<i64>,
    flag: String,
    value: bool,
) -> Result<usize> {
    state.with_db(|conn| db::bulk_set_flag(conn, &ids, &flag, value))
}

#[tauri::command]
pub fn delete_clips(app: AppHandle, state: State<'_, AppState>, ids: Vec<i64>) -> Result<usize> {
    let uuids = state.with_db(|conn| {
        let uuids = db::clip_uuids(conn, &ids)?;
        let deleted = db::delete_ids(conn, &ids)?;
        Ok((uuids, deleted))
    })?;
    for uuid in &uuids.0 {
        db::remove_assets_for(&state.paths, uuid);
    }
    state.invalidate_assets();
    let _ = app.emit("clips-changed", ());
    Ok(uuids.1)
}

#[tauri::command]
pub fn clear_history(app: AppHandle, state: State<'_, AppState>) -> Result<usize> {
    let count = state.with_db(|conn| db::clear_all(conn))?;
    remove_dir_contents(&state.paths.images)?;
    remove_dir_contents(&state.paths.thumbnails)?;
    remove_dir_contents(&state.paths.attachments)?;
    remove_dir_contents(&state.paths.files)?;
    state.invalidate_assets();
    let _ = app.emit("clips-changed", ());
    Ok(count)
}

#[tauri::command]
pub fn get_stats(state: State<'_, AppState>) -> Result<Stats> {
    let asset_bytes = state.asset_bytes();
    state.with_db(|conn| db::stats(conn, &state.paths, asset_bytes))
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings_snapshot()
}

fn persist_settings(state: &AppState, settings: &Settings) -> Result<()> {
    state.with_db(|conn| db::save_settings(conn, settings))?;
    *state
        .settings
        .lock()
        .map_err(|_| AppError::msg("Settings lock is poisoned"))? = settings.clone();
    state
        .monitoring
        .store(settings.monitoring_enabled, Ordering::Relaxed);
    Ok(())
}

fn apply_autostart(app: &AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(err) = result {
        tracing::warn!("could not update launch-at-startup: {err}");
    }
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<Settings> {
    let next = settings.normalized();
    persist_settings(&state, &next)?;
    apply_autostart(&app, next.launch_at_startup);
    if let Err(err) = crate::shortcut::register(&app) {
        tracing::warn!("{err}");
    }
    crate::tray::refresh(&app);
    let _ = app.emit("settings-changed", ());
    Ok(next)
}

#[tauri::command]
pub fn set_monitoring(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<bool> {
    let mut next = state.settings_snapshot();
    next.monitoring_enabled = enabled;
    persist_settings(&state, &next)?;
    let _ = app.emit("monitoring-changed", serde_json::json!({ "enabled": enabled }));
    let _ = app.emit("settings-changed", ());
    crate::tray::refresh(&app);
    Ok(enabled)
}

#[tauri::command]
pub fn get_monitoring(state: State<'_, AppState>) -> bool {
    state.monitoring.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn get_app_info(app: AppHandle, state: State<'_, AppState>) -> Result<AppInfo> {
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        schema_version: db::SCHEMA_VERSION,
        data_dir: state.paths.root.to_string_lossy().to_string(),
        db_path: state.paths.db_file.to_string_lossy().to_string(),
        logs_dir: state.paths.logs.to_string_lossy().to_string(),
        platform: clipboard::win::os_version(),
        monitoring: state.monitoring.load(Ordering::Relaxed),
    })
}

#[tauri::command]
pub fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupInfo>> {
    backup::list_backups(&state)
}

#[tauri::command]
pub fn create_backup(app: AppHandle, state: State<'_, AppState>) -> Result<BackupInfo> {
    backup::create_backup(&app, &state)
}

#[tauri::command]
pub fn restore_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    password: Option<String>,
) -> Result<()> {
    backup::restore_archive(&app, &state, &path, password.as_deref())?;
    state.invalidate_assets();
    if let Err(err) = crate::shortcut::register(&app) {
        tracing::warn!("{err}");
    }
    crate::tray::refresh(&app);
    Ok(())
}

#[tauri::command]
pub fn import_archive(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    password: Option<String>,
) -> Result<()> {
    backup::restore_archive(&app, &state, &path, password.as_deref())?;
    state.invalidate_assets();
    if let Err(err) = crate::shortcut::register(&app) {
        tracing::warn!("{err}");
    }
    crate::tray::refresh(&app);
    Ok(())
}

#[tauri::command]
pub fn export_archive(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    password: Option<String>,
) -> Result<String> {
    backup::export_archive(&app, &state, &path, password.as_deref())
}

#[tauri::command]
pub fn delete_backup(state: State<'_, AppState>, path: String) -> Result<()> {
    backup::delete_backup(&state, &path)
}

#[tauri::command]
pub fn move_storage(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<String> {
    backup::move_storage(&app, &state, &path)
}

#[tauri::command]
pub fn check_integrity(state: State<'_, AppState>) -> Result<IntegrityReport> {
    state.with_db(|conn| db::integrity_check(conn, &state.paths))
}

#[tauri::command]
pub fn picker_list(state: State<'_, AppState>, search: String, limit: Option<u32>) -> Result<Vec<ClipSummary>> {
    let query = ClipQuery {
        view: Some("all".into()),
        search: if search.trim().is_empty() {
            None
        } else {
            Some(search)
        },
        limit: Some(limit.unwrap_or(40).clamp(1, 100)),
        ..Default::default()
    };
    state
        .with_db(|conn| db::list_clips(conn, &query, &state.paths))
        .map(|page| page.items)
}

#[tauri::command]
pub fn hide_picker(app: AppHandle) {
    crate::shortcut::hide_picker(&app);
}

#[tauri::command]
pub fn show_main(app: AppHandle) {
    crate::tray::show_main(&app);
}

#[tauri::command]
pub fn refresh_tray(app: AppHandle) {
    crate::tray::refresh(&app);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn set_window_theme(app: AppHandle, theme: String) -> Result<()> {
    let value = match theme.as_str() {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        _ => None,
    };
    for label in ["main", "picker"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_theme(value);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_theme(app: AppHandle, theme: String) -> Result<()> {
    set_window_theme(app, theme)
}

fn explorer(args: &[&str]) -> Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("explorer")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|err| AppError::msg(format!("Could not open Explorer: {err}")))
}

#[tauri::command]
pub fn open_data_dir(state: State<'_, AppState>) -> Result<()> {
    explorer(&[&state.paths.root.to_string_lossy()])
}

/// Only allow Explorer shortcuts to the data folder or to a path that this
/// app actually captured as a file clip. Without this, any script running in
/// the webview could ask the app to open arbitrary folders.
fn ensure_path_allowed(state: &AppState, requested: &Path) -> Result<PathBuf> {
    let text = requested.to_string_lossy();
    if text.contains('"') || text.chars().any(|c| c.is_control()) {
        return Err(AppError::msg("That path contains unsupported characters"));
    }
    let canonical = std::fs::canonicalize(requested)
        .map_err(|_| AppError::msg("That path no longer exists"))?;
    if canonical.starts_with(&state.paths.root) {
        return Ok(canonical);
    }
    let known = state.with_db(|conn| {
        let mut stmt = conn.prepare("SELECT files_json FROM clips")?;
        let rows = stmt.query_map([], |row| row.get::<_, Option<String>>(0))?;
        for row in rows {
            let Some(json) = row? else { continue };
            let files: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap_or_default();
            for file in files {
                let Some(path) = file.get("path").and_then(|v| v.as_str()) else {
                    continue;
                };
                if std::fs::canonicalize(path).map(|p| p == canonical).unwrap_or(false) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    })?;
    if !known {
        return Err(AppError::msg("That path is not part of your ClipVault archive"));
    }
    Ok(canonical)
}

#[tauri::command]
pub fn open_path(state: State<'_, AppState>, path: String) -> Result<()> {
    let path = ensure_path_allowed(&state, &PathBuf::from(path))?;
    explorer(&[&path.to_string_lossy()])
}

#[tauri::command]
pub fn reveal_path(state: State<'_, AppState>, path: String) -> Result<()> {
    let path = ensure_path_allowed(&state, &PathBuf::from(path))?;
    let arg = format!("/select,\"{}\"", path.to_string_lossy());
    explorer(&[&arg])
}
