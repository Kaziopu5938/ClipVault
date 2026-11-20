use serde::{Deserialize, Serialize};

pub const DEFAULT_HOTKEY: &str = "Ctrl+Shift+V";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipFile {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: Option<i64>,
    pub modified: Option<i64>,
    pub exists: bool,
    pub backed_up: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipFormat {
    pub id: u32,
    pub name: String,
    pub size: u64,
    pub captured: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipSummary {
    pub id: i64,
    pub kind: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub use_count: i64,
    pub pinned: bool,
    pub favorite: bool,
    pub source_app: Option<String>,
    pub source_title: Option<String>,
    pub preview: String,
    pub byte_size: i64,
    pub image_width: Option<i64>,
    pub image_height: Option<i64>,
    pub thumbnail: Option<String>,
    pub files: Vec<ClipFile>,
    pub formats: Vec<ClipFormat>,
    pub has_html: bool,
    pub has_rtf: bool,
    pub has_full_text: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipDetail {
    #[serde(flatten)]
    pub base: ClipSummary,
    pub text_content: Option<String>,
    pub html_content: Option<String>,
    pub rtf_content: Option<String>,
    pub image: Option<String>,
    pub image_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipPage {
    pub items: Vec<ClipSummary>,
    pub has_more: bool,
    pub total: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ClipQuery {
    pub kind: Option<String>,
    pub view: Option<String>,
    pub search: Option<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub sort: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub total: i64,
    pub text: i64,
    pub images: i64,
    pub files: i64,
    pub links: i64,
    pub favorites: i64,
    pub pinned: i64,
    pub recent: i64,
    pub db_bytes: u64,
    pub asset_bytes: u64,
    pub last_capture_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: String,
    pub launch_at_startup: bool,
    pub close_to_tray: bool,
    pub minimize_to_tray: bool,
    pub hotkey: String,
    pub monitoring_enabled: bool,
    pub capture_text: bool,
    pub capture_html: bool,
    pub capture_rtf: bool,
    pub capture_images: bool,
    pub capture_files: bool,
    pub capture_other_formats: bool,
    pub max_item_mb: u64,
    pub duplicate_detection: String,
    pub excluded_apps: Vec<String>,
    pub backup_enabled: bool,
    pub backup_frequency_hours: u64,
    pub backup_keep: u32,
    pub backup_dir: String,
    pub preview_open: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            launch_at_startup: false,
            close_to_tray: true,
            minimize_to_tray: true,
            hotkey: DEFAULT_HOTKEY.into(),
            monitoring_enabled: true,
            capture_text: true,
            capture_html: true,
            capture_rtf: true,
            capture_images: true,
            capture_files: true,
            capture_other_formats: true,
            max_item_mb: 50,
            duplicate_detection: "consecutive".into(),
            excluded_apps: vec![
                "1password.exe".into(),
                "bitwarden.exe".into(),
                "dashlane.exe".into(),
                "keepass.exe".into(),
                "keepassxc.exe".into(),
                "lastpass.exe".into(),
                "protonpass.exe".into(),
            ],
            backup_enabled: false,
            backup_frequency_hours: 24,
            backup_keep: 7,
            backup_dir: String::new(),
            preview_open: true,
        }
    }
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.hotkey = self.hotkey.trim().to_string();
        if self.hotkey.is_empty() {
            self.hotkey = DEFAULT_HOTKEY.into();
        }
        self.max_item_mb = self.max_item_mb.clamp(1, 500);
        self.backup_frequency_hours = self.backup_frequency_hours.clamp(1, 24 * 30);
        self.backup_keep = self.backup_keep.clamp(1, 100);
        if self.theme != "light" && self.theme != "system" {
            self.theme = "dark".into();
        }
        if !matches!(self.duplicate_detection.as_str(), "off" | "all" | "consecutive") {
            self.duplicate_detection = "consecutive".into();
        }
        self.excluded_apps = self
            .excluded_apps
            .iter()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        self.excluded_apps.sort();
        self.excluded_apps.dedup();
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub created_at: i64,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub schema_version: i64,
    pub data_dir: String,
    pub db_path: String,
    pub logs_dir: String,
    pub platform: String,
    pub monitoring: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub ok: bool,
    pub message: String,
    pub orphaned_assets: u32,
    pub missing_assets: u32,
    pub clips: i64,
}
