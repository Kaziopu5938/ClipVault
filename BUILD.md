# Building ClipVault

Reproducible build notes for ClipVault 1.0.1.

## Prerequisites (as used for this release)

- Windows 11 (build 26100); Windows 10 should work but was not tested.
- Node.js 26.x and npm (bundled with Node).
- Rust 1.100 with the GNU toolchain: `rustup toolchain install stable-x86_64-pc-windows-gnu`
- MinGW-w64 (for `gcc`/`ld`/windres). In this build it lives at `C:\Users\Opu\tools\mingw64\mingw64\bin`.
- WebView2 runtime (already present on Windows 11).
- Tauri CLI via the project's dev dependency (`@tauri-apps/cli`). The NSIS and WiX toolchains are downloaded automatically by `tauri build` when first needed; this requires network access on the first build.

The Rust library is intentionally declared with `crate-type = ["rlib"]` in `src-tauri/Cargo.toml`. With the GNU toolchain, producing a DLL as well makes the linker fail with `export ordinal too large`. The final executable is produced from `src-tauri/src/main.rs`, which calls `clipvault_lib::run()`.

## Commands

From the repository root:

```powershell
$env:Path = "C:\Users\Opu\tools\mingw64\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:Path"

# Frontend type-check and production bundle
cmd /c "npm run build"

# Debug application (target\debug\clipvault.exe)
cd src-tauri
cargo build

# Installers → src-tauri\target\release\bundle\nsis and \msi
cd ..
cmd /c "npm run tauri build"
```

Use `cmd /c` for npm on machines where PowerShell blocks `npm.ps1`.

## Verification

The application was exercised by running the debug build and changing the real Windows clipboard:

1. Unicode text capture (including emoji) → row inserted in `clips` with `kind = 'text'`.
2. Bitmap-only clipboard content → converted to DIB and stored as an image with its width/height and generated thumbnail.
3. `CF_HDROP` with two files → row inserted with `kind = 'files'`, names, paths, sizes and timestamps; file contents are not copied.
4. Database inspected read-only with Python's `sqlite3` against `%LOCALAPPDATA%\ClipVault\database\clipvault.db`.
5. Release installers built and executed with silent flags; installation, uninstall and data-folder behavior checked where noted in `RELEASE-NOTES.md`.

`cargo test` compiles and links but its test binary fails to load on this GNU setup with `STATUS_ENTRYPOINT_NOT_FOUND` (0xC0000139). The same happens with a separate example binary, because only the main executable carries the manifest Tauri embeds. To get equivalent coverage, the debug executable exposes a `--selftest` flag:

```powershell
cd src-tauri
cargo build
.\target\debug\clipvault.exe --selftest
```

It checks HTML-to-text conversion, FTS query construction, storage layout, database migrations, clip insertion and search, settings round-trip, database integrity and PNG/DIB encode-decode. All 10 checks pass.

The NSIS installer uses `installMode: "currentUser"` so silent deployment (`/S`) works without elevation; a per-machine MSI is supplied for managed deployment.

## Release layout

After building, copy the artifacts into `ClipVault-Release\`:

- `installers\ClipVault-Setup-x64.exe` (NSIS)
- `installers\ClipVault-x64.msi` (WiX)
- `portable\ClipVault.exe`
- `source\` (complete project without `node_modules` or `target`)
- `README.md`, `BUILD.md`, `CHANGELOG.md`, `RELEASE-NOTES.md`
- `SHA256SUMS.txt`

Generate checksums from inside `ClipVault-Release\`:

```powershell
Get-ChildItem -Recurse -File | Where-Object { $_.Name -ne 'SHA256SUMS.txt' } |
  ForEach-Object { "{0}  {1}" -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.FullName.Replace((Get-Location).Path + '\', '') } |
  Sort-Object | Set-Content SHA256SUMS.txt
```
