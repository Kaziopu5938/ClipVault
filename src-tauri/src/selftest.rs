//! A small, self-contained verification runner.
//!
//! `cargo test` on this machine's GNU toolchain produces a test harness that
//! Windows refuses to load (STATUS_ENTRYPOINT_NOT_FOUND). This runner is a
//! regular binary built with the same linker as the application, so it gives
//! us the same checks without relying on the test harness.

use std::path::PathBuf;

use crate::selftest_support::*;

fn check(name: &str, ok: bool, detail: &str) -> bool {
    let mark = if ok { "PASS" } else { "FAIL" };
    println!("[{mark}] {name}{}", if detail.is_empty() { "".into() } else { format!(" — {detail}") });
    ok
}

pub fn run() -> bool {
    let mut results = Vec::new();

    let text = "<p>Hello &amp; <b>world</b></p><script>alert('x')</script><p>Line&nbsp;two</p>";
    let plain = html_to_text(text);
    results.push(check(
        "HTML to plain text",
        plain.contains("Hello & world") && plain.contains("Line two") && !plain.contains("alert"),
        &format!("{plain:?}"),
    ));

    let query = build_fts_query("hello  world");
    results.push(check(
        "FTS query builder",
        query.as_deref() == Some("\"hello\"* AND \"world\"*"),
        &format!("{query:?}"),
    ));

    let root = temp_root("selftest");
    let paths = make_paths(&root);
    let paths_result = paths.ensure().map(|_| ()).map_err(|e| e.to_string());
    results.push(check("storage layout", paths_result.is_ok(), &format!("{paths_result:?}")));

    let mut conn = match open_db(&paths) {
        Ok(conn) => conn,
        Err(err) => {
            println!("[FAIL] database open — {err}");
            return false;
        }
    };
    results.push(check("database open + migrations", true, ""));

    let inserted = insert_sample(&mut conn);
    results.push(check("clip insert", inserted.is_ok(), &format!("{inserted:?}")));

    let found = search_sample(&conn, &paths);
    results.push(check(
        "full-text search",
        found.as_ref().map(|v| *v == 1).unwrap_or(false),
        &format!("{found:?}"),
    ));

    let roundtrip = settings_roundtrip(&mut conn);
    results.push(check("settings round-trip", roundtrip.is_ok(), &format!("{roundtrip:?}")));

    let integrity = integrity(&conn, &paths);
    results.push(check(
        "database integrity",
        integrity.as_ref().map(|ok| *ok).unwrap_or(false),
        &format!("{integrity:?}"),
    ));

    let png = encode_sample_png();
    results.push(check(
        "PNG encode/decode",
        png.as_ref().map(|&(w, h)| w == 24 && h == 16).unwrap_or(false),
        &format!("{png:?}"),
    ));

    let dib = encode_sample_dib();
    results.push(check(
        "DIB encode/decode",
        dib.as_ref().map(|&(w, h)| w == 24 && h == 16).unwrap_or(false),
        &format!("{dib:?}"),
    ));

    drop(conn);
    let _ = std::fs::remove_dir_all(&root);

    let passed = results.iter().filter(|v| **v).count();
    println!("\n{passed}/{} checks passed", results.len());
    passed == results.len()
}

fn temp_root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("clipvault-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}
