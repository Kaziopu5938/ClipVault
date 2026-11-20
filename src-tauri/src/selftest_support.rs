//! Narrow, public bridge used by the `selftest` example binary.
//!
//! This module is not part of the application's behaviour; it only exposes
//! crate-internal functions so they can be exercised without `cargo test`'s
//! broken harness on this machine.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::clipboard::image;
use crate::db::{self, NewClip};
use crate::error::Result;
use crate::models::{ClipFormat, ClipQuery, Settings};
use crate::storage::StoragePaths;

pub use crate::clipboard::html_to_text;
pub use crate::db::build_fts_query;

pub fn make_paths(root: &Path) -> StoragePaths {
    StoragePaths::from_root(root.to_path_buf())
}

pub fn open_db(paths: &StoragePaths) -> Result<Connection> {
    db::open(&paths.db_file)
}

pub fn insert_sample(conn: &mut Connection) -> Result<i64> {
    let clip = NewClip {
        uuid: "selftest-clip-1".into(),
        kind: "text".into(),
        created_at: crate::storage::now_ms(),
        source_app: Some("selftest".into()),
        source_title: None,
        content_hash: "selftest-hash".into(),
        text_content: Some("ClipVault fulltext needleword sample".into()),
        html_content: None,
        rtf_content: None,
        preview: "ClipVault fulltext needleword sample".into(),
        byte_size: 38,
        image_path: None,
        thumb_path: None,
        preview_path: None,
        image_width: None,
        image_height: None,
        files: Vec::new(),
        formats: vec![ClipFormat {
            id: 13,
            name: "CF_UNICODETEXT".into(),
            size: 76,
            captured: true,
            note: None,
        }],
        metadata: serde_json::Value::Null,
    };
    db::insert_clip(conn, &clip)
}

pub fn search_sample(conn: &Connection, paths: &StoragePaths) -> Result<usize> {
    let query = ClipQuery {
        search: Some("needleword".into()),
        view: Some("all".into()),
        ..Default::default()
    };
    Ok(db::list_clips(conn, &query, paths)?.items.len())
}

pub fn settings_roundtrip(conn: &mut Connection) -> Result<bool> {
    let mut settings = Settings::default();
    settings.hotkey = "Ctrl+Alt+C".into();
    settings.max_item_mb = 17;
    db::save_settings(conn, &settings)?;
    let loaded = db::load_settings(conn)?;
    Ok(loaded.hotkey == settings.hotkey && loaded.max_item_mb == settings.max_item_mb)
}

pub fn integrity(conn: &Connection, paths: &StoragePaths) -> Result<bool> {
    Ok(db::integrity_check(conn, paths)?.ok)
}

pub fn encode_sample_png() -> Result<(u32, u32)> {
    let mut picture = ::image::RgbaImage::new(24, 16);
    for pixel in picture.pixels_mut() {
        *pixel = ::image::Rgba([7, 19, 31, 255]);
    }
    let bytes = image::encode_png(&picture)?;
    let decoded = image::decode_bytes(&bytes)?;
    Ok(decoded.dimensions())
}

pub fn encode_sample_dib() -> Result<(u32, u32)> {
    let mut picture = ::image::RgbaImage::new(24, 16);
    for pixel in picture.pixels_mut() {
        *pixel = ::image::Rgba([201, 169, 110, 255]);
    }
    let bytes = image::parse_dib(&image::to_dibv5(&picture))?;
    Ok(bytes.dimensions())
}

#[allow(dead_code)]
fn _unused(_: PathBuf) {}
