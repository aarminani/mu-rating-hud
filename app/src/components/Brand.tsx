import markUrl from "@/assets/mark.png";

export function Logo({ size = 18 }: { size?: number }) {
  return (
    <img
      src={markUrl}
      width={size}
      height={size}
      alt=""
      aria-hidden
      className="logo-mark"
      draggable={false}
      data-tauri-drag-region
    />
  );
}

export function Wordmark({ text = "Tekken Resource Hub Mu Rating HUD" }: { text?: string }) {
  return (
    <span className="wordmark" data-tauri-drag-region>
      {text}
    </span>
  );
}

export function DotsRing({ size = 26, dots = 8 }: { size?: number; dots?: number }) {
  const count = Math.max(4, Math.floor(dots));
  return (
    <span className="dots-ring" style={{ width: size, height: size }} role="status">
      {Array.from({ length: count }, (_, i) => {
        const angle = (i / count) * Math.PI * 2;
        const r = size * 0.34;
        return (
          <span
            key={i}
            className="dots-ring-seat"
            style={{
              width: size * 0.17,
              height: size * 0.17,
              transform: `translate(-50%, -50%) translate(${(Math.sin(angle) * r).toFixed(2)}px, ${(
                -Math.cos(angle) * r
              ).toFixed(2)}px)`,
            }}
          >
            <span
              className="dots-ring-dot"
              style={{ animationDelay: `${(i / count) * -1000}ms` }}
            />
          </span>
        );
      })}
      <span className="sr-only">Loading</span>
    </span>
  );
}

export function Meter({ level, tone }: { level: number; tone: string }) {
  return (
    <span className="meter" aria-hidden>
      {[0, 1, 2].map((bar) => (
        <span
          key={bar}
          className="meter-bar"
          style={{ background: bar < level ? tone : "var(--line-strong)" }}
        />
      ))}
    </span>
  );
}
