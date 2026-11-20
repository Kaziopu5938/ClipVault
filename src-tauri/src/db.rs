use std::path::Path;
use std::time::Duration;

use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::models::{
    ClipDetail, ClipFile, ClipFormat, ClipPage, ClipQuery, ClipSummary, IntegrityReport, Settings, Stats,
};
use crate::storage::{data_url_from_file, now_ms, StoragePaths};

pub const SCHEMA_VERSION: i64 = 1;

const SELECT_COLUMNS: &str = "c.id, c.kind, c.created_at, c.last_used_at, c.use_count, c.pinned, c.favorite, \
    c.source_app, c.source_title, c.preview, c.byte_size, c.image_width, c.image_height, c.thumb_path, \
    c.files_json, c.formats_json, (c.text_content IS NOT NULL) AS has_text, \
    (c.html_content IS NOT NULL) AS has_html, (c.rtf_content IS NOT NULL) AS has_rtf";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewClip {
    pub uuid: String,
    pub kind: String,
    pub created_at: i64,
    pub source_app: Option<String>,
    pub source_title: Option<String>,
    pub content_hash: String,
    pub text_content: Option<String>,
    pub html_content: Option<String>,
    pub rtf_content: Option<String>,
    pub preview: String,
    pub byte_size: i64,
    pub image_path: Option<String>,
    pub thumb_path: Option<String>,
    pub preview_path: Option<String>,
    pub image_width: Option<i64>,
    pub image_height: Option<i64>,
    pub files: Vec<ClipFile>,
    pub formats: Vec<ClipFormat>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ClipRecord {
    pub id: i64,
    pub uuid: String,
    pub kind: String,
    pub text_content: Option<String>,
    pub html_content: Option<String>,
    pub rtf_content: Option<String>,
    pub preview: String,
    pub image_path: Option<String>,
    pub files: Vec<ClipFile>,
    pub formats: Vec<ClipFormat>,
    pub source_title: Option<String>,
    pub metadata: serde_json::Value,
}

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(8))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "wal_autocheckpoint", 1000)?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < 1 {
        conn.execute_batch(
            r#"
            BEGIN IMMEDIATE;
            CREATE TABLE IF NOT EXISTS clips (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT NOT NULL UNIQUE,
                kind TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_used_at INTEGER,
                use_count INTEGER NOT NULL DEFAULT 0,
                pinned INTEGER NOT NULL DEFAULT 0,
                favorite INTEGER NOT NULL DEFAULT 0,
                source_app TEXT,
                source_title TEXT,
                content_hash TEXT NOT NULL,
                text_content TEXT,
                html_content TEXT,
                rtf_content TEXT,
                preview TEXT NOT NULL DEFAULT '',
                byte_size INTEGER NOT NULL DEFAULT 0,
                image_path TEXT,
                thumb_path TEXT,
                preview_path TEXT,
                image_width INTEGER,
                image_height INTEGER,
                files_json TEXT,
                formats_json TEXT,
                metadata_json TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_clips_created ON clips(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_clips_kind ON clips(kind, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_clips_hash ON clips(content_hash);
            CREATE INDEX IF NOT EXISTS idx_clips_last_used ON clips(last_used_at DESC);
            CREATE INDEX IF NOT EXISTS idx_clips_pinned ON clips(pinned, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_clips_favorite ON clips(favorite, created_at DESC);
            CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
                text, filenames, urls, tokenize = 'unicode61 remove_diacritics 2'
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            PRAGMA user_version = 1;
            COMMIT;
            "#,
        )?;
    }
    Ok(())
}

pub fn load_settings(conn: &Connection) -> Result<Settings> {
    let raw: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = 'app'", [], |row| row.get(0))
        .optional()?;
    let settings = raw
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .unwrap_or_default()
        .normalized();
    Ok(settings)
}

pub fn save_settings(conn: &Connection, settings: &Settings) -> Result<()> {
    let value = serde_json::to_string(settings)?;
    conn.execute(
        "INSERT INTO settings(key, value) VALUES('app', ?1) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![value],
    )?;
    Ok(())
}

pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |row| row.get(0))
        .optional()?)
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES(?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn insert_clip(conn: &mut Connection, clip: &NewClip) -> Result<i64> {
    let files_json = serde_json::to_string(&clip.files)?;
    let formats_json = serde_json::to_string(&clip.formats)?;
    let metadata_json = serde_json::to_string(&clip.metadata)?;
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO clips (
            uuid, kind, created_at, last_used_at, use_count, pinned, favorite, source_app, source_title,
            content_hash, text_content, html_content, rtf_content, preview, byte_size,
            image_path, thumb_path, preview_path, image_width, image_height,
            files_json, formats_json, metadata_json
         ) VALUES (?1, ?2, ?3, NULL, 0, 0, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        params![
            clip.uuid,
            clip.kind,
            clip.created_at,
            clip.source_app,
            clip.source_title,
            clip.content_hash,
            clip.text_content,
            clip.html_content,
            clip.rtf_content,
            clip.preview,
            clip.byte_size,
            clip.image_path,
            clip.thumb_path,
            clip.preview_path,
            clip.image_width,
            clip.image_height,
            files_json,
            formats_json,
            metadata_json,
        ],
    )?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO clips_fts(rowid, text, filenames, urls) VALUES (?1, ?2, ?3, ?4)",
        params![
            id,
            clip.text_content.clone().unwrap_or_else(|| clip.preview.clone()),
            filenames_for_search(&clip.files),
            urls_for_search(clip.text_content.as_deref()),
        ],
    )?;
    tx.commit()?;
    Ok(id)
}

pub fn latest_hash(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT content_hash FROM clips ORDER BY id DESC LIMIT 1", [], |row| row.get(0))
        .optional()?)
}

pub fn hash_exists(conn: &Connection, hash: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM clips WHERE content_hash = ?1 LIMIT 1",
        params![hash],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn build_fts_query(search: &str) -> Option<String> {
    let tokens: Vec<String> = search
        .split_whitespace()
        .filter(|token| token.chars().any(|c| c.is_alphanumeric()))
        .take(12)
        .map(|token| {
            let cleaned = token.replace('"', "\"\"");
            format!("\"{cleaned}\"*")
        })
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" AND "))
    }
}

fn filenames_for_search(files: &[ClipFile]) -> String {
    files
        .iter()
        .map(|f| format!("{} {}", f.name, f.path))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn urls_for_search(text: Option<&str>) -> String {
    let Some(text) = text else { return String::new() };
    text.split_whitespace()
        .filter(|token| {
            let t = token.trim_matches(|c: char| "()[]{}<>,;\"'".contains(c));
            t.starts_with("http://") || t.starts_with("https://") || t.starts_with("www.") || t.starts_with("mailto:")
        })
        .take(40)
        .collect::<Vec<_>>()
        .join("\n")
}

fn row_to_summary(row: &Row<'_>, paths: &StoragePaths) -> rusqlite::Result<ClipSummary> {
    let thumb: Option<String> = row.get("thumb_path")?;
    let thumbnail = thumb
        .as_deref()
        .and_then(|p| paths.try_asset_path(p))
        .and_then(|path| data_url_from_file(&path));
    let files_json: Option<String> = row.get("files_json")?;
    let formats_json: Option<String> = row.get("formats_json")?;
    let files: Vec<ClipFile> = files_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let formats: Vec<ClipFormat> = formats_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    Ok(ClipSummary {
        id: row.get("id")?,
        kind: row.get("kind")?,
        created_at: row.get("created_at")?,
        last_used_at: row.get("last_used_at")?,
        use_count: row.get("use_count")?,
        pinned: row.get::<_, i64>("pinned")? != 0,
        favorite: row.get::<_, i64>("favorite")? != 0,
        source_app: row.get("source_app")?,
        source_title: row.get("source_title")?,
        preview: row.get("preview")?,
        byte_size: row.get("byte_size")?,
        image_width: row.get("image_width")?,
        image_height: row.get("image_height")?,
        thumbnail,
        files,
        formats,
        has_html: row.get::<_, i64>("has_html")? != 0,
        has_rtf: row.get::<_, i64>("has_rtf")? != 0,
        has_full_text: row.get::<_, i64>("has_text")? != 0,
    })
}

pub fn list_clips(conn: &Connection, query: &ClipQuery, paths: &StoragePaths) -> Result<ClipPage> {
    let limit = query.limit.unwrap_or(80).clamp(1, 500) as i64;
    let offset = query.offset.unwrap_or(0) as i64;
    let search = query.search.as_deref().unwrap_or("").trim();
    let fts = build_fts_query(search);

    let mut from = "FROM clips c".to_string();
    let mut where_parts: Vec<String> = Vec::new();
    let mut values: Vec<Value> = Vec::new();

    if let Some(fts_query) = &fts {
        from.push_str(" JOIN clips_fts f ON f.rowid = c.id");
        where_parts.push("clips_fts MATCH ?".into());
        values.push(Value::Text(fts_query.clone()));
    }

    if let Some(kind) = query.kind.as_deref() {
        if matches!(kind, "text" | "image" | "files" | "link") {
            where_parts.push("c.kind = ?".into());
            values.push(Value::Text(kind.into()));
        }
    }

    match query.view.as_deref() {
        Some("favorites") => where_parts.push("c.favorite = 1".into()),
        Some("pinned") => where_parts.push("c.pinned = 1".into()),
        Some("recent") => where_parts.push("c.last_used_at IS NOT NULL".into()),
        _ => {}
    }

    if let Some(from_ts) = query.from {
        where_parts.push("c.created_at >= ?".into());
        values.push(Value::Integer(from_ts));
    }
    if let Some(to_ts) = query.to {
        where_parts.push("c.created_at <= ?".into());
        values.push(Value::Integer(to_ts));
    }

    let where_sql = if where_parts.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_parts.join(" AND "))
    };

    let order = match query.sort.as_deref() {
        Some("oldest") => "c.created_at ASC, c.id ASC",
        Some("used") => "c.use_count DESC, COALESCE(c.last_used_at, 0) DESC, c.created_at DESC",
        _ if query.view.as_deref() == Some("recent") => "COALESCE(c.last_used_at, c.created_at) DESC, c.id DESC",
        _ => "c.created_at DESC, c.id DESC",
    };

    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) {from}{where_sql}"),
        params_from_iter(values.iter()),
        |row| row.get(0),
    )?;

    let sql = format!("SELECT {SELECT_COLUMNS} {from}{where_sql} ORDER BY {order} LIMIT ? OFFSET ?");
    let mut page_values = values.clone();
    page_values.push(Value::Integer(limit + 1));
    page_values.push(Value::Integer(offset));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(page_values.iter()), |row| row_to_summary(row, paths))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    let has_more = items.len() as i64 > limit;
    if has_more {
        items.truncate(limit as usize);
    }

    Ok(ClipPage {
        items,
        has_more,
        total: Some(total),
    })
}

pub fn get_detail(conn: &Connection, paths: &StoragePaths, id: i64) -> Result<ClipDetail> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS}, c.text_content, c.html_content, c.rtf_content, c.image_path, c.preview_path \
         FROM clips c WHERE c.id = ?1"
    ))?;
    let detail = stmt.query_row(params![id], |row| {
        let base = row_to_summary(row, paths)?;
        let image_path: Option<String> = row.get("image_path")?;
        let preview_path: Option<String> = row.get("preview_path")?;
        let display_path = preview_path.as_deref().or(image_path.as_deref());
        let image = display_path
            .and_then(|p| paths.try_asset_path(p))
            .and_then(|path| data_url_from_file(&path));
        Ok(ClipDetail {
            base,
            text_content: clip_text(row.get("text_content")?),
            html_content: clip_text(row.get("html_content")?),
            rtf_content: clip_text(row.get("rtf_content")?),
            image: image.clone(),
            image_preview: image,
        })
    })?;
    Ok(detail)
}

fn clip_text(value: Option<String>) -> Option<String> {
    const LIMIT: usize = 2_000_000;
    value.map(|text| {
        if text.len() > LIMIT {
            let mut cut = LIMIT;
            while cut > 0 && !text.is_char_boundary(cut) {
                cut -= 1;
            }
            format!(
                "{}\n\n[Preview truncated at 2 MB. Copy restores the complete content.]",
                &text[..cut]
            )
        } else {
            text
        }
    })
}

pub fn get_record(conn: &Connection, id: i64) -> Result<ClipRecord> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, kind, text_content, html_content, rtf_content, preview, image_path, files_json, formats_json, source_title, metadata_json \
         FROM clips WHERE id = ?1",
    )?;
    let record = stmt.query_row(params![id], |row| {
        let files_json: Option<String> = row.get("files_json")?;
        let formats_json: Option<String> = row.get("formats_json")?;
        let metadata_json: Option<String> = row.get("metadata_json")?;
        Ok(ClipRecord {
            id: row.get("id")?,
            uuid: row.get("uuid")?,
            kind: row.get("kind")?,
            text_content: row.get("text_content")?,
            html_content: row.get("html_content")?,
            rtf_content: row.get("rtf_content")?,
            preview: row.get("preview")?,
            image_path: row.get("image_path")?,
            files: files_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            formats: formats_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            source_title: row.get("source_title")?,
            metadata: metadata_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::Value::Null),
        })
    })?;
    Ok(record)
}

pub fn bump_use(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE clips SET use_count = use_count + 1, last_used_at = ?1 WHERE id = ?2",
        params![now_ms(), id],
    )?;
    Ok(())
}

pub fn toggle_pin(conn: &Connection, id: i64) -> Result<bool> {
    let pinned: i64 = conn.query_row(
        "UPDATE clips SET pinned = CASE pinned WHEN 1 THEN 0 ELSE 1 END WHERE id = ?1 RETURNING pinned",
        params![id],
        |row| row.get(0),
    )?;
    Ok(pinned != 0)
}

pub fn toggle_favorite(conn: &Connection, id: i64) -> Result<bool> {
    let favorite: i64 = conn.query_row(
        "UPDATE clips SET favorite = CASE favorite WHEN 1 THEN 0 ELSE 1 END WHERE id = ?1 RETURNING favorite",
        params![id],
        |row| row.get(0),
    )?;
    Ok(favorite != 0)
}

pub fn bulk_set_flag(conn: &Connection, ids: &[i64], flag: &str, value: bool) -> Result<usize> {
    if ids.is_empty() {
        return Ok(0);
    }
    let column = match flag {
        "pinned" => "pinned",
        "favorite" => "favorite",
        _ => return Err(AppError::msg("Unknown flag")),
    };
    let placeholders = std::iter::repeat("?")
        .take(ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("UPDATE clips SET {column} = ? WHERE id IN ({placeholders})");
    let mut values: Vec<Value> = Vec::with_capacity(ids.len() + 1);
    values.push(Value::Integer(if value { 1 } else { 0 }));
    values.extend(ids.iter().map(|id| Value::Integer(*id)));
    let changed = conn.execute(&sql, params_from_iter(values.iter()))?;
    Ok(changed)
}

pub fn clip_uuids(conn: &Connection, ids: &[i64]) -> Result<Vec<String>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = std::iter::repeat("?")
        .take(ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let mut stmt = conn.prepare(&format!("SELECT uuid FROM clips WHERE id IN ({placeholders})"))?;
    let rows = stmt.query_map(params_from_iter(ids.iter()), |row| row.get(0))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn delete_ids(conn: &mut Connection, ids: &[i64]) -> Result<usize> {
    if ids.is_empty() {
        return Ok(0);
    }
    let tx = conn.transaction()?;
    let mut deleted = 0;
    for id in ids {
        tx.execute("DELETE FROM clips_fts WHERE rowid = ?1", params![id])?;
        deleted += tx.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
    }
    tx.commit()?;
    Ok(deleted)
}

pub fn clear_all(conn: &mut Connection) -> Result<usize> {
    let tx = conn.transaction()?;
    let count: i64 = tx.query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))?;
    tx.execute("DELETE FROM clips_fts", [])?;
    tx.execute("DELETE FROM clips", [])?;
    tx.execute("DELETE FROM sqlite_sequence WHERE name = 'clips'", [])?;
    tx.commit()?;
    Ok(count as usize)
}

pub fn stats(conn: &Connection, paths: &StoragePaths, asset_bytes: u64) -> Result<Stats> {
    let row = conn.query_row(
        "SELECT COUNT(*) AS total,
                COALESCE(SUM(CASE WHEN kind = 'text' THEN 1 ELSE 0 END), 0) AS text,
                COALESCE(SUM(CASE WHEN kind = 'image' THEN 1 ELSE 0 END), 0) AS images,
                COALESCE(SUM(CASE WHEN kind = 'files' THEN 1 ELSE 0 END), 0) AS files,
                COALESCE(SUM(CASE WHEN kind = 'link' THEN 1 ELSE 0 END), 0) AS links,
                COALESCE(SUM(CASE WHEN favorite = 1 THEN 1 ELSE 0 END), 0) AS favorites,
                COALESCE(SUM(CASE WHEN pinned = 1 THEN 1 ELSE 0 END), 0) AS pinned,
                COALESCE(SUM(CASE WHEN last_used_at IS NOT NULL THEN 1 ELSE 0 END), 0) AS recent,
                MAX(created_at) AS last_capture
         FROM clips",
        [],
        |row| {
            Ok(Stats {
                total: row.get("total")?,
                text: row.get("text")?,
                images: row.get("images")?,
                files: row.get("files")?,
                links: row.get("links")?,
                favorites: row.get("favorites")?,
                pinned: row.get("pinned")?,
                recent: row.get("recent")?,
                db_bytes: 0,
                asset_bytes,
                last_capture_at: row.get("last_capture")?,
            })
        },
    )?;
    Ok(Stats {
        db_bytes: paths.db_size(),
        ..row
    })
}

pub fn integrity_check(conn: &Connection, paths: &StoragePaths) -> Result<IntegrityReport> {
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    let clips: i64 = conn.query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))?;

    let mut expected: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut expected_uuids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut missing = 0u32;
    {
        let mut stmt = conn.prepare("SELECT uuid, image_path, thumb_path, preview_path FROM clips")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let uuid: String = row.get(0)?;
            expected_uuids.insert(uuid);
            for index in 1..4 {
                let path: Option<String> = row.get(index)?;
                if let Some(path) = path {
                    if paths.try_asset_path(&path).map(|p| p.is_file()).unwrap_or(false) {
                        expected.insert(path.replace('\\', "/"));
                    } else {
                        missing += 1;
                    }
                }
            }
        }
    }

    let mut orphaned = 0u32;
    for dir in [&paths.images, &paths.thumbnails] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let relative = format!(
                    "{}/{}",
                    if dir.ends_with("images") { "images" } else { "thumbnails" },
                    name
                );
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) && !expected.contains(&relative) {
                    orphaned += 1;
                }
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(&paths.attachments) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().to_string();
                if !expected_uuids.contains(&name) {
                    orphaned += 1;
                }
            }
        }
    }

    let ok = integrity.eq_ignore_ascii_case("ok") && missing == 0;
    let message = if !integrity.eq_ignore_ascii_case("ok") {
        format!("SQLite integrity check failed: {integrity}")
    } else if missing > 0 {
        format!("Database is healthy, but {missing} referenced asset files are missing.")
    } else if orphaned > 0 {
        format!("Database is healthy. {orphaned} unreferenced asset files can be cleaned up.")
    } else {
        "Database and assets are consistent.".into()
    };

    Ok(IntegrityReport {
        ok,
        message,
        orphaned_assets: orphaned,
        missing_assets: missing,
        clips,
    })
}

pub fn remove_assets_for(paths: &StoragePaths, uuid: &str) {
    // UUIDs come from database rows, including restored archives. Never let an
    // attacker-chosen value escape the asset directories.
    let Some(uuid) = crate::storage::safe_uuid(uuid) else {
        tracing::warn!("refusing to remove assets for an invalid UUID");
        return;
    };
    let _ = std::fs::remove_file(paths.images.join(format!("{uuid}.png")));
    let _ = std::fs::remove_file(paths.images.join(format!("{uuid}-preview.png")));
    let _ = std::fs::remove_file(paths.thumbnails.join(format!("{uuid}.png")));
    let _ = std::fs::remove_dir_all(paths.attachments.join(&uuid));
    let _ = std::fs::remove_dir_all(paths.files.join(&uuid));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_query_quotes_and_prefixes() {
        assert_eq!(build_fts_query("hello world").unwrap(), "\"hello\"* AND \"world\"*");
        assert_eq!(build_fts_query("a\"b").unwrap(), "\"a\"\"b\"*");
        assert!(build_fts_query("   ").is_none());
        assert!(build_fts_query("!!!").is_none());
    }

    #[test]
    fn urls_are_extracted() {
        let urls = urls_for_search(Some("see https://example.com/a and www.test.org."));
        assert!(urls.contains("https://example.com/a"));
        assert!(urls.contains("www.test.org"));
    }
}
