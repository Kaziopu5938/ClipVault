import type {
  AppInfo,
  BackupInfo,
  Clip,
  ClipDetail,
  ClipKind,
  ClipPage,
  ClipQuery,
  Settings,
  Stats,
} from "./types";

/* Browser-only mock used for design preview and `npm run dev`.
   The real application always talks to the Rust backend. */

const svg = (body: string, w = 640, h = 420) =>
  `data:image/svg+xml;utf8,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">${body}</svg>`,
  )}`;

const mountain = svg(`
<rect width="640" height="420" fill="#1b2428"/>
<circle cx="486" cy="112" r="52" fill="#d9b77c" opacity=".9"/>
<path d="M0 330 L150 190 L265 285 L360 205 L520 330 L640 250 L640 420 L0 420Z" fill="#2f4144"/>
<path d="M0 380 L120 300 L245 360 L390 290 L540 380 L640 330 L640 420 L0 420Z" fill="#162022"/>
<path d="M0 400 L180 350 L330 395 L500 340 L640 390 L640 420 L0 420Z" fill="#0d1416"/>
<g fill="#d9b77c" opacity=".35"><circle cx="90" cy="80" r="2"/><circle cx="150" cy="50" r="1.6"/><circle cx="240" cy="100" r="1.8"/><circle cx="580" cy="70" r="2"/></g>`);

const ui = svg(`
<rect width="640" height="420" fill="#141518"/>
<rect x="24" y="24" width="150" height="372" rx="10" fill="#1b1c21"/>
<rect x="44" y="48" width="80" height="8" rx="4" fill="#c9a96e"/>
<rect x="44" y="78" width="104" height="6" rx="3" fill="#3a3d46"/>
<rect x="44" y="96" width="88" height="6" rx="3" fill="#2a2c33"/>
<rect x="44" y="114" width="96" height="6" rx="3" fill="#2a2c33"/>
<rect x="198" y="24" width="418" height="56" rx="10" fill="#1b1c21"/>
<rect x="216" y="44" width="150" height="14" rx="7" fill="#edeae3" opacity=".85"/>
<rect x="198" y="98" width="418" height="84" rx="10" fill="#1b1c21"/>
<rect x="218" y="118" width="240" height="9" rx="4" fill="#edeae3" opacity=".7"/>
<rect x="218" y="138" width="340" height="7" rx="3" fill="#3a3d46"/>
<rect x="218" y="154" width="300" height="7" rx="3" fill="#2a2c33"/>
<rect x="198" y="196" width="418" height="84" rx="10" fill="#1b1c21"/>
<rect x="218" y="216" width="180" height="9" rx="4" fill="#edeae3" opacity=".7"/>
<rect x="218" y="236" width="320" height="7" rx="3" fill="#3a3d46"/>
<rect x="198" y="294" width="418" height="102" rx="10" fill="#1b1c21"/>
<rect x="218" y="314" width="110" height="9" rx="4" fill="#c9a96e"/>
<rect x="218" y="336" width="350" height="7" rx="3" fill="#2a2c33"/>
<rect x="218" y="352" width="280" height="7" rx="3" fill="#2a2c33"/>`);

const chart = svg(`
<rect width="640" height="420" fill="#101114"/>
<g stroke="#26282e"><path d="M70 70H590M70 140H590M70 210H590M70 280H590M70 350H590"/></g>
<path d="M70 320 L145 270 L220 292 L295 205 L370 232 L445 150 L520 176 L590 96 L590 350 L70 350Z" fill="#c9a96e" opacity=".12"/>
<path d="M70 320 L145 270 L220 292 L295 205 L370 232 L445 150 L520 176 L590 96" fill="none" stroke="#c9a96e" stroke-width="3" stroke-linejoin="round"/>
<g fill="#c9a96e"><circle cx="145" cy="270" r="4"/><circle cx="295" cy="205" r="4"/><circle cx="445" cy="150" r="4"/><circle cx="590" cy="96" r="4"/></g>
<g fill="#6e6f76" font-family="monospace" font-size="11"><text x="70" y="372">JAN</text><text x="215" y="372">MAR</text><text x="365" y="372">MAY</text><text x="515" y="372">JUL</text></g>`);

const code = `fn reconcile(ledger: &mut Ledger, entries: &[Entry]) -> Result<usize> {
    let mut applied = 0;
    for entry in entries {
        if ledger.contains(entry.id) {
            continue;
        }
        ledger.apply(entry)?;
        applied += 1;
    }
    Ok(applied)
}`;

const readme = `# ClipVault

A permanent clipboard archive for Windows. Everything you copy is stored locally,
indexed for search, and kept until you choose to delete it.

- No cloud. No telemetry.
- Full-text search across years of copies.
- Text, images, files, links, and registered clipboard formats.`;

const contract = `The parties agree that the deliverables described in Schedule A shall be
provided no later than 30 days after the effective date. All work product
remains the property of the client upon final payment.

Confidentiality obligations survive termination of this agreement for a
period of three years.`;

const palette = `#0F1012  graphite
#C9A96E  brass
#EDEAE3  paper
#83B892  sage
#E0705F  clay`;

const email = `Hi Marta,

The Windows build is signed and the installers are in the release folder.
Could you verify the MSI on a clean Windows 10 machine before we publish?

Thanks,
Jonas`;

const clips: ClipDetail[] = [
  {
    id: 1,
    kind: "text",
    createdAt: Date.now() - 1000 * 60 * 3,
    lastUsedAt: Date.now() - 1000 * 60 * 2,
    useCount: 7,
    pinned: true,
    favorite: true,
    sourceApp: "Code.exe",
    sourceTitle: "reconcile.rs — clipvault",
    preview: code,
    byteSize: 342,
    imageWidth: null,
    imageHeight: null,
    thumbnail: null,
    files: [],
    formats: [
      { id: 13, name: "CF_UNICODETEXT", size: 342, captured: true, note: null },
      { id: 49346, name: "HTML Format", size: 710, captured: true, note: null },
    ],
    hasHtml: true,
    hasRtf: false,
    hasFullText: true,
    textContent: code,
    htmlContent: null,
    rtfContent: null,
    image: null,
    imagePreview: null,
  },
  {
    id: 2,
    kind: "image",
    createdAt: Date.now() - 1000 * 60 * 12,
    lastUsedAt: null,
    useCount: 1,
    pinned: false,
    favorite: false,
    sourceApp: "Photos.exe",
    sourceTitle: "Dolomites — photo library",
    preview: "Image · 640 × 420",
    byteSize: 284_112,
    imageWidth: 640,
    imageHeight: 420,
    thumbnail: mountain,
    files: [],
    formats: [
      { id: 17, name: "CF_DIBV5", size: 1_075_240, captured: true, note: null },
      { id: 8, name: "CF_DIB", size: 1_075_240, captured: true, note: null },
      { id: 49378, name: "PNG", size: 284_112, captured: true, note: null },
    ],
    hasHtml: false,
    hasRtf: false,
    hasFullText: false,
    textContent: null,
    htmlContent: null,
    rtfContent: null,
    image: mountain,
    imagePreview: mountain,
  },
  {
    id: 3,
    kind: "link",
    createdAt: Date.now() - 1000 * 60 * 34,
    lastUsedAt: Date.now() - 1000 * 60 * 30,
    useCount: 4,
    pinned: false,
    favorite: true,
    sourceApp: "msedge.exe",
    sourceTitle: "Tauri 2.0 — Build smaller, faster, more secure desktop apps",
    preview: "https://v2.tauri.app/start/",
    byteSize: 31,
    imageWidth: null,
    imageHeight: null,
    thumbnail: null,
    files: [],
    formats: [{ id: 13, name: "CF_UNICODETEXT", size: 31, captured: true, note: null }],
    hasHtml: false,
    hasRtf: false,
    hasFullText: true,
    textContent: "https://v2.tauri.app/start/",
    htmlContent: null,
    rtfContent: null,
    image: null,
    imagePreview: null,
  },
  {
    id: 4,
    kind: "text",
    createdAt: Date.now() - 1000 * 60 * 60 * 2,
    lastUsedAt: null,
    useCount: 0,
    pinned: false,
    favorite: false,
    sourceApp: "Notion.exe",
    sourceTitle: "Release notes — Notion",
    preview: readme,
    byteSize: 428,
    imageWidth: null,
    imageHeight: null,
    thumbnail: null,
    files: [],
    formats: [
      { id: 13, name: "CF_UNICODETEXT", size: 428, captured: true, note: null },
      { id: 49346, name: "HTML Format", size: 1_240, captured: true, note: null },
      { id: 49347, name: "Rich Text Format", size: 986, captured: true, note: null },
    ],
    hasHtml: true,
    hasRtf: true,
    hasFullText: true,
    textContent: readme,
    htmlContent: null,
    rtfContent: null,
    image: null,
    imagePreview: null,
  },
  {
    id: 5,
    kind: "files",
    createdAt: Date.now() - 1000 * 60 * 60 * 5,
    lastUsedAt: Date.now() - 1000 * 60 * 60 * 4,
    useCount: 2,
    pinned: false,
    favorite: false,
    sourceApp: "explorer.exe",
    sourceTitle: "ClipVault-Release",
    preview: "ClipVault-Setup-x64.exe, ClipVault-x64.msi, SHA256SUMS.txt",
    byteSize: 0,
    imageWidth: null,
    imageHeight: null,
    thumbnail: null,
    files: [
      { path: "D:\\Work\\ClipVault\\release\\ClipVault-Setup-x64.exe", name: "ClipVault-Setup-x64.exe", isDir: false, size: 4_812_800, modified: Date.now() - 86400000 * 2, exists: true, backedUp: false },
      { path: "D:\\Work\\ClipVault\\release\\ClipVault-x64.msi", name: "ClipVault-x64.msi", isDir: false, size: 5_201_920, modified: Date.now() - 86400000 * 2, exists: true, backedUp: false },
      { path: "D:\\Work\\ClipVault\\release\\SHA256SUMS.txt", name: "SHA256SUMS.txt", isDir: false, size: 210, modified: Date.now() - 86400000 * 2, exists: true, backedUp: false },
    ],
    formats: [{ id: 15, name: "CF_HDROP", size: 274, captured: true, note: null }],
    hasHtml: false,
    hasRtf: false,
    hasFullText: false,
    textContent: null,
    htmlContent: null,
    rtfContent: null,
    image: null,
    imagePreview: null,
  },
  {
    id: 6,
    kind: "image",
    createdAt: Date.now() - 1000 * 60 * 60 * 9,
    lastUsedAt: null,
    useCount: 1,
    pinned: false,
    favorite: false,
    sourceApp: "Figma.exe",
    sourceTitle: "ClipVault — interface exploration",
    preview: "Image · 640 × 420",
    byteSize: 96_420,
    imageWidth: 640,
    imageHeight: 420,
    thumbnail: ui,
    files: [],
    formats: [{ id: 17, name: "CF_DIBV5", size: 1_075_240, captured: true, note: null }],
    hasHtml: false,
    hasRtf: false,
    hasFullText: false,
    textContent: null,
    htmlContent: null,
    rtfContent: null,
    image: ui,
    imagePreview: ui,
  },
  {
    id: 7,
    kind: "text",
    createdAt: Date.now() - 1000 * 60 * 60 * 26,
    lastUsedAt: null,
    useCount: 0,
    pinned: false,
    favorite: false,
    sourceApp: "WINWORD.EXE",
    sourceTitle: "Consulting agreement — Draft 4",
    preview: contract,
    byteSize: 780,
    imageWidth: null,
    imageHeight: null,
    thumbnail: null,
    files: [],
    formats: [
      { id: 13, name: "CF_UNICODETEXT", size: 780, captured: true, note: null },
      { id: 49347, name: "Rich Text Format", size: 3_140, captured: true, note: null },
    ],
    hasHtml: false,
    hasRtf: true,
    hasFullText: true,
    textContent: contract,
    htmlContent: null,
    rtfContent: null,
    image: null,
    imagePreview: null,
  },
  {
    id: 8,
    kind: "link",
    createdAt: Date.now() - 1000 * 60 * 60 * 30,
    lastUsedAt: null,
    useCount: 2,
    pinned: false,
    favorite: false,
    sourceApp: "msedge.exe",
    sourceTitle: "Microsoft Edge",
    preview: "https://learn.microsoft.com/en-us/windows/apps/design/",
    byteSize: 58,
    imageWidth: null,
    imageHeight: null,
    thumbnail: null,
    files: [],
    formats: [{ id: 13, name: "CF_UNICODETEXT", size: 58, captured: true, note: null }],
    hasHtml: false,
    hasRtf: false,
    hasFullText: true,
    textContent: "https://learn.microsoft.com/en-us/windows/apps/design/",
    htmlContent: null,
    rtfContent: null,
    image: null,
    imagePreview: null,
  },
  {
    id: 9,
    kind: "text",
    createdAt: Date.now() - 1000 * 60 * 60 * 49,
    lastUsedAt: Date.now() - 1000 * 60 * 60 * 20,
    useCount: 9,
    pinned: false,
    favorite: true,
    sourceApp: "Figma.exe",
    sourceTitle: "Brand tokens",
    preview: palette,
    byteSize: 92,
    imageWidth: null,
    imageHeight: null,
    thumbnail: null,
    files: [],
    formats: [{ id: 13, name: "CF_UNICODETEXT", size: 92, captured: true, note: null }],
    hasHtml: false,
    hasRtf: false,
    hasFullText: true,
    textContent: palette,
    htmlContent: null,
    rtfContent: null,
    image: null,
    imagePreview: null,
  },
  {
    id: 10,
    kind: "text",
    createdAt: Date.now() - 1000 * 60 * 60 * 72,
    lastUsedAt: null,
    useCount: 1,
    pinned: false,
    favorite: false,
    sourceApp: "OUTLOOK.EXE",
    sourceTitle: "Re: Windows build verification",
    preview: email,
    byteSize: 240,
    imageWidth: null,
    imageHeight: null,
    thumbnail: null,
    files: [],
    formats: [{ id: 13, name: "CF_UNICODETEXT", size: 240, captured: true, note: null }],
    hasHtml: false,
    hasRtf: false,
    hasFullText: true,
    textContent: email,
    htmlContent: null,
    rtfContent: null,
    image: null,
    imagePreview: null,
  },
  {
    id: 11,
    kind: "image",
    createdAt: Date.now() - 1000 * 60 * 60 * 96,
    lastUsedAt: null,
    useCount: 0,
    pinned: false,
    favorite: false,
    sourceApp: "Code.exe",
    sourceTitle: "benchmark.png",
    preview: "Image · 640 × 420",
    byteSize: 71_002,
    imageWidth: 640,
    imageHeight: 420,
    thumbnail: chart,
    files: [],
    formats: [{ id: 8, name: "CF_DIB", size: 1_075_240, captured: true, note: null }],
    hasHtml: false,
    hasRtf: false,
    hasFullText: false,
    textContent: null,
    htmlContent: null,
    rtfContent: null,
    image: chart,
    imagePreview: chart,
  },
  {
    id: 12,
    kind: "text",
    createdAt: Date.now() - 1000 * 60 * 60 * 120,
    lastUsedAt: null,
    useCount: 0,
    pinned: false,
    favorite: false,
    sourceApp: "WindowsTerminal.exe",
    sourceTitle: "PowerShell",
    preview: "Get-ChildItem -Path .\\src -Recurse -Filter *.rs | Measure-Object -Property Length -Sum",
    byteSize: 86,
    imageWidth: null,
    imageHeight: null,
    thumbnail: null,
    files: [],
    formats: [{ id: 13, name: "CF_UNICODETEXT", size: 86, captured: true, note: null }],
    hasHtml: false,
    hasRtf: false,
    hasFullText: true,
    textContent:
      "Get-ChildItem -Path .\\src -Recurse -Filter *.rs | Measure-Object -Property Length -Sum",
    htmlContent: null,
    rtfContent: null,
    image: null,
    imagePreview: null,
  },
];

let mockClips = clips;
let mockSettings: Settings = {
  theme: "dark",
  launchAtStartup: false,
  closeToTray: true,
  minimizeToTray: true,
  hotkey: "Ctrl+Shift+V",
  monitoringEnabled: true,
  captureText: true,
  captureHtml: true,
  captureRtf: true,
  captureImages: true,
  captureFiles: true,
  captureOtherFormats: true,
  maxItemMb: 50,
  duplicateDetection: "consecutive",
  excludedApps: ["keepass.exe", "1password.exe", "bitwarden.exe"],
  backupEnabled: false,
  backupFrequencyHours: 24,
  backupKeep: 7,
  backupDir: "C:\\Users\\Opu\\AppData\\Local\\ClipVault\\backups",
  previewOpen: true,
};

const toSummary = (c: ClipDetail): Clip => {
  const { textContent, htmlContent, rtfContent, image, imagePreview, ...rest } = c;
  return rest;
};

const detail = (id: number): ClipDetail => {
  const found = mockClips.find((c) => c.id === id);
  if (!found) throw new Error("Clip not found");
  return found;
};

export async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await new Promise((r) => setTimeout(r, 40));
  switch (cmd) {
    case "list_clips": {
      const q = (args?.query ?? {}) as ClipQuery;
      let items = mockClips.slice();
      if (q.kind) items = items.filter((c) => c.kind === q.kind);
      if (q.view === "favorites") items = items.filter((c) => c.favorite);
      if (q.view === "pinned") items = items.filter((c) => c.pinned);
      if (q.view === "recent") {
        items = items.filter((c) => c.lastUsedAt).sort((a, b) => (b.lastUsedAt ?? 0) - (a.lastUsedAt ?? 0));
      }
      if (q.search) {
        const s = q.search.toLowerCase();
        items = items.filter(
          (c) =>
            c.preview.toLowerCase().includes(s) ||
            (c.textContent ?? "").toLowerCase().includes(s) ||
            c.files.some((f) => f.name.toLowerCase().includes(s)),
        );
      }
      if (q.from) items = items.filter((c) => c.createdAt >= q.from!);
      if (q.to) items = items.filter((c) => c.createdAt <= q.to!);
      if (q.sort === "oldest") items.sort((a, b) => a.createdAt - b.createdAt);
      else if (q.sort === "used") items.sort((a, b) => b.useCount - a.useCount || b.createdAt - a.createdAt);
      else items.sort((a, b) => b.createdAt - a.createdAt);
      const offset = q.offset ?? 0;
      const limit = q.limit ?? 80;
      const page = items.slice(offset, offset + limit).map(toSummary);
      return { items: page, hasMore: offset + limit < items.length, total: items.length } as T;
    }
    case "get_clip":
      return detail(Number(args?.id)) as T;
    case "copy_clip":
    case "copy_clip_plain": {
      const id = Number(args?.id);
      mockClips = mockClips.map((c) =>
        c.id === id ? { ...c, useCount: c.useCount + 1, lastUsedAt: Date.now() } : c,
      );
      return id as T;
    }
    case "toggle_pin": {
      const id = Number(args?.id);
      let next = false;
      mockClips = mockClips.map((c) => {
        if (c.id !== id) return c;
        next = !c.pinned;
        return { ...c, pinned: next };
      });
      return next as T;
    }
    case "toggle_favorite": {
      const id = Number(args?.id);
      let next = false;
      mockClips = mockClips.map((c) => {
        if (c.id !== id) return c;
        next = !c.favorite;
        return { ...c, favorite: next };
      });
      return next as T;
    }
    case "delete_clips": {
      const ids = new Set((args?.ids as number[]) ?? []);
      mockClips = mockClips.filter((c) => !ids.has(c.id));
      return ids.size as T;
    }
    case "bulk_set_flag": {
      const ids = new Set((args?.ids as number[]) ?? []);
      const flag = String(args?.flag ?? "");
      const value = Boolean(args?.value);
      mockClips = mockClips.map((c) =>
        ids.has(c.id) ? { ...c, [flag]: value } : c,
      );
      return ids.size as T;
    }
    case "clear_history":
      mockClips = [];
      return 0 as T;
    case "get_stats": {
      const assetBytes = mockClips.reduce((n, c) => n + c.byteSize, 0);
      return {
        total: mockClips.length,
        text: mockClips.filter((c) => c.kind === "text").length,
        images: mockClips.filter((c) => c.kind === "image").length,
        files: mockClips.filter((c) => c.kind === "files").length,
        links: mockClips.filter((c) => c.kind === "link").length,
        favorites: mockClips.filter((c) => c.favorite).length,
        pinned: mockClips.filter((c) => c.pinned).length,
        recent: mockClips.filter((c) => c.lastUsedAt).length,
        dbBytes: 1_802_240,
        assetBytes,
        lastCaptureAt: mockClips[0]?.createdAt ?? null,
      } as T;
    }
    case "get_settings":
      return mockSettings as T;
    case "save_settings":
      mockSettings = args?.settings as Settings;
      return mockSettings as T;
    case "set_monitoring":
      mockSettings.monitoringEnabled = Boolean(args?.enabled);
      return mockSettings.monitoringEnabled as T;
    case "get_monitoring":
      return mockSettings.monitoringEnabled as T;
    case "get_app_info":
      return {
        version: "1.0.2",
        schemaVersion: 1,
        dataDir: "C:\\Users\\Opu\\AppData\\Local\\ClipVault",
        dbPath: "C:\\Users\\Opu\\AppData\\Local\\ClipVault\\database\\clipvault.db",
        logsDir: "C:\\Users\\Opu\\AppData\\Local\\ClipVault\\logs",
        platform: "Windows 11 Pro · 10.0.26100",
        monitoring: mockSettings.monitoringEnabled,
      } as T;
    case "list_backups":
      return [] as T;
    case "create_backup":
      return {
        name: "clipvault-backup-demo.zip",
        path: "C:\\Users\\Opu\\AppData\\Local\\ClipVault\\backups\\clipvault-backup-demo.zip",
        size: 2_400_000,
        createdAt: Date.now(),
        encrypted: false,
      } as T;
    case "picker_list": {
      const s = String(args?.search ?? "").toLowerCase();
      const items = mockClips
        .filter((c) => !s || c.preview.toLowerCase().includes(s))
        .sort((a, b) => b.createdAt - a.createdAt)
        .slice(0, Number(args?.limit ?? 40))
        .map(toSummary);
      return items as T;
    }
    case "hide_picker":
    case "show_main":
    case "refresh_tray":
    case "open_data_dir":
    case "open_path":
    case "reveal_path":
    case "set_theme":
      return undefined as T;
    case "check_integrity":
      return { ok: true, message: "Database integrity check passed.", orphanedAssets: 0, missingAssets: 0, clips: mockClips.length } as T;
    default:
      return undefined as T;
  }
}
