import { useEffect, useRef, useState, type ReactNode } from "react";

function Ticker({ children }: { children: ReactNode }) {
  const boxRef = useRef<HTMLSpanElement | null>(null);
  const runRef = useRef<HTMLSpanElement | null>(null);
  const [shift, setShift] = useState(0);

  useEffect(() => {
    const box = boxRef.current;
    const run = runRef.current;
    if (!box || !run) return;
    const measure = () => setShift(Math.max(0, Math.ceil(run.scrollWidth - box.clientWidth)));
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(box);
    ro.observe(run);
    return () => ro.disconnect();
  }, []);

  const total = Math.max(6, (shift * 2) / 35 / 0.6);
  const style = shift
    ? ({ "--tick-shift": `-${shift}px`, "--tick-duration": `${total.toFixed(2)}s` } as React.CSSProperties)
    : undefined;

  return (
    <span className={`lb-text ticker${shift ? " moving" : ""}`} ref={boxRef}>
      <span className="ticker-run" ref={runRef} style={style}>
        {children}
      </span>
    </span>
  );
}

import type { AccountRow, SessionSummary } from "../lib/types";
import { ago, monogram, signedOrZero } from "../lib/format";
import { Logo } from "./Brand";
import { BattleIcon, ChevronLeftIcon, ChevronRightIcon, PlusIcon } from "./Icons";

const PAGES = ["accounts", "trend", "goal", "session"] as const;
type Page = (typeof PAGES)[number];
export type PagerPage = Page;

export function PagerMark({ art }: { art: string | null }) {
  const [shown, setShown] = useState<string | null>(art);
  useEffect(() => {
    if (art) setShown(art);
  }, [art]);
  return (
    <div className={`connect-mark pager-mark${art ? " art" : ""}`} data-tauri-drag-region>
      <Logo size={96} />
      {shown && (
        <img
          className="pager-art"
          src={shown}
          alt=""
          aria-hidden
          draggable={false}
          decoding="async"
          data-tauri-drag-region
        />
      )}
    </div>
  );
}
const LABELS: Record<Page, string> = {
  accounts: "Accounts",
  trend: "Trend",
  goal: "Goal",
  session: "Session",
};

const AUTO_MS = 5000;

export function LogoPager({
  session,
  accounts,
  onSwitch,
  onAdd,
  switching,
  onPage,
  autoScroll = true,
}: {
  session: SessionSummary | null;
  accounts: AccountRow[];
  onSwitch: (tekkenId: string) => void;
  onAdd: () => void;
  switching: string | null;
  autoScroll?: boolean;
  onPage?: (page: PagerPage) => void;
}) {
  const [page, setPage] = useState<Page>("accounts");
  useEffect(() => {
    onPage?.(page);
  }, [page, onPage]);
  const [hover, setHover] = useState(false);
  const [keyboard, setKeyboard] = useState(false);
  const [hidden, setHidden] = useState(() => document.hidden);
  const [picked, setPicked] = useState(0);
  const pickGen = useRef(0);

  const show = (next: Page) => {
    pickGen.current += 1;
    setPage(next);
    setPicked((n) => n + 1);
  };
  const go = (step: number) => show(PAGES[(PAGES.indexOf(page) + step + PAGES.length) % PAGES.length]);

  useEffect(() => {
    const onVisibility = () => setHidden(document.hidden);
    document.addEventListener("visibilitychange", onVisibility);
    return () => document.removeEventListener("visibilitychange", onVisibility);
  }, []);

  useEffect(() => {
    if (!autoScroll || hover || keyboard || hidden || switching !== null) return;
    const gen = pickGen.current;
    const id = window.setTimeout(() => {
      if (pickGen.current !== gen) return;
      setPage((p) => PAGES[(PAGES.indexOf(p) + 1) % PAGES.length]);
    }, AUTO_MS);
    return () => window.clearTimeout(id);
  }, [page, picked, hover, keyboard, hidden, switching, autoScroll]);

  return (
    <div
      className="pager"
      onPointerEnter={() => setHover(true)}
      onPointerLeave={() => setHover(false)}
      onFocus={(e) => setKeyboard(e.target.matches(":focus-visible"))}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setKeyboard(false);
      }}
    >
      <div className="pager-stack">
        {PAGES.map((p) => (
          <div key={p} className={`pager-page${p === page ? " on" : ""}`} aria-hidden={p !== page}>
            {p === "trend" && <TrendPage session={session} />}
            {p === "goal" && <GoalPage session={session} />}
            {p === "session" && <SessionPage session={session} />}
            {p === "accounts" && (
              <AccountsPage accounts={accounts} onSwitch={onSwitch} onAdd={onAdd} switching={switching} />
            )}
          </div>
        ))}
      </div>
      <div className="pager-nav">
        <button className="pager-arrow" aria-label="Previous page" onClick={() => go(-1)}>
          <ChevronLeftIcon size={12} />
        </button>
        <span className="pager-dots">
          {PAGES.map((p) => (
            <button
              key={p}
              className={`pager-dot${p === page ? " on" : ""}`}
              aria-label={LABELS[p]}
              title={LABELS[p]}
              onClick={() => show(p)}
            />
          ))}
        </span>
        <button className="pager-arrow" aria-label="Next page" onClick={() => go(1)}>
          <ChevronRightIcon size={12} />
        </button>
      </div>
    </div>
  );
}

function TrendPage({ session }: { session: SessionSummary | null }) {
  const t = session?.trend;
  if (!t || t.points.length === 0) {
    return (
      <>
        <span className="pg-lab">Trend</span>
        <span className="pg-empty">Play a match to start the line</span>
      </>
    );
  }
  const W = 88;
  const H = 46;
  const lo = Math.min(...t.points);
  const hi = Math.max(...t.points);
  const span = Math.max(1, hi - lo);
  const step = t.points.length > 1 ? W / (t.points.length - 1) : 0;
  const xy = t.points.map((v, i) => [
    t.points.length > 1 ? i * step : W / 2,
    3 + (H - 6) * (1 - (v - lo) / span),
  ]);
  const last = xy[xy.length - 1];
  return (
    <>
      <span className="pg-lab" title={t.character}>
        {t.short}, last {t.points.length}
      </span>
      <svg width={W + 6} height={H} viewBox={`-3 0 ${W + 6} ${H}`} aria-hidden="true" className="pg-spark">
        <polyline
          fill="none"
          stroke="var(--accent)"
          strokeWidth="2"
          strokeLinejoin="round"
          strokeLinecap="round"
          points={xy.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" ")}
        />
        <circle cx={last[0]} cy={last[1]} r="2.5" fill="var(--accent)" />
      </svg>
      <span className="pg-big">&#956; {t.mu}</span>
      <span className="pg-rec">
        <span className={t.change_over >= 0 ? "up" : "down"}>{signedOrZero(t.change_over)}</span> over{" "}
        {t.points.length}
      </span>
    </>
  );
}

function GoalPage({ session }: { session: SessionSummary | null }) {
  const g = session?.goal;
  if (!g) {
    return (
      <>
        <span className="pg-lab">Next milestone</span>
        <span className="pg-empty">Waiting for a rating</span>
      </>
    );
  }
  return (
    <>
      <span className="pg-lab" title={g.character}>
        Next milestone
      </span>
      <span className="pg-big">{g.target}</span>
      <span className="pg-bar">
        <span style={{ width: `${Math.round(Math.min(1, Math.max(0, g.progress)) * 100)}%` }} />
      </span>
      <span className="pg-rec">{g.to_go} to go</span>
      {g.wins_est != null && (
        <span className="pg-lab pg-foot">
          about {g.wins_est} {g.wins_est === 1 ? "win" : "wins"}
        </span>
      )}
    </>
  );
}

function SessionPage({ session }: { session: SessionSummary | null }) {
  const s = session;
  const net = s?.net ?? 0;
  const squares = s ? [...s.last8] : [];
  const streak = s?.streak ?? 0;
  return (
    <>
      <span className="pg-lab">This session</span>
      <span className={`pg-big ${net > 0 ? "up" : net < 0 ? "down" : ""}`}>
        {signedOrZero(net)} <small>MR</small>
      </span>
      <span className="wl-row" aria-label={`${s?.wins ?? 0} wins, ${s?.losses ?? 0} losses`}>
        {Array.from({ length: 8 }, (_, i) => {
          const r = squares[i - (8 - squares.length)];
          return <span key={i} className={`wl-sq ${r === undefined ? "e" : r ? "w" : "l"}`} />;
        })}
      </span>
      <span className="pg-rec">
        {s?.wins ?? 0} W &middot; {s?.losses ?? 0} L
      </span>
      <span className="pg-hr" />
      <span className="pg-lab">Streak</span>
      <span className={`pg-rec ${streak > 0 ? "up" : streak < 0 ? "down" : ""}`}>
        {streak === 0
          ? "—"
          : `${Math.abs(streak)} ${streak > 0 ? (streak === 1 ? "win" : "wins") : streak === -1 ? "loss" : "losses"}`}
      </span>
    </>
  );
}

function AccountsPage({
  accounts,
  onSwitch,
  onAdd,
  switching,
}: {
  accounts: AccountRow[];
  onSwitch: (tekkenId: string) => void;
  onAdd: () => void;
  switching: string | null;
}) {
  return (
    <>
      <span className="pg-lab">Accounts</span>
      <div className="pg-accounts">
        {accounts.map((a) => (
          <button
            key={a.tekken_id}
            className={`pg-acc${a.active ? " on" : ""}`}
            disabled={a.active || switching !== null}
            title={a.active ? `Following ${a.name}` : `Follow ${a.name}`}
            onClick={() => onSwitch(a.tekken_id)}
          >
            <span className={`pg-av${a.active ? " on" : ""}${switching === a.tekken_id ? " busy" : ""}`}>
              {monogram(a.name)}
            </span>
            <span className="pg-an">{a.name}</span>
          </button>
        ))}
        {accounts.length < 5 && (
          <button className="pg-acc" onClick={onAdd} title="Add an account">
            <span className="pg-av">
              <PlusIcon size={11} />
            </span>
            <span className="pg-an">Add</span>
          </button>
        )}
      </div>
    </>
  );
}

export function LastBattle({ session, live }: { session: SessionSummary | null; live: boolean }) {
  const l = session?.last;
  return (
    <span className="last-battle" title={l ? new Date(l.at * 1000).toLocaleString() : undefined}>
      <BattleIcon size={14}className={`lb-icon${live ? " live" : ""}`} />
      {l ? (
        <Ticker>
          vs {l.opponent_name || "Unknown"}
          {l.opponent_character ? ` (${l.opponent_character})` : ""}{" "}
          <span className={l.change >= 0 ? "up" : "down"}>{signedOrZero(l.change)}</span>
          <span className="faint"> &middot; {ago(l.at)}</span>
        </Ticker>
      ) : (
        <span className="lb-text faint">Watching for battles</span>
      )}
    </span>
  );
}
