pub mod image;
pub mod win;

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use serde_json::json;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::db::{self, ClipRecord, NewClip};
use crate::error::{AppError, Result};
use crate::models::{ClipFile, ClipFormat};
use crate::state::AppState;
use crate::storage::{atomic_write, now_ms, StoragePaths};

const PREVIEW_CHARS: usize = 700;

pub fn handle_update(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.monitoring.load(Ordering::Relaxed) {
        return;
    }

    let sequence = win::current_sequence();
    if sequence != 0 && sequence <= state.own_sequence.load(Ordering::Relaxed) {
        return;
    }

    let settings = state.settings_snapshot();
    if !settings.monitoring_enabled {
        return;
    }

    let snapshot = match win::read_snapshot(&settings) {
        Ok(Some(snapshot)) => snapshot,
        Ok(None) => return,
        Err(err) => {
            tracing::warn!("clipboard read failed: {err}");
            let _ = app.emit("capture-error", err.to_string());
            return;
        }
    };

    if !snapshot.has_content() {
        return;
    }

    if let Some(source) = snapshot.source_app.as_deref() {
        if settings
            .excluded_apps
            .iter()
            .any(|excluded| excluded.eq_ignore_ascii_case(source))
        {
            tracing::debug!("ignored copy from excluded app {source}");
            return;
        }
    }

    if let Err(err) = store_snapshot(app, &state, snapshot) {
        tracing::warn!("could not store clipboard item: {err}");
        let _ = app.emit("capture-error", err.to_string());
    }
}

fn store_snapshot(app: &AppHandle, state: &AppState, snapshot: win::Snapshot) -> Result<()> {
    let settings = state.settings_snapshot();
    let uuid = Uuid::new_v4().to_string();
    let created_at = now_ms();
    let mut formats = snapshot.formats.clone();

    let image = decode_image(&snapshot, &mut formats);

    let text = snapshot
        .text
        .as_deref()
        .map(|s| s.trim_matches('\0').to_string())
        .filter(|s| !s.trim().is_empty());
    let html = snapshot.html.clone();
    let rtf = snapshot.rtf.clone();

    let (kind, preview): (&str, String) = if !snapshot.file_entries.is_empty() {
        let names: Vec<String> = snapshot
            .file_entries
            .iter()
            .map(|f| f.name.clone())
            .collect();
        let preview = truncate_chars(&names.join(", "), PREVIEW_CHARS);
        ("files", preview)
    } else if let Some(text) = text.as_deref() {
        let trimmed = text.trim();
        if is_single_url(trimmed) {
            ("link", truncate_chars(trimmed, PREVIEW_CHARS))
        } else {
            ("text", truncate_chars(trimmed, PREVIEW_CHARS))
        }
    } else if let Some(ref image) = image {
        ("image", format!("Image · {} × {}", image.width(), image.height()))
    } else if let Some(html) = html.as_deref() {
        ("text", truncate_chars(&html_to_text(html), PREVIEW_CHARS))
    } else {
        return Err(AppError::msg(
            "The clipboard content could not be decoded into a supported item",
        ));
    };

    let mut byte_size: i64 = 0;
    byte_size += text.as_ref().map(|s| s.len() as i64).unwrap_or(0);
    byte_size += html.as_ref().map(|s| s.len() as i64).unwrap_or(0);
    byte_size += rtf.as_ref().map(|s| s.len() as i64).unwrap_or(0);

    let mut image_path = None;
    let mut thumb_path = None;
    let mut preview_path = None;
    let mut image_width = None;
    let mut image_height = None;
    let mut png_bytes: Vec<u8> = Vec::new();

    if let Some(ref decoded) = image {
        let saved = image::save_assets(&state.paths, &uuid, decoded)?;
        png_bytes = saved.png;
        byte_size += png_bytes.len() as i64;
        image_width = Some(saved.width as i64);
        image_height = Some(saved.height as i64);
        image_path = Some(saved.image_rel);
        thumb_path = Some(saved.thumb_rel);
        preview_path = Some(saved.preview_rel);
    }

    let mut metadata = json!({ "sequence": snapshot.sequence });
    if let Some(ref url) = snapshot.html_source_url {
        metadata["sourceUrl"] = json!(url);
    }

    if settings.capture_other_formats && !snapshot.extra.is_empty() {
        let mut saved_formats = Vec::new();
        for extra in &snapshot.extra {
            let safe_name = sanitize_name(&extra.name);
            let relative = format!("attachments/{uuid}/{}-{}.bin", extra.id, safe_name);
            atomic_write(&state.paths.asset_path(&relative), &extra.bytes)?;
            byte_size += extra.bytes.len() as i64;
            saved_formats.push(json!({ "name": extra.name, "path": relative }));
        }
        metadata["extraFormats"] = json!(saved_formats);
    }

    let hash = content_hash(
        kind,
        text.as_deref(),
        html.as_deref(),
        rtf.as_deref(),
        &png_bytes,
        &snapshot.file_entries,
    );

    let mut guard = state
        .db
        .lock()
        .map_err(|_| AppError::msg("Database lock is poisoned"))?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| AppError::msg("The database is closed"))?;

    let duplicate = match settings.duplicate_detection.as_str() {
        "off" => false,
        "all" => db::hash_exists(conn, &hash)?,
        _ => db::latest_hash(conn)?.as_deref() == Some(hash.as_str()),
    };
    if duplicate {
        tracing::debug!("skipped duplicate clipboard item");
        return Ok(());
    }

    let files: Vec<ClipFile> = snapshot.file_entries.clone();
    let new_clip = NewClip {
        uuid,
        kind: kind.to_string(),
        created_at,
        source_app: snapshot.source_app.clone(),
        source_title: snapshot.source_title.clone(),
        content_hash: hash,
        text_content: text,
        html_content: html,
        rtf_content: rtf,
        preview,
        byte_size,
        image_path,
        thumb_path,
        preview_path,
        image_width,
        image_height,
        files,
        formats,
        metadata,
    };

    db::insert_clip(conn, &new_clip)?;
    drop(guard);

    let _ = app.emit("clips-changed", ());
    Ok(())
}

fn decode_image(snapshot: &win::Snapshot, formats: &mut [ClipFormat]) -> Option<::image::RgbaImage> {
    if let Some(bytes) = snapshot.png.as_deref() {
        match image::decode_bytes(bytes) {
            Ok(decoded) => return Some(decoded),
            Err(err) => {
                tracing::warn!("PNG clipboard data could not be decoded: {err}");
                note_format(formats, "PNG", &format!("Decode failed: {err}"));
            }
        }
    }
    if let Some(bytes) = snapshot.dibv5.as_deref() {
        match image::parse_dib(bytes) {
            Ok(decoded) => return Some(decoded),
            Err(err) => {
                tracing::warn!("DIBV5 clipboard data could not be decoded: {err}");
                note_format(formats, "CF_DIBV5", &format!("Decode failed: {err}"));
            }
        }
    }
    if let Some(bytes) = snapshot.dib.as_deref() {
        match image::parse_dib(bytes) {
            Ok(decoded) => return Some(decoded),
            Err(err) => {
                tracing::warn!("DIB clipboard data could not be decoded: {err}");
                note_format(formats, "CF_DIB", &format!("Decode failed: {err}"));
            }
        }
    }
    None
}

fn note_format(formats: &mut [ClipFormat], name: &str, note: &str) {
    for format in formats.iter_mut() {
        if format.name == name {
            format.note = Some(note.to_string());
        }
    }
}

fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "format".into()
    } else {
        cleaned.chars().take(48).collect()
    }
}

fn content_hash(
    kind: &str,
    text: Option<&str>,
    html: Option<&str>,
    rtf: Option<&str>,
    png: &[u8],
    files: &[ClipFile],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    if let Some(text) = text {
        hasher.update(b"t");
        hasher.update(text.as_bytes());
    }
    if let Some(html) = html {
        hasher.update(b"h");
        hasher.update(html.as_bytes());
    }
    if let Some(rtf) = rtf {
        hasher.update(b"r");
        hasher.update(rtf.as_bytes());
    }
    if !png.is_empty() {
        hasher.update(b"p");
        hasher.update(png);
    }
    for file in files {
        hasher.update(b"f");
        hasher.update(file.path.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn truncate_chars(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let mut out: String = value.chars().take(limit).collect();
    out.push('…');
    out
}

fn is_single_url(text: &str) -> bool {
    if text.len() > 2048 || text.contains(char::is_whitespace) {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("www.")
        || lower.starts_with("mailto:")
}

pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    let mut in_script = false;
    let mut tag = String::new();
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        if in_tag {
            if c == '>' {
                in_tag = false;
                let name = tag
                    .trim()
                    .trim_start_matches('/')
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if name == "script" || name == "style" {
                    in_script = !tag.trim().starts_with('/');
                }
                if matches!(name.as_str(), "br" | "p" | "div" | "li" | "tr" | "h1" | "h2" | "h3" | "h4") {
                    out.push('\n');
                }
                tag.clear();
            } else {
                tag.push(c);
            }
            continue;
        }
        if c == '<' {
            in_tag = true;
            continue;
        }
        if in_script {
            continue;
        }
        if c == '&' {
            let mut entity = String::new();
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == ';' || entity.len() > 8 {
                    break;
                }
                entity.push(next);
            }
            match entity.as_str() {
                "amp" => out.push('&'),
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "quot" => out.push('"'),
                "apos" => out.push('\''),
                "nbsp" => out.push(' '),
                other if other.starts_with('#') => {
                    let code = other
                        .trim_start_matches('#')
                        .trim_start_matches('x')
                        .trim_start_matches('X');
                    let radix = if other.starts_with("#x") || other.starts_with("#X") {
                        16
                    } else {
                        10
                    };
                    if let Ok(value) = u32::from_str_radix(code, radix) {
                        if let Some(ch) = char::from_u32(value) {
                            out.push(ch);
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        out.push(c);
    }
    let mut lines: Vec<&str> = out.lines().map(|l| l.trim()).collect();
    lines.retain(|line| !line.is_empty());
    lines.join("\n")
}

pub fn copy_clip(app: &AppHandle, id: i64, plain: bool) -> Result<()> {
    let state = app.state::<AppState>();
    let record = state.with_db(|conn| {
        let record = db::get_record(conn, id)?;
        db::bump_use(conn, id)?;
        Ok(record)
    })?;
    write_record_to_clipboard(&record, &state.paths, plain)?;
    state
        .own_sequence
        .store(win::current_sequence(), Ordering::SeqCst);
    Ok(())
}

fn write_record_to_clipboard(record: &ClipRecord, paths: &StoragePaths, plain: bool) -> Result<()> {
    if !win::open_clipboard() {
        return Err(AppError::Clipboard(
            "The clipboard is busy. Close the app holding it and try again.".into(),
        ));
    }
    win::empty_clipboard();

    let mut wrote = false;
    let kind = record.kind.as_str();

    if plain && kind != "image" {
        let text = match kind {
            "files" => record
                .files
                .iter()
                .map(|f| f.path.clone())
                .collect::<Vec<_>>()
                .join("\r\n"),
            _ => record
                .text_content
                .clone()
                .or_else(|| record.html_content.as_deref().map(html_to_text))
                .unwrap_or_else(|| record.preview.clone()),
        };
        wrote = win::set_utf16_text(&text);
    } else {
        match kind {
            "image" => {
                if let Some(relative) = record.image_path.as_deref() {
                    match paths.try_asset_path(relative) {
                        Some(path) => match image::decode_file(&path) {
                            Ok(decoded) => {
                                let dibv5 = image::to_dibv5(&decoded);
                                let dib = image::to_dib(&decoded);
                                win::set_bytes(win::CF_DIBV5, &dibv5);
                                win::set_bytes(win::CF_DIB, &dib);
                                if let Ok(png) = image::encode_png(&decoded) {
                                    win::set_bytes(win::register_format("PNG"), &png);
                                }
                                wrote = true;
                            }
                            Err(err) => {
                                tracing::warn!("stored image could not be read: {err}");
                            }
                        },
                        None => tracing::warn!("stored image path was rejected as unsafe"),
                    }
                }
            }
            "files" => {
                let existing: Vec<PathBuf> = record
                    .files
                    .iter()
                    .map(|f| PathBuf::from(&f.path))
                    .filter(|p| p.exists())
                    .collect();
                if !existing.is_empty() {
                    wrote = win::set_hdrop(&existing);
                }
                if !wrote {
                    let text = record
                        .files
                        .iter()
                        .map(|f| f.path.clone())
                        .collect::<Vec<_>>()
                        .join("\r\n");
                    wrote = win::set_utf16_text(&text);
                }
            }
            _ => {
                if let Some(html) = record.html_content.as_deref() {
                    win::set_bytes(
                        win::register_format("HTML Format"),
                        &win::build_html_format(html, None),
                    );
                }
                if let Some(rtf) = record.rtf_content.as_deref() {
                    win::set_bytes(win::register_format("Rich Text Format"), rtf.as_bytes());
                }
                let text = record
                    .text_content
                    .clone()
                    .or_else(|| record.html_content.as_deref().map(html_to_text))
                    .unwrap_or_else(|| record.preview.clone());
                wrote = win::set_utf16_text(&text);
            }
        }
    }

    restore_extra_formats(record, paths);

    win::close_clipboard();

    if wrote {
        Ok(())
    } else {
        Err(AppError::msg(
            "This item's original content is no longer available to restore",
        ))
    }
}

fn restore_extra_formats(record: &ClipRecord, paths: &StoragePaths) {
    let Some(list) = record.metadata.get("extraFormats").and_then(|v| v.as_array()) else {
        return;
    };
    for entry in list {
        let Some(name) = entry.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(relative) = entry.get("path").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(path) = paths.try_asset_path(relative) else {
            tracing::warn!("rejected unsafe extra-format path from an archived item");
            continue;
        };
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let format = win::register_format(name);
        if format != 0 {
            win::set_bytes(format, &bytes);
        }
    }
}
