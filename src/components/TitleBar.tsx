import { useEffect, useState } from "react";
import { Minus, Pause, Play, Settings2, Square, X } from "lucide-react";
import { BrandMark } from "./BrandMark";
import { useApp } from "../lib/app-context";
import { isTauri } from "../lib/api";
import { useToast } from "./Toast";
import { useNavigate } from "../lib/navigation";

export function TitleBar() {
  const { monitoring, setMonitoring, info } = useApp();
  const toast = useToast();
  const navigate = useNavigate();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!isTauri) return;
    import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
      getCurrentWindow().isMaximized().then(setMaximized).catch(() => {});
    });
  }, []);

  const windowAction = async (action: "minimize" | "maximize" | "close") => {
    if (!isTauri) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    if (action === "minimize") await win.minimize();
    if (action === "maximize") {
      await win.toggleMaximize();
      setMaximized(await win.isMaximized());
    }
    if (action === "close") await win.close();
  };

  const toggleMonitoring = async () => {
    try {
      await setMonitoring(!monitoring);
      toast(monitoring ? "Clipboard monitoring paused" : "Clipboard monitoring resumed", "info");
    } catch (e) {
      toast(String(e), "error");
    }
  };

  return (
    <header className="titlebar">
      <div className="drag-region flex min-w-0 flex-1 items-center gap-2.5" onDoubleClick={() => windowAction("maximize")}>
        <BrandMark size={25} />
        <span className="brand-word">ClipVault</span>
        <span className="brand-tag hidden lg:inline">Copy once. Keep forever.</span>
      </div>

      <div className="no-drag flex items-center gap-1">
        <button
          className="flex h-7 items-center gap-2 rounded-md px-2.5 text-[11.5px] transition-colors hover:bg-[var(--surface-2)]"
          onClick={toggleMonitoring}
          title={monitoring ? "Pause clipboard monitoring" : "Resume clipboard monitoring"}
          style={{ color: "var(--ink-faint)" }}
        >
          <span className={`status-dot ${monitoring ? "" : "paused"}`} />
          <span className="hidden sm:inline">{monitoring ? "Monitoring" : "Paused"}</span>
          {monitoring ? <Pause size={11} /> : <Play size={11} />}
        </button>
        <span className="mx-1 h-4 w-px" style={{ background: "var(--line)" }} />
        <button
          className="titlebar-btn"
          style={{ width: 34 }}
          onClick={() => navigate({ type: "settings" })}
          title="Settings"
          aria-label="Settings"
        >
          <Settings2 size={15} />
        </button>
        <button className="titlebar-btn" onClick={() => windowAction("minimize")} title="Minimize" aria-label="Minimize">
          <Minus size={15} />
        </button>
        <button
          className="titlebar-btn"
          onClick={() => windowAction("maximize")}
          title={maximized ? "Restore" : "Maximize"}
          aria-label="Maximize"
        >
          <Square size={12} />
        </button>
        <button className="titlebar-btn close" onClick={() => windowAction("close")} title="Close to tray" aria-label="Close">
          <X size={16} />
        </button>
      </div>
      <span className="sr-only">{info?.version}</span>
    </header>
  );
}
