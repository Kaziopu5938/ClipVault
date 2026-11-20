import type {
  AppInfo,
  BackupInfo,
  Clip,
  ClipDetail,
  ClipKind,
  ClipPage,
  ClipQuery,
  IntegrityReport,
  Settings,
  Stats,
} from "./types";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    const { mockInvoke } = await import("./mock");
    return mockInvoke<T>(cmd, args);
  }
  const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
  // The webview can mount before Rust's synchronous setup() has finished
  // opening the database and calling manage(). Retry only that startup race;
  // every other error is surfaced to the caller immediately.
  const deadline = Date.now() + 8000;
  for (;;) {
    try {
      return await tauriInvoke<T>(cmd, args);
    } catch (error) {
      if (
        String(error).includes("state not managed") &&
        Date.now() < deadline
      ) {
        await new Promise((resolve) => setTimeout(resolve, 80));
        continue;
      }
      throw error;
    }
  }
}

export const api = {
  listClips: (query: ClipQuery) => invoke<ClipPage>("list_clips", { query }),
  getClip: (id: number) => invoke<ClipDetail>("get_clip", { id }),
  copyClip: (id: number) => invoke<number>("copy_clip", { id }),
  copyPlain: (id: number) => invoke<number>("copy_clip_plain", { id }),
  togglePin: (id: number) => invoke<boolean>("toggle_pin", { id }),
  toggleFavorite: (id: number) => invoke<boolean>("toggle_favorite", { id }),
  deleteClips: (ids: number[]) => invoke<number>("delete_clips", { ids }),
  bulkSetFlag: (ids: number[], flag: "pinned" | "favorite", value: boolean) =>
    invoke<number>("bulk_set_flag", { ids, flag, value }),
  clearHistory: () => invoke<number>("clear_history"),
  getStats: () => invoke<Stats>("get_stats"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  setMonitoring: (enabled: boolean) => invoke<boolean>("set_monitoring", { enabled }),
  getMonitoring: () => invoke<boolean>("get_monitoring"),
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  listBackups: () => invoke<BackupInfo[]>("list_backups"),
  createBackup: () => invoke<BackupInfo>("create_backup"),
  restoreBackup: (path: string, password?: string | null) =>
    invoke<string>("restore_backup", { path, password: password ?? null }),
  deleteBackup: (path: string) => invoke<void>("delete_backup", { path }),
  exportArchive: (path: string, password?: string | null) =>
    invoke<string>("export_archive", { path, password: password ?? null }),
  importArchive: (path: string, password?: string | null) =>
    invoke<string>("import_archive", { path, password: password ?? null }),
  moveStorage: (path: string) => invoke<string>("move_storage", { path }),
  openDataDir: () => invoke<void>("open_data_dir"),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  revealPath: (path: string) => invoke<void>("reveal_path", { path }),
  checkIntegrity: () => invoke<IntegrityReport>("check_integrity"),
  pickerList: (search: string, limit?: number) =>
    invoke<Clip[]>("picker_list", { search, limit: limit ?? 40 }),
  hidePicker: () => invoke<void>("hide_picker"),
  showMain: () => invoke<void>("show_main"),
  refreshTray: () => invoke<void>("refresh_tray"),
  quitApp: () => invoke<void>("quit_app"),
  setTheme: (theme: string) => invoke<void>("set_theme", { theme }),
};

export const kindLabel: Record<ClipKind, string> = {
  text: "Text",
  image: "Image",
  files: "Files",
  link: "Link",
};

export function formatBytes(bytes: number): string {
  if (!bytes || bytes < 1) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  return `${value >= 100 || i === 0 ? Math.round(value) : value.toFixed(1)} ${units[i]}`;
}

export function formatTime(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}

export function formatDateTime(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

export function dayKey(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

export function dayLabel(ms: number): string {
  const now = new Date();
  const d = new Date(ms);
  const start = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const day = 86400000;
  if (ms >= start) return "Today";
  if (ms >= start - day) return "Yesterday";
  if (d.getFullYear() === now.getFullYear()) {
    return d.toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric" });
  }
  return d.toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" });
}

export function relativeTime(ms: number): string {
  const diff = Date.now() - ms;
  if (diff < 45000) return "just now";
  if (diff < 3600000) return `${Math.max(1, Math.round(diff / 60000))} min ago`;
  if (diff < 86400000) return `${Math.round(diff / 3600000)} hr ago`;
  if (diff < 604800000) return `${Math.round(diff / 86400000)} d ago`;
  return new Date(ms).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function looksLikeCode(text: string): boolean {
  if (!text) return false;
  const lines = text.split("\n");
  const codeSignals = [
    /^\s*(function|const|let|var|class|def|fn|pub fn|import|export|package|use |#include|SELECT|INSERT|UPDATE|CREATE|public |private |interface |type )/m,
    /[{};]\s*$/m,
    /^\s*[\w."'[\]-]+\s*[:=]\s*.+;?\s*$/m,
    /<\/?[a-z][\w-]*(\s|>)/i,
    /=>|::|->|\|\||&&/,
  ];
  let score = 0;
  for (const s of codeSignals) if (s.test(text)) score++;
  if (text.includes("\n") && score >= 2) return true;
  return score >= 3;
}

export function stripHtml(html: string): string {
  try {
    const doc = new DOMParser().parseFromString(html, "text/html");
    return doc.body?.innerText ?? "";
  } catch {
    return html.replace(/<[^>]+>/g, " ");
  }
}

export function truncate(s: string, n: number): string {
  return s.length > n ? s.slice(0, n).trimEnd() + "…" : s;
}

export function fileName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}
