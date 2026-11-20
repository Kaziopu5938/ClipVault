use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::db;
use crate::error::{AppError, Result};
use crate::models::{BackupInfo, Settings};
use crate::state::AppState;
use crate::storage::{copy_dir_recursive, now_ms, remove_dir_contents, StoragePaths};

const MAGIC: &[u8; 9] = b"CVBACKUP1";
const CHUNK: usize = 1024 * 1024;
const FORMAT: &str = "clipvault-backup";
const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ARCHIVE_ENTRY: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_TOTAL: u64 = 4 * 1024 * 1024 * 1024;

pub fn backup_dir(state: &AppState) -> PathBuf {
    let settings = state.settings_snapshot();
    let custom = settings.backup_dir.trim();
    if custom.is_empty() {
        state.paths.backups.clone()
    } else {
        PathBuf::from(custom)
    }
}

fn backup_name() -> String {
    format!(
        "clipvault-backup-{}.zip",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    )
}

pub fn create_backup(app: &AppHandle, state: &AppState) -> Result<BackupInfo> {
    let dir = backup_dir(state);
    fs::create_dir_all(&dir)?;
    let name = backup_name();
    let final_path = dir.join(&name);
    let temp_path = dir.join(format!(".{name}.{}.part", Uuid::new_v4().simple()));
    let snapshot_db = std::env::temp_dir().join(format!(
        "clipvault-snapshot-{}.db",
        Uuid::new_v4().simple()
    ));
    let _ = fs::remove_file(&snapshot_db);

    let (clips, settings) = state.with_db(|conn| {
        conn.execute(
            "VACUUM INTO ?1",
            rusqlite::params![snapshot_db.to_string_lossy().to_string()],
        )?;
        let clips: i64 = conn.query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))?;
        Ok((clips, db::load_settings(conn)?))
    })?;

    let result = write_archive(&final_path, &temp_path, state, &snapshot_db, &settings, clips);
    let _ = fs::remove_file(&snapshot_db);
    result?;

    let size = fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0);
    let created_at = now_ms();
    let _ = state.with_db(|conn| db::set_meta(conn, "last_backup_at", &created_at.to_string()));
    let _ = app.emit("backups-changed", ());

    Ok(BackupInfo {
        name,
        path: final_path.to_string_lossy().to_string(),
        size,
        created_at,
        encrypted: false,
    })
}

fn write_archive(
    final_path: &Path,
    temp_path: &Path,
    state: &AppState,
    snapshot_db: &Path,
    settings: &Settings,
    clips: i64,
) -> Result<()> {
    let file = File::create(temp_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    zip.start_file("manifest.json", options)?;
    let manifest = json!({
        "format": FORMAT,
        "version": 1,
        "schemaVersion": db::SCHEMA_VERSION,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "createdAt": now_ms(),
        "encrypted": false,
        "clips": clips,
    });
    zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;

    zip.start_file("settings.json", options)?;
    zip.write_all(&serde_json::to_vec_pretty(settings)?)?;

    zip.start_file("database/clipvault.db", options)?;
    let mut db_file = File::open(snapshot_db)?;
    std::io::copy(&mut db_file, &mut zip)?;

    for entry in WalkDir::new(&state.paths.assets).min_depth(1) {
        let entry = entry.map_err(|e| AppError::msg(format!("asset walk failed: {e}")))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(&state.paths.assets)
            .map_err(|_| AppError::msg("invalid asset path"))?;
        let archive_name = format!("assets/{}", relative.to_string_lossy().replace('\\', "/"));
        zip.start_file(archive_name, options)?;
        let mut asset = File::open(entry.path())?;
        std::io::copy(&mut asset, &mut zip)?;
    }

    zip.finish()?;
    fs::rename(temp_path, final_path)?;
    Ok(())
}

pub fn list_backups(state: &AppState) -> Result<Vec<BackupInfo>> {
    let dir = backup_dir(state);
    let mut items = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let encrypted = name.ends_with(".cvbackup");
            let is_backup = (name.starts_with("clipvault-backup-") && name.ends_with(".zip")) || encrypted;
            if !is_backup {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            items.push(BackupInfo {
                name,
                path: path.to_string_lossy().to_string(),
                size: metadata.len(),
                created_at: crate::storage::file_modified_ms(&path).unwrap_or(0),
                encrypted,
            });
        }
    }
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}

pub fn delete_backup(state: &AppState, path: &str) -> Result<()> {
    let target = PathBuf::from(path);
    let dir = backup_dir(state);
    let target_abs = fs::canonicalize(&target).unwrap_or_else(|_| target.clone());
    let dir_abs = fs::canonicalize(&dir).unwrap_or_else(|_| dir.clone());
    if !target_abs.starts_with(&dir_abs) {
        return Err(AppError::msg(
            "Only files inside the ClipVault backup folder can be deleted here",
        ));
    }
    if target_abs.is_file() {
        fs::remove_file(target_abs)?;
    }
    Ok(())
}

pub fn prune(state: &AppState, keep: u32) -> Result<()> {
    let dir = backup_dir(state);
    let mut files: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                path.is_file() && name.starts_with("clipvault-backup-")
            })
            .collect(),
        Err(_) => return Ok(()),
    };
    files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    for path in files.into_iter().skip(keep.max(1) as usize) {
        if let Err(err) = fs::remove_file(&path) {
            tracing::warn!("could not prune backup {}: {err}", path.display());
        }
    }
    Ok(())
}

pub fn maybe_auto_backup(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.settings_snapshot();
    if !settings.backup_enabled {
        return;
    }
    let last = state
        .with_db(|conn| db::get_meta(conn, "last_backup_at"))
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    let interval = (settings.backup_frequency_hours as i64).saturating_mul(3_600_000);
    if now_ms() - last < interval {
        return;
    }
    match create_backup(app, &state) {
        Ok(info) => {
            tracing::info!("automatic backup created: {}", info.name);
            let _ = prune(&state, settings.backup_keep);
        }
        Err(err) => tracing::warn!("automatic backup failed: {err}"),
    }
}

pub fn spawn_scheduler(app: AppHandle) {
    std::thread::Builder::new()
        .name("clipvault-backups".into())
        .spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(45));
            loop {
                maybe_auto_backup(&app);
                std::thread::sleep(std::time::Duration::from_secs(300));
            }
        })
        .ok();
}

pub fn export_archive(
    app: &AppHandle,
    state: &AppState,
    target: &str,
    password: Option<&str>,
) -> Result<String> {
    let target = PathBuf::from(target);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let info = create_backup(app, state)?;
    let zip_path = PathBuf::from(&info.path);
    let temp_zip = zip_path.clone();
    let result = match password {
        Some(password) if !password.is_empty() => {
            let temp_out = target.with_extension("cvbackup-part");
            encrypt_file(&temp_zip, &temp_out, password)?;
            if target.exists() {
                let _ = fs::remove_file(&target);
            }
            fs::rename(&temp_out, &target)?;
            Ok(target.to_string_lossy().to_string())
        }
        _ => {
            if target.exists() {
                let _ = fs::remove_file(&target);
            }
            fs::copy(&temp_zip, &target)?;
            Ok(target.to_string_lossy().to_string())
        }
    };
    let _ = fs::remove_file(&zip_path);
    result
}

pub fn restore_archive(
    app: &AppHandle,
    state: &AppState,
    path: &str,
    password: Option<&str>,
) -> Result<()> {
    let source = PathBuf::from(path);
    if !source.is_file() {
        return Err(AppError::msg("That archive file does not exist"));
    }

    let mut temp_zip: Option<PathBuf> = None;
    let zip_path = if is_encrypted(&source)? {
        let password = password.unwrap_or("").trim();
        if password.is_empty() {
            return Err(AppError::PasswordRequired);
        }
        let decrypted = std::env::temp_dir().join(format!(
            "clipvault-restore-{}.zip",
            Uuid::new_v4().simple()
        ));
        decrypt_file(&source, &decrypted, password)?;
        temp_zip = Some(decrypted.clone());
        decrypted
    } else {
        source.clone()
    };

    let outcome = restore_from_zip(app, state, &zip_path);
    if let Some(temp) = temp_zip {
        let _ = fs::remove_file(temp);
    }
    outcome
}

fn read_small_entry(entry: &mut zip::read::ZipFile<'_, File>, max: u64) -> Result<Vec<u8>> {
    if entry.size() > max {
        return Err(AppError::msg("an archive metadata entry is too large"));
    }
    let mut buffer = Vec::new();
    entry.take(max + 1).read_to_end(&mut buffer)?;
    if buffer.len() as u64 > max {
        return Err(AppError::msg("an archive metadata entry is too large"));
    }
    Ok(buffer)
}

fn copy_budgeted(
    entry: &mut zip::read::ZipFile<'_, File>,
    out: &mut File,
    budget: &mut u64,
) -> Result<()> {
    let size = entry.size();
    if size > MAX_ARCHIVE_ENTRY || budget.saturating_add(size) > MAX_ARCHIVE_TOTAL {
        return Err(AppError::msg("archive contents exceed the extraction limit"));
    }
    *budget = budget.saturating_add(size);
    let copied = std::io::copy(&mut entry.take(size + 1), out)?;
    if copied > size {
        return Err(AppError::msg("an archive entry exceeded its declared size"));
    }
    Ok(())
}

/// Reject archives whose database contains paths that could escape our asset
/// directories or UUIDs that could be used as traversal in file names.
fn validate_archive_db(conn: &rusqlite::Connection) -> Result<()> {
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if !integrity.eq_ignore_ascii_case("ok") {
        return Err(AppError::msg("the archived database failed its integrity check"));
    }
    let mut stmt = conn.prepare(
        "SELECT uuid, image_path, thumb_path, preview_path, metadata_json FROM clips",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;
    for row in rows {
        let (uuid, image, thumb, preview, metadata) = row?;
        if crate::storage::safe_uuid(&uuid).is_none() {
            return Err(AppError::msg("the archive contains an invalid clip identifier"));
        }
        for path in [image, thumb, preview].into_iter().flatten() {
            if crate::storage::safe_relative_path(&path).is_none() {
                return Err(AppError::msg("the archive contains an unsafe asset path"));
            }
        }
        if let Some(metadata) = metadata {
            let value: serde_json::Value = serde_json::from_str(&metadata).unwrap_or(serde_json::Value::Null);
            if let Some(formats) = value.get("extraFormats").and_then(|v| v.as_array()) {
                for format in formats {
                    let Some(path) = format.get("path").and_then(|v| v.as_str()) else {
                        return Err(AppError::msg("the archive has malformed format metadata"));
                    };
                    if crate::storage::safe_relative_path(path).is_none() {
                        return Err(AppError::msg("the archive contains an unsafe attachment path"));
                    }
                }
            }
        }
    }
    Ok(())
}

fn restore_from_zip(app: &AppHandle, state: &AppState, zip_path: &Path) -> Result<()> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    let manifest: serde_json::Value = {
        let mut entry = archive
            .by_name("manifest.json")
            .map_err(|_| AppError::msg("This file is not a ClipVault archive (manifest missing)"))?;
        let buffer = read_small_entry(&mut entry, MAX_MANIFEST_BYTES)?;
        serde_json::from_slice(&buffer)?
    };
    if manifest.get("format").and_then(|v| v.as_str()) != Some(FORMAT) {
        return Err(AppError::msg("This archive was not created by ClipVault"));
    }
    let archive_schema = manifest
        .get("schemaVersion")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    if archive_schema > db::SCHEMA_VERSION {
        return Err(AppError::msg(
            "This archive was created by a newer version of ClipVault",
        ));
    }

    // Stage everything in a private folder. The live archive stays untouched
    // until the incoming database and assets have been fully validated.
    let staging = state
        .paths
        .root
        .join(format!(".restore-{}", Uuid::new_v4().simple()));
    fs::create_dir_all(&staging)?;
    let outcome = restore_staged(&mut archive, app, state, &staging);
    if outcome.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    outcome
}

fn restore_staged(
    archive: &mut ZipArchive<File>,
    app: &AppHandle,
    state: &AppState,
    staging: &Path,
) -> Result<()> {
    let mut budget = 0u64;
    let mut settings_bytes: Option<Vec<u8>> = None;
    let mut found_db = false;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(name) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };
        let normalized = name.to_string_lossy().replace('\\', "/");
        if normalized == "settings.json" {
            settings_bytes = Some(read_small_entry(&mut entry, MAX_MANIFEST_BYTES)?);
            continue;
        }
        if normalized == "database/clipvault.db" {
            let target = staging.join("database").join("clipvault.db");
            fs::create_dir_all(target.parent().expect("parent exists"))?;
            let mut out = File::create(&target)?;
            copy_budgeted(&mut entry, &mut out, &mut budget)?;
            out.sync_all()?;
            found_db = true;
            continue;
        }
        if let Some(rest) = normalized.strip_prefix("assets/") {
            let Some(relative) = crate::storage::safe_relative_path(rest) else {
                continue;
            };
            let target = staging.join("assets").join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = File::create(&target)?;
            copy_budgeted(&mut entry, &mut out, &mut budget)?;
        }
    }

    if !found_db {
        return Err(AppError::msg(
            "The archive does not contain a ClipVault database",
        ));
    }

    // Validate the staged database before replacing anything.
    {
        let staged_db = staging.join("database").join("clipvault.db");
        let conn = db::open(&staged_db)?;
        validate_archive_db(&conn)?;
    }

    let mut settings = settings_bytes
        .and_then(|bytes| serde_json::from_slice::<Settings>(&bytes).ok())
        .map(|s| s.normalized())
        .unwrap_or_default();
    // Never let an archive point our backup folder somewhere else.
    settings.backup_dir = state.settings_snapshot().backup_dir;

    // Best-effort safety snapshot while our own database is still open.
    if let Err(err) = create_backup(app, state) {
        tracing::warn!("could not create a safety backup before restore: {err}");
    }

    {
        let mut guard = state
            .db
            .lock()
            .map_err(|_| AppError::msg("Database lock is poisoned"))?;
        let conn = guard.take();
        drop(conn);
    }

    let swap = (|| -> Result<()> {
        remove_dir_contents(&state.paths.database_dir)?;
        remove_dir_contents(&state.paths.images)?;
        remove_dir_contents(&state.paths.thumbnails)?;
        remove_dir_contents(&state.paths.attachments)?;
        remove_dir_contents(&state.paths.files)?;

        fs::copy(
            staging.join("database").join("clipvault.db"),
            &state.paths.db_file,
        )?;
        copy_dir_recursive(&staging.join("assets"), &state.paths.assets)?;

        let conn = db::open(&state.paths.db_file)?;
        db::save_settings(&conn, &settings)?;
        *state
            .settings
            .lock()
            .map_err(|_| AppError::msg("Settings lock is poisoned"))? = settings.clone();
        state
            .monitoring
            .store(settings.monitoring_enabled, std::sync::atomic::Ordering::Relaxed);
        *state
            .db
            .lock()
            .map_err(|_| AppError::msg("Database lock is poisoned"))? = Some(conn);
        Ok(())
    })();

    if let Err(err) = swap {
        // Even a failed swap must not leave every command complaining that the
        // database is closed: reopen an empty database if needed.
        if let Ok(mut guard) = state.db.lock() {
            if guard.is_none() {
                *guard = db::open(&state.paths.db_file).ok();
            }
        }
        return Err(err);
    }

    let _ = fs::remove_dir_all(staging);
    let _ = app.emit("data-restored", ());
    let _ = app.emit("settings-changed", ());
    Ok(())
}

fn is_encrypted(path: &Path) -> Result<bool> {
    let mut file = File::open(path)?;
    let mut header = [0u8; 9];
    let read = file.read(&mut header)?;
    Ok(read == 9 && &header == MAGIC)
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let params = Params::new(64 * 1024, 3, 1, Some(32))
        .map_err(|e| AppError::msg(format!("key derivation failed: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| AppError::msg(format!("key derivation failed: {e}")))?;
    Ok(key)
}

fn encrypt_file(source: &Path, target: &Path, password: &str) -> Result<()> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    rand::rng().fill_bytes(&mut salt);
    rand::rng().fill_bytes(&mut nonce);
    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| AppError::msg("could not initialise encryption"))?;

    let mut input = File::open(source)?;
    let mut output = File::create(target)?;
    output.write_all(MAGIC)?;
    output.write_all(&salt)?;
    output.write_all(&nonce)?;
    output.write_all(&(CHUNK as u32).to_le_bytes())?;

    let mut buffer = vec![0u8; CHUNK];
    let mut counter: u32 = 0;
    loop {
        let read = read_chunk(&mut input, &mut buffer)?;
        if read == 0 {
            break;
        }
        let mut chunk_nonce = nonce;
        chunk_nonce[8..12].copy_from_slice(&counter.to_be_bytes());
        let aad = counter.to_be_bytes();
        let encrypted = cipher
            .encrypt(
                Nonce::from_slice(&chunk_nonce),
                Payload {
                    msg: &buffer[..read],
                    aad: &aad,
                },
            )
            .map_err(|_| AppError::msg("encryption failed"))?;
        output.write_all(&encrypted)?;
        counter = counter.wrapping_add(1);
    }
    output.sync_all()?;
    Ok(())
}

fn decrypt_file(source: &Path, target: &Path, password: &str) -> Result<()> {
    let mut input = File::open(source)?;
    let mut header = [0u8; 9];
    input.read_exact(&mut header)?;
    if &header != MAGIC {
        return Err(AppError::msg("This is not an encrypted ClipVault archive"));
    }
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    let mut chunk_bytes = [0u8; 4];
    input.read_exact(&mut salt)?;
    input.read_exact(&mut nonce)?;
    input.read_exact(&mut chunk_bytes)?;
    let chunk = u32::from_le_bytes(chunk_bytes) as usize;
    if chunk == 0 || chunk > 64 * 1024 * 1024 {
        return Err(AppError::msg("The archive header is corrupted"));
    }
    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| AppError::msg("could not initialise decryption"))?;

    let mut output = File::create(target)?;
    let mut buffer = vec![0u8; chunk + 16];
    let mut counter: u32 = 0;
    loop {
        let read = read_chunk(&mut input, &mut buffer)?;
        if read == 0 {
            break;
        }
        if read < 16 {
            return Err(AppError::msg("The archive is truncated"));
        }
        let mut chunk_nonce = nonce;
        chunk_nonce[8..12].copy_from_slice(&counter.to_be_bytes());
        let aad = counter.to_be_bytes();
        let decrypted = cipher
            .decrypt(
                Nonce::from_slice(&chunk_nonce),
                Payload {
                    msg: &buffer[..read],
                    aad: &aad,
                },
            )
            .map_err(|_| {
                AppError::msg("Incorrect password, or the archive is corrupted")
            })?;
        output.write_all(&decrypted)?;
        counter = counter.wrapping_add(1);
    }
    output.sync_all()?;
    Ok(())
}

fn read_chunk(input: &mut File, buffer: &mut [u8]) -> Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match input.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(AppError::Io(err)),
        }
    }
    Ok(filled)
}

pub fn move_storage(app: &AppHandle, state: &AppState, target: &str) -> Result<String> {
    let target = PathBuf::from(target.trim());
    if target.as_os_str().is_empty() {
        return Err(AppError::msg("Choose a folder first"));
    }
    fs::create_dir_all(&target)?;
    let root = state.paths.root.clone();
    let target_abs = fs::canonicalize(&target).unwrap_or(target.clone());
    let root_abs = fs::canonicalize(&root).unwrap_or(root.clone());
    if target_abs == root_abs {
        return Err(AppError::msg("That folder is already the ClipVault data folder"));
    }
    if target_abs.starts_with(&root_abs) {
        return Err(AppError::msg(
            "Choose a folder outside the current ClipVault data folder",
        ));
    }

    {
        let mut guard = state
            .db
            .lock()
            .map_err(|_| AppError::msg("Database lock is poisoned"))?;
        let conn = guard.take();
        drop(conn);
    }

    let moved = (|| -> Result<()> {
        copy_dir_recursive(&root_abs, &target_abs)?;
        let new_paths = StoragePaths::from_root(target_abs.clone());
        new_paths.ensure()?;
        // Verify the copy before pointing the app at it.
        {
            let conn = db::open(&new_paths.db_file)?;
            let integrity: String =
                conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
            if !integrity.eq_ignore_ascii_case("ok") {
                return Err(AppError::msg("the copied database failed its integrity check"));
            }
        }
        state.paths.write_storage_config(Some(&target_abs))?;
        let conn = db::open(&new_paths.db_file)?;
        *state
            .db
            .lock()
            .map_err(|_| AppError::msg("Database lock is poisoned"))? = Some(conn);
        Ok(())
    })();

    if let Err(err) = moved {
        // Never leave the running app with no database after a failed move.
        if let Ok(mut guard) = state.db.lock() {
            if guard.is_none() {
                *guard = db::open(&state.paths.db_file).ok();
            }
        }
        return Err(err);
    }

    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(600));
        handle.restart();
    });

    Ok(target_abs.to_string_lossy().to_string())
}
