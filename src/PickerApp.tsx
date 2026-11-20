import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { File, Folder, Image as ImageIcon, Link2, Search, Type } from "lucide-react";
import type { Clip } from "./lib/types";
import { api, isTauri, relativeTime } from "./lib/api";
import { cn } from "./lib/cn";
import { BrandMark } from "./components/BrandMark";

function Glyph({ clip }: { clip: Clip }) {
  if (clip.kind === "image") return <ImageIcon size={15} />;
  if (clip.kind === "files") return clip.files[0]?.isDir ? <Folder size={15} /> : <File size={15} />;
  if (clip.kind === "link") return <Link2 size={15} />;
  return <Type size={15} />;
}

export function PickerApp() {
  const [search, setSearch] = useState("");
  const [clips, setClips] = useState<Clip[]>([]);
  const [active, setActive] = useState(0);
  const [busy, setBusy] = useState(false);
  const [openCount, setOpenCount] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    document.documentElement.classList.add("picker-mode");
    inputRef.current?.focus();
    return () => document.documentElement.classList.remove("picker-mode");
  }, []);

  useEffect(() => {
    let alive = true;
    const timer = window.setTimeout(() => {
      api
        .pickerList(search, 60)
        .then((items) => {
          if (alive) {
            setClips(items);
            setActive(0);
          }
        })
        .catch(() => {
          if (alive) setClips([]);
        });
    }, 50);
    return () => {
      alive = false;
      window.clearTimeout(timer);
    };
  }, [search, openCount]);

  useEffect(() => {
    if (!isTauri) return;
    let disposed = false;
    const cleanups: Array<() => void> = [];
    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const un = await listen("picker-open", () => {
        setSearch("");
        setActive(0);
        // Reload every time: items may have been copied while the picker was
        // hidden, and resetting an already-empty search changes no state.
        setOpenCount((count) => count + 1);
        window.setTimeout(() => inputRef.current?.focus(), 30);
      });
      if (disposed) un();
      else cleanups.push(un);
    })();
    return () => {
      disposed = true;
      cleanups.forEach((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(`[data-index="${active}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [active]);

  const choose = async (clip?: Clip) => {
    const target = clip ?? clips[active];
    if (!target || busy) return;
    setBusy(true);
    try {
      await api.copyClip(target.id);
    } catch {
      /* the archive is still open behind the picker */
    }
    await api.hidePicker().catch(() => {});
    setBusy(false);
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((index) => Math.min(clips.length - 1, index + 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((index) => Math.max(0, index - 1));
    } else if (event.key === "Enter") {
      event.preventDefault();
      void choose();
    } else if (event.key === "Escape") {
      event.preventDefault();
      api.hidePicker().catch(() => {});
    } else if (event.key === "Home") {
      event.preventDefault();
      setActive(0);
    } else if (event.key === "End") {
      event.preventDefault();
      setActive(Math.max(0, clips.length - 1));
    }
  };

  return (
    <div className="picker-root" onKeyDown={onKeyDown}>
      <div className="picker">
        <div className="picker-search">
          <BrandMark size={21} />
          <input
            ref={inputRef}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search your archive…"
            spellCheck={false}
            aria-label="Search clipboard history"
          />
          <span className="kbd">Esc</span>
        </div>

        <div ref={listRef} className="picker-list">
          {clips.length === 0 ? (
            <div className="flex h-full flex-col items-center justify-center gap-2 px-6 text-center">
              <Search size={18} style={{ color: "var(--ink-faint)" }} />
              <p className="text-[12.5px] font-medium">{search ? "No matches" : "Nothing copied yet"}</p>
              <p className="text-[11px] leading-relaxed" style={{ color: "var(--ink-faint)" }}>
                {search
                  ? "Try a shorter phrase or a filename."
                  : "Copy something and it will appear here instantly."}
              </p>
            </div>
          ) : (
            clips.map((clip, index) => (
              <div
                key={clip.id}
                data-index={index}
                className={cn("picker-row", index === active && "active")}
                onMouseEnter={() => setActive(index)}
                onClick={() => void choose(clip)}
              >
                <div className="picker-thumb">
                  {clip.kind === "image" && clip.thumbnail ? (
                    <img src={clip.thumbnail} alt="" draggable={false} />
                  ) : (
                    <Glyph clip={clip} />
                  )}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[12.5px]" style={{ color: "var(--ink)" }}>
                    {clip.kind === "files"
                      ? clip.files.map((file) => file.name).join(", ")
                      : clip.kind === "image"
                        ? `Image · ${clip.imageWidth ?? "?"} × ${clip.imageHeight ?? "?"}`
                        : clip.preview}
                  </div>
                  <div className="mt-0.5 flex items-center gap-1.5 text-[10.5px]" style={{ color: "var(--ink-faint)" }}>
                    <span className="truncate">{clip.sourceApp?.replace(/\.exe$/i, "") ?? "Unknown app"}</span>
                    <span>·</span>
                    <span className="font-mono">{relativeTime(clip.createdAt)}</span>
                    {clip.pinned && (
                      <>
                        <span>·</span>
                        <span style={{ color: "var(--brass)" }}>pinned</span>
                      </>
                    )}
                  </div>
                </div>
                <span className="picker-enter">↵</span>
              </div>
            ))
          )}
        </div>

        <div className="picker-foot">
          <span className="flex items-center gap-1">
            <span className="kbd">↑</span>
            <span className="kbd">↓</span>
            navigate
          </span>
          <span className="flex items-center gap-1">
            <span className="kbd">↵</span>
            copy
          </span>
          <span className="ml-auto font-mono text-[9.5px]">
            {clips.length ? `${clips.length} ${clips.length === 1 ? "item" : "items"}` : "empty"}
          </span>
        </div>
      </div>
    </div>
  );
}
