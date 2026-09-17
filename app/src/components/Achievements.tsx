import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { flushSync } from "react-dom";

import type { Achievements as Data, AchievementRow, FeedItem } from "../lib/types";
import { RARITY_ICON, RARITY_NAME, rarityVar } from "../lib/rarity";
import { remember, remembered, shortDate, signedOrZero } from "../lib/format";
import { ScrollBox } from "./ScrollBox";
import { withEmphasis } from "./Emphasis";
import { ContextMenu } from "./ContextMenu";
import { AnimatedNumber } from "./core/animated-number";
import { Button } from "./ui/button";
import { FilterIcon, GridIcon, ListIcon } from "./Icons";

const TABS = ["feed", "collection"] as const;
type Tab = (typeof TABS)[number];
const TAB_KEY = "murating.achTab";
const LAYOUT_KEY = "murating.achLayout";
const SORT_KEY = "murating.achSort";

const SORTS = [
  { key: "sections", label: "Sections" },
  { key: "name", label: "Name, A to Z" },
  { key: "name-desc", label: "Name, Z to A" },
  { key: "rarity-desc", label: "Rarest first" },
  { key: "rarity", label: "Common first" },
] as const;
type Sort = (typeof SORTS)[number]["key"];
const SORT_KEYS = SORTS.map((s) => s.key);
const LAYOUTS = ["grid", "list"] as const;
type Layout = (typeof LAYOUTS)[number];

const time = (at: number) =>
  new Date(at * 1000).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });

function dayLabel(at: number): string {
  const d = new Date(at * 1000);
  const today = new Date();
  const midnight = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((midnight(today) - midnight(d)) / 86_400_000);
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  return d.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" });
}

export function Achievements({ data }: { data: Data | null }) {
  const [tab, setTab] = useState<Tab>(() => remembered(TAB_KEY, "feed", TABS));
  const tabsRef = useRef<HTMLDivElement | null>(null);
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const [tabBox, setTabBox] = useState<{ left: number; width: number } | null>(null);
  const [glide, setGlide] = useState(false);

  useLayoutEffect(() => {
    const container = tabsRef.current;
    const target = tabRefs.current[tab];
    if (!container || !target) return;
    const c = container.getBoundingClientRect();
    const t = target.getBoundingClientRect();
    setTabBox({ left: t.left - c.left, width: t.width });
  }, [tab, data?.earned, data?.total]);

  useEffect(() => {
    if (!tabBox || glide) return;
    const id = requestAnimationFrame(() => setGlide(true));
    return () => cancelAnimationFrame(id);
  }, [tabBox, glide]);

  const switchTab = (next: Tab) => {
    if (next === tab) return;
    remember(TAB_KEY, next);
    const doc = document as Document & { startViewTransition?: (cb: () => void) => void };
    document.documentElement.dataset.slide = TABS.indexOf(next) > TABS.indexOf(tab) ? "forward" : "back";
    if (!doc.startViewTransition) {
      setTab(next);
      return;
    }
    doc.startViewTransition(() => flushSync(() => setTab(next)));
  };

  return (
    <div className="ach">
      <div className="tabs" ref={tabsRef} role="tablist">
        <span
          aria-hidden
          className="tabs-thumb"
          style={{
            left: tabBox?.left ?? 0,
            width: tabBox?.width ?? 0,
            opacity: tabBox ? 1 : 0,
            transition: glide ? undefined : "none",
          }}
        />
        {TABS.map((t) => (
          <button
            key={t}
            ref={(el) => {
              tabRefs.current[t] = el;
            }}
            role="tab"
            aria-selected={tab === t}
            className={`tab ${tab === t ? "on" : ""}`}
            onClick={() => switchTab(t)}
          >
            {t === "feed" ? "Feed" : "Achievements"}
            {t === "collection" && data && (
              <span className="tab-count">
                <AnimatedNumber value={data.earned} springOptions={{ bounce: 0, duration: 700 }} />/{data.total}
              </span>
            )}
          </button>
        ))}
      </div>
      <div className="ach-pane">
        {tab === "feed" ? <Feed items={data?.feed ?? []} /> : <Directory rows={data?.rows ?? []} />}
      </div>
    </div>
  );
}

function Feed({ items }: { items: FeedItem[] }) {
  const days = useMemo(() => {
    const out: [string, FeedItem[]][] = [];
    for (const it of items) {
      const label = dayLabel(it.at);
      const last = out[out.length - 1];
      if (last && last[0] === label) last[1].push(it);
      else out.push([label, [it]]);
    }
    return out;
  }, [items]);

  if (days.length === 0) {
    return (
      <ScrollBox className="ratings ach-box">
        <div className="ach-empty">
          <p>Wins, peaks and streaks show up here as you play.</p>
          <p className="faint">Nothing yet, the HUD is watching for your next ranked match.</p>
        </div>
      </ScrollBox>
    );
  }

  return (
    <ScrollBox className="ratings ach-box">
      {days.map(([label, rows]) => (
        <div key={label}>
          <div className="group-head">
            <span className="group-word">{label}</span>
            <span className="faint">&middot; {rows.length}</span>
          </div>
          {rows.map((it) =>
            it.kind === "award" ? (
              <div className="row feed-row award-row" key={`a${it.at}${it.id}`} style={rarityVar(it.rarity)}>
                <span className="sub fd-time">{time(it.at)}</span>
                <img className="rarity-icon" src={RARITY_ICON[it.rarity]} width={18} height={18} alt="" title={RARITY_NAME[it.rarity]} />
                <span className="award-body">
                  <span className="award-eyebrow">
                    {RARITY_NAME[it.rarity]} &middot; {it.title}
                  </span>
                  <span className="award-text">{withEmphasis(it.text, it.emphasis)}</span>
                </span>
              </div>
            ) : (
              <div className="row feed-row battle-row" key={`b${it.at}`}>
                <span className="sub fd-time">{time(it.at)}</span>
                <span className="fd-char">{it.character ?? "—"}</span>
                <span className="fd-mr">
                  {it.mu_after - it.change}
                  <em className={it.change >= 0 ? "up" : "down"}>{signedOrZero(it.change)}</em>
                </span>
                <span className="sub fd-rounds">
                  {it.rounds_own ?? "–"}-{it.rounds_opp ?? "–"}
                </span>
                <span className="fd-mr opp">
                  {it.opponent_mu_before ?? "—"}
                  {it.opponent_change != null && (
                    <em className={it.opponent_change >= 0 ? "up" : "down"}>{signedOrZero(it.opponent_change)}</em>
                  )}
                </span>
                <span className="fd-char">{it.opponent_character ?? "—"}</span>
                <span className="fd-name" title={it.opponent_name}>
                  {it.opponent_name}
                </span>
              </div>
            ),
          )}
        </div>
      ))}
    </ScrollBox>
  );
}

function Directory({ rows }: { rows: AchievementRow[] }) {
  const [q, setQ] = useState("");
  const [onlyEarned, setOnlyEarned] = useState(false);
  const [sort, setSort] = useState<Sort>(() => remembered(SORT_KEY, "sections", SORT_KEYS));
  const [layout, setLayout] = useState<Layout>(() => remembered(LAYOUT_KEY, "grid", LAYOUTS));
  const [landed, setLanded] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);
  const [sortMenu, setSortMenu] = useState<{ x: number; y: number } | null>(null);
  const chooseSort = (next: Sort) => {
    setSort(next);
    remember(SORT_KEY, next);
  };

  const groups = useMemo(() => {
    const needle = q.trim().toLowerCase();
    const hay = (r: AchievementRow) => `${r.title} ${r.condition} ${r.section}`.toLowerCase();
    let list = rows.filter((r) => (!onlyEarned || r.earned) && (!needle || hay(r).includes(needle)));
    const byName = (a: AchievementRow, b: AchievementRow) => a.title.localeCompare(b.title);
    if (sort === "name") list = [...list].sort(byName);
    if (sort === "name-desc") list = [...list].sort((a, b) => byName(b, a));
    if (sort === "rarity") list = [...list].sort((a, b) => a.rarity - b.rarity || byName(a, b));
    if (sort === "rarity-desc") list = [...list].sort((a, b) => b.rarity - a.rarity || byName(a, b));
    if (sort !== "sections" || needle || onlyEarned) {
      const head = needle ? `Matches · ${list.length}` : `All achievements · ${list.filter((r) => r.earned).length} of ${list.length}`;
      return [[head, list] as [string, AchievementRow[]]];
    }
    const out: [string, AchievementRow[]][] = [];
    for (const r of list) {
      const last = out[out.length - 1];
      if (last && last[0].startsWith(r.section)) last[1].push(r);
      else out.push([r.section, [r]]);
    }
    return out.map(([name, items]) => [
      `${name} · ${items.filter((r) => r.earned).length} of ${items.length}`,
      items,
    ]) as [string, AchievementRow[]][];
  }, [rows, q, onlyEarned, sort]);

  const empty = groups.every(([, items]) => items.length === 0);

  return (
    <>
      <div className="ach-bar">
        <label className="search-field">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden>
            <circle cx="11" cy="11" r="7" />
            <path d="M21 21l-4.3-4.3" />
          </svg>
          <input
            ref={searchRef}
            value={q}
            placeholder="Search achievements"
            spellCheck={false}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => e.key === "Escape" && (setQ(""), e.currentTarget.blur())}
          />
          {q && (
            <button className="search-clear" aria-label="Clear search" onClick={() => setQ("")}>
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round">
                <path d="M18 6L6 18M6 6l12 12" />
              </svg>
            </button>
          )}
        </label>
        <div className="chips">
          <button
            className={`chip ${onlyEarned ? "on" : ""}`}
            aria-pressed={onlyEarned}
            title="Show only what you have earned"
            onClick={() => setOnlyEarned((v) => !v)}
          >
            Earned
          </button>
          <button
            className={`chip ${sort !== "sections" ? "on" : ""}`}
            aria-haspopup="menu"
            aria-expanded={sortMenu !== null}
            onClick={(e) => {
              const r = e.currentTarget.getBoundingClientRect();
              setSortMenu(sortMenu ? null : { x: r.left, y: r.bottom + 6 });
            }}
          >
            <FilterIcon size={12} />
            {SORTS.find((s) => s.key === sort)?.label ?? "Sort"}
          </button>
        </div>
        <span className="ach-layout">
          <Button
            variant="tertiary"
            size="icon-compact"
            active={layout === "grid"}
            title="Grid"
            aria-label="Grid"
            onClick={() => {
              setLayout("grid");
              remember(LAYOUT_KEY, "grid");
            }}
          >
            <GridIcon />
          </Button>
          <Button
            variant="tertiary"
            size="icon-compact"
            active={layout === "list"}
            title="List"
            aria-label="List"
            onClick={() => {
              setLayout("list");
              remember(LAYOUT_KEY, "list");
            }}
          >
            <ListIcon />
          </Button>
        </span>
      </div>

      <ScrollBox className="ratings ach-box">
        {empty ? (
          <div className="ach-empty">
            <p>{q ? `No achievement matches “${q}”.` : "Nothing earned yet."}</p>
            <p className="faint">{q ? "Try a shorter word, or clear the search." : "Every achievement here is still open."}</p>
          </div>
        ) : (
          groups.map(([head, items]) => (
            <div key={head}>
              <div className="group-head">
                <span className="group-word">{head.split("·")[0].trim()}</span>
                <span className="faint">&middot; {head.split("·").slice(1).join("·").trim()}</span>
              </div>
              {layout === "grid" ? (
                <div className="ach-grid">
                  {items.map((it) => (
                    <button
                      key={it.id}
                      className={`ach-tile${it.earned ? " earned" : " locked"}`}
                      style={rarityVar(it.rarity)}
                      title={tileTitle(it)}
                      onClick={() => {
                        setLayout("list");
                        remember(LAYOUT_KEY, "list");
                        setLanded(it.id);
                      }}
                    >
                      <img className="tile-icon" src={RARITY_ICON[it.rarity]} width={40} height={40} alt="" />
                      <span className="tile-title">{it.title}</span>
                      <span className="tile-sub">
                        {it.earned ? shortDate(it.first_at) : it.progress ?? RARITY_NAME[it.rarity]}
                      </span>
                      {it.times > 1 && <span className="tile-count">&times;{it.times}</span>}
                    </button>
                  ))}
                </div>
              ) : (
                items.map((it) => (
                  <div
                    key={it.id}
                    className={`row ach-row${it.earned ? "" : " locked"}${landed === it.id ? " landed" : ""}`}
                    style={rarityVar(it.rarity)}
                  >
                    <img
                      className="rarity-icon"
                      src={RARITY_ICON[it.rarity]}
                      width={21}
                      height={21}
                      alt=""
                      title={RARITY_NAME[it.rarity]}
                    />
                    <span className="name">
                      {it.title}
                      <span className="sub">{it.condition}</span>
                    </span>
                    <span className="ach-right">
                      {it.earned ? (
                        <span className="sub">
                          {shortDate(it.first_at)}
                          {it.times > 1 && ` · ×${it.times}`}
                        </span>
                      ) : it.target ? (
                        <>
                          <span className="pg-bar">
                            <span style={{ width: `${Math.min(100, Math.round(((it.current ?? 0) / it.target) * 100))}%` }} />
                          </span>
                          <span className="sub">{it.progress}</span>
                        </>
                      ) : (
                        <span className="sub">—</span>
                      )}
                      {it.tier > 0 && it.tier_max > 0 && (
                        <span className="sub">
                          Tier {it.tier} of {it.tier_max}
                        </span>
                      )}
                    </span>
                  </div>
                ))
              )}
            </div>
          ))
        )}
      </ScrollBox>

      {sortMenu && (
        <ContextMenu
          x={sortMenu.x}
          y={sortMenu.y}
          items={SORTS.map((s) => ({
            label: s.label,
            hint: s.key === sort ? "current" : undefined,
            onSelect: () => chooseSort(s.key),
          }))}
          onClose={() => setSortMenu(null)}
        />
      )}
    </>
  );
}

function tileTitle(it: AchievementRow): string {
  if (!it.earned) return it.progress ? `${it.condition}, ${it.progress}` : it.condition;
  if (it.times > 1) return `Earned ${it.times} times, last on ${shortDate(it.last_at)}`;
  return `Earned ${shortDate(it.first_at)}`;
}
