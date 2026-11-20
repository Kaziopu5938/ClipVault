import { createContext, useCallback, useContext, useMemo, useRef, useState, type ReactNode } from "react";
import { AlertTriangle, Check, Info, X } from "lucide-react";

type Tone = "success" | "error" | "info";
interface ToastItem {
  id: number;
  message: string;
  tone: Tone;
}

const ToastContext = createContext<(message: string, tone?: Tone) => void>(() => {});

export function useToast() {
  return useContext(ToastContext);
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<ToastItem[]>([]);
  const counter = useRef(0);

  const toast = useCallback((message: string, tone: Tone = "success") => {
    const id = ++counter.current;
    setItems((prev) => [...prev.slice(-2), { id, message, tone }]);
    window.setTimeout(() => {
      setItems((prev) => prev.filter((item) => item.id !== id));
    }, 2800);
  }, []);

  const value = useMemo(() => toast, [toast]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div className="toasts" role="status" aria-live="polite">
        {items.map((item) => (
          <div key={item.id} className="toast">
            {item.tone === "success" && <Check size={14} />}
            {item.tone === "error" && <AlertTriangle size={14} style={{ color: "var(--danger)" }} />}
            {item.tone === "info" && <Info size={14} style={{ color: "var(--info)" }} />}
            <span>{item.message}</span>
            <button
              className="icon-btn"
              style={{ width: 20, height: 20, marginLeft: 2 }}
              onClick={() => setItems((prev) => prev.filter((x) => x.id !== item.id))}
              aria-label="Dismiss"
            >
              <X size={12} />
            </button>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}
