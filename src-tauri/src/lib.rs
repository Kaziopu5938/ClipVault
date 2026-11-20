mod backup;
mod clipboard;
mod commands;
mod db;
mod error;
mod models;
mod shortcut;
pub mod selftest;
pub mod selftest_support;
mod state;
mod storage;
mod tray;

use tauri::{Manager, WindowEvent};

fn init_logging(paths: &storage::StoragePaths) {
    let writer = tracing_appender::rolling::daily(&paths.logs, "clipvault.log");
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("clipvault=info,tauri=warn"));
    let _ = tracing_subscriber::fmt()
        .with_writer(writer)
        .with_env_filter(filter)
        .with_ansi(false)
        .try_init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let paths = storage::StoragePaths::resolve()?;
            init_logging(&paths);
            tracing::info!("ClipVault {} starting", env!("CARGO_PKG_VERSION"));

            let conn = db::open(&paths.db_file)?;
            let settings = db::load_settings(&conn)?;
            let state = state::AppState::new(paths, conn, settings.clone());
            app.manage(state);

            tray::create(app.handle())?;
            if let Err(err) = shortcut::register(app.handle()) {
                tracing::warn!("{err}");
            }
            clipboard::win::spawn_monitor(app.handle().clone());
            backup::spawn_scheduler(app.handle().clone());

            {
                use tauri_plugin_autostart::ManagerExt;
                let manager = app.autolaunch();
                let result = if settings.launch_at_startup {
                    manager.enable()
                } else {
                    manager.disable()
                };
                if let Err(err) = result {
                    tracing::warn!("could not update launch-at-startup: {err}");
                }
            }

            if std::env::args().any(|arg| arg == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label().to_string();
            let app = window.app_handle().clone();
            let settings = app.state::<state::AppState>().settings_snapshot();
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    if label == "main" {
                        if settings.close_to_tray {
                            api.prevent_close();
                            let _ = window.hide();
                        } else {
                            app.exit(0);
                        }
                    } else if label == "picker" {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
                WindowEvent::Resized(_) => {
                    if label == "main"
                        && settings.minimize_to_tray
                        && window.is_minimized().unwrap_or(false)
                    {
                        let _ = window.hide();
                    }
                }
                WindowEvent::Focused(false) => {
                    // A flyout dismisses when you click away from it.
                    if label == "picker" {
                        let _ = window.hide();
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_clips,
            commands::get_clip,
            commands::copy_clip,
            commands::copy_clip_plain,
            commands::toggle_pin,
            commands::toggle_favorite,
            commands::bulk_set_flag,
            commands::delete_clips,
            commands::clear_history,
            commands::get_stats,
            commands::get_settings,
            commands::save_settings,
            commands::set_monitoring,
            commands::get_monitoring,
            commands::get_app_info,
            commands::list_backups,
            commands::create_backup,
            commands::restore_backup,
            commands::import_archive,
            commands::export_archive,
            commands::delete_backup,
            commands::move_storage,
            commands::check_integrity,
            commands::picker_list,
            commands::hide_picker,
            commands::show_main,
            commands::refresh_tray,
            commands::quit_app,
            commands::set_window_theme,
            commands::set_theme,
            commands::open_data_dir,
            commands::open_path,
            commands::reveal_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ClipVault");
}
