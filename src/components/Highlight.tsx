import { useMemo } from "react";

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function Highlight({ text, query }: { text: string; query: string }) {
  const parts = useMemo(() => {
    const terms = query
      .trim()
      .split(/\s+/)
      .filter((t) => t.length > 1)
      .slice(0, 6)
      .map(escapeRegExp);
    if (!terms.length) return [text];
    const re = new RegExp(`(${terms.join("|")})`, "gi");
    return text.split(re);
  }, [text, query]);

  if (parts.length === 1) return <>{text}</>;
  return (
    <>
      {parts.map((part, i) =>
        i % 2 === 1 ? (
          <mark
            key={i}
            style={{
              background: "var(--brass-wash)",
              color: "inherit",
              borderRadius: 2,
              padding: "0 1px",
            }}
          >
            {part}
          </mark>
        ) : (
          <span key={i}>{part}</span>
        ),
      )}
    </>
  );
}
