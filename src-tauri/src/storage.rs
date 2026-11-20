use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::error::{AppError, Result};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn file_modified_ms(path: &Path) -> Option<i64> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
}

pub fn local_appdata() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir())
}

pub fn default_root() -> PathBuf {
    local_appdata().join("ClipVault")
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Bootstrap {
    storage_dir: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoragePaths {
    pub root: PathBuf,
    pub database_dir: PathBuf,
    pub db_file: PathBuf,
    pub assets: PathBuf,
    pub images: PathBuf,
    pub thumbnails: PathBuf,
    pub files: PathBuf,
    pub attachments: PathBuf,
    pub backups: PathBuf,
    pub logs: PathBuf,
    pub settings_dir: PathBuf,
    pub config_file: PathBuf,
}

impl StoragePaths {
    pub fn resolve() -> Result<Self> {
        let fallback = default_root();
        let config_file = fallback.join("config.json");
        let mut root = fallback.clone();

        if let Ok(bytes) = fs::read(&config_file) {
            if let Ok(config) = serde_json::from_slice::<Bootstrap>(&bytes) {
                if let Some(dir) = config.storage_dir {
                    let trimmed = dir.trim();
                    if !trimmed.is_empty() {
                        root = PathBuf::from(trimmed);
                    }
                }
            }
        }

        if fs::create_dir_all(&root).is_err() && root != fallback {
            tracing::warn!("storage folder unavailable, falling back to default");
            root = fallback;
        }

        let paths = Self::from_root(root);
        paths.ensure()?;
        Ok(paths)
    }

    pub fn from_root(root: PathBuf) -> Self {
        let database_dir = root.join("database");
        let assets = root.join("assets");
        let config_file = default_root().join("config.json");
        Self {
            db_file: database_dir.join("clipvault.db"),
            database_dir,
            images: assets.join("images"),
            thumbnails: assets.join("thumbnails"),
            files: assets.join("files"),
            attachments: assets.join("attachments"),
            assets,
            backups: root.join("backups"),
            logs: root.join("logs"),
            settings_dir: root.join("settings"),
            config_file,
            root,
        }
    }

    pub fn ensure(&self) -> Result<()> {
        for dir in [
            &self.root,
            &self.database_dir,
            &self.assets,
            &self.images,
            &self.thumbnails,
            &self.files,
            &self.attachments,
            &self.backups,
            &self.logs,
            &self.settings_dir,
        ] {
            fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    pub fn write_storage_config(&self, custom: Option<&Path>) -> Result<()> {
        if let Some(parent) = self.config_file.parent() {
            fs::create_dir_all(parent)?;
        }
        let config = Bootstrap {
            storage_dir: custom.map(|p| p.to_string_lossy().to_string()),
        };
        let bytes = serde_json::to_vec_pretty(&config)?;
        atomic_write(&self.config_file, &bytes)
    }

    /// Resolve an asset path stored in the database or an archive. Paths are
    /// untrusted input: absolute paths, parent segments and prefixes must never
    /// be allowed to escape the assets directory.
    pub fn try_asset_path(&self, relative: &str) -> Option<PathBuf> {
        let path = safe_relative_path(relative)?;
        Some(self.assets.join(path))
    }

    pub fn asset_path(&self, relative: &str) -> PathBuf {
        self.try_asset_path(relative)
            .unwrap_or_else(|| self.assets.join("__invalid__"))
    }

    pub fn db_size(&self) -> u64 {
        [self.db_file.clone(), self.db_file.with_extension("db-wal"), self.db_file.with_extension("db-shm")]
            .iter()
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum()
    }

    pub fn assets_size(&self) -> u64 {
        WalkDir::new(&self.assets)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    }
}

/// Validate a path that came from a database row, archive entry or IPC call.
/// Only plain relative paths made of normal components are accepted.
pub fn safe_relative_path(value: &str) -> Option<PathBuf> {
    if value.is_empty() || value.contains('\0') || value.len() > 1024 {
        return None;
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return None;
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return None;
        }
    }
    Some(path.to_path_buf())
}

/// Validate a UUID before it is used as a directory or file name.
pub fn safe_uuid(value: &str) -> Option<String> {
    let parsed = uuid::Uuid::parse_str(value).ok()?;
    Some(parsed.hyphenated().to_string())
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}",
        uuid::Uuid::new_v4().simple()
    ));
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(AppError::Io(e))
        }
    }
}

pub fn data_url_from_file(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/png;base64,{encoded}"))
}

pub fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in WalkDir::new(from).min_depth(1) {
        let entry = entry.map_err(|e| AppError::msg(format!("walk error: {e}")))?;
        let relative = entry
            .path()
            .strip_prefix(from)
            .map_err(|_| AppError::msg("invalid path"))?;
        let target = to.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

pub fn remove_dir_contents(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let target = entry.path();
        if target.is_dir() {
            fs::remove_dir_all(target)?;
        } else {
            fs::remove_file(target)?;
        }
    }
    Ok(())
}

