import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";

import { Achievements as AchievementsView } from "./components/Achievements";
import { Button } from "./components/ui/button";
import { SizeProvider } from "./lib/size-context";
import { Logo } from "./components/Brand";
import { MenuIcon, StarIcon } from "./components/Icons";
import { RARITY_ICON, RARITY_NAME, rarityVar } from "./lib/rarity";
import { withEmphasis } from "./components/Emphasis";
import { FighterSelect } from "./components/FighterSelect";
import { RatingsList } from "./components/RatingsList";
import { LogoPager, PagerMark, type PagerPage } from "./components/LogoPager";
import { portraitFor } from "./lib/portraits";
import { SettingsPane } from "./components/SettingsPane";
import type { AccountRow, Achievements, CharRating, SessionSummary, SettingKey, Settings } from "./lib/types";
import "./styles.css";
import "./theme.css";
import "./app.css";

const AWARD = {
  rarity: 4,
  kind: "achievement" as const,
  title: "Giant Killer",
  text: "Beat an opponent 320 MR above you",
  emphasis: "320 MR",
};

function Mock() {
  const [data, setData] = useState<Achievements | null>(null);
  const [view, setView] = useState<"ratings" | "achievements">("achievements");

  useEffect(() => {
    fetch("/mock/achievements.json")
      .then((r) => r.json())
      .then(setData)
      .catch(() => {});
  }, []);

  return (
    <SizeProvider size="compact">
      <div className="app" style={{ width: 560, height: 480 }}>
        <div className="connected">
          <div className="cx-left">
            <div className="connect-mark">
              <Logo size={96} />
            </div>
          </div>
          <div className="cx-right">
            <div className="me-head">
              <h1>MOCKPLAYER</h1>
              <span className="me-tools">
                <Button
                  variant="tertiary"
                  size="icon-compact"
                  className="tool-btn"
                  active={view === "achievements"}
                  onClick={() => setView((v) => (v === "achievements" ? "ratings" : "achievements"))}
                >
                  <StarIcon />
                  {data && data.unread > 0 && view !== "achievements" && (
                    <span className="unread-dot" style={rarityVar(data.unread_rarity || 1)} />
                  )}
                </Button>
                <Button variant="tertiary" size="icon-compact" className="tool-btn">
                  <MenuIcon />
                </Button>
              </span>
            </div>
            <p className="me-line">
              <strong className="mr">
                {data?.earned ?? 0} of {data?.total ?? 0}
              </strong>{" "}
              earned
            </p>
            <AchievementsView data={data} />
          </div>
        </div>
      </div>

      <div className="is-toast-preview" style={{ width: 620, paddingTop: 8 }}>
        <div className="toast-card award" style={rarityVar(AWARD.rarity)}>
          <span className="toast-mark">
            <img className="toast-rarity" src={RARITY_ICON[AWARD.rarity]} width={40} height={40} alt="" />
          </span>
          <p className="toast-text">
            <span className="toast-eyebrow">
              {RARITY_NAME[AWARD.rarity]} {AWARD.kind}
            </span>
            <strong className="toast-title">{AWARD.title}</strong>
            <span className="toast-line">{withEmphasis(AWARD.text, AWARD.emphasis)}</span>
          </p>
        </div>
      </div>
    </SizeProvider>
  );
}

const ACCOUNTS: AccountRow[] = [
  { tekken_id: "2yh7ByTerD8a", name: "MRmani", paired: true, signed_in: true, active: true, character: "Anna", mu: 2248 },
  { tekken_id: "4kQp9mZtLx2c", name: "PyreLion", paired: true, signed_in: false, active: false, character: "Bryan", mu: 2324 },
];

const FALLBACK_ACCOUNTS: AccountRow[] = [
  ...ACCOUNTS,
  { tekken_id: "9aaa1bbb2ccc", name: "Unrated", paired: true, signed_in: false, active: false, character: null, mu: null },
  { tekken_id: "8ddd3eee4fff", name: "DlcMain", paired: true, signed_in: false, active: false, character: "Roger Jr.", mu: 1600 },
];

const LONG_NAMES: AccountRow[] = [
  { tekken_id: "2yh7ByTerD8a", name: "ObsidianTempest", paired: true, signed_in: true, active: true, character: "Law", mu: 2248 },
  { tekken_id: "4kQp9mZtLx2c", name: "ttv/FrostJackal", paired: true, signed_in: false, active: false, character: "Kunimitsu", mu: 2199 },
  { tekken_id: "7gGg5hHh6iIi", name: "all frames safe", paired: true, signed_in: false, active: false, character: "Yoshimitsu", mu: 2129 },
  { tekken_id: "6jJj7kKk8lLl", name: "HeatBurst&Rage", paired: true, signed_in: false, active: false, character: "Zafina", mu: 1945 },
  { tekken_id: "5mMm6nNn7oOo", name: "VOID|SableFang", paired: true, signed_in: false, active: false, character: "Eddy", mu: 2131 },
];

function FighterMock() {
  const q = new URLSearchParams(location.search);
  const add = q.has("add");
  return (
    <SizeProvider size="compact">
      <div className="app" style={{ width: 572, height: 342 }}>
        <FighterSelect
          accounts={
            q.has("empty")
              ? []
              : q.has("five")
                ? LONG_NAMES
                : q.has("four")
                  ? LONG_NAMES.slice(0, 4)
              : q.has("unfollowed")
                ? ACCOUNTS.map((a) => ({ ...a, active: false, signed_in: false }))
                : q.has("fallbacks")
                  ? FALLBACK_ACCOUNTS
                  : ACCOUNTS
          }
          addRow={add}
          busy={false}
          progress={{}}
          error={null}
          cancelLabel="Cancel"
          onCancel={() => {}}
          onConnect={() => {}}
          onDisconnect={() => {}}
        />
      </div>
    </SizeProvider>
  );
}

const SESSION: SessionSummary = {
  since: 0,
  net: -77,
  wins: 9,
  losses: 11,
  last8: [true, false, false, true, false, true, false, false],
  streak: -2,
  last: { at: Math.floor(Date.now() / 1000) - 600, opponent_name: "PyreLion", opponent_character: "Jun", change: -5, won: false },
  trend: { character: "Lili Rochefort", short: "Lili", points: [2081, 2074, 2090, 2066, 2059, 2071, 2048, 2040, 2052, 2004], change_over: -77, mu: 2004 },
  goal: { character: "Anna Williams", mu: 2248, target: 2300, to_go: 52, progress: 0.48, wins_est: 5 },
};

function PagerMock() {
  const q = new URLSearchParams(location.search);
  const switching = q.has("switching") ? ACCOUNTS[1].tekken_id : null;
  const session = q.has("notrend") ? { ...SESSION, trend: null } : SESSION;
  const [page, setPage] = useState<PagerPage>("accounts");
  return (
    <div className="app" style={{ width: 560, height: 480 }}>
      <div className="connected">
        <div className="cx-left">
          <PagerMark art={page === "trend" ? portraitFor(session.trend?.short) : null} />
          <LogoPager session={session} accounts={ACCOUNTS} switching={switching} onSwitch={() => {}} onAdd={() => {}} onPage={setPage} />
        </div>
      </div>
    </div>
  );
}

const RATINGS: CharRating[] = [
  ["Nina Williams", 2205, 71, 2487, "Leaderboard (σ² < 75)"],
  ["Lili Rochefort", 1996, 64, 2433, "Leaderboard (σ² < 75)"],
  ["Bob Richards", 1679, 65, 96, "Leaderboard (σ² < 75)"],
  ["Anna Williams", 2248, 75, 2010, "Unqualified (σ² < 110)"],
  ["Miary Zo", 2021, 104, 182, "Unqualified (σ² < 110)"],
  ["Alisa Bosconovitch", 1959, 155, 96, "Provisional (σ² ≥ 110)"],
  ["Sergei Dragunov", 1920, null, 63, "Provisional (σ² ≥ 110)"],
].map(([character, mu, sigma, games, group], i) => ({
  character: character as string,
  mu: mu as number,
  sigma: sigma as number | null,
  games: games as number,
  group: group as string,
  last_seen: 1789056853 - i * 86400 * 9,
  change: i === 1 ? -9 : null,
}));

function RatingsMock() {
  const w = Number(new URLSearchParams(location.search).get("w") ?? 560);
  return (
    <div className="app" style={{ width: w, height: 480 }}>
      <div className="connected">
        <div className="cx-left" />
        <div className="cx-right">
          <RatingsList ratings={RATINGS} />
        </div>
      </div>
    </div>
  );
}

const SETTING_FIELD: Record<SettingKey, keyof Settings> = {
  mr: "mr_enabled",
  autohide: "auto_hide",
  autostart: "start_with_windows",
  notifications: "hide_notifications",
  autoconnect: "auto_connect",
  fighter: "fighter_select",
  pagerscroll: "pager_auto_scroll",
  overgame: "over_game",
  headless: "headless",
};

function SettingsMock() {
  const [settings, setSettings] = useState<Settings>({
    mr_enabled: true,
    auto_hide: true,
    start_with_windows: false,
    hide_notifications: false,
    auto_connect: true,
    fighter_select: false,
    pager_auto_scroll: true,
    over_game: true,
    headless: false,
  });
  return (
    <div className="app" style={{ width: 560, height: 480 }}>
      <div className="connected">
        <div className="cx-left" />
        <div className="cx-right">
          <SettingsPane
            settings={settings}
            flip={(key, on) => setSettings((s) => ({ ...s, [SETTING_FIELD[key]]: on }))}
          />
        </div>
      </div>
    </div>
  );
}

const params = new URLSearchParams(location.search);
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {params.has("fighter") ? (
      <FighterMock />
    ) : params.has("pager") ? (
      <PagerMock />
    ) : params.has("ratings") ? (
      <RatingsMock />
    ) : params.has("settings") ? (
      <SettingsMock />
    ) : (
      <Mock />
    )}
  </React.StrictMode>,
);
