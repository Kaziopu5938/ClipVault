export type ClipKind = "text" | "image" | "files" | "link";
export type ThemeMode = "dark" | "light" | "system";
export type SortMode = "newest" | "oldest" | "used";
export type DuplicateMode = "consecutive" | "all" | "off";

export interface ClipFile {
  path: string;
  name: string;
  isDir: boolean;
  size: number | null;
  modified: number | null;
  exists: boolean;
  backedUp: boolean;
}

export interface ClipFormat {
  id: number;
  name: string;
  size: number;
  captured: boolean;
  note: string | null;
}

export interface Clip {
  id: number;
  kind: ClipKind;
  createdAt: number;
  lastUsedAt: number | null;
  useCount: number;
  pinned: boolean;
  favorite: boolean;
  sourceApp: string | null;
  sourceTitle: string | null;
  preview: string;
  byteSize: number;
  imageWidth: number | null;
  imageHeight: number | null;
  thumbnail: string | null;
  files: ClipFile[];
  formats: ClipFormat[];
  hasHtml: boolean;
  hasRtf: boolean;
  hasFullText: boolean;
}

export interface ClipDetail extends Clip {
  textContent: string | null;
  htmlContent: string | null;
  rtfContent: string | null;
  image: string | null;
  imagePreview: string | null;
}

export interface ClipQuery {
  kind?: ClipKind | null;
  view?: "all" | "favorites" | "pinned" | "recent" | null;
  search?: string | null;
  from?: number | null;
  to?: number | null;
  sort?: SortMode;
  limit?: number;
  offset?: number;
}

export interface ClipPage {
  items: Clip[];
  hasMore: boolean;
  total: number | null;
}

export interface Stats {
  total: number;
  text: number;
  images: number;
  files: number;
  links: number;
  favorites: number;
  pinned: number;
  recent: number;
  dbBytes: number;
  assetBytes: number;
  lastCaptureAt: number | null;
}

export interface Settings {
  theme: ThemeMode;
  launchAtStartup: boolean;
  closeToTray: boolean;
  minimizeToTray: boolean;
  hotkey: string;
  monitoringEnabled: boolean;
  captureText: boolean;
  captureHtml: boolean;
  captureRtf: boolean;
  captureImages: boolean;
  captureFiles: boolean;
  captureOtherFormats: boolean;
  maxItemMb: number;
  duplicateDetection: DuplicateMode;
  excludedApps: string[];
  backupEnabled: boolean;
  backupFrequencyHours: number;
  backupKeep: number;
  backupDir: string;
  previewOpen: boolean;
}

export interface BackupInfo {
  name: string;
  path: string;
  size: number;
  createdAt: number;
  encrypted: boolean;
}

export interface AppInfo {
  version: string;
  schemaVersion: number;
  dataDir: string;
  dbPath: string;
  logsDir: string;
  platform: string;
  monitoring: boolean;
}

export interface IntegrityReport {
  ok: boolean;
  message: string;
  orphanedAssets: number;
  missingAssets: number;
  clips: number;
}
