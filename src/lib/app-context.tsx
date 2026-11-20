import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { api, isTauri } from "./api";
import type { AppInfo, Settings, Stats } from "./types";

interface AppContextValue {
  settings: Settings | null;
  info: AppInfo | null;
  stats: Stats | null;
  monitoring: boolean;
  dataVersion: number;
  ready: boolean;
  error: string | null;
  saveSettings: (next: Settings) => Promise<void>;
  refreshStats: () => Promise<void>;
  refreshData: () => void;
  setMonitoring: (enabled: boolean) => Promise<void>;
  applyTheme: (mode: Settings["theme"]) => void;
}

const AppContext = createContext<AppContextValue | null>(null);

export function useApp() {
  const value = useContext(AppContext);
  if (!value) throw new Error("useApp outside provider");
  return value;
}

export function applyTheme(mode: Settings["theme"]) {
  const resolved =
    mode === "system"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : mode;
  document.documentElement.setAttribute("data-theme", resolved);
  try {
    localStorage.setItem("cv-theme", mode);
  } catch {
    /* storage can be unavailable */
  }
}

export function AppProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [stats, setStats] = useState<Stats | null>(null);
  const [monitoring, setMonitoringState] = useState(true);
  const [dataVersion, setDataVersion] = useState(0);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const settingsRef = useRef<Settings | null>(null);

  const refreshStats = useCallback(async () => {
    try {
      setStats(await api.getStats());
    } catch {
      /* keep the last known values */
    }
  }, []);

  const refreshData = useCallback(() => setDataVersion((v) => v + 1), []);

  const apply = useCallback((mode: Settings["theme"]) => {
    applyTheme(mode);
    if (isTauri) {
      import("@tauri-apps/api/core")
        .then(({ invoke }) => invoke("set_window_theme", { theme: mode }))
        .catch(() => {});
    }
  }, []);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const [s, i, st, m] = await Promise.all([
          api.getSettings(),
          api.getAppInfo(),
          api.getStats(),
          api.getMonitoring(),
        ]);
        if (!alive) return;
        settingsRef.current = s;
        setSettings(s);
        setInfo(i);
        setStats(st);
        setMonitoringState(m);
        apply(s.theme);
        setReady(true);
      } catch (e) {
        if (!alive) return;
        setError(String(e));
        setReady(true);
      }
    })();
    return () => {
      alive = false;
    };
  }, [apply]);

  useEffect(() => {
    if (!settings || settings.theme !== "system") return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyTheme("system");
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [settings?.theme]);

  useEffect(() => {
    if (!isTauri) return;
    let disposed = false;
    const cleanups: Array<() => void> = [];
    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const pairs: Array<[string, (payload?: unknown) => void]> = [
        ["clips-changed", () => refreshData()],
        ["monitoring-changed", (payload: unknown) => {
          const detail = payload as { enabled?: boolean } | undefined;
          if (typeof detail?.enabled === "boolean") setMonitoringState(detail.enabled);
        }],
        ["data-restored", () => {
          refreshData();
          refreshStats();
        }],
        ["settings-changed", () => {
          api.getSettings().then((s) => {
            settingsRef.current = s;
            setSettings(s);
            apply(s.theme);
          });
        }],
      ];
      for (const [event, handler] of pairs) {
        const un = await listen(event, (e) => handler(e.payload));
        if (disposed) un();
        else cleanups.push(un);
      }
    })();
    return () => {
      disposed = true;
      cleanups.forEach((fn) => fn());
    };
  }, [refreshData, refreshStats, apply]);

  const saveSettings = useCallback(
    async (next: Settings) => {
      const saved = await api.saveSettings(next);
      settingsRef.current = saved;
      setSettings(saved);
      apply(saved.theme);
      await refreshStats();
    },
    [apply, refreshStats],
  );

  const setMonitoring = useCallback(async (enabled: boolean) => {
    const actual = await api.setMonitoring(enabled);
    setMonitoringState(actual);
    if (settingsRef.current) {
      settingsRef.current = { ...settingsRef.current, monitoringEnabled: actual };
    }
  }, []);

  const value = useMemo<AppContextValue>(
    () => ({
      settings,
      info,
      stats,
      monitoring,
      dataVersion,
      ready,
      error,
      saveSettings,
      refreshStats,
      refreshData,
      setMonitoring,
      applyTheme: apply,
    }),
    [settings, info, stats, monitoring, dataVersion, ready, error, saveSettings, refreshStats, refreshData, setMonitoring, apply],
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}
