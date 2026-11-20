/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ["Hanken Grotesk Variable", "Segoe UI", "sans-serif"],
        mono: ["JetBrains Mono Variable", "Cascadia Mono", "monospace"],
      },
      colors: {
        bg: "var(--bg)",
        surface: "var(--surface)",
        raised: "var(--surface-2)",
        sunken: "var(--surface-3)",
        line: "var(--line)",
        "line-strong": "var(--line-strong)",
        ink: "var(--ink)",
        muted: "var(--ink-muted)",
        faint: "var(--ink-faint)",
        brass: "var(--brass)",
        "brass-strong": "var(--brass-strong)",
        danger: "var(--danger)",
        success: "var(--success)",
        info: "var(--info)",
      },
      borderRadius: {
        xl2: "14px",
      },
      boxShadow: {
        lift: "0 8px 24px -12px rgba(0,0,0,.55)",
        panel: "0 24px 70px -24px rgba(0,0,0,.7)",
      },
      transitionTimingFunction: {
        out: "cubic-bezier(.22,1,.36,1)",
      },
      keyframes: {
        "fade-up": {
          from: { opacity: "0", transform: "translateY(6px)" },
          to: { opacity: "1", transform: "translateY(0)" },
        },
        "scale-in": {
          from: { opacity: "0", transform: "scale(.985)" },
          to: { opacity: "1", transform: "scale(1)" },
        },
        "toast-in": {
          from: { opacity: "0", transform: "translateY(8px) scale(.98)" },
          to: { opacity: "1", transform: "translateY(0) scale(1)" },
        },
      },
      animation: {
        "fade-up": "fade-up .28s cubic-bezier(.22,1,.36,1) both",
        "scale-in": "scale-in .18s cubic-bezier(.22,1,.36,1) both",
        "toast-in": "toast-in .22s cubic-bezier(.22,1,.36,1) both",
      },
    },
  },
  plugins: [],
};
