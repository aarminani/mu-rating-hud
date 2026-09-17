import { useEffect, useLayoutEffect, useRef, useState } from "react";

export type MenuItem =
  | { kind: "separator" }
  | {
      kind?: "item";
      label: string;
      hint?: string;
      danger?: boolean;
      disabled?: boolean;
      onSelect: () => void;
    };

interface Props {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

const MARGIN = 8;

export function ContextMenu({ x, y, items, onClose }: Props) {
  const ref = useRef<HTMLDivElement | null>(null);
  const [pos, setPos] = useState({ x, y });

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const nx = x + r.width > window.innerWidth - MARGIN ? x - r.width : x;
    const ny = y + r.height > window.innerHeight - MARGIN ? y - r.height : y;
    setPos({ x: Math.max(MARGIN, nx), y: Math.max(MARGIN, ny) });
  }, [x, y]);

  useEffect(() => {
    const close = () => onClose();
    const closeOutside = (e: MouseEvent) => {
      if (e.target instanceof Node && ref.current?.contains(e.target)) return;
      onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", closeOutside, true);
    window.addEventListener("wheel", close, true);
    window.addEventListener("resize", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", closeOutside, true);
      window.removeEventListener("wheel", close, true);
      window.removeEventListener("resize", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      className="ctx-menu"
      style={{ left: pos.x, top: pos.y }}
      onMouseDown={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
      role="menu"
    >
      {items.map((item, i) =>
        item.kind === "separator" ? (
          <div key={i} className="ctx-sep" />
        ) : (
          <button
            key={i}
            role="menuitem"
            className={`ctx-item ${item.danger ? "danger" : ""}`}
            disabled={item.disabled}
            onClick={() => {
              onClose();
              item.onSelect();
            }}
          >
            <span className="ctx-label">{item.label}</span>
            {item.hint && <span className="ctx-hint">{item.hint}</span>}
          </button>
        )
      )}
    </div>
  );
}
