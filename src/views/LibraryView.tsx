import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
} from "react";
import {
  ArrowDownWideNarrow,
  ArrowUpNarrowWide,
  CalendarDays,
  CheckSquare,
  ChevronDown,
  Copy,
  PanelRight,
  Pin,
  RefreshCw,
  Search,
  Star,
  Trash2,
  X,
} from "lucide-react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { Clip, ClipKind, SortMode } from "../lib/types";
import { api, dayKey, dayLabel, formatDateTime, formatTime } from "../lib/api";
import { useApp } from "../lib/app-context";
import { useToast } from "../components/Toast";
import { ClipRow } from "../components/ClipRow";
import { PreviewPanel } from "../components/PreviewPanel";
import { EmptyState } from "../components/EmptyState";
import { Dropdown } from "../components/Dropdown";
import { ConfirmDialog, Modal } from "../components/Modal";
import { cn } from "../lib/cn";

type View = "all" | "favorites" | "pinned" | "recent";
type RangeKey = "all" | "today" | "7d" | "30d" | "year" | "custom";

type Row =
  | { type: "group"; key: string; label: string; count: number }
  | { type: "clip"; key: string; clip: Clip };

const PAGE = 60;

const TITLES: Record<string, { title: string; sub: string }> = {
  all: { title: "All items", sub: "Every copy, kept. Nothing expires." },
  text: { title: "Text", sub: "Notes, code, messages, and everything typed." },
  image: { title: "Images", sub: "Screenshots and pictures, stored as files." },
  files: { title: "Files", sub: "File and folder references from Explorer." },
  link: { title: "Links", sub: "Every URL you copied, ready to restore." },
  favorites: { title: "Favorites", sub: "The copies you marked to keep close." },
  pinned: { title: "Pinned", sub: "Items you want within reach." },
  recent: { title: "Recently used", sub: "Restored from your archive, most recent first." },
};

function useDebounced<T>(value: T, delay: number) {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = window.setTimeout(() => setDebounced(value), delay);
    return () => window.clearTimeout(timer);
  }, [value, delay]);
  return debounced;
}

function rangeBounds(range: RangeKey, from: string, to: string): { from: number | null; to: number | null } {
  const now = new Date();
  const startOfDay = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  if (range === "today") return { from: startOfDay, to: null };
  if (range === "7d") return { from: startOfDay - 6 * 86400000, to: null };
  if (range === "30d") return { from: startOfDay - 29 * 86400000, to: null };
  if (range === "year") return { from: new Date(now.getFullYear(), 0, 1).getTime(), to: null };
  if (range === "custom") {
    const f = from ? new Date(`${from}T00:00:00`).getTime() : null;
    const t = to ? new Date(`${to}T23:59:59`).getTime() : null;
    return { from: f, to: t };
  }
  return { from: null, to: null };
}

export function LibraryView({ kind, view }: { kind: ClipKind | null; view: View }) {
  const { dataVersion, refreshStats, settings } = useApp();
  const toast = useToast();
  const [search, setSearch] = useState("");
  const debounced = useDebounced(search, 180);
  const [sort, setSort] = useState<SortMode>("newest");
  const [range, setRange] = useState<RangeKey>("all");
  const [customFrom, setCustomFrom] = useState("");
  const [customTo, setCustomTo] = useState("");
  const [rangeOpen, setRangeOpen] = useState(false);
  const [clips, setClips] = useState<Clip[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasMore, setHasMore] = useState(false);
  const [total, setTotal] = useState<number | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [selectionMode, setSelectionMode] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [previewOpen, setPreviewOpen] = useState(() => localStorage.getItem("cv-preview") !== "0");
  const [previewWidth, setPreviewWidth] = useState(() => Number(localStorage.getItem("cv-preview-width")) || 372);
  const [confirm, setConfirm] = useState<{ ids: number[]; label: string } | null>(null);
  const [newAvailable, setNewAvailable] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const scrollRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const reqRef = useRef(0);
  const loadingRef = useRef(false);
  const offsetRef = useRef(0);
  const hasMoreRef = useRef(false);
  const clipsRef = useRef<Clip[]>([]);
  clipsRef.current = clips;

  const bounds = useMemo(() => rangeBounds(range, customFrom, customTo), [range, customFrom, customTo]);
  const key = `${kind ?? "all"}|${view}|${debounced}|${bounds.from ?? ""}|${bounds.to ?? ""}|${sort}`;
  const title = TITLES[kind ?? view] ?? TITLES.all;

  const load = useCallback(
    async (mode: "reset" | "more") => {
      if (mode === "more" && (loadingRef.current || !hasMoreRef.current)) return;
      if (mode === "reset") reqRef.current += 1;
      const request = reqRef.current;
      loadingRef.current = true;
      setLoading(true);
      const offset = mode === "reset" ? 0 : offsetRef.current;
      try {
        const page = await api.listClips({
          kind: kind ?? undefined,
          view,
          search: debounced.trim() || undefined,
          from: bounds.from,
          to: bounds.to,
          sort,
          limit: PAGE,
          offset,
        });
        if (request !== reqRef.current) return;
        setError(null);
        if (mode === "reset") {
          setClips(page.items);
          setSelectedId((current) =>
            current != null && page.items.some((c) => c.id === current) ? current : (page.items[0]?.id ?? null),
          );
          setSelected(new Set());
          setSelectionMode(false);
          if (scrollRef.current) scrollRef.current.scrollTop = 0;
        } else {
          setClips((prev) => {
            const seen = new Set(prev.map((c) => c.id));
            return [...prev, ...page.items.filter((c) => !seen.has(c.id))];
          });
        }
        offsetRef.current = offset + page.items.length;
        hasMoreRef.current = page.hasMore;
        setHasMore(page.hasMore);
        setTotal(page.total);
      } catch (e) {
        if (request === reqRef.current) setError(String(e));
      } finally {
        if (request === reqRef.current) {
          loadingRef.current = false;
          setLoading(false);
        }
      }
    },
    [kind, view, debounced, bounds.from, bounds.to, sort],
  );

  const loadRef = useRef(load);
  loadRef.current = load;

  useEffect(() => {
    setNewAvailable(false);
    loadRef.current("reset");
  }, [key]);

  useEffect(() => {
    if (dataVersion === 0) return;
    const el = scrollRef.current;
    if (el && el.scrollTop > 140 && clipsRef.current.length > 0) {
      setNewAvailable(true);
      return;
    }
    loadRef.current("reset");
  }, [dataVersion]);

  const rows = useMemo<Row[]>(() => {
    if (sort === "used") {
      return [
        { type: "group", key: "g-used", label: "Most used", count: clips.length },
        ...clips.map((clip) => ({ type: "clip" as const, key: `c-${clip.id}`, clip })),
      ];
    }
    const out: Row[] = [];
    let last = "";
    let group: { key: string; label: string; count: number } | null = null;
    for (const clip of clips) {
      const ts = sort === "oldest" ? clip.createdAt : clip.createdAt;
      const dk = dayKey(ts);
      if (dk !== last) {
        if (group) out.push({ type: "group", key: `g-${group.key}`, label: group.label, count: group.count });
        group = { key: dk, label: dayLabel(ts), count: 1 };
        last = dk;
      } else if (group) {
        group.count += 1;
      }
      out.push({ type: "clip", key: `c-${clip.id}`, clip });
    }
    if (group) out.push({ type: "group", key: `g-${group.key}`, label: group.label, count: group.count });
    return out;
  }, [clips, sort]);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (index) => {
      const row = rows[index];
      if (!row) return 90;
      if (row.type === "group") return 34;
      if (row.clip.kind === "files" && row.clip.files.length > 1) return 110;
      return 90;
    },
    overscan: 10,
    getItemKey: (index) => rows[index]?.key ?? index,
  });

  const virtualItems = virtualizer.getVirtualItems();

  useEffect(() => {
    const last = virtualItems[virtualItems.length - 1];
    if (last && last.index >= rows.length - 6 && hasMore && !loadingRef.current) {
      void loadRef.current("more");
    }
  }, [virtualItems, rows.length, hasMore]);

  const refreshPreview = () => window.dispatchEvent(new Event("clipvault:refresh-preview"));

  const updateClip = (id: number, patch: Partial<Clip>) => {
    setClips((prev) => prev.map((c) => (c.id === id ? { ...c, ...patch } : c)));
  };

  const copy = useCallback(
    async (id: number, plain = false) => {
      try {
        await (plain ? api.copyPlain(id) : api.copyClip(id));
        updateClip(id, {
          useCount: (clipsRef.current.find((c) => c.id === id)?.useCount ?? 0) + 1,
          lastUsedAt: Date.now(),
        });
        refreshPreview();
        void refreshStats();
        toast(plain ? "Copied as plain text" : "Copied to the Windows clipboard");
      } catch (e) {
        toast(String(e), "error");
      }
    },
    [refreshStats, toast],
  );

  const togglePin = useCallback(
    async (id: number) => {
      try {
        const pinned = await api.togglePin(id);
        updateClip(id, { pinned });
        refreshPreview();
        void refreshStats();
        if (view === "pinned" && !pinned) {
          setClips((prev) => prev.filter((c) => c.id !== id));
          setSelectedId((current) => (current === id ? null : current));
        }
        toast(pinned ? "Pinned" : "Unpinned");
      } catch (e) {
        toast(String(e), "error");
      }
    },
    [refreshStats, toast, view],
  );

  const toggleFavorite = useCallback(
    async (id: number) => {
      try {
        const favorite = await api.toggleFavorite(id);
        updateClip(id, { favorite });
        refreshPreview();
        void refreshStats();
        if (view === "favorites" && !favorite) {
          setClips((prev) => prev.filter((c) => c.id !== id));
          setSelectedId((current) => (current === id ? null : current));
        }
        toast(favorite ? "Added to favorites" : "Removed from favorites");
      } catch (e) {
        toast(String(e), "error");
      }
    },
    [refreshStats, toast, view],
  );

  const deleteClips = async (ids: number[]) => {
    try {
      const count = await api.deleteClips(ids);
      setClips((prev) => prev.filter((c) => !ids.includes(c.id)));
      setSelectedId((current) => (current != null && ids.includes(current) ? null : current));
      setSelected(new Set());
      setSelectionMode(false);
      void refreshStats();
      toast(`${count} ${count === 1 ? "item" : "items"} deleted`);
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const toggleCheck = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const bulkFlag = async (flag: "pinned" | "favorite") => {
    const ids = [...selected];
    if (!ids.length) return;
    try {
      await api.bulkSetFlag(ids, flag, true);
      setClips((prev) => prev.map((c) => (ids.includes(c.id) ? { ...c, [flag]: true } : c)));
      refreshPreview();
      void refreshStats();
      toast(`${ids.length} ${ids.length === 1 ? "item" : "items"} ${flag === "pinned" ? "pinned" : "added to favorites"}`);
      setSelected(new Set());
      setSelectionMode(false);
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const reveal = async (path: string) => {
    try {
      await api.revealPath(path);
    } catch (e) {
      toast(String(e), "error");
    }
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const typing = target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable);
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        searchRef.current?.focus();
        return;
      }
      if (typing) {
        if (event.key === "Escape") (target as HTMLInputElement).blur();
        return;
      }
      if (event.key === "Escape") {
        if (selectionMode) {
          setSelectionMode(false);
          setSelected(new Set());
        } else {
          setSelectedId(null);
        }
        return;
      }
      if (!rows.length) return;
      const clipRows = rows.filter((r): r is Extract<Row, { type: "clip" }> => r.type === "clip");
      if (!clipRows.length) return;
      const index = clipRows.findIndex((r) => r.clip.id === selectedId);
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const next = Math.max(0, Math.min(clipRows.length - 1, index + (event.key === "ArrowDown" ? 1 : -1)));
        const id = clipRows[next].clip.id;
        setSelectedId(id);
        const rowIndex = rows.findIndex((r) => r.type === "clip" && r.clip.id === id);
        if (rowIndex >= 0) virtualizer.scrollToIndex(rowIndex, { align: "auto" });
        return;
      }
      if (selectedId == null) return;
      if (event.key === "Enter") {
        event.preventDefault();
        void copy(selectedId);
      } else if (event.key === "Delete") {
        event.preventDefault();
        setConfirm({ ids: [selectedId], label: "1 item" });
      } else if (event.key.toLowerCase() === "p") {
        void togglePin(selectedId);
      } else if (event.key.toLowerCase() === "f") {
        void toggleFavorite(selectedId);
      } else if (event.key === " " && selectionMode) {
        event.preventDefault();
        toggleCheck(selectedId);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [rows, selectedId, selectionMode, copy, togglePin, toggleFavorite, virtualizer]);

  useEffect(() => {
    if (!previewOpen) return;
    const onMove = (event: MouseEvent) => {
      const width = Math.min(560, Math.max(300, window.innerWidth - event.clientX));
      setPreviewWidth(width);
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      setPreviewWidth((w) => {
        localStorage.setItem("cv-preview-width", String(w));
        return w;
      });
    };
    const onDown = (event: ReactMouseEvent) => {
      event.preventDefault();
      document.body.style.cursor = "col-resize";
      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    };
    const handle = document.getElementById("preview-resizer");
    handle?.addEventListener("mousedown", onDown as unknown as EventListener);
    return () => {
      handle?.removeEventListener("mousedown", onDown as unknown as EventListener);
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
  }, [previewOpen]);

  const togglePreview = () => {
    const next = !previewOpen;
    setPreviewOpen(next);
    localStorage.setItem("cv-preview", next ? "1" : "0");
    if (next && selectedId == null && clips[0]) setSelectedId(clips[0].id);
  };

  const rangeLabel =
    range === "all"
      ? "Any time"
      : range === "today"
        ? "Today"
        : range === "7d"
          ? "Last 7 days"
          : range === "30d"
            ? "Last 30 days"
            : range === "year"
              ? "This year"
              : "Custom range";

  const totalLabel = total ?? clips.length;
  const showEmpty = !loading && clips.length === 0 && !error;

  return (
    <div className="flex h-full min-h-0 min-w-0 flex-1">
      <div className="main">
        <div className="main-head">
          <div className="flex items-start justify-between gap-4">
            <div>
              <h1 className="page-title">{title.title}</h1>
              <p className="page-sub">{title.sub}</p>
            </div>
            <div className="flex flex-none items-center gap-1.5 pt-1">
              <span className="mr-1 font-mono text-[10.5px]" style={{ color: "var(--ink-faint)" }}>
                {totalLabel.toLocaleString()} {totalLabel === 1 ? "item" : "items"}
              </span>
              <button
                className={cn("icon-btn", previewOpen && "on")}
                onClick={togglePreview}
                title={previewOpen ? "Hide preview" : "Show preview"}
              >
                <PanelRight size={15} />
              </button>
            </div>
          </div>

          <div className="search-field mt-4">
            <Search size={15} style={{ color: "var(--ink-faint)" }} />
            <input
              ref={searchRef}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search text, links, filenames…"
              spellCheck={false}
              aria-label="Search clipboard history"
            />
            {search ? (
              <button className="icon-btn" style={{ width: 20, height: 20 }} onClick={() => setSearch("")} aria-label="Clear search">
                <X size={13} />
              </button>
            ) : (
              <span className="kbd">Ctrl K</span>
            )}
          </div>

          <div className="filter-row">
            <button
              className={cn("chip", range !== "all" && "on")}
              onClick={() => setRangeOpen(true)}
            >
              <CalendarDays size={12} />
              {rangeLabel}
              <ChevronDown size={11} style={{ opacity: 0.6 }} />
            </button>
            <Dropdown
              label={sort === "newest" ? "Newest first" : sort === "oldest" ? "Oldest first" : "Most used"}
              icon={
                sort === "oldest" ? <ArrowUpNarrowWide size={12} /> : sort === "used" ? <Star size={12} /> : <ArrowDownWideNarrow size={12} />
              }
              options={[
                { label: "Newest first", selected: sort === "newest", onClick: () => setSort("newest") },
                { label: "Oldest first", selected: sort === "oldest", onClick: () => setSort("oldest") },
                { label: "Most used", selected: sort === "used", onClick: () => setSort("used") },
              ]}
            />
            <button
              className={cn("chip", selectionMode && "on")}
              onClick={() => {
                setSelectionMode((v) => !v);
                setSelected(new Set());
              }}
            >
              <CheckSquare size={12} />
              Select
            </button>
            <div className="ml-auto flex items-center gap-1">
              <button
                className="icon-btn"
                title="Refresh"
                onClick={() => {
                  setNewAvailable(false);
                  void load("reset");
                }}
              >
                <RefreshCw size={13} className={loading ? "animate-spin" : ""} />
              </button>
            </div>
          </div>
        </div>

        {selectionMode && (
          <div
            className="flex h-10 flex-none items-center gap-2 border-b px-6"
            style={{ borderColor: "var(--line)", background: "var(--brass-wash)" }}
          >
            <span className="text-[12px] font-medium">
              {selected.size} selected
            </span>
            <span className="text-[11px]" style={{ color: "var(--ink-faint)" }}>
              Esc to cancel
            </span>
            <div className="ml-auto flex items-center gap-1.5">
              <button className="btn btn-sm" disabled={!selected.size} onClick={() => bulkFlag("pinned")}>
                <Pin size={12} />
                Pin
              </button>
              <button className="btn btn-sm" disabled={!selected.size} onClick={() => bulkFlag("favorite")}>
                <Star size={12} />
                Favorite
              </button>
              <button
                className="btn btn-sm btn-danger"
                disabled={!selected.size}
                onClick={() => setConfirm({ ids: [...selected], label: `${selected.size} items` })}
              >
                <Trash2 size={12} />
                Delete
              </button>
            </div>
          </div>
        )}

        <div className="relative min-h-0 flex-1">
          {newAvailable && (
            <button
              className="absolute left-1/2 top-2 z-10 -translate-x-1/2 rounded-full border px-3 py-1.5 text-[11px] font-medium shadow-lift"
              style={{ background: "var(--surface-2)", borderColor: "var(--line-strong)", color: "var(--ink)" }}
              onClick={() => {
                setNewAvailable(false);
                void load("reset");
              }}
            >
              New clips available · refresh
            </button>
          )}
          <div ref={scrollRef} className="list-scroll">
            {error ? (
              <div className="empty">
                <h2>Could not load your archive</h2>
                <p>{error}</p>
                <button className="btn mt-4" onClick={() => load("reset")}>
                  Try again
                </button>
              </div>
            ) : showEmpty ? (
              <EmptyState search={debounced} view={view} onClear={() => setSearch("")} />
            ) : loading && clips.length === 0 ? (
              <div className="space-y-0">
                {Array.from({ length: 8 }).map((_, i) => (
                  <div key={i} className="flex gap-3 border-b px-6 py-4" style={{ borderColor: "var(--line)" }}>
                    <div className="skeleton h-10 w-10 flex-none" />
                    <div className="flex-1 space-y-2 pt-1">
                      <div className="skeleton h-3" style={{ width: `${55 + ((i * 13) % 30)}%` }} />
                      <div className="skeleton h-2.5 w-40" />
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <>
                <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
                  {virtualItems.map((item) => {
                    const row = rows[item.index];
                    if (!row) return null;
                    return (
                      <div
                        key={item.key}
                        data-index={item.index}
                        ref={virtualizer.measureElement}
                        style={{
                          position: "absolute",
                          top: 0,
                          left: 0,
                          width: "100%",
                          transform: `translateY(${item.start}px)`,
                        }}
                      >
                        {row.type === "group" ? (
                          <div className="group-head">
                            <span>{row.label}</span>
                            <span className="rule" />
                            <span className="count">{row.count}</span>
                          </div>
                        ) : (
                          <ClipRow
                            clip={row.clip}
                            active={selectedId === row.clip.id}
                            checked={selected.has(row.clip.id)}
                            selectionMode={selectionMode}
                            query={debounced}
                            onSelect={() => setSelectedId(row.clip.id)}
                            onToggleCheck={() => toggleCheck(row.clip.id)}
                            onCopy={() => copy(row.clip.id)}
                            onPin={() => togglePin(row.clip.id)}
                            onFavorite={() => toggleFavorite(row.clip.id)}
                            onDelete={() => setConfirm({ ids: [row.clip.id], label: "1 item" })}
                            onReveal={row.clip.kind === "files" ? () => reveal(row.clip.files[0]?.path ?? "") : undefined}
                          />
                        )}
                      </div>
                    );
                  })}
                </div>
                {hasMore && (
                  <div className="flex h-12 items-center justify-center gap-2 font-mono text-[10px]" style={{ color: "var(--ink-faint)" }}>
                    {loading ? <RefreshCw size={11} className="animate-spin" /> : null}
                    {loading ? "loading more" : "scroll for more"}
                  </div>
                )}
                {!hasMore && clips.length > 0 && (
                  <div className="flex h-14 items-center justify-center font-mono text-[9.5px]" style={{ color: "var(--ink-faint)" }}>
                    end of archive · {formatDateTime(clips[clips.length - 1].createdAt)}
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      </div>

      {previewOpen && (
        <>
          <div id="preview-resizer" className="preview-resizer" title="Drag to resize" />
          <PreviewPanel
            clipId={selectedId}
            width={previewWidth}
            onClose={() => {
              setPreviewOpen(false);
              localStorage.setItem("cv-preview", "0");
            }}
            onCopy={(id) => copy(id)}
            onCopyPlain={(id) => copy(id, true)}
            onTogglePin={togglePin}
            onToggleFavorite={toggleFavorite}
            onDelete={(clip) => setConfirm({ ids: [clip.id], label: "1 item" })}
            onReveal={reveal}
          />
        </>
      )}

      {rangeOpen && (
        <Modal title="Filter by date" description="Choose a window, or set an exact range." onClose={() => setRangeOpen(false)}>
          <div className="grid grid-cols-2 gap-1.5">
            {[
              { key: "all", label: "Any time" },
              { key: "today", label: "Today" },
              { key: "7d", label: "Last 7 days" },
              { key: "30d", label: "Last 30 days" },
              { key: "year", label: "This year" },
            ].map((option) => (
              <button
                key={option.key}
                className={cn("btn justify-start", range === option.key && "btn-primary")}
                onClick={() => {
                  setRange(option.key as RangeKey);
                  setRangeOpen(false);
                }}
              >
                {option.label}
              </button>
            ))}
          </div>
          <div className="mt-5 grid grid-cols-2 gap-3">
            <label className="block">
              <span className="field-label">From</span>
              <input type="date" className="input mt-1.5" value={customFrom} onChange={(e) => setCustomFrom(e.target.value)} />
            </label>
            <label className="block">
              <span className="field-label">To</span>
              <input type="date" className="input mt-1.5" value={customTo} onChange={(e) => setCustomTo(e.target.value)} />
            </label>
          </div>
          <div className="mt-4 flex justify-end">
            <button
              className="btn btn-primary"
              disabled={!customFrom && !customTo}
              onClick={() => {
                setRange("custom");
                setRangeOpen(false);
              }}
            >
              Apply range
            </button>
          </div>
        </Modal>
      )}

      {confirm && (
        <ConfirmDialog
          title="Delete from archive?"
          description={`${confirm.label} will be permanently removed from ClipVault, including stored images and formats. This cannot be undone.`}
          confirmLabel="Delete permanently"
          danger
          onConfirm={() => deleteClips(confirm.ids)}
          onClose={() => setConfirm(null)}
        />
      )}
    </div>
  );
}
