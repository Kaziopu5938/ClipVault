import { useEffect, useState } from "react";
import {
  ArrowUpRight,
  Copy,
  File,
  Folder,
  FolderSearch,
  Image as ImageIcon,
  Link2,
  Maximize2,
  Pin,
  Star,
  Trash2,
  Type,
  X,
} from "lucide-react";
import type { ClipDetail } from "../lib/types";
import { api, formatBytes, formatDateTime, looksLikeCode, relativeTime, stripHtml } from "../lib/api";
import { cn } from "../lib/cn";
import { Modal } from "./Modal";

export function PreviewPanel({
  clipId,
  onClose,
  onCopy,
  onCopyPlain,
  onTogglePin,
  onToggleFavorite,
  onDelete,
  onReveal,
  width = 372,
}: {
  clipId: number | null;
  onClose: () => void;
  onCopy: (id: number) => void;
  onCopyPlain: (id: number) => void;
  onTogglePin: (id: number) => void;
  onToggleFavorite: (id: number) => void;
  onDelete: (clip: ClipDetail) => void;
  onReveal: (path: string) => void;
  width?: number;
}) {
  const [clip, setClip] = useState<ClipDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [zoom, setZoom] = useState(false);

  useEffect(() => {
    if (clipId == null) {
      setClip(null);
      return;
    }
    let alive = true;
    setLoading(true);
    api
      .getClip(clipId)
      .then((detail) => {
        if (alive) setClip(detail);
      })
      .catch(() => {
        if (alive) setClip(null);
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [clipId]);

  useEffect(() => {
    if (!clip) return;
    const refresh = () => {
      api.getClip(clip.id).then(setClip).catch(() => {});
    };
    window.addEventListener("clipvault:refresh-preview", refresh);
    return () => window.removeEventListener("clipvault:refresh-preview", refresh);
  }, [clip]);

  if (clipId == null) {
    return (
      <aside className="preview" style={{ width }}>
        <div className="preview-head">
          <span className="text-[12px] font-medium" style={{ color: "var(--ink-faint)" }}>
            Preview
          </span>
          <button className="icon-btn" onClick={onClose} aria-label="Hide preview">
            <X size={15} />
          </button>
        </div>
        <div className="flex flex-1 flex-col items-center justify-center px-8 text-center">
          <div
            className="grid h-11 w-11 place-items-center rounded-xl border"
            style={{ borderColor: "var(--line)", color: "var(--ink-faint)" }}
          >
            <ArrowUpRight size={18} />
          </div>
          <p className="mt-4 text-[12.5px] font-medium">Select an item</p>
          <p className="mt-1 text-[11.5px] leading-relaxed" style={{ color: "var(--ink-faint)" }}>
            The full content, formats, and capture details appear here.
          </p>
        </div>
      </aside>
    );
  }

  if (loading || !clip) {
    return (
      <aside className="preview" style={{ width }}>
        <div className="preview-head">
          <span className="text-[12px] font-medium" style={{ color: "var(--ink-faint)" }}>
            Preview
          </span>
          <button className="icon-btn" onClick={onClose} aria-label="Hide preview">
            <X size={15} />
          </button>
        </div>
        <div className="space-y-3 p-5">
          <div className="skeleton h-3 w-24" />
          <div className="skeleton h-24 w-full" />
          <div className="skeleton h-3 w-40" />
          <div className="skeleton h-3 w-32" />
        </div>
      </aside>
    );
  }

  const text = clip.textContent ?? (clip.htmlContent ? stripHtml(clip.htmlContent) : null);
  const code = text ? looksLikeCode(text) : false;

  return (
    <aside className="preview" style={{ width }}>
      <div className="preview-head">
        <div className="flex items-center gap-2">
          <span className="text-[12px] font-medium">Preview</span>
          <span className="format-chip" style={{ height: 19, fontSize: 9 }}>
            {clip.kind}
          </span>
        </div>
        <div className="flex items-center gap-0.5">
          <button
            className={cn("icon-btn", clip.favorite && "on")}
            title={clip.favorite ? "Remove from favorites" : "Add to favorites"}
            onClick={() => onToggleFavorite(clip.id)}
          >
            <Star size={14} fill={clip.favorite ? "currentColor" : "none"} />
          </button>
          <button
            className={cn("icon-btn", clip.pinned && "on")}
            title={clip.pinned ? "Unpin" : "Pin"}
            onClick={() => onTogglePin(clip.id)}
          >
            <Pin size={14} fill={clip.pinned ? "currentColor" : "none"} />
          </button>
          <button className="icon-btn danger" title="Delete" onClick={() => onDelete(clip)}>
            <Trash2 size={14} />
          </button>
          <button className="icon-btn ml-1" onClick={onClose} aria-label="Hide preview">
            <X size={15} />
          </button>
        </div>
      </div>

      <div className="preview-scroll">
        <div className="flex gap-2">
          <button className="btn btn-primary flex-1" onClick={() => onCopy(clip.id)}>
            <Copy size={13} />
            Copy
          </button>
          {clip.kind !== "image" && (
            <button className="btn" onClick={() => onCopyPlain(clip.id)} title="Copy without formatting">
              Plain text
            </button>
          )}
        </div>

        <div className="mt-5">
          {clip.kind === "image" && (clip.imagePreview || clip.image) ? (
            <>
              <div className="preview-title">Image</div>
              <button className="group relative block w-full" onClick={() => setZoom(true)} title="View full size">
                <img className="preview-image" src={clip.imagePreview ?? clip.image ?? ""} alt="Clipboard image" />
                <span
                  className="absolute bottom-2 right-2 grid h-7 w-7 place-items-center rounded-md opacity-0 transition-opacity group-hover:opacity-100"
                  style={{ background: "rgba(10,10,12,.72)", color: "#fff" }}
                >
                  <Maximize2 size={13} />
                </span>
              </button>
            </>
          ) : clip.kind === "files" ? (
            <>
              <div className="preview-title">Files</div>
              <div className="overflow-hidden rounded-lg border" style={{ borderColor: "var(--line)" }}>
                {clip.files.map((file, index) => (
                  <div
                    key={file.path}
                    className="flex items-center gap-2.5 px-3 py-2.5"
                    style={{ borderTop: index ? "1px solid var(--line)" : undefined }}
                  >
                    <span style={{ color: file.isDir ? "var(--brass)" : "var(--ink-faint)" }}>
                      {file.isDir ? <Folder size={15} /> : <File size={15} />}
                    </span>
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[12px]" title={file.path}>
                        {file.name}
                      </div>
                      <div className="truncate font-mono text-[9.5px]" style={{ color: "var(--ink-faint)" }} title={file.path}>
                        {file.path}
                      </div>
                    </div>
                    <div className="flex flex-none items-center gap-2">
                      <span className="font-mono text-[9.5px]" style={{ color: "var(--ink-faint)" }}>
                        {file.size != null ? formatBytes(file.size) : ""}
                      </span>
                      <button
                        className="icon-btn"
                        title="Show in Explorer"
                        onClick={() => onReveal(file.path)}
                        disabled={!file.exists}
                      >
                        <FolderSearch size={13} />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
              {clip.files.some((f) => !f.exists) && (
                <p className="mt-2 text-[10.5px] leading-relaxed" style={{ color: "var(--ink-faint)" }}>
                  Some paths no longer exist. ClipVault stores the reference, not a copy of the file.
                </p>
              )}
            </>
          ) : (
            <>
              <div className="preview-title flex items-center justify-between">
                <span>{clip.kind === "link" ? "Link" : code ? "Content · code" : "Content"}</span>
                {(clip.hasHtml || clip.hasRtf) && (
                  <span className="flex gap-1">
                    {clip.hasHtml && <span className="format-chip">HTML</span>}
                    {clip.hasRtf && <span className="format-chip">RTF</span>}
                  </span>
                )}
              </div>
              <div className={cn("paper", code && "mono")}>{text ?? clip.preview}</div>
              {clip.kind === "link" && (
                <div className="mt-2 flex items-center gap-1.5 text-[10.5px]" style={{ color: "var(--ink-faint)" }}>
                  <Link2 size={11} />
                  Copy restores the link to the Windows clipboard.
                </div>
              )}
            </>
          )}
        </div>

        <div className="mt-6">
          <div className="preview-title">Details</div>
          <dl className="detail-grid">
            <dt>Captured</dt>
            <dd>{formatDateTime(clip.createdAt)}</dd>
            <dt>Last used</dt>
            <dd>{clip.lastUsedAt ? `${relativeTime(clip.lastUsedAt)} · ${clip.useCount}×` : "Never restored"}</dd>
            <dt>Size</dt>
            <dd className="mono">
              {clip.kind === "files" ? `${clip.files.length} items` : formatBytes(clip.byteSize)}
              {clip.kind === "image" && clip.imageWidth ? ` · ${clip.imageWidth}×${clip.imageHeight}` : ""}
            </dd>
            <dt>Source</dt>
            <dd>{clip.sourceApp?.replace(/\.exe$/i, "") ?? "Unknown"}</dd>
            {clip.sourceTitle && (
              <>
                <dt>Window</dt>
                <dd title={clip.sourceTitle}>{clip.sourceTitle}</dd>
              </>
            )}
            <dt>Stored</dt>
            <dd>Local archive · permanent</dd>
          </dl>
        </div>

        <div className="mt-6">
          <div className="preview-title flex items-center justify-between">
            <span>Clipboard formats</span>
            <span className="font-mono text-[9.5px]" style={{ color: "var(--ink-faint)" }}>
              {clip.formats.filter((f) => f.captured).length}/{clip.formats.length}
            </span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {clip.formats.map((format) => (
              <span
                key={`${format.id}-${format.name}`}
                className={cn("format-chip", !format.captured && "miss")}
                title={format.captured ? `${format.name} · ${formatBytes(format.size)}` : `${format.name} · ${format.note ?? "Not captured"}`}
              >
                {format.name}
              </span>
            ))}
            {!clip.formats.length && <span className="text-[11px]" style={{ color: "var(--ink-faint)" }}>None recorded</span>}
          </div>
        </div>
      </div>

      {zoom && clip.image && (
        <Modal title="Image preview" onClose={() => setZoom(false)} width={920}>
          <img src={clip.image} alt="Clipboard image" className="max-h-[70vh] w-full rounded-lg object-contain" />
          <div className="mt-3 flex items-center justify-between">
            <span className="font-mono text-[10.5px]" style={{ color: "var(--ink-faint)" }}>
              {clip.imageWidth} × {clip.imageHeight} · {formatBytes(clip.byteSize)}
            </span>
            <button className="btn btn-sm btn-primary" onClick={() => onCopy(clip.id)}>
              <Copy size={12} />
              Copy image
            </button>
          </div>
        </Modal>
      )}
    </aside>
  );
}

export const PreviewIcons = { Type, ImageIcon };
