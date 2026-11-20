import {
  Clock3,
  FolderClosed,
  Image as ImageIcon,
  Layers3,
  Link2,
  Pin,
  Settings2,
  ShieldCheck,
  Star,
  Type,
} from "lucide-react";
import type { ReactNode } from "react";
import { useApp } from "../lib/app-context";
import { formatBytes } from "../lib/api";
import { cn } from "../lib/cn";
import { useNav, type Route } from "../lib/navigation";

function NavButton({
  icon,
  label,
  count,
  active,
  onClick,
}: {
  icon: ReactNode;
  label: string;
  count?: number;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button className={cn("nav-item", active && "active")} onClick={onClick} title={label}>
      {icon}
      <span>{label}</span>
      {typeof count === "number" && count > 0 && <span className="nav-count">{count > 9999 ? "9k+" : count}</span>}
    </button>
  );
}

export function Sidebar() {
  const { stats } = useApp();
  const { route, navigate } = useNav();
  const is = (kind: Route extends never ? never : string | null, view: string) =>
    route.type === "library" && route.kind === kind && route.view === view;

  const total = (stats?.dbBytes ?? 0) + (stats?.assetBytes ?? 0);
  const dbPct = total > 0 ? Math.max(4, Math.round(((stats?.dbBytes ?? 0) / total) * 100)) : 0;

  return (
    <aside className="sidebar">
      <p className="nav-label">Library</p>
      <nav className="flex flex-col gap-0.5">
        <NavButton
          icon={<Layers3 size={16} />}
          label="All items"
          count={stats?.total}
          active={is(null, "all")}
          onClick={() => navigate({ type: "library", kind: null, view: "all" })}
        />
        <NavButton
          icon={<Type size={16} />}
          label="Text"
          count={stats?.text}
          active={is("text", "all")}
          onClick={() => navigate({ type: "library", kind: "text", view: "all" })}
        />
        <NavButton
          icon={<ImageIcon size={16} />}
          label="Images"
          count={stats?.images}
          active={is("image", "all")}
          onClick={() => navigate({ type: "library", kind: "image", view: "all" })}
        />
        <NavButton
          icon={<FolderClosed size={16} />}
          label="Files"
          count={stats?.files}
          active={is("files", "all")}
          onClick={() => navigate({ type: "library", kind: "files", view: "all" })}
        />
        <NavButton
          icon={<Link2 size={16} />}
          label="Links"
          count={stats?.links}
          active={is("link", "all")}
          onClick={() => navigate({ type: "library", kind: "link", view: "all" })}
        />
      </nav>

      <p className="nav-label mt-6">Collections</p>
      <nav className="flex flex-col gap-0.5">
        <NavButton
          icon={<Star size={16} />}
          label="Favorites"
          count={stats?.favorites}
          active={is(null, "favorites")}
          onClick={() => navigate({ type: "library", kind: null, view: "favorites" })}
        />
        <NavButton
          icon={<Pin size={16} />}
          label="Pinned"
          count={stats?.pinned}
          active={is(null, "pinned")}
          onClick={() => navigate({ type: "library", kind: null, view: "pinned" })}
        />
        <NavButton
          icon={<Clock3 size={16} />}
          label="Recently used"
          count={stats?.recent}
          active={is(null, "recent")}
          onClick={() => navigate({ type: "library", kind: null, view: "recent" })}
        />
      </nav>

      <div className="storage-card">
        <div className="mb-2 flex items-center justify-between">
          <span className="text-[11.5px]" style={{ color: "var(--ink-faint)" }}>
            Local archive
          </span>
          <span className="font-mono text-[10.5px]" style={{ color: "var(--ink-muted)" }}>
            {formatBytes(total)}
          </span>
        </div>
        <div className="meter" aria-hidden="true">
          <span style={{ width: `${dbPct}%` }} />
        </div>
        <div className="mt-2 flex items-center justify-between">
          <span className="flex items-center gap-1.5 text-[10.5px]" style={{ color: "var(--ink-faint)" }}>
            <ShieldCheck size={11} />
            Local only
          </span>
          <span className="font-mono text-[9.5px]" style={{ color: "var(--ink-faint)" }}>
            {stats?.total ?? 0} items
          </span>
        </div>
      </div>

      <div className="mt-3 border-t pt-3" style={{ borderColor: "var(--line)" }}>
        <NavButton
          icon={<Settings2 size={16} />}
          label="Settings"
          active={route.type === "settings"}
          onClick={() => navigate({ type: "settings" })}
        />
      </div>
    </aside>
  );
}
