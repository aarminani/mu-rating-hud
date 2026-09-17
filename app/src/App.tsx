import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import { listen } from "@tauri-apps/api/event";

import { TitleBar } from "./components/TitleBar";
import { DotsRing, Logo } from "./components/Brand";
import { StripedPattern } from "./components/magicui/striped-pattern";
import { Button } from "./components/ui/button";
import { HomeIcon, MenuIcon } from "./components/Icons";
import { SizeProvider } from "./lib/size-context";
import { ConnectList } from "./components/ConnectList";
import { FighterSelect } from "./components/FighterSelect";
import { RatingsList } from "./components/RatingsList";
import { SettingsPane } from "./components/SettingsPane";
import { LastBattle, LogoPager, PagerMark, type PagerPage } from "./components/LogoPager";
import { portraitFor } from "./lib/portraits";
import { Achievements as AchievementsView } from "./components/Achievements";
import { StarIcon } from "./components/Icons";
import { rarityVar } from "./lib/rarity";
import type {
  Achievements,
  AccountRow,
  ConnectProgress,
  Connected,
  FollowState,
  Probe,
  RatingsUpdate,
  SessionSummary,
  SettingKey,
  Settings,
} from "./lib/types";

const ID_KEY = "murating.tekkenId";
const IDS_KEY = "murating.tekkenIds";

function legacyIds(): string[] {
  try {
    const list = JSON.parse(localStorage.getItem(IDS_KEY) ?? "null");
    if (Array.isArray(list) && list.length) return list.map(String);
    return [localStorage.getItem(ID_KEY) ?? ""];
  } catch {
    return [];
  }
}

const SPLASH_MIN_MS = 3000;
const SPLASH_MAX_MS = 5000;

const GRACE_MS = 4000;
const COUNT_FROM = 10;

const quiet = () => {};

export default function App() {
  const [splashDone, setSplashDone] = useState(false);
  const splashWaitRef = useRef(SPLASH_MIN_MS + Math.random() * (SPLASH_MAX_MS - SPLASH_MIN_MS));
  useEffect(() => {
    const t = window.setTimeout(() => setSplashDone(true), splashWaitRef.current);
    return () => window.clearTimeout(t);
  }, []);
  const [probe, setProbe] = useState<Probe | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [follow, setFollow] = useState<FollowState | null>(null);
  const followRef = useRef(follow);
  followRef.current = follow;
  const [accounts, setAccounts] = useState<AccountRow[]>([]);
  const [session, setSession] = useState<SessionSummary | null>(null);
  const [pagerPage, setPagerPage] = useState<PagerPage>("accounts");
  const [live, setLive] = useState<{ id: string; u: RatingsUpdate } | null>(null);

  const [editing, setEditing] = useState<{ addRow: boolean } | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<Record<string, ConnectProgress>>({});
  const [connectError, setConnectError] = useState<string | null>(null);
  const [switching, setSwitching] = useState<string | null>(null);

  const [view, setView] = useState<"ratings" | "settings" | "achievements">("ratings");
  const [ach, setAch] = useState<Achievements | null>(null);

  const [closeIn, setCloseIn] = useState<number | null>(null);
  const [hovering, setHovering] = useState(false);

  const [polledAt, setPolledAt] = useState<number | null>(null);
  const [centralAt, setCentralAt] = useState<number | null>(null);
  const [feedLoading, setFeedLoading] = useState(false);
  const [, setTick] = useState(0);
  useEffect(() => {
    const h = setInterval(() => setTick((n) => n + 1), 30_000);
    return () => clearInterval(h);
  }, []);
  const pollAge = polledAt === null ? null : Date.now() / 1000 - polledAt;
  const pollFresh = pollAge !== null && pollAge < 150;
  const pollLabel =
    polledAt === null
      ? "at -----"
      : `at ${new Date(polledAt * 1000).toLocaleTimeString([], {
          hour: "2-digit",
          minute: "2-digit",
          second: "2-digit",
        })}`;
  const centralBehind = centralAt === null ? null : Date.now() / 1000 - centralAt;
  const centralLate = centralBehind !== null && centralBehind > 900;
  const centralDead = centralBehind !== null && centralBehind > 8 * 3600;
  const behindLabel = (secs: number) =>
    secs >= 3600 ? `${(secs / 3600).toFixed(1)} h` : `${Math.max(1, Math.round(secs / 60))} min`;

  const pollTitle =
    polledAt === null
      ? "Waiting for the first rating update"
      : pollFresh
        ? "Live: ratings updated within the last couple of minutes"
        : "No update recently - the feed may be stalled";

  const refreshAccounts = () => invoke<AccountRow[]>("accounts_list").then(setAccounts).catch(quiet);
  const refreshAch = () => invoke<Achievements | null>("achievements").then(setAch).catch(quiet);

  const openFor = (intent: string) => {
    setCloseIn(null);
    setView("ratings");
    if (intent === "app:add-account") setEditing({ addRow: true });
    else if (intent === "app:change-account") setEditing({ addRow: false });
  };

  useEffect(() => {
    invoke<{ reopened: boolean; intent: string | null }>("main_window_ready")
      .then(({ reopened, intent }) => {
        if (reopened) setSplashDone(true);
        if (intent) openFor(intent);
      })
      .catch(quiet);
  }, []);

  useEffect(() => {
    invoke("set_utc_offset", { minutes: -new Date().getTimezoneOffset() }).catch(quiet);
  }, []);

  useEffect(() => {
    if (view === "achievements") {
      invoke<Achievements | null>("achievements_seen").then((s) => s && setAch(s)).catch(quiet);
    }
  }, [view]);

  useEffect(() => {
    invoke<Probe>("probe").then(setProbe).catch(quiet);
    invoke<number | null>("last_polled")
      .then((t) => t && setPolledAt((cur) => (cur && cur > t ? cur : t)))
      .catch(quiet);
    invoke<number | null>("central_at").then((t) => t && setCentralAt(t)).catch(quiet);
    invoke<boolean>("feed_loading").then(setFeedLoading).catch(quiet);
    invoke<Settings>("settings").then(setSettings).catch(quiet);
    const ask = setInterval(() => {
      if (followRef.current) return clearInterval(ask);
      invoke<FollowState | null>("follow_state")
        .then((f) => f && setFollow((cur) => cur ?? f))
        .catch(quiet);
    }, 400);
    invoke<SessionSummary | null>("session_summary").then(setSession).catch(quiet);
    refreshAccounts();
    refreshAch();

    const uns = [
      listen<Settings>("settings:changed", (e) => {
        setSettings(e.payload);
        if (!e.payload.auto_hide) setCloseIn(null);
      }),
      listen<FollowState>("follow:state", (e) => {
        setFollow(e.payload);
        if (e.payload.at !== "following") {
          setSession(null);
          setAch(null);
        }
        refreshAccounts();
        refreshAch();
      }),
      listen<Achievements>("achievements:new", (e) => setAch(e.payload)),
      listen("achievements:changed", () => refreshAch()),
      listen<AccountRow[]>("accounts:changed", (e) => setAccounts(e.payload)),
      listen<SessionSummary>("session:update", (e) => setSession(e.payload)),
      listen<ConnectProgress>("connect:progress", (e) =>
        setProgress((p) => ({ ...p, [e.payload.tekken_id]: e.payload })),
      ),
      listen("app:unparked", () => refreshAccounts()),
      listen("app:add-account", () => openFor("app:add-account")),
      listen("app:change-account", () => openFor("app:change-account")),
      listen("app:show-ratings", () => openFor("app:show-ratings")),
      listen<number>("feed:polled", (e) => setPolledAt(e.payload)),
      listen<number>("feed:central", (e) => setCentralAt(e.payload)),
      listen<boolean>("feed:loading", (e) => setFeedLoading(e.payload)),
      listen<RatingsUpdate>("ratings:update", (e) => {
        const f = followRef.current;
        if (f?.at === "following") setLive({ id: f.me.tekken_id, u: e.payload });
      }),
    ];
    return () => {
      clearInterval(ask);
      uns.forEach((u) => u.then((f) => f()).catch(quiet));
    };
  }, []);

  const flip = (key: SettingKey, on: boolean) =>
    invoke<Settings>("set_setting", { key, on })
      .then((s) => {
        setSettings(s);
        if (!s.auto_hide) setCloseIn(null);
      })
      .catch(quiet);

  const following = follow?.at === "following" ? follow : null;
  const me: Connected | null = following
    ? live && live.id === following.me.tekken_id
      ? { ...following.me, character: live.u.character, mu: live.u.mu, ratings: live.u.ratings }
      : following.me
    : null;

  const screen: "loading" | "connect" | "connected" | "panel" = !follow || !splashDone
    ? "loading"
    : editing || busy || follow.at === "setup" || follow.at === "manual"
      ? "connect"
      : following
        ? "connected"
        : "panel";
  const fighter = screen === "connect" && Boolean(settings?.fighter_select);
  const autoHide = settings?.auto_hide ?? true;
  const pagerScroll = settings?.pager_auto_scroll ?? true;

  const disconnect = () => {
    setBusy(true);
    setConnectError(null);
    invoke("disconnect_all")
      .catch(quiet)
      .finally(() => {
        setBusy(false);
        setEditing(null);
        setSession(null);
        setLive(null);
        setPolledAt(null);
        try {
          localStorage.removeItem(IDS_KEY);
          localStorage.removeItem(ID_KEY);
        } catch {
        }
        refreshAccounts();
      });
  };

  const connect = (ids: string[], prefer?: string) => {
    setBusy(true);
    setConnectError(null);
    setProgress({});
    invoke<Connected>("connect", { tekkenIds: ids, prefer: prefer ?? null })
      .then(() => {
        setBusy(false);
        setEditing(null);
        setView("ratings");
      })
      .catch((e) => {
        setBusy(false);
        setConnectError(String(e));
      });
  };

  const switchAccount = (id: string) => {
    setSwitching(id);
    invoke<Connected>("switch_account", { tekkenId: id })
      .catch(quiet)
      .finally(() => setSwitching(null));
  };

  const fitRef = useRef<HTMLDivElement | null>(null);
  const lastMode = useRef("");
  const [sizing, setSizing] = useState(false);
  const sizingGen = useRef(0);
  const applyMode = (args: { mode: string; height?: number }) => {
    const gen = ++sizingGen.current;
    setSizing(true);
    const settle = () => {
      window.removeEventListener("resize", settle);
      requestAnimationFrame(() =>
        requestAnimationFrame(() => gen === sizingGen.current && setSizing(false)),
      );
    };
    invoke("set_window_mode", args)
      .catch(quiet)
      .finally(() => {
        window.addEventListener("resize", settle);
        setTimeout(settle, 250);
      });
  };
  useLayoutEffect(() => {
    if (screen === "loading") return;
    if (screen === "connected" || fighter) {
      const mode = screen === "connected" ? "connected" : "fighter";
      if (lastMode.current !== mode) {
        lastMode.current = mode;
        applyMode({ mode });
      }
      return;
    }
    fitSmall();
  });

  const fitSmall = () => {
    const el = fitRef.current;
    if (!el) return;
    const kids = Array.from(el.children) as HTMLElement[];
    const bottom = Math.max(0, ...kids.map((k) => k.offsetTop + k.offsetHeight));
    const pad = parseFloat(getComputedStyle(el).paddingBottom) || 0;
    const status = document.querySelector<HTMLElement>(".statusbar")?.offsetHeight ?? 24;
    const height = Math.ceil(el.offsetTop + bottom + pad + status);
    const key = `small:${height}`;
    if (lastMode.current !== key) {
      lastMode.current = key;
      applyMode({ mode: "small", height });
    }
  };
  const fitSmallRef = useRef(fitSmall);
  fitSmallRef.current = fitSmall;

  useEffect(() => {
    if (screen === "loading" || screen === "connected" || fighter) return;
    const el = fitRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => fitSmallRef.current());
    Array.from(el.children).forEach((k) => ro.observe(k));
    return () => ro.disconnect();
  });

  const counting = screen === "connected" && view === "ratings" && !hovering && autoHide;
  useEffect(() => {
    if (!counting) {
      setCloseIn(null);
      return;
    }
    const t = setTimeout(() => setCloseIn(COUNT_FROM), GRACE_MS);
    return () => clearTimeout(t);
  }, [counting]);

  useEffect(() => {
    if (closeIn === null) return;
    if (closeIn === 0) {
      getCurrentWindow().close();
      return;
    }
    const t = setTimeout(() => setCloseIn((n) => (n === null ? null : n - 1)), 1000);
    return () => clearTimeout(t);
  }, [closeIn]);

  const cancelLabel: "Cancel" | "Exit" = following || (follow && follow.at !== "setup" && follow.at !== "manual") ? "Cancel" : "Exit";
  const cancelEditing = () => {
    if (cancelLabel === "Cancel") {
      setEditing(null);
      setConnectError(null);
    } else {
      invoke("exit_app");
    }
  };

  const lastUpdated = (
    <span className={`updated-swap${screen === "connected" ? " armed" : ""}`}>
      <span className="faint updated-text">last updated {pollLabel}</span>
      <button className="back-link" onClick={() => openFor("app:change-account")}>
        back to accounts
      </button>
    </span>
  );

  const closeButton = (
    <button onClick={() => getCurrentWindow().close()}>
      {closeIn !== null && closeIn > 0 ? (
        <span aria-live="polite">
          Closing in <span className="count">{closeIn}..</span>
        </span>
      ) : (
        "Close"
      )}
    </button>
  );

  if (screen === "loading") {
    return (
      <div className="empty" data-tauri-drag-region>
        <StripedPattern className="[mask-image:radial-gradient(320px_circle_at_center,white,transparent)]" />
        <Logo size={44} />
        <DotsRing size={22} />
        <p className="splash-welcome">Welcome to the Tekken Resource Hub Mu Rating HUD!</p>
        <p className="muted">The app is currently reading your accounts and getting your rating ready. Please wait.</p>
      </div>
    );
  }

  return (
    <SizeProvider size="compact">
      <div
        className={`app${sizing ? " sizing" : ""}`}
        onPointerEnter={() => setHovering(true)}
        onPointerMove={() => setHovering(true)}
        onPointerLeave={() => setHovering(false)}
      >
        <TitleBar />

        {screen === "connect" &&
          (fighter ? (
            <FighterSelect
              key={`fs:${editing?.addRow ?? ""}`}
              accounts={accounts}
              addRow={editing?.addRow ?? false}
              busy={busy}
              progress={progress}
              error={connectError}
              cancelLabel={cancelLabel}
              onCancel={cancelEditing}
              onConnect={connect}
              onDisconnect={disconnect}
            />
          ) : (
            <ConnectList
              key={`cl:${editing?.addRow ?? ""}:${accounts.length}`}
              accounts={accounts}
              addRow={editing?.addRow ?? false}
              legacy={accounts.length ? [] : legacyIds()}
              busy={busy}
              progress={progress}
              error={connectError}
              cancelLabel={cancelLabel}
              onCancel={cancelEditing}
              onConnect={(ids) => connect(ids)}
              onDisconnect={disconnect}
              fitRef={fitRef}
            />
          ))}

        {screen === "panel" && follow && (
          <Panel
            follow={follow}
            fitRef={fitRef}
            onEdit={(addRow) => setEditing({ addRow })}
            closeButton={<button onClick={() => getCurrentWindow().close()}>Close</button>}
          />
        )}

        {screen === "connected" && me && (
          <div className="connected">
            <div className="cx-left" data-tauri-drag-region>
              <PagerMark art={pagerPage === "trend" ? portraitFor(session?.trend?.short) : null} />
              <LogoPager
                session={session}
                accounts={accounts}
                switching={switching}
                onSwitch={switchAccount}
                onPage={setPagerPage}
                onAdd={() => setEditing({ addRow: true })}
                autoScroll={pagerScroll}
              />
            </div>

            <div className="cx-right">
              <div className="me-head">
                <h1>{me.name}</h1>
                <span className="me-tools">
                  <Button
                    variant="tertiary"
                    size="icon-compact"
                    className="tool-btn"
                    active={view === "ratings"}
                    aria-label="Ratings list"
                    title="Ratings list"
                    onClick={() => setView("ratings")}
                  >
                    <HomeIcon />
                  </Button>
                  <Button
                    variant="tertiary"
                    size="icon-compact"
                    className="tool-btn"
                    active={view === "achievements"}
                    aria-label={view === "achievements" ? "Show ratings" : "Achievements"}
                    title={view === "achievements" ? "Show ratings" : "Achievements"}
                    onClick={() => setView((v) => (v === "achievements" ? "ratings" : "achievements"))}
                  >
                    <StarIcon />
                    {ach !== null && ach.unread > 0 && view !== "achievements" && (
                      <span
                        className="unread-dot"
                        style={rarityVar(ach.unread_rarity || 1)}
                        title={`${ach.unread} new since you last looked`}
                      />
                    )}
                  </Button>
                  <Button
                    variant="tertiary"
                    size="icon-compact"
                    className="tool-btn"
                    active={view === "settings"}
                    aria-label={view === "settings" ? "Show ratings" : "Settings"}
                    title={view === "settings" ? "Show ratings" : "Settings"}
                    onClick={() => setView((v) => (v === "settings" ? "ratings" : "settings"))}
                  >
                    <MenuIcon />
                  </Button>
                </span>
              </div>
              <p className="me-line">
                {view === "achievements" ? (
                  <>
                    <strong className="mr">
                      {ach?.earned ?? 0} of {ach?.total ?? 0}
                    </strong>{" "}
                    earned
                  </>
                ) : (
                  <>
                    <strong className="mr">{me.mu} MR</strong> on {me.character}
                    <span className="badge soft mr-badge">Highest Rating</span>
                  </>
                )}
              </p>
              {view === "settings" ? (
                <SettingsPane settings={settings} flip={flip} />
              ) : view === "achievements" ? (
                <AchievementsView data={ach} />
              ) : (
                <RatingsList ratings={me.ratings} />
              )}
            </div>

            <div className="cx-actions">
              <LastBattle session={session} live={pollFresh} />
              {view !== "ratings" && (
                <Button variant="secondary" onClick={() => setView("ratings")}>
                  Back to Ratings List
                </Button>
              )}
              <Button
                variant="brand"
                className="text-[12.5px]"
                onClick={() => invoke("start_tekken").catch(quiet)}
              >
                Start Tekken
              </Button>
              {closeButton}
            </div>
          </div>
        )}

        <div className="statusbar">
          {following && pollFresh ? (
            <span className="poll-status" title={pollTitle}>
              <span className="status-live">
                <i aria-hidden />
                connected
              </span>
              <span className="faint" aria-hidden>
                &middot;
              </span>
              {lastUpdated}
            </span>
          ) : (
            <span className="poll-status" title={pollTitle}>
              <span className={`poll-dot${pollFresh ? " live" : ""}`} />
              {lastUpdated}
            </span>
          )}
          {feedLoading ? (
            <span className="feed-behind loading" title="Reading the central feed: the day's hour files, tens of megabytes.">
              <DotsRing size={11} />
              loading feed
            </span>
          ) : (
            centralLate &&
            centralBehind !== null && (
              <button
                className={`feed-behind${centralDead ? " dead" : ""}`}
                onClick={() => invoke("reconnect_feed").catch(quiet)}
                title={
                  (centralDead
                    ? "The central feed's newest record is past the 8 hour name horizon, so opponents will show no MR until it catches up."
                    : "The central feed is answering but its newest record is falling behind.") +
                  " Click to read it again from scratch."
                }
              >
                feed {behindLabel(centralBehind)} behind &middot; reconnect
              </button>
            )
          )}
          <span className="spacer" />
          {probe?.build_id && (
            <span className={`build-credit${screen === "connected" ? " armed" : ""}`}>
              <span className="faint build-text">build {probe.build_id}</span>
              {view === "settings" || view === "achievements" ? (
                <span className="credit-line">
                  built for{" "}
                  <button className="sitelink credit-site" onClick={() => openUrl("https://tekkenresourcehub.com/")}>
                    tekkenresourcehub.com
                  </button>{" "}
                  &middot;{" "}
                  <button className="creator-link" onClick={() => openUrl("https://linktr.ee/aarminani")}>
                    a
                  </button>
                </span>
              ) : (
                <span className="credit-line">
                  <button className="sitelink credit-site" onClick={() => openUrl("https://wank.wavu.wiki/")}>
                    wank.wavu.wiki
                  </button>{" "}
                  by{" "}
                  <button className="sitelink credit-site" onClick={() => openUrl("https://x.com/6weetbix")}>
                    6weetbix
                  </button>{" "}
                  &middot;{" "}
                  <button className="creator-link" onClick={() => openUrl("https://linktr.ee/aarminani")}>
                    a
                  </button>
                </span>
              )}
            </span>
          )}
        </div>
      </div>
    </SizeProvider>
  );
}

function Panel({
  follow,
  fitRef,
  onEdit,
  closeButton,
}: {
  follow: FollowState;
  fitRef: React.Ref<HTMLDivElement>;
  onEdit: (addRow: boolean) => void;
  closeButton: React.ReactNode;
}) {
  const [, setTick] = useState(0);
  useEffect(() => {
    if (follow.at !== "failed") return;
    const h = setInterval(() => setTick((n) => n + 1), 1000);
    return () => clearInterval(h);
  }, [follow.at]);
  const now = () => invoke("follow_now").catch(quiet);

  let title: React.ReactNode = null;
  let body: React.ReactNode = null;
  let foot: React.ReactNode = null;
  let actions: React.ReactNode = null;

  switch (follow.at) {
    case "waiting":
      title = "Waiting for Tekken 8";
      body = follow.expected ? (
        <>
          When you start the game, the HUD connects to <strong className="em">{follow.expected}</strong>.
        </>
      ) : (
        <>When you start the game, the HUD connects to the account signed in to Steam.</>
      );
      foot = "Auto-connect is on. Turn it off in Settings or the tray menu to connect by hand.";
      actions = (
        <>
          <Button variant="secondary" onClick={() => onEdit(false)}>
            Edit accounts
          </Button>
          <Button variant="brand" onClick={now}>
            View Ratings
          </Button>
        </>
      );
      break;
    case "connecting":
      title = (
        <>
          Connecting to {follow.name}
        </>
      );
      body = "Reading the Wavu Wank page…";
      actions = (
        <Button variant="brand" loading disabled>
          Connecting
        </Button>
      );
      break;
    case "failed": {
      const secs = follow.retry_at ? Math.max(0, Math.round(follow.retry_at - Date.now() / 1000)) : null;
      title = "Could not connect";
      body = <span className="connect-error">{follow.error}</span>;
      foot =
        secs === null
          ? null
          : secs > 0
            ? `Trying ${follow.name} again in ${secs >= 60 ? `${Math.ceil(secs / 60)} min` : `${secs} s`}.`
            : `Trying ${follow.name} again…`;
      actions = (
        <>
          <Button variant="secondary" onClick={() => onEdit(false)}>
            Edit accounts
          </Button>
          <Button variant="brand" onClick={now}>
            Try again
          </Button>
        </>
      );
      break;
    }
    case "unpaired":
      title = "New Steam account";
      body = "This Steam account has no Tekken ID saved yet. Add it, and the HUD follows it from then on.";
      foot = "Nothing is shown on the HUD until then, so it never shows another account's rating.";
      actions = (
        <Button variant="brand" onClick={() => onEdit(true)}>
          Add account
        </Button>
      );
      break;
    case "paused":
      title = "Disconnected from Wavu Wank";
      body = "The Wavu Wank table is open in Explorer. Nothing writes to it until you reconnect.";
      foot = "The HUD keeps the last ratings it was given in the meantime.";
      actions = (
        <Button variant="brand" onClick={() => invoke("resume_follow").catch(quiet)}>
          Reconnect
        </Button>
      );
      break;
  }

  return (
    <div className="connect" data-tauri-drag-region ref={fitRef}>
      <div className="connect-mark" data-tauri-drag-region>
        <Logo size={96} />
      </div>
      <div className="connect-copy">
        <h1>{title}</h1>
        {body && <p>{body}</p>}
        {foot && <p className="connect-reassure">{foot}</p>}
      </div>
      <div className="connect-actions">
        {actions}
        {closeButton}
      </div>
    </div>
  );
}
