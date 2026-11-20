use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{AppError, Result};
use crate::models::Settings;
use crate::storage::StoragePaths;

pub struct AppState {
    pub paths: StoragePaths,
    pub db: Mutex<Option<Connection>>,
    pub settings: Mutex<Settings>,
    pub monitoring: AtomicBool,
    /// Last clipboard sequence number written by ClipVault itself, so restoring a
    /// clip does not immediately capture it again.
    pub own_sequence: AtomicU32,
    asset_cache: Mutex<Option<(i64, u64)>>,
}

impl AppState {
    pub fn new(paths: StoragePaths, conn: Connection, settings: Settings) -> Self {
        let monitoring = AtomicBool::new(settings.monitoring_enabled);
        Self {
            paths,
            db: Mutex::new(Some(conn)),
            settings: Mutex::new(settings),
            monitoring,
            own_sequence: AtomicU32::new(0),
            asset_cache: Mutex::new(None),
        }
    }

    /// Asset folder size, cached for 30 seconds so stats calls stay cheap.
    pub fn asset_bytes(&self) -> u64 {
        let now = crate::storage::now_ms();
        if let Ok(cache) = self.asset_cache.lock() {
            if let Some((at, bytes)) = *cache {
                if now - at < 30_000 {
                    return bytes;
                }
            }
        }
        let bytes = self.paths.assets_size();
        if let Ok(mut cache) = self.asset_cache.lock() {
            *cache = Some((now, bytes));
        }
        bytes
    }

    pub fn invalidate_assets(&self) {
        if let Ok(mut cache) = self.asset_cache.lock() {
            *cache = None;
        }
    }

    pub fn settings_snapshot(&self) -> Settings {
        self.settings
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    pub fn with_db<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut guard = self
            .db
            .lock()
            .map_err(|_| AppError::msg("Database lock is poisoned"))?;
        let conn = guard
            .as_mut()
            .ok_or_else(|| AppError::msg("The database is currently closed for maintenance"))?;
        f(conn)
    }
}
