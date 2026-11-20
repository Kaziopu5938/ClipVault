import { useEffect, useRef, useState, type ReactNode } from "react";
import { Check, ChevronDown } from "lucide-react";
import { cn } from "../lib/cn";

export interface MenuOption {
  label: string;
  icon?: ReactNode;
  selected?: boolean;
  onClick: () => void;
}

export function Dropdown({
  label,
  icon,
  options,
  align = "left",
  className,
}: {
  label: ReactNode;
  icon?: ReactNode;
  options: MenuOption[];
  align?: "left" | "right";
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDown = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={ref} className={cn("relative", className)}>
      <button className={cn("chip", open && "on")} onClick={() => setOpen((v) => !v)} aria-haspopup="menu" aria-expanded={open}>
        {icon}
        {label}
        <ChevronDown size={12} style={{ opacity: 0.6 }} />
      </button>
      {open && (
        <div className="menu" style={align === "right" ? { right: 0 } : { left: 0 }}>
          {options.map((option) => (
            <button
              key={option.label}
              className={cn("menu-item", option.selected && "on")}
              onClick={() => {
                option.onClick();
                setOpen(false);
              }}
            >
              {option.icon}
              {option.label}
              {option.selected && <Check size={13} className="check" />}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
