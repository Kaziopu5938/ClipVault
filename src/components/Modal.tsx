import { useEffect, type ReactNode } from "react";
import { X } from "lucide-react";
import { cn } from "../lib/cn";

export function Modal({
  title,
  description,
  children,
  footer,
  onClose,
  width = 440,
}: {
  title: string;
  description?: string;
  children?: ReactNode;
  footer?: ReactNode;
  onClose: () => void;
  width?: number;
}) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal" style={{ maxWidth: width }} role="dialog" aria-modal="true" aria-label={title}>
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-[15px] font-semibold tracking-[-0.01em]">{title}</h2>
            {description && (
              <p className="mt-1.5 text-[12.5px] leading-relaxed" style={{ color: "var(--ink-faint)" }}>
                {description}
              </p>
            )}
          </div>
          <button className="icon-btn" onClick={onClose} aria-label="Close">
            <X size={15} />
          </button>
        </div>
        {children && <div className="mt-5">{children}</div>}
        {footer && <div className="mt-6 flex justify-end gap-2">{footer}</div>}
      </div>
    </div>
  );
}

export function ConfirmDialog({
  title,
  description,
  confirmLabel = "Confirm",
  danger = false,
  onConfirm,
  onClose,
}: {
  title: string;
  description: string;
  confirmLabel?: string;
  danger?: boolean;
  onConfirm: () => void;
  onClose: () => void;
}) {
  return (
    <Modal
      title={title}
      description={description}
      onClose={onClose}
      footer={
        <>
          <button className="btn btn-ghost" onClick={onClose}>
            Cancel
          </button>
          <button
            className={cn("btn", danger ? "btn-danger" : "btn-primary")}
            onClick={() => {
              onConfirm();
              onClose();
            }}
          >
            {confirmLabel}
          </button>
        </>
      }
    />
  );
}
