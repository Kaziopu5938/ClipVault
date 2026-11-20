import { memo } from "react";
import {
  Check,
  Copy,
  File,
  Folder,
  Image as ImageIcon,
  Link2,
  Pin,
  Star,
  Trash2,
  Type,
} from "lucide-react";
import type { Clip } from "../lib/types";
import { cn } from "../lib/cn";
import { formatBytes, formatTime, looksLikeCode } from "../lib/api";
import { Highlight } from "./Highlight";

function listTime(ms: number) {
  const d = new Date(ms);
  const now = new Date();
  if (d.toDateString() === now.toDateString()) return formatTime(ms);
  const sameYear = d.getFullYear() === now.getFullYear();
  return d.toLocaleDateString(undefined, sameYear ? { month: "short", day: "numeric" } : { year: "2-digit", month: "short", day: "numeric" });
}

function appName(app: string | null) {
  if (!app) return "Unknown app";
  return app.replace(/\.exe$/i, "");
}

function KindGlyph({ kind }: { kind: Clip["kind"] }) {
  if (kind === "image") return <ImageIcon size={17} />;
  if (kind === "files") return <Folder size={17} />;
  if (kind === "link") return <Link2 size={17} />;
  return <Type size={17} />;
}

export const ClipRow = memo(function ClipRow({
  clip,
  active,
  checked,
  selectionMode,
  query,
  onSelect,
  onToggleCheck,
  onCopy,
  onPin,
  onFavorite,
  onDelete,
  onReveal,
}: {
  clip: Clip;
  active: boolean;
  checked: boolean;
  selectionMode: boolean;
  query: string;
  onSelect: () => void;
  onToggleCheck: () => void;
  onCopy: () => void;
  onPin: () => void;
  onFavorite: () => void;
  onDelete: () => void;
  onReveal?: () => void;
}) {
  const code = clip.kind === "text" && looksLikeCode(clip.preview);
  const fileCount = clip.files.length;
  const subKind =
    clip.kind === "image" && clip.imageWidth && clip.imageHeight
      ? `${clip.imageWidth} × ${clip.imageHeight}`
      : clip.kind === "files"
        ? `${fileCount} ${fileCount === 1 ? "item" : "items"}`
        : clip.kind === "link"
          ? "Link"
          : code
            ? "Code"
            : "Text";

  const handleClick = () => {
    if (selectionMode) onToggleCheck();
    else onSelect();
  };

  return (
    <div
      className={cn("clip-row", active && "active", checked && "selected", clip.pinned && "pinned")}
      onClick={handleClick}
      onDoubleClick={() => {
        if (!selectionMode) onCopy();
      }}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          onCopy();
        }
        if (event.key === " " && selectionMode) {
          event.preventDefault();
          onToggleCheck();
        }
      }}
      tabIndex={0}
      role="option"
      aria-selected={checked || active}
      data-clip-id={clip.id}
    >
      {selectionMode ? (
        <button
          className={cn("select-box mt-2.5", checked && "on")}
          onClick={(event) => {
            event.stopPropagation();
            onToggleCheck();
          }}
          aria-label={checked ? "Deselect" : "Select"}
        >
          {checked && <Check size={11} strokeWidth={3} />}
        </button>
      ) : (
        <div className="type-tile mt-0.5">
          {clip.kind === "image" && clip.thumbnail ? (
            <img src={clip.thumbnail} alt="" draggable={false} />
          ) : (
            <KindGlyph kind={clip.kind} />
          )}
        </div>
      )}

      <div className="clip-body">
        {clip.kind === "files" ? (
          <>
            <div className="clip-title single" style={{ fontWeight: 520 }}>
              <Highlight text={clip.files[0]?.name ?? "Files"} query={query} />
              {fileCount > 1 && (
                <span style={{ color: "var(--ink-faint)", fontWeight: 400 }}> and {fileCount - 1} more</span>
              )}
            </div>
            <div className="file-stack">
              {clip.files.slice(1, 3).map((file) => (
                <div className="file-line" key={file.path}>
                  {file.isDir ? <Folder size={11} /> : <File size={11} />}
                  <span>{file.name}</span>
                  <span className="ml-auto font-mono text-[9.5px]" style={{ color: "var(--ink-faint)" }}>
                    {file.size != null ? formatBytes(file.size) : ""}
                  </span>
                </div>
              ))}
              {fileCount > 3 && <span className="file-more">+{fileCount - 3} more</span>}
            </div>
          </>
        ) : (
          <div className={cn("clip-title", clip.kind === "link" && "single", code && "mono")}>
            <Highlight text={clip.kind === "image" ? `Image · ${clip.imageWidth ?? "?"} × ${clip.imageHeight ?? "?"}` : clip.preview} query={query} />
          </div>
        )}
        <div className="clip-sub">
          <span className="truncate">{appName(clip.sourceApp)}</span>
          <span className="dot" />
          <span className="truncate">{subKind}</span>
          <span className="dot" />
          <span className="font-mono text-[10px]">{listTime(clip.createdAt)}</span>
          {clip.useCount > 1 && (
            <>
              <span className="dot" />
              <span className="font-mono text-[10px]">used {clip.useCount}×</span>
            </>
          )}
        </div>
      </div>

      <div className="flex min-w-[96px] flex-col items-end gap-1.5">
        <div className="clip-meta">
          {clip.pinned && <Pin size={10} fill="currentColor" style={{ color: "var(--brass)" }} />}
          {clip.favorite && <Star size={10} fill="currentColor" style={{ color: "var(--brass)" }} />}
          <span>{clip.kind === "files" ? `${fileCount} files` : formatBytes(clip.byteSize)}</span>
        </div>
        <div className="clip-actions">
          {onReveal && clip.kind === "files" && (
            <button
              className="icon-btn"
              title="Show in Explorer"
              onClick={(event) => {
                event.stopPropagation();
                onReveal();
              }}
            >
              <Folder size={13} />
            </button>
          )}
          <button
            className="icon-btn"
            title="Copy (Enter)"
            onClick={(event) => {
              event.stopPropagation();
              onCopy();
            }}
          >
            <Copy size={13} />
          </button>
          <button
            className={cn("icon-btn", clip.pinned && "on")}
            title={clip.pinned ? "Unpin" : "Pin"}
            onClick={(event) => {
              event.stopPropagation();
              onPin();
            }}
          >
            <Pin size={13} fill={clip.pinned ? "currentColor" : "none"} />
          </button>
          <button
            className={cn("icon-btn", clip.favorite && "on")}
            title={clip.favorite ? "Remove from favorites" : "Add to favorites"}
            onClick={(event) => {
              event.stopPropagation();
              onFavorite();
            }}
          >
            <Star size={13} fill={clip.favorite ? "currentColor" : "none"} />
          </button>
          <button
            className="icon-btn danger"
            title="Delete"
            onClick={(event) => {
              event.stopPropagation();
              onDelete();
            }}
          >
            <Trash2 size={13} />
          </button>
        </div>
      </div>
    </div>
  );
});
