import { useEffect } from "react";
import { TitleBar } from "./components/TitleBar";
import { Sidebar } from "./components/Sidebar";
import { BrandMark } from "./components/BrandMark";
import { LibraryView } from "./views/LibraryView";
import { SettingsView } from "./views/SettingsView";
import { useNav } from "./lib/navigation";
import { useApp } from "./lib/app-context";
import { isTauri } from "./lib/api";

export function App() {
  const { route, navigate } = useNav();
  const { ready, error } = useApp();

  useEffect(() => {
    if (!isTauri) return;
    let disposed = false;
    const cleanups: Array<() => void> = [];
    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const un = await listen("open-settings", () => navigate({ type: "settings" }));
      if (disposed) un();
      else cleanups.push(un);
    })();
    return () => {
      disposed = true;
      cleanups.forEach((fn) => fn());
    };
  }, [navigate]);

  if (!ready) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4" style={{ background: "var(--bg)" }}>
        <BrandMark size={54} />
        <div className="text-center">
          <div className="text-[15px] font-semibold tracking-[-0.015em]">ClipVault</div>
          <div className="mt-1 font-mono text-[10px]" style={{ color: "var(--ink-faint)" }}>
            opening your archive
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 px-10 text-center" style={{ background: "var(--bg)" }}>
        <BrandMark size={44} />
        <h1 className="text-[16px] font-semibold">ClipVault could not open its archive</h1>
        <p className="max-w-[460px] text-[12px] leading-relaxed" style={{ color: "var(--ink-muted)" }}>
          {error}
        </p>
        <button className="btn btn-primary mt-2" onClick={() => window.location.reload()}>
          Try again
        </button>
      </div>
    );
  }

  return (
    <div className="app-shell">
      <TitleBar />
      <div className="workspace">
        <Sidebar />
        {route.type === "settings" ? (
          <SettingsView />
        ) : (
          <LibraryView kind={route.kind} view={route.view} />
        )}
      </div>
    </div>
  );
}
