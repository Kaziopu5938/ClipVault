use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use tauri::AppHandle;
use windows_sys::Win32::Foundation::{CloseHandle, GlobalFree, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Globalization::{MultiByteToWideChar, CP_ACP};
use windows_sys::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    GetClipboardFormatNameW, GetClipboardSequenceNumber, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
use windows_sys::Win32::UI::Shell::DragQueryFileW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowTextW,
    GetWindowThreadProcessId, RegisterClassW, TranslateMessage, HWND_MESSAGE, MSG, WNDCLASSW,
};

use crate::error::{AppError, Result};
use crate::models::{ClipFile, ClipFormat, Settings};
use crate::storage::file_modified_ms;

pub const CF_TEXT: u32 = 1;
pub const CF_BITMAP: u32 = 2;
pub const CF_METAFILEPICT: u32 = 3;
pub const CF_DIB: u32 = 8;
pub const CF_PALETTE: u32 = 9;
pub const CF_UNICODETEXT: u32 = 13;
pub const CF_ENHMETAFILE: u32 = 14;
pub const CF_HDROP: u32 = 15;
pub const CF_LOCALE: u32 = 16;
pub const CF_DIBV5: u32 = 17;
pub const CF_OWNERDISPLAY: u32 = 0x0080;

const WM_CLIPBOARDUPDATE: u32 = 0x031D;

static APP: OnceLock<AppHandle> = OnceLock::new();

pub struct ExtraFormat {
    pub id: u32,
    pub name: String,
    pub bytes: Vec<u8>,
}

pub struct Snapshot {
    pub sequence: u32,
    pub text: Option<String>,
    pub html: Option<String>,
    pub html_source_url: Option<String>,
    pub rtf: Option<String>,
    pub dib: Option<Vec<u8>>,
    pub dibv5: Option<Vec<u8>>,
    pub png: Option<Vec<u8>>,
    pub files: Vec<PathBuf>,
    pub file_entries: Vec<ClipFile>,
    pub extra: Vec<ExtraFormat>,
    pub formats: Vec<ClipFormat>,
    pub source_app: Option<String>,
    pub source_title: Option<String>,
}

impl Snapshot {
    pub fn has_content(&self) -> bool {
        self.text.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
            || self.html.is_some()
            || self.rtf.is_some()
            || self.dib.is_some()
            || self.dibv5.is_some()
            || self.png.is_some()
            || !self.files.is_empty()
    }
}

pub fn spawn_monitor(app: AppHandle) {
    let _ = APP.set(app);
    let _ = std::thread::Builder::new()
        .name("clipboard-monitor".into())
        .spawn(|| unsafe { run_message_loop() });
}

unsafe fn run_message_loop() {
    let hinstance = GetModuleHandleW(std::ptr::null());
    let class_name = wide("ClipVaultClipboardMonitor");
    let window_name = wide("ClipVault Clipboard Monitor");
    let mut class: WNDCLASSW = std::mem::zeroed();
    class.lpfnWndProc = Some(wndproc);
    class.hInstance = hinstance;
    class.lpszClassName = class_name.as_ptr();

    if RegisterClassW(&class) == 0 {
        tracing::error!("could not register clipboard monitor window class");
        return;
    }

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        window_name.as_ptr(),
        0,
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        std::ptr::null_mut(),
        hinstance,
        std::ptr::null_mut(),
    );
    if hwnd.is_null() {
        tracing::error!("could not create clipboard monitor window");
        return;
    }
    if AddClipboardFormatListener(hwnd) == 0 {
        tracing::error!("AddClipboardFormatListener failed");
        return;
    }
    tracing::info!("clipboard monitor started");

    let mut message: MSG = std::mem::zeroed();
    while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if message == WM_CLIPBOARDUPDATE {
        if let Some(app) = APP.get() {
            crate::clipboard::handle_update(app);
        }
        return 0;
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

pub fn current_sequence() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

pub fn foreground_app() -> (Option<String>, Option<String>) {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return (None, None);
        }
        let mut title_buffer = vec![0u16; 512];
        let title_len = GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), title_buffer.len() as i32);
        let title = if title_len > 0 {
            Some(String::from_utf16_lossy(&title_buffer[..title_len as usize]))
        } else {
            None
        };

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return (None, title);
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return (None, title);
        }
        let mut path_buffer = vec![0u16; 1024];
        let mut size = path_buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, path_buffer.as_mut_ptr(), &mut size);
        CloseHandle(process);
        if ok == 0 {
            return (None, title);
        }
        let path = String::from_utf16_lossy(&path_buffer[..size as usize]);
        let name = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string());
        (name, title)
    }
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(std::iter::once(0)).collect()
}

pub fn register_format(name: &str) -> u32 {
    let name = wide(name);
    unsafe { RegisterClipboardFormatW(name.as_ptr()) }
}

pub fn open_clipboard() -> bool {
    for _ in 0..14 {
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(12));
    }
    false
}

pub fn close_clipboard() {
    unsafe {
        CloseClipboard();
    }
}

fn read_global(format: u32) -> Option<Vec<u8>> {
    unsafe {
        let handle = GetClipboardData(format);
        if handle.is_null() {
            return None;
        }
        let size = GlobalSize(handle);
        if size == 0 {
            return None;
        }
        let pointer = GlobalLock(handle) as *const u8;
        if pointer.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(pointer, size).to_vec();
        GlobalUnlock(handle);
        Some(bytes)
    }
}

fn global_size(format: u32) -> u64 {
    unsafe {
        let handle = GetClipboardData(format);
        if handle.is_null() {
            0
        } else {
            GlobalSize(handle) as u64
        }
    }
}

fn decode_utf16(bytes: &[u8]) -> String {
    let mut units = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let unit = u16::from_le_bytes([chunk[0], chunk[1]]);
        if unit == 0 {
            break;
        }
        units.push(unit);
    }
    String::from_utf16_lossy(&units)
}

fn decode_ansi(bytes: &[u8]) -> String {
    // CF_TEXT is in the system ANSI code page, not UTF-8. Convert with Windows
    // so legacy apps' non-ASCII text is stored and searched correctly.
    unsafe {
        let len = MultiByteToWideChar(
            CP_ACP,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            std::ptr::null_mut(),
            0,
        );
        if len > 0 {
            let mut buffer = vec![0u16; len as usize];
            let written = MultiByteToWideChar(
                CP_ACP,
                0,
                bytes.as_ptr(),
                bytes.len() as i32,
                buffer.as_mut_ptr(),
                len,
            );
            if written > 0 {
                return String::from_utf16_lossy(&buffer[..written as usize])
                    .trim_end_matches('\0')
                    .to_string();
            }
        }
    }
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_string()
}

pub fn format_name(format: u32) -> String {
    let standard = match format {
        CF_TEXT => Some("CF_TEXT"),
        CF_BITMAP => Some("CF_BITMAP"),
        CF_METAFILEPICT => Some("CF_METAFILEPICT"),
        CF_DIB => Some("CF_DIB"),
        CF_PALETTE => Some("CF_PALETTE"),
        CF_UNICODETEXT => Some("CF_UNICODETEXT"),
        CF_ENHMETAFILE => Some("CF_ENHMETAFILE"),
        CF_HDROP => Some("CF_HDROP"),
        CF_LOCALE => Some("CF_LOCALE"),
        CF_DIBV5 => Some("CF_DIBV5"),
        CF_OWNERDISPLAY => Some("CF_OWNERDISPLAY"),
        _ => None,
    };
    if let Some(name) = standard {
        return name.into();
    }
    let mut buffer = vec![0u16; 256];
    let len = unsafe { GetClipboardFormatNameW(format, buffer.as_mut_ptr(), buffer.len() as i32) };
    if len > 0 {
        String::from_utf16_lossy(&buffer[..len as usize])
    } else {
        format!("Format {format}")
    }
}

fn is_handle_format(format: u32) -> bool {
    matches!(
        format,
        CF_BITMAP | CF_METAFILEPICT | CF_PALETTE | CF_ENHMETAFILE | CF_OWNERDISPLAY | 0x0081 | 0x0082 | 0x0083
    )
}

pub fn read_snapshot(settings: &Settings) -> Result<Option<Snapshot>> {
    if !open_clipboard() {
        return Err(AppError::Clipboard("The clipboard is busy. ClipVault will try again on the next copy.".into()));
    }
    let sequence = current_sequence();
    let max_bytes = settings.max_item_mb.saturating_mul(1024 * 1024);

    let mut formats = Vec::new();
    let mut text = None;
    let mut html = None;
    let mut html_source_url = None;
    let mut rtf = None;
    let mut dib = None;
    let mut dibv5 = None;
    let mut png = None;
    let mut files = Vec::new();
    let mut file_entries = Vec::new();
    let mut extra = Vec::new();
    let mut extra_total = 0u64;
    let mut has_unicode = false;

    let mut format_ids = Vec::new();
    let mut current = 0u32;
    loop {
        current = unsafe { EnumClipboardFormats(current) };
        if current == 0 {
            break;
        }
        format_ids.push(current);
    }

    for format in &format_ids {
        let id = *format;
        let name = format_name(id);
        let mut entry = ClipFormat {
            id,
            name: name.clone(),
            size: global_size(id),
            captured: false,
            note: None,
        };

        match id {
            CF_UNICODETEXT => {
                has_unicode = true;
                if settings.capture_text {
                    if let Some(bytes) = read_global(id) {
                        if bytes.len() as u64 > max_bytes {
                            close_clipboard();
                            return Err(AppError::msg(format!(
                                "Clipboard text is larger than the {} MB item limit.",
                                settings.max_item_mb
                            )));
                        }
                        entry.size = bytes.len() as u64;
                        text = Some(decode_utf16(&bytes));
                        entry.captured = true;
                    }
                } else {
                    entry.note = Some("Text capture is off".into());
                }
            }
            CF_TEXT => {
                if !has_unicode && settings.capture_text {
                    if let Some(bytes) = read_global(id) {
                        entry.size = bytes.len() as u64;
                        text = Some(decode_ansi(&bytes));
                        entry.captured = true;
                    }
                } else if has_unicode {
                    entry.note = Some("Unicode text preferred".into());
                } else {
                    entry.note = Some("Text capture is off".into());
                }
            }
            CF_HDROP => {
                if settings.capture_files {
                    if let Some((paths, entries)) = read_files() {
                        entry.size = paths.iter().map(|p| p.to_string_lossy().len() as u64).sum();
                        files = paths;
                        file_entries = entries;
                        entry.captured = true;
                    }
                } else {
                    entry.note = Some("File capture is off".into());
                }
            }
            CF_DIB => {
                if settings.capture_images {
                    if let Some(bytes) = read_global(id) {
                        if bytes.len() as u64 > max_bytes {
                            close_clipboard();
                            return Err(AppError::msg(format!(
                                "Clipboard image is larger than the {} MB item limit.",
                                settings.max_item_mb
                            )));
                        }
                        entry.size = bytes.len() as u64;
                        dib = Some(bytes);
                        entry.captured = true;
                    }
                } else {
                    entry.note = Some("Image capture is off".into());
                }
            }
            CF_DIBV5 => {
                if settings.capture_images {
                    if let Some(bytes) = read_global(id) {
                        if bytes.len() as u64 > max_bytes {
                            close_clipboard();
                            return Err(AppError::msg(format!(
                                "Clipboard image is larger than the {} MB item limit.",
                                settings.max_item_mb
                            )));
                        }
                        entry.size = bytes.len() as u64;
                        dibv5 = Some(bytes);
                        entry.captured = true;
                    }
                } else {
                    entry.note = Some("Image capture is off".into());
                }
            }
            _ if name.eq_ignore_ascii_case("HTML Format") => {
                if settings.capture_html {
                    if let Some(bytes) = read_global(id) {
                        if bytes.len() as u64 > max_bytes {
                            close_clipboard();
                            return Err(AppError::msg("Clipboard HTML exceeds the item size limit."));
                        }
                        entry.size = bytes.len() as u64;
                        if let Some(parsed) = parse_html(&bytes) {
                            html = Some(parsed.0);
                            html_source_url = parsed.1;
                            entry.captured = true;
                        } else {
                            entry.note = Some("HTML header could not be parsed".into());
                        }
                    }
                } else {
                    entry.note = Some("HTML capture is off".into());
                }
            }
            _ if name.eq_ignore_ascii_case("Rich Text Format") => {
                if settings.capture_rtf {
                    if let Some(bytes) = read_global(id) {
                        if bytes.len() as u64 > max_bytes {
                            close_clipboard();
                            return Err(AppError::msg("Clipboard RTF exceeds the item size limit."));
                        }
                        entry.size = bytes.len() as u64;
                        rtf = Some(String::from_utf8_lossy(&bytes).trim_end_matches('\0').to_string());
                        entry.captured = true;
                    }
                } else {
                    entry.note = Some("RTF capture is off".into());
                }
            }
            _ if name.eq_ignore_ascii_case("PNG") || name.eq_ignore_ascii_case("JFIF") || name.eq_ignore_ascii_case("JPEG") => {
                if settings.capture_images {
                    if let Some(bytes) = read_global(id) {
                        if bytes.len() as u64 > max_bytes {
                            close_clipboard();
                            return Err(AppError::msg("Clipboard image exceeds the item size limit."));
                        }
                        entry.size = bytes.len() as u64;
                        if png.is_none() || name.eq_ignore_ascii_case("PNG") {
                            png = Some(bytes);
                        }
                        entry.captured = true;
                    }
                } else {
                    entry.note = Some("Image capture is off".into());
                }
            }
            CF_LOCALE => {
                entry.note = Some("Locale metadata is not restored".into());
            }
            _ if is_handle_format(id) => {
                entry.note = Some("Handle-based format; Windows does not expose raw bytes".into());
            }
            _ => {
                if settings.capture_other_formats {
                    if let Some(bytes) = read_global(id) {
                        if extra_total + bytes.len() as u64 > max_bytes {
                            entry.note = Some("Skipped: item size limit reached".into());
                        } else if !bytes.is_empty() {
                            entry.size = bytes.len() as u64;
                            entry.captured = true;
                            extra_total += bytes.len() as u64;
                            extra.push(ExtraFormat {
                                id,
                                name: name.clone(),
                                bytes,
                            });
                        }
                    } else {
                        entry.note = Some("Format data could not be read".into());
                    }
                } else {
                    entry.note = Some("Other format capture is off".into());
                }
            }
        }
        formats.push(entry);
    }

    // Windows can render a device-independent bitmap from a CF_BITMAP on
    // request, but EnumClipboardFormats only lists CF_BITMAP. Without this
    // fallback, images copied by apps that offer just a GDI bitmap would be
    // silently missed.
    if settings.capture_images
        && format_ids.contains(&CF_BITMAP)
        && dibv5.is_none()
        && dib.is_none()
        && png.is_none()
    {
        if let Some(bytes) = read_global(CF_DIBV5) {
            dibv5 = Some(bytes);
            if let Some(entry) = formats.iter_mut().find(|f| f.id == CF_BITMAP) {
                entry.captured = true;
                entry.note = Some("Converted to CF_DIBV5".into());
                entry.size = dibv5.as_ref().unwrap().len() as u64;
            }
        } else if let Some(bytes) = read_global(CF_DIB) {
            dib = Some(bytes);
            if let Some(entry) = formats.iter_mut().find(|f| f.id == CF_BITMAP) {
                entry.captured = true;
                entry.note = Some("Converted to CF_DIB".into());
                entry.size = dib.as_ref().unwrap().len() as u64;
            }
        }
    }

    close_clipboard();

    let (source_app, source_title) = foreground_app();
    Ok(Some(Snapshot {
        sequence,
        text,
        html,
        html_source_url,
        rtf,
        dib,
        dibv5,
        png,
        files,
        file_entries,
        extra,
        formats,
        source_app,
        source_title,
    }))
}

fn read_files() -> Option<(Vec<PathBuf>, Vec<ClipFile>)> {
    unsafe {
        let hdrop = GetClipboardData(CF_HDROP);
        if hdrop.is_null() {
            return None;
        }
        let count = DragQueryFileW(hdrop as _, u32::MAX, std::ptr::null_mut(), 0);
        if count == 0 {
            return None;
        }
        let mut paths = Vec::new();
        let mut entries = Vec::new();
        for index in 0..count {
            let len = DragQueryFileW(hdrop as _, index, std::ptr::null_mut(), 0);
            if len == 0 {
                continue;
            }
            let mut buffer = vec![0u16; len as usize + 1];
            let copied = DragQueryFileW(hdrop as _, index, buffer.as_mut_ptr(), buffer.len() as u32);
            if copied == 0 {
                continue;
            }
            let path_string = String::from_utf16_lossy(&buffer[..copied as usize]);
            let path = PathBuf::from(&path_string);
            let metadata = std::fs::metadata(&path).ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata
                .as_ref()
                .filter(|m| m.is_file())
                .map(|m| m.len() as i64);
            entries.push(ClipFile {
                path: path_string,
                name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string()),
                is_dir,
                size,
                modified: file_modified_ms(&path),
                exists: path.exists(),
                backed_up: false,
            });
            paths.push(path);
        }
        if paths.is_empty() {
            None
        } else {
            Some((paths, entries))
        }
    }
}

fn parse_html(bytes: &[u8]) -> Option<(String, Option<String>)> {
    let header_end = find_header_end(bytes).unwrap_or(0);
    let header = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let field = |name: &str| -> Option<usize> {
        header
            .lines()
            .find_map(|line| {
                let (key, value) = line.split_once(':')?;
                if key.trim().eq_ignore_ascii_case(name) {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
    };
    let source_url = header.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.trim().eq_ignore_ascii_case("SourceURL") {
            Some(value.trim().to_string())
        } else {
            None
        }
    });

    let start_fragment = field("StartFragment");
    let end_fragment = field("EndFragment");
    let start_html = field("StartHTML");
    let end_html = field("EndHTML");

    let (start, end) = match (start_fragment, end_fragment) {
        (Some(start), Some(end)) if start < end && end <= bytes.len() => (start, end),
        _ => match (start_html, end_html) {
            (Some(start), Some(end)) if start < end && end <= bytes.len() => (start, end),
            _ => (header_end, bytes.len()),
        },
    };
    if start >= end {
        return None;
    }
    let fragment = String::from_utf8_lossy(&bytes[start..end]).to_string();
    if fragment.trim().is_empty() {
        None
    } else {
        Some((fragment, source_url))
    }
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .or_else(|| bytes.windows(2).position(|w| w == b"\n\n").map(|p| p + 2))
}

pub fn build_html_format(fragment: &str, source_url: Option<&str>) -> Vec<u8> {
    let prefix = "<html><body><!--StartFragment-->";
    let suffix = "<!--EndFragment--></body></html>";
    let source_line = source_url
        .map(|url| format!("SourceURL:{url}\r\n"))
        .unwrap_or_default();
    let header_with = |start_html: usize, end_html: usize, start_frag: usize, end_frag: usize| {
        format!(
            "Version:0.9\r\nStartHTML:{start_html:010}\r\nEndHTML:{end_html:010}\r\nStartFragment:{start_frag:010}\r\nEndFragment:{end_frag:010}\r\n{source_line}\r\n"
        )
    };
    let dummy = header_with(0, 0, 0, 0);
    let start_html = dummy.len();
    let start_frag = start_html + prefix.len();
    let end_frag = start_frag + fragment.len();
    let end_html = end_frag + suffix.len();
    let header = header_with(start_html, end_html, start_frag, end_frag);
    let mut out = Vec::with_capacity(header.len() + prefix.len() + fragment.len() + suffix.len());
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(prefix.as_bytes());
    out.extend_from_slice(fragment.as_bytes());
    out.extend_from_slice(suffix.as_bytes());
    out
}

pub fn set_bytes(format: u32, bytes: &[u8]) -> bool {
    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
        if handle.is_null() {
            return false;
        }
        let pointer = GlobalLock(handle);
        if pointer.is_null() {
            GlobalFree(handle);
            return false;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer as *mut u8, bytes.len());
        GlobalUnlock(handle);
        let result = SetClipboardData(format, handle);
        if result.is_null() {
            GlobalFree(handle);
            false
        } else {
            true
        }
    }
}

pub fn set_utf16_text(text: &str) -> bool {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    units.push(0);
    let mut bytes = Vec::with_capacity(units.len() * 2);
    for unit in units {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    set_bytes(CF_UNICODETEXT, &bytes)
}

pub fn set_hdrop(paths: &[PathBuf]) -> bool {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&20u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 16]);
    for path in paths {
        for unit in path.as_os_str().encode_wide() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes.extend_from_slice(&0u16.to_le_bytes());
    }
    bytes.extend_from_slice(&0u16.to_le_bytes());
    set_bytes(CF_HDROP, &bytes)
}

pub fn empty_clipboard() -> bool {
    unsafe { EmptyClipboard() != 0 }
}

pub fn os_version() -> String {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = std::process::Command::new("cmd")
        .args(["/C", "ver"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    if let Ok(output) = output {
        let text = String::from_utf8_lossy(&output.stdout);
        if let Some(version) = text.split("Version ").nth(1) {
            let version = version.trim().trim_end_matches(']');
            let build = version
                .split('.')
                .nth(2)
                .and_then(|b| b.parse::<u32>().ok())
                .unwrap_or(0);
            let name = if build >= 22000 { "Windows 11" } else { "Windows 10" };
            return format!("{name} · {version}");
        }
    }
    "Windows".into()
}

#[allow(dead_code)]
pub fn hwnd_is_null(hwnd: HWND) -> bool {
    hwnd.is_null()
}

#[allow(dead_code)]
fn _assert_handle(_handle: HANDLE) {}
