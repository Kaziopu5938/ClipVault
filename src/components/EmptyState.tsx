import { SearchX, ClipboardCopy } from "lucide-react";

export function EmptyState({
  search,
  view,
  onClear,
}: {
  search?: string;
  view?: string;
  onClear?: () => void;
}) {
  const isSearch = Boolean(search);
  return (
    <div className="empty">
      <svg width="132" height="132" viewBox="0 0 132 132" fill="none" aria-hidden="true">
        <rect x="20" y="18" width="92" height="96" rx="18" stroke="var(--line-strong)" strokeWidth="1.25" />
        <rect x="32" y="30" width="68" height="72" rx="12" fill="var(--surface)" stroke="var(--line)" />
        <path d="M48 24h36" stroke="var(--brass)" strokeWidth="5" strokeLinecap="round" />
        <path d="M40 60h24M40 72h38M40 84h30" stroke="var(--line-strong)" strokeWidth="2" strokeLinecap="round" />
        <path d="M92 92l12 12" stroke="var(--brass)" strokeWidth="2.5" strokeLinecap="round" />
        <circle cx="87" cy="87" r="12" stroke="var(--brass)" strokeWidth="2" />
        <circle cx="26" cy="110" r="2" fill="var(--brass)" opacity=".55" />
        <circle cx="114" cy="38" r="1.5" fill="var(--brass)" opacity=".4" />
      </svg>
      {isSearch ? (
        <>
          <h2>No matches for “{search}”</h2>
          <p>Try a shorter phrase, a filename, or a domain. Search covers text, links, and file names.</p>
          {onClear && (
            <button className="btn btn-sm mt-4" onClick={onClear}>
              Clear search
            </button>
          )}
        </>
      ) : (
        <>
          <h2>{view === "favorites" ? "No favorites yet" : view === "pinned" ? "Nothing pinned" : view === "recent" ? "Nothing used yet" : "Your archive starts here"}</h2>
          <p>
            {view === "favorites"
              ? "Star the copies you reach for often and they will stay in this collection."
              : view === "pinned"
                ? "Pin a copy to keep it within reach. Pinned items never move to the bottom."
                : view === "recent"
                  ? "Restore something from your history and it will appear here."
                  : "Copy anything — text, an image, or files — and ClipVault will keep it here, permanently."}
          </p>
          {view === "all" && (
            <span className="mt-5 flex items-center gap-2 font-mono text-[10.5px]" style={{ color: "var(--ink-faint)" }}>
              <ClipboardCopy size={12} />
              Try copying a line of text now
            </span>
          )}
        </>
      )}
      {isSearch && <SearchX size={0} aria-hidden="true" />}
    </div>
  );
}
