import { useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  ArchiveRestore,
  CheckCircle2,
  ClipboardList,
  Database,
  FolderOpen,
  HardDrive,
  Info,
  Keyboard,
  Lock,
  Monitor,
  Moon,
  Plus,
  RefreshCw,
  Save,
  Settings2,
  ShieldCheck,
  Sun,
  Trash2,
  Upload,
  X,
} from "lucide-react";
import type { BackupInfo, IntegrityReport, Settings } from "../lib/types";
import { api, formatBytes, formatDateTime } from "../lib/api";
import { useApp } from "../lib/app-context";
import { useNav } from "../lib/navigation";
import { cn } from "../lib/cn";
import { useToast } from "../components/Toast";
import { BrandMark } from "../components/BrandMark";
import { ConfirmDialog, Modal } from "../components/Modal";
import { pickFile, pickFolder, pickSave } from "../lib/dialogs";

const TABS = [
  { id: "general", label: "General", icon: Settings2 },
  { id: "clipboard", label: "Clipboard", icon: ClipboardList },
  { id: "storage", label: "Storage", icon: Database },
  { id: "privacy", label: "Privacy", icon: ShieldCheck },
  { id: "about", label: "About", icon: Info },
];

function Switch({ on, onClick, disabled }: { on: boolean; onClick: () => void; disabled?: boolean }) {
  return (
    <button
      className={cn("switch", on && "on")}
      role="switch"
      aria-checked={on}
      onClick={onClick}
      disabled={disabled}
      style={disabled ? { opacity: 0.5 } : undefined}
    />
  );
}

function Row({
  title,
  help,
  children,
  stacked = false,
}: {
  title: string;
  help?: string;
  children: React.ReactNode;
  stacked?: boolean;
}) {
  return (
    <div className={cn("setting-row", stacked && "flex-col items-stretch gap-3")}>
      <div className="min-w-0">
        <div className="field-label">{title}</div>
        {help && <div className="field-help">{help}</div>}
      </div>
      <div className={cn("control", stacked && "w-full")}>{children}</div>
    </div>
  );
}

function ShortcutRecorder({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const [recording, setRecording] = useState(false);
  const [hint, setHint] = useState("");

  useEffect(() => {
    if (!recording) return;
    const onKey = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        setRecording(false);
        setHint("");
        return;
      }
      if (["Control", "Shift", "Alt", "Meta"].includes(event.key)) return;
      const mods: string[] = [];
      if (event.ctrlKey) mods.push("Ctrl");
      if (event.altKey) mods.push("Alt");
      if (event.shiftKey) mods.push("Shift");
      if (event.metaKey) mods.push("Super");
      if (!mods.length) {
        setHint("Add Ctrl, Alt, or Shift");
        return;
      }
      let key = event.key;
      if (event.code.startsWith("Key")) key = event.code.slice(3);
      else if (event.code.startsWith("Digit")) key = event.code.slice(5);
      else if (event.code === "Space") key = "Space";
      else if (/^F\d{1,2}$/.test(event.key)) key = event.key;
      else if (key.length === 1) key = key.toUpperCase();
      if (["Control", "Shift", "Alt", "Meta"].includes(key)) return;
      onChange([...mods, key].join("+"));
      setRecording(false);
      setHint("");
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange]);

  return (
    <div className="flex flex-col items-end gap-1">
      <button
        className={cn("btn", recording && "btn-primary")}
        onClick={() => {
          setRecording((v) => !v);
          setHint("");
        }}
      >
        <Keyboard size={13} />
        {recording ? "Press a shortcut…" : value || "Set shortcut"}
      </button>
      {hint && (
        <span className="text-[10.5px]" style={{ color: "var(--danger)" }}>
          {hint}
        </span>
      )}
    </div>
  );
}

export function SettingsView() {
  const { settings, info, stats, saveSettings, setMonitoring, monitoring, refreshStats, refreshData, applyTheme } = useApp();
  const { settingsTab, setSettingsTab } = useNav();
  const toast = useToast();
  const [draft, setDraft] = useState<Settings | null>(settings);
  const [saving, setSaving] = useState(false);
  const [appInput, setAppInput] = useState("");
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<{ title: string; description: string; label: string; action: () => void; danger?: boolean } | null>(null);
  const [moveTarget, setMoveTarget] = useState<string | null>(null);
  const [passwordFor, setPasswordFor] = useState<{ kind: "import" | "restore"; path: string } | null>(null);
  const [password, setPassword] = useState("");
  const [exportOpen, setExportOpen] = useState(false);
  const [exportProtected, setExportProtected] = useState(false);
  const [exportPassword, setExportPassword] = useState("");
  const [clearOpen, setClearOpen] = useState(false);
  const [clearText, setClearText] = useState("");
  const [integrity, setIntegrity] = useState<IntegrityReport | null>(null);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  useEffect(() => {
    if (settingsTab === "storage") void loadBackups();
  }, [settingsTab]);

  const dirty = useMemo(() => {
    if (!draft || !settings) return false;
    return JSON.stringify(draft) !== JSON.stringify(settings);
  }, [draft, settings]);

  const update = <K extends keyof Settings>(key: K, value: Settings[K]) => {
    setDraft((prev) => (prev ? { ...prev, [key]: value } : prev));
  };

  const loadBackups = async () => {
    try {
      setBackups(await api.listBackups());
    } catch {
      setBackups([]);
    }
  };

  const save = async () => {
    if (!draft) return;
    setSaving(true);
    try {
      await saveSettings(draft);
      const fresh = await api.getSettings();
      setDraft(fresh);
      toast("Settings saved");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setSaving(false);
    }
  };

  const backupNow = async () => {
    setBusy("backup");
    try {
      const backup = await api.createBackup();
      toast(`Backup created · ${formatBytes(backup.size)}`);
      await loadBackups();
      await refreshStats();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setBusy(null);
    }
  };

  const restorePath = async (path: string, passwordValue?: string | null) => {
    setBusy("restore");
    try {
      await api.restoreBackup(path, passwordValue ?? null);
      toast("Backup restored. Your archive has been reloaded.");
      setPasswordFor(null);
      setPassword("");
      await loadBackups();
      await refreshStats();
      refreshData();
    } catch (e) {
      const message = String(e);
      if (message.includes("PASSWORD_REQUIRED")) {
        setPasswordFor({ kind: "restore", path });
        toast("This archive is password-protected", "info");
      } else {
        toast(message, "error");
      }
    } finally {
      setBusy(null);
    }
  };

  const importPath = async (path: string, passwordValue?: string | null) => {
    setBusy("import");
    try {
      await api.importArchive(path, passwordValue ?? null);
      toast("Archive imported. Your archive has been reloaded.");
      setPasswordFor(null);
      setPassword("");
      await loadBackups();
      await refreshStats();
      refreshData();
    } catch (e) {
      const message = String(e);
      if (message.includes("PASSWORD_REQUIRED")) {
        setPasswordFor({ kind: "import", path });
        toast("This archive is password-protected", "info");
      } else {
        toast(message, "error");
      }
    } finally {
      setBusy(null);
    }
  };

  const exportArchive = async () => {
    if (!exportProtected && exportPassword) setExportPassword("");
    const stamp = new Date().toISOString().slice(0, 10);
    const path = await pickSave(
      "Export ClipVault archive",
      `ClipVault-archive-${stamp}.${exportProtected ? "cvbackup" : "zip"}`,
      [{ name: "ClipVault archive", extensions: ["zip", "cvbackup"] }],
    );
    if (!path) return;
    setBusy("export");
    try {
      const result = await api.exportArchive(path, exportProtected ? exportPassword : null);
      toast(`Archive exported to ${result}`);
      setExportOpen(false);
      setExportPassword("");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setBusy(null);
    }
  };

  const chooseImport = async () => {
    const path = await pickFile("Import ClipVault archive", [{ name: "ClipVault archive", extensions: ["zip", "cvbackup"] }]);
    if (!path) return;
    setConfirm({
      title: "Replace your current archive?",
      description:
        "Importing replaces the clipboard history and assets currently in ClipVault with the contents of this archive. A safety backup is created first.",
      label: "Import archive",
      danger: true,
      action: () => void importPath(path),
    });
  };

  if (!draft || !settings) {
    return (
      <div className="flex flex-1 items-center justify-center" style={{ color: "var(--ink-faint)" }}>
        Loading settings…
      </div>
    );
  }

  const total = (stats?.dbBytes ?? 0) + (stats?.assetBytes ?? 0);

  return (
    <div className="settings-page">
      <div className="settings-nav">
        <p className="px-2 pb-3 text-[15px] font-semibold tracking-[-0.015em]">Settings</p>
        {TABS.map((tab) => {
          const Icon = tab.icon;
          return (
            <button
              key={tab.id}
              className={cn("nav-item", settingsTab === tab.id && "active")}
              onClick={() => setSettingsTab(tab.id)}
            >
              <Icon size={16} />
              <span>{tab.label}</span>
            </button>
          );
        })}
        <div className="mt-6 px-2">
          <div className="flex items-center gap-1.5 text-[10.5px]" style={{ color: "var(--ink-faint)" }}>
            <ShieldCheck size={11} />
            Local only · no telemetry
          </div>
        </div>
      </div>

      <div className="settings-body">
        <div className="mb-5 flex items-center justify-between gap-4" style={{ maxWidth: 640 }}>
          <span className="text-[11.5px]" style={{ color: dirty ? "var(--brass)" : "var(--ink-faint)" }}>
            {dirty ? "Unsaved changes" : "All changes saved"}
          </span>
          <button className="btn btn-primary" disabled={!dirty || saving} onClick={save}>
            {saving ? <RefreshCw size={13} className="animate-spin" /> : <Save size={13} />}
            {saving ? "Saving…" : "Save changes"}
          </button>
        </div>

        {settingsTab === "general" && (
          <div className="settings-section">
            <h2 className="settings-title">General</h2>
            <p className="settings-desc">How ClipVault looks and behaves on this PC.</p>

            <Row title="Appearance" help="Dark is the default. System follows Windows.">
              <div className="segmented">
                {[
                  { id: "dark", label: "Dark", icon: Moon },
                  { id: "light", label: "Light", icon: Sun },
                  { id: "system", label: "System", icon: Monitor },
                ].map((option) => {
                  const Icon = option.icon;
                  return (
                    <button
                      key={option.id}
                      className={cn(draft.theme === option.id && "on")}
                      onClick={() => {
                        update("theme", option.id as Settings["theme"]);
                        applyTheme(option.id as Settings["theme"]);
                      }}
                    >
                      <span className="flex items-center gap-1.5">
                        <Icon size={12} />
                        {option.label}
                      </span>
                    </button>
                  );
                })}
              </div>
            </Row>

            <Row title="Start with Windows" help="Launch ClipVault when you sign in, so monitoring begins automatically.">
              <Switch on={draft.launchAtStartup} onClick={() => update("launchAtStartup", !draft.launchAtStartup)} />
            </Row>

            <Row title="Close to tray" help="Closing the window keeps monitoring in the background. Exit from the tray to quit.">
              <Switch on={draft.closeToTray} onClick={() => update("closeToTray", !draft.closeToTray)} />
            </Row>

            <Row title="Minimize to tray" help="Minimizing hides the taskbar button and keeps ClipVault in the notification area.">
              <Switch on={draft.minimizeToTray} onClick={() => update("minimizeToTray", !draft.minimizeToTray)} />
            </Row>

            <Row title="Quick picker shortcut" help="Opens the compact picker from anywhere in Windows.">
              <ShortcutRecorder value={draft.hotkey} onChange={(value) => update("hotkey", value)} />
            </Row>

            <Row title="Show preview panel" help="Open the details panel by default on the library screen.">
              <Switch on={draft.previewOpen} onClick={() => update("previewOpen", !draft.previewOpen)} />
            </Row>
          </div>
        )}

        {settingsTab === "clipboard" && (
          <div className="settings-section">
            <h2 className="settings-title">Clipboard</h2>
            <p className="settings-desc">Choose what ClipVault captures and what it ignores.</p>

            <Row title="Monitoring" help="When paused, nothing is read from the clipboard.">
              <div className="flex items-center gap-3">
                <span className="text-[11.5px]" style={{ color: monitoring ? "var(--success)" : "var(--ink-faint)" }}>
                  {monitoring ? "Running" : "Paused"}
                </span>
                <Switch
                  on={monitoring}
                  onClick={async () => {
                    try {
                      await setMonitoring(!monitoring);
                      update("monitoringEnabled", !monitoring);
                    } catch (e) {
                      toast(String(e), "error");
                    }
                  }}
                />
              </div>
            </Row>

            <Row title="Text and links" help="Plain and Unicode text, including code, URLs, and email addresses.">
              <Switch on={draft.captureText} onClick={() => update("captureText", !draft.captureText)} />
            </Row>
            <Row title="HTML formatting" help="Keeps rich text formatting when the source app provides HTML.">
              <Switch on={draft.captureHtml} onClick={() => update("captureHtml", !draft.captureHtml)} />
            </Row>
            <Row title="RTF formatting" help="Keeps Rich Text Format data for apps that paste RTF.">
              <Switch on={draft.captureRtf} onClick={() => update("captureRtf", !draft.captureRtf)} />
            </Row>
            <Row title="Images" help="Stores a PNG copy, a display preview, and a thumbnail. Original DIB data is kept when available.">
              <Switch on={draft.captureImages} onClick={() => update("captureImages", !draft.captureImages)} />
            </Row>
            <Row title="Files and folders" help="Stores paths and file metadata. File contents are not copied unless the source path still exists.">
              <Switch on={draft.captureFiles} onClick={() => update("captureFiles", !draft.captureFiles)} />
            </Row>
            <Row title="Other clipboard formats" help="Preserves registered formats where Windows exposes raw data. Unsupported handle-based formats are recorded, not silently dropped.">
              <Switch on={draft.captureOtherFormats} onClick={() => update("captureOtherFormats", !draft.captureOtherFormats)} />
            </Row>

            <Row title="Maximum item size" help="Copies larger than this are skipped and logged. Range 1–500 MB.">
              <div className="flex items-center gap-2">
                <input
                  className="input w-24 text-right"
                  type="number"
                  min={1}
                  max={500}
                  value={draft.maxItemMb}
                  onChange={(e) => update("maxItemMb", Math.max(1, Math.min(500, Number(e.target.value) || 1)))}
                />
                <span className="text-[12px]" style={{ color: "var(--ink-faint)" }}>
                  MB
                </span>
              </div>
            </Row>

            <Row title="Duplicate detection" help="Consecutive ignores the same content copied twice in a row. All also skips older matches.">
              <select
                className="select w-48"
                value={draft.duplicateDetection}
                onChange={(e) => update("duplicateDetection", e.target.value as Settings["duplicateDetection"])}
              >
                <option value="consecutive">Consecutive copies</option>
                <option value="all">All duplicates</option>
                <option value="off">Keep every copy</option>
              </select>
            </Row>

            <Row
              title="Excluded apps"
              help="Copies made while one of these apps is in front are ignored. Match by process name."
              stacked
            >
              <div className="flex w-full flex-wrap gap-1.5">
                {draft.excludedApps.map((app) => (
                  <span
                    key={app}
                    className="inline-flex h-7 items-center gap-1.5 rounded-md border px-2 font-mono text-[10.5px]"
                    style={{ borderColor: "var(--line)", background: "var(--surface-2)", color: "var(--ink-muted)" }}
                  >
                    {app}
                    <button
                      className="icon-btn"
                      style={{ width: 16, height: 16 }}
                      onClick={() => update("excludedApps", draft.excludedApps.filter((x) => x !== app))}
                      aria-label={`Remove ${app}`}
                    >
                      <X size={10} />
                    </button>
                  </span>
                ))}
                {!draft.excludedApps.length && (
                  <span className="text-[11.5px]" style={{ color: "var(--ink-faint)" }}>
                    No excluded apps.
                  </span>
                )}
              </div>
              <div className="mt-3 flex w-full gap-2">
                <input
                  className="input flex-1 font-mono text-[12px]"
                  placeholder="e.g. keepassxc.exe"
                  value={appInput}
                  onChange={(e) => setAppInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key !== "Enter") return;
                    const value = appInput.trim().toLowerCase();
                    if (value && !draft.excludedApps.includes(value)) {
                      update("excludedApps", [...draft.excludedApps, value]);
                      setAppInput("");
                    }
                  }}
                />
                <button
                  className="btn"
                  onClick={() => {
                    const value = appInput.trim().toLowerCase();
                    if (value && !draft.excludedApps.includes(value)) {
                      update("excludedApps", [...draft.excludedApps, value]);
                      setAppInput("");
                    }
                  }}
                >
                  <Plus size={13} />
                  Add
                </button>
              </div>
            </Row>
          </div>
        )}

        {settingsTab === "storage" && (
          <div className="settings-section">
            <h2 className="settings-title">Storage</h2>
            <p className="settings-desc">Everything lives on this PC. Nothing is uploaded or synchronized.</p>

            <div className="mb-5 grid grid-cols-3 gap-2">
              {[
                { label: "Items", value: (stats?.total ?? 0).toLocaleString() },
                { label: "Database", value: formatBytes(stats?.dbBytes ?? 0) },
                { label: "Assets", value: formatBytes(stats?.assetBytes ?? 0) },
              ].map((stat) => (
                <div key={stat.label} className="rounded-lg border p-3" style={{ borderColor: "var(--line)", background: "var(--surface)" }}>
                  <div className="font-mono text-[16px] tracking-tight">{stat.value}</div>
                  <div className="mt-0.5 text-[10.5px]" style={{ color: "var(--ink-faint)" }}>
                    {stat.label}
                  </div>
                </div>
              ))}
            </div>

            <Row title="Data folder" help="Database, images, thumbnails, backups, and logs." stacked>
              <div className="flex w-full items-center gap-2">
                <div className="input flex-1 truncate font-mono text-[11px]" style={{ display: "flex", alignItems: "center" }}>
                  {info?.dataDir}
                </div>
                <button className="btn" onClick={() => api.openDataDir().catch((e) => toast(String(e), "error"))}>
                  <FolderOpen size={13} />
                  Open
                </button>
                <button
                  className="btn"
                  onClick={async () => {
                    const folder = await pickFolder("Choose a new ClipVault data folder");
                    if (folder) setMoveTarget(folder);
                  }}
                >
                  Change
                </button>
              </div>
            </Row>

            <Row title="Automatic backups" help="Snapshots the database and assets into the backups folder on a schedule. Old backups are pruned; clipboard history is never touched.">
              <Switch on={draft.backupEnabled} onClick={() => update("backupEnabled", !draft.backupEnabled)} />
            </Row>
            {draft.backupEnabled && (
              <>
                <Row title="Frequency">
                  <select
                    className="select w-36"
                    value={draft.backupFrequencyHours}
                    onChange={(e) => update("backupFrequencyHours", Number(e.target.value))}
                  >
                    <option value={6}>Every 6 hours</option>
                    <option value={12}>Every 12 hours</option>
                    <option value={24}>Every day</option>
                    <option value={72}>Every 3 days</option>
                  </select>
                </Row>
                <Row title="Backups to keep" help="Only ClipVault backup files are removed.">
                  <select
                    className="select w-28"
                    value={draft.backupKeep}
                    onChange={(e) => update("backupKeep", Number(e.target.value))}
                  >
                    {[3, 5, 7, 14, 30].map((n) => (
                      <option key={n} value={n}>
                        {n}
                      </option>
                    ))}
                  </select>
                </Row>
              </>
            )}

            <Row title="Backups and archives" help="Backups stay local. Exported archives can be moved to another PC." stacked>
              <div className="flex w-full flex-wrap gap-2">
                <button className="btn" disabled={busy === "backup"} onClick={backupNow}>
                  {busy === "backup" ? <RefreshCw size={13} className="animate-spin" /> : <ArchiveRestore size={13} />}
                  Back up now
                </button>
                <button className="btn" onClick={() => setExportOpen(true)}>
                  <HardDrive size={13} />
                  Export archive
                </button>
                <button className="btn" onClick={chooseImport}>
                  <Upload size={13} />
                  Import archive
                </button>
                <button className="btn" onClick={() => api.openPath(info?.dataDir ?? "").catch(() => {})}>
                  <FolderOpen size={13} />
                  Backups folder
                </button>
              </div>
            </Row>

            <div className="mt-2 overflow-hidden rounded-lg border" style={{ borderColor: "var(--line)" }}>
              {backups.length === 0 ? (
                <div className="px-4 py-6 text-center text-[12px]" style={{ color: "var(--ink-faint)" }}>
                  No backups yet. Create one now or enable automatic backups.
                </div>
              ) : (
                backups.map((backup, index) => (
                  <div
                    key={backup.path}
                    className="flex items-center gap-3 px-3.5 py-2.5"
                    style={{ borderTop: index ? "1px solid var(--line)" : undefined }}
                  >
                    <ArchiveRestore size={14} style={{ color: "var(--ink-faint)", flex: "none" }} />
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[12px]">{backup.name}</div>
                      <div className="font-mono text-[9.5px]" style={{ color: "var(--ink-faint)" }}>
                        {formatDateTime(backup.createdAt)} · {formatBytes(backup.size)}
                        {backup.encrypted ? " · encrypted" : ""}
                      </div>
                    </div>
                    <button
                      className="btn btn-sm"
                      onClick={() =>
                        setConfirm({
                          title: "Restore this backup?",
                          description:
                            "Your current clipboard history and assets will be replaced. ClipVault creates a safety backup before restoring.",
                          label: "Restore",
                          danger: true,
                          action: () => void restorePath(backup.path),
                        })
                      }
                    >
                      Restore
                    </button>
                    <button
                      className="icon-btn danger"
                      title="Delete backup"
                      onClick={() =>
                        setConfirm({
                          title: "Delete this backup?",
                          description: "Only the backup file is removed. Your clipboard history is not affected.",
                          label: "Delete backup",
                          danger: true,
                          action: () => {
                            api
                              .deleteBackup(backup.path)
                              .then(() => {
                                toast("Backup deleted");
                                return loadBackups();
                              })
                              .catch((e) => toast(String(e), "error"));
                          },
                        })
                      }
                    >
                      <Trash2 size={13} />
                    </button>
                  </div>
                ))
              )}
            </div>

            <div className="mt-6 rounded-lg border p-4" style={{ borderColor: "var(--line)" }}>
              <div className="flex items-center justify-between gap-3">
                <div>
                  <div className="field-label">Database integrity</div>
                  <div className="field-help">Checks the SQLite database and looks for missing or orphaned asset files.</div>
                </div>
                <button
                  className="btn"
                  onClick={async () => {
                    setBusy("integrity");
                    try {
                      const report = await api.checkIntegrity();
                      setIntegrity(report);
                      toast(report.ok ? "Integrity check passed" : "Integrity check found issues", report.ok ? "success" : "error");
                    } catch (e) {
                      toast(String(e), "error");
                    } finally {
                      setBusy(null);
                    }
                  }}
                  disabled={busy === "integrity"}
                >
                  {busy === "integrity" ? <RefreshCw size={13} className="animate-spin" /> : <CheckCircle2 size={13} />}
                  Check now
                </button>
              </div>
              {integrity && (
                <div className="mt-3 flex items-start gap-2 rounded-md p-2.5 text-[11.5px]" style={{ background: "var(--surface-2)" }}>
                  {integrity.ok ? (
                    <CheckCircle2 size={13} style={{ color: "var(--success)", marginTop: 1 }} />
                  ) : (
                    <AlertTriangle size={13} style={{ color: "var(--danger)", marginTop: 1 }} />
                  )}
                  <div>
                    <div>{integrity.message}</div>
                    <div className="mt-0.5 font-mono text-[10px]" style={{ color: "var(--ink-faint)" }}>
                      {integrity.clips} clips · {integrity.missingAssets} missing assets · {integrity.orphanedAssets} orphaned files
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {settingsTab === "privacy" && (
          <div className="settings-section">
            <h2 className="settings-title">Privacy</h2>
            <p className="settings-desc">Clipboard data is sensitive. ClipVault treats it that way.</p>

            <div className="mb-5 flex gap-3 rounded-lg border p-4" style={{ borderColor: "var(--line)", background: "var(--surface)" }}>
              <ShieldCheck size={18} style={{ color: "var(--brass)", flex: "none", marginTop: 1 }} />
              <div>
                <div className="text-[12.5px] font-medium">Local-only storage</div>
                <p className="mt-1 text-[11.5px] leading-relaxed" style={{ color: "var(--ink-faint)" }}>
                  There is no account, no cloud sync, and no telemetry. ClipVault does not make network requests. Your archive stays in{" "}
                  <span className="font-mono text-[10.5px]">{info?.dataDir}</span>.
                </p>
              </div>
            </div>

            <Row title="Monitoring" help="Pause without quitting. The tray icon and title bar show the current state.">
              <div className="flex items-center gap-3">
                <span className="text-[11.5px]" style={{ color: monitoring ? "var(--success)" : "var(--ink-faint)" }}>
                  {monitoring ? "Running" : "Paused"}
                </span>
                <Switch
                  on={monitoring}
                  onClick={async () => {
                    await setMonitoring(!monitoring);
                    update("monitoringEnabled", !monitoring);
                  }}
                />
              </div>
            </Row>

            <Row title="Sensitive apps" help="Excluded process names are managed on the Clipboard tab.">
              <span className="font-mono text-[11px]" style={{ color: "var(--ink-faint)" }}>
                {draft.excludedApps.length} excluded
              </span>
            </Row>

            <Row title="Encryption at rest" help="The local database is not encrypted by default. Protect this PC with BitLocker or Windows device encryption. Exported archives can be password-protected with AES-256-GCM.">
              <span className="flex items-center gap-1.5 text-[11px]" style={{ color: "var(--ink-faint)" }}>
                <Lock size={12} />
                Backups only
              </span>
            </Row>

            <div className="mt-6 rounded-lg border p-4" style={{ borderColor: "color-mix(in srgb, var(--danger) 30%, var(--line))" }}>
              <div className="flex items-center justify-between gap-3">
                <div>
                  <div className="field-label">Clear clipboard history</div>
                  <div className="field-help">Permanently deletes every clip and stored asset. There is no undo.</div>
                </div>
                <button className="btn btn-danger" onClick={() => setClearOpen(true)}>
                  <Trash2 size={13} />
                  Clear
                </button>
              </div>
            </div>
          </div>
        )}

        {settingsTab === "about" && (
          <div className="settings-section">
            <div className="flex items-center gap-4">
              <BrandMark size={58} />
              <div>
                <div className="text-[21px] font-semibold tracking-[-0.02em]">ClipVault</div>
                <div className="text-[12px]" style={{ color: "var(--ink-faint)" }}>
                  Copy once. Keep forever.
                </div>
              </div>
              <span className="ml-auto rounded-md border px-2 py-1 font-mono text-[10px]" style={{ borderColor: "var(--line)", color: "var(--ink-muted)" }}>
                v{info?.version ?? "1.0.2"}
              </span>
            </div>

            <p className="mt-5 max-w-[520px] text-[12.5px] leading-relaxed" style={{ color: "var(--ink-muted)" }}>
              A permanent, local clipboard archive for Windows 10 and 11. ClipVault reads the native clipboard, stores text, images,
              files, and registered formats in SQLite with assets on disk, and keeps them until you delete them.
            </p>

            <div className="mt-6 overflow-hidden rounded-lg border" style={{ borderColor: "var(--line)" }}>
              {[
                { label: "Version", value: info?.version ?? "1.0.2" },
                { label: "Schema", value: String(info?.schemaVersion ?? 1) },
                { label: "Platform", value: info?.platform ?? "Windows" },
                { label: "Data folder", value: info?.dataDir ?? "" },
                { label: "Database", value: info?.dbPath ?? "" },
                { label: "Logs", value: info?.logsDir ?? "" },
                { label: "Archive size", value: formatBytes(total) },
              ].map((item, index) => (
                <div
                  key={item.label}
                  className="grid grid-cols-[110px_minmax(0,1fr)] gap-3 px-3.5 py-2.5 text-[11.5px]"
                  style={{ borderTop: index ? "1px solid var(--line)" : undefined }}
                >
                  <span style={{ color: "var(--ink-faint)" }}>{item.label}</span>
                  <span className="truncate font-mono text-[10.5px]" style={{ color: "var(--ink-muted)" }} title={item.value}>
                    {item.value}
                  </span>
                </div>
              ))}
            </div>

            <div className="mt-5 flex flex-wrap gap-2">
              <button className="btn" onClick={() => api.openDataDir().catch((e) => toast(String(e), "error"))}>
                <FolderOpen size={13} />
                Open data folder
              </button>
              <button className="btn" onClick={() => api.openPath(info?.logsDir ?? "").catch(() => {})}>
                <FolderOpen size={13} />
                Open logs
              </button>
            </div>

            <p className="mt-6 font-mono text-[10px] leading-relaxed" style={{ color: "var(--ink-faint)" }}>
              Rust · Tauri 2 · React · TypeScript · Tailwind CSS · SQLite FTS5 · Windows clipboard APIs
              <br />
              Clipboard content is never executed or uploaded.
            </p>
          </div>
        )}
      </div>

      {confirm && (
        <ConfirmDialog
          title={confirm.title}
          description={confirm.description}
          confirmLabel={confirm.label}
          danger={confirm.danger}
          onConfirm={confirm.action}
          onClose={() => setConfirm(null)}
        />
      )}

      {moveTarget && (
        <ConfirmDialog
          title="Move the archive?"
          description={`ClipVault will copy the database and all assets to ${moveTarget}, then restart. The original folder is left intact as a fallback.`}
          confirmLabel="Move and restart"
          onConfirm={async () => {
            try {
              await api.moveStorage(moveTarget);
              toast("Moving archive… ClipVault will restart.");
            } catch (e) {
              toast(String(e), "error");
            }
          }}
          onClose={() => setMoveTarget(null)}
        />
      )}

      {exportOpen && (
        <Modal
          title="Export archive"
          description="Creates a ZIP with the database, images, thumbnails, and attachments. Import it on any PC running ClipVault."
          onClose={() => setExportOpen(false)}
          footer={
            <>
              <button className="btn btn-ghost" onClick={() => setExportOpen(false)}>
                Cancel
              </button>
              <button className="btn btn-primary" disabled={busy === "export" || (exportProtected && exportPassword.length < 6)} onClick={exportArchive}>
                <HardDrive size={13} />
                Choose location…
              </button>
            </>
          }
        >
          <label className="flex items-center justify-between gap-3">
            <div>
              <div className="field-label">Password-protect this archive</div>
              <div className="field-help">AES-256-GCM with an Argon2id-derived key. The password is not stored.</div>
            </div>
            <Switch on={exportProtected} onClick={() => setExportProtected((v) => !v)} />
          </label>
          {exportProtected && (
            <label className="mt-4 block">
              <span className="field-label">Password</span>
              <input
                className="input mt-1.5"
                type="password"
                value={exportPassword}
                onChange={(e) => setExportPassword(e.target.value)}
                placeholder="At least 6 characters"
                autoFocus
              />
            </label>
          )}
        </Modal>
      )}

      {passwordFor && (
        <Modal
          title="Password required"
          description="This archive is encrypted. Enter the password used when it was created."
          onClose={() => {
            setPasswordFor(null);
            setPassword("");
          }}
          footer={
            <>
              <button
                className="btn btn-ghost"
                onClick={() => {
                  setPasswordFor(null);
                  setPassword("");
                }}
              >
                Cancel
              </button>
              <button
                className="btn btn-primary"
                disabled={!password || busy !== null}
                onClick={() => {
                  if (passwordFor.kind === "import") void importPath(passwordFor.path, password);
                  else void restorePath(passwordFor.path, password);
                }}
              >
                Unlock and restore
              </button>
            </>
          }
        >
          <input
            className="input"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoFocus
            placeholder="Archive password"
            onKeyDown={(e) => {
              if (e.key === "Enter" && password) {
                if (passwordFor.kind === "import") void importPath(passwordFor.path, password);
                else void restorePath(passwordFor.path, password);
              }
            }}
          />
        </Modal>
      )}

      {clearOpen && (
        <Modal
          title="Clear all clipboard history?"
          description="This permanently deletes every clip and stored asset from this PC. Backups are not affected."
          onClose={() => {
            setClearOpen(false);
            setClearText("");
          }}
          footer={
            <>
              <button
                className="btn btn-ghost"
                onClick={() => {
                  setClearOpen(false);
                  setClearText("");
                }}
              >
                Cancel
              </button>
              <button
                className="btn btn-danger"
                disabled={clearText !== "DELETE"}
                onClick={async () => {
                  try {
                    const count = await api.clearHistory();
                    toast(`${count} ${count === 1 ? "item" : "items"} deleted`);
                    setClearOpen(false);
                    setClearText("");
                    await refreshStats();
                    refreshData();
                  } catch (e) {
                    toast(String(e), "error");
                  }
                }}
              >
                <Trash2 size={13} />
                Delete everything
              </button>
            </>
          }
        >
          <label className="block">
            <span className="field-label">
              Type <span className="font-mono">DELETE</span> to confirm
            </span>
            <input
              className="input mt-1.5 font-mono"
              value={clearText}
              onChange={(e) => setClearText(e.target.value)}
              placeholder="DELETE"
              autoFocus
            />
          </label>
        </Modal>
      )}
    </div>
  );
}
