# Changelog

## 1.0.2 - 2026-11-20

- Left-clicking the tray icon now opens the picker as a tray-anchored flyout at the bottom-right of the working area, with a pointer and close-on-outside-click, instead of a centred popup.
- The Ctrl+Shift+V picker remains centred for keyboard use.
- Version bumped across frontend, backend, installer metadata and documentation.

## 1.0.1 - 2026-11-20

Security and correctness release. All 1.0.0 features are retained.

### Fixed

- Left-clicking the system tray icon now opens the compact quick picker with all copied items; right-click still opens the menu.
- The picker reloads the archive every time it opens, so items copied while it was hidden are visible immediately.
- Restored archives and databases can no longer escape the asset folders: asset paths and clip UUIDs are validated before any file is read or deleted.
- Archive restore is staged and validated before it touches live data, and a failed restore or storage move no longer leaves the database closed.
- Archives whose metadata or entries exceed size limits are rejected; extraction has a total budget.
- A crafted DIB palette header can no longer trigger a huge allocation; PNG/JPEG/DIB decoding is limited to safe dimensions and memory.
- The webview now has a strict Content Security Policy.
- Open/Reveal in Explorer only accepts paths in the data folder or paths captured as file clips; quotes and control characters are rejected.
- Archived settings can no longer redirect the local backup folder.
- Legacy CF_TEXT clipboard data is decoded using the system ANSI code page instead of UTF-8.
- Application version bumped consistently across frontend, backend, installer and documentation.

## 1.0.0 - 2026-11-20

First public build.

### Added

- Permanent, local-only clipboard archive with no expiry or automatic deletion.
- Capture of Unicode text, links, HTML, RTF, PNG/JPEG/WebP/BMP/DIB/DIBV5 images, CF_BITMAP sources, file/folder references and registered clipboard formats.
- Native event-driven clipboard monitoring (AddClipboardFormatListener / WM_CLIPBOARDUPDATE) with own-write suppression and consecutive/all/off duplicate detection.
- SQLite storage with FTS5 full-text search, schema migrations, WAL journaling, atomic asset writes, integrity checks and crash-safe restore.
- Quick picker with Ctrl+Shift+V or a left-click on the tray icon, keyboard navigation and filtering.
- System-tray integration, start-minimized and close-to-tray behavior, optional launch at startup.
- Library with date grouping, filters, sorting, multi-select, pin/favorite, a resizable preview panel and a format inspector that explains skipped formats.
- Full light and dark themes with a custom title bar and original clipboard-and-V branding.
- Local ZIP backups with pruning, password-protected AES-256-GCM/Argon2id exports and safety backups before restore.
- Movable storage folder, export/import, path reveal and data-folder shortcuts.
- NSIS and MSI installers plus a portable executable.
