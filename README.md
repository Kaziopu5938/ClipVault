# ClipVault

A permanent, local-first clipboard archive for Windows.

ClipVault keeps what you copy — text, links, images, files and the extra clipboard formats Windows applications publish — in one searchable archive on your own machine. Nothing expires, nothing is uploaded, and the only way an item disappears is when you delete it.

## Install

| File | What it is |
| --- | --- |
| `ClipVault-Setup-x64.exe` | NSIS installer, per-user (no administrator rights required) |
| `ClipVault-x64.msi` | Windows Installer package for managed deployment |
| `ClipVault.exe` | Portable build — run it directly, no installation |

ClipVault needs Windows 10 or later with the WebView2 runtime (present on Windows 11 and on most updated Windows 10 systems). The installers are unsigned, so Windows SmartScreen may warn on first run; choose *More info → Run anyway* if you trust the build.

## First run

1. Start ClipVault. It appears in the system tray; the main window opens as well.
2. Copy anything. New items appear in the library instantly and in the quick picker.
3. Press **Ctrl+Shift+V** anywhere — or left-click the ClipVault icon in the system tray — to open the small picker with everything you have copied. Type to filter, use **↑/↓** to move and **Enter** to put an item back on the clipboard.
4. Closing the main window hides it in the tray. Right-click the tray icon for **Open ClipVault**, **Settings** and **Quit ClipVault**.

## What gets captured

| Content | Stored |
| --- | --- |
| Plain text, Unicode text | Yes, including emoji and right-to-left text |
| Links | Detected and grouped separately |
| HTML and RTF | Raw markup is kept alongside the plain-text rendering |
| Images | PNG, JPEG, WebP, BMP/DIB, DIBV5 and CF_BITMAP sources are decoded and saved as PNG with a thumbnail |
| Files and folders | Name, path, size and timestamp are stored, with a reference to the original location |
| Registered clipboard formats | Saved as raw binary attachments; shown in the item's format list |
| Password managers | Skipped by default (1Password, Bitwarden, KeePass, LastPass and others); configurable |

**Clipboard formats that cannot be stored:** handle-based formats (`CF_BITMAP` as a raw handle, `CF_ENHMETAFILE`, `CF_METAFILEPICT`, `CF_PALETTE`, `CF_OWNERDISPLAY`) do not expose raw bytes through the Windows clipboard API. ClipVault renders `CF_BITMAP` to a DIB whenever Windows allows it; other handle formats are listed in the item's format panel with an explanation instead of being silently dropped. Items larger than the configurable limit (50 MB by default) are rejected with a visible error.

**File items are references, not content copies.** ClipVault records where the files were and can put them back on the clipboard, but it does not copy the files into its own storage. If a file is moved or deleted, the entry remains in the archive and the Copy action falls back to copying the path as text.

## Where your data lives

By default, everything is under `%LOCALAPPDATA%\ClipVault\`:

```
ClipVault\
  config.json                 custom storage location, if you changed it
  database\clipvault.db       SQLite database with FTS5 full-text index
  assets\images\              full-size PNGs
  assets\thumbnails\          list thumbnails
  assets\files\               file-reference metadata
  assets\attachments\         registered-format payloads
  backups\                    local ZIP backups
  logs\                       rotating diagnostic logs
  settings\
```

You can move the whole data folder from **Settings → Storage**. The app closes its database, copies the data and restarts.

## Privacy

- There are no accounts, no network calls and no telemetry. ClipVault works with the network disconnected.
- Imported archives are extracted to a staging folder and validated before anything is replaced. Paths and clip IDs inside an archive are validated, extraction has size limits, and a failed restore or storage move leaves your current data usable.
- The webview runs with a strict Content Security Policy, and Explorer shortcuts are limited to data-folder paths or paths captured as file clips.
- The SQLite database is **not encrypted at rest**. Password protection is available for exported archives (AES-256-GCM with an Argon2id-derived key); automatic local backups are plain ZIPs so they can be restored without a password. If you need encryption at rest for the database itself, enable BitLocker or store ClipVault's data folder on an encrypted volume.
- Nothing is deleted automatically. There is no retention policy, no expiry and no silent cleanup. Use the library's Delete action or *Clear history* when you want to remove things.

## Backup and restore

- **Settings → Backups → Create backup now** writes a ZIP containing a consistent SQLite snapshot (`VACUUM INTO`), settings and all assets to `backups\`.
- Automatic backups can be enabled with an interval and retention count. The oldest backups are pruned beyond the count you choose.
- **Export archive** can password-protect the ZIP (`.cvbackup`). **Import archive** restores from either format; a safety backup of the current data is created before any restore.
- Restoring an archive replaces the current library. Export a backup first if you are unsure.

## Keyboard and interface

- The main window has a custom title bar, a graphite-and-brass dark theme by default and a complete light theme.
- The library groups items by date, supports filters (all, text, links, images, files, favorites, pinned), sorting, full-text search, multi-select and a resizable/hideable preview panel.
- Each item shows the clipboard formats that were present, what was captured and why a format was skipped.
- The picker is a small, always-on-top window with no taskbar entry.

## Known limitations

- Built and tested on Windows 11 (26100). Windows 10 is supported by the toolchain but was not available for testing in this release.
- `cargo test`'s harness does not load on this machine's GNU toolchain (`STATUS_ENTRYPOINT_NOT_FOUND`). The `selftest` example binary exercises the core logic instead; see `BUILD.md`.
- The NSIS/MSI installers are unsigned. Signing requires a code-signing certificate and is not included here.
- The local database is unencrypted; see **Privacy** above.

See `BUILD.md` to reproduce the build and `CHANGELOG.md` for release history.
