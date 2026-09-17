import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import type { SettingKey, Settings, TroubleshootStatus } from "../lib/types";
import { Button } from "./ui/button";
import { ScrollBox } from "./ScrollBox";

type Item = { key: SettingKey; label: string; field: keyof Settings; sub?: string };

const SECTIONS: [string, Item[]][] = [
  [
    "HUD",
    [
      { key: "mr", label: "Enable MR", field: "mr_enabled" },
      {
        key: "autoconnect",
        label: "Auto-connect",
        field: "auto_connect",
        sub: "Follow the Steam account signed in to Tekken, no Connect click needed.",
      },
    ],
  ],
  ["Notifications", [{ key: "notifications", label: "Hide Notifications", field: "hide_notifications" }]],
  [
    "App",
    [
      { key: "autohide", label: "Auto-hide Helper", field: "auto_hide" },
      {
        key: "autostart",
        label: "Start with Windows",
        field: "start_with_windows",
        sub: "Keeps the helper ready in the tray, so it opens and connects the moment Tekken starts.",
      },
      { key: "fighter", label: "Fighter Select Style", field: "fighter_select", sub: "Pick accounts from cards instead of a list." },
      {
        key: "pagerscroll",
        label: "Page Auto Scroll",
        field: "pager_auto_scroll",
        sub: "Turns the pages under the logo every 5 seconds.",
      },
      {
        key: "overgame",
        label: "Notifications Over the Game",
        field: "over_game",
        sub: "Keeps them above Tekken. Needs Borderless or Windowed, nothing draws over exclusive Fullscreen.",
      },
      {
        key: "headless",
        label: "Headless Mode",
        field: "headless",
        sub: "Silences notifications, starts with Windows and forces Auto-connect. The tray icon greys out; clicking it or launching the helper again opens this window, which turns Headless Mode off.",
      },
    ],
  ],
];

const CHANGELOG: {
  version: string;
  date: string;
  lead: string;
  sections: { title: string; notes: string[] }[];
}[] = [
  {
    version: "0.1.0",
    date: "September 17th, 2026",
    lead: "First release.",
    sections: [
      {
        title: "In game",
        notes: [
          "Your Mu Rating on the badge. The helper reads your ratings from Wavu Wank and keeps the badge's save slot current, so the number beside your rank is the one Wavu has.",
          "Your MR change after a ranked match, on screen when the match ends rather than whenever Wavu is next read.",
          "Both sides' ratings in the replay menu. A replay is matched by name and Tekken Power, so the rating each player carried into that battle is the one shown.",
        ],
      },
      {
        title: "Accounts",
        notes: [
          "Up to five accounts. Alts, a second Tekken ID, or a rival you want to watch; the HUD follows the one with the check mark and the rest stay a click away.",
          "Auto-connect follows the Steam account signed in to Tekken, so switching accounts needs nothing from you.",
          "Tekken starting opens the helper and it connects on its own. In Headless Mode it stays out of sight and connects anyway.",
          "Fighter Select, another way to pick an account: a card each with the character's portrait, instead of a list.",
        ],
      },
      {
        title: "Pages",
        notes: [
          "Four pages under the logo, turning by themselves every five seconds, or by the arrows either side.",
          "The trend page graphs the last twenty battles on the character you are playing, and follows you when you switch character.",
          "The milestone page counts the MR left to your next hundred and estimates the wins it takes, from what your own recent wins have actually been worth.",
          "The session page keeps wins, losses, the net change, the last eight results and the current streak, with the last opponent and what they cost you.",
        ],
      },
      {
        title: "Ratings",
        notes: [
          "The ratings list is every character Wavu has a rating for, in Wavu's own groups: Leaderboard, Unqualified and Provisional. A character's row flashes with its change as it lands.",
          "Drag the window wider and σ², games and the last played date appear, in Wavu's column order. The size is remembered.",
        ],
      },
      {
        title: "Feed and achievements",
        notes: [
          "A feed of everything that happened: every battle with the opponent, the character, the change and the MR it left you on, and every award as it is earned. Five hundred rows an account.",
          "Seventy-five achievements and milestones across six sections, each one Common, Uncommon, Rare, Epic or Legendary.",
          "An achievement card over the game as you earn one, in the rarity's colour. Notifications Over the Game keeps it above Tekken in Borderless or Windowed; nothing draws over exclusive Fullscreen.",
          "A dot on the Achievements button while something earned has not been looked at.",
          "The catalog searches, sorts and filters to what you have earned, as a grid to glance at or a list to read.",
        ],
      },
      {
        title: "The app",
        notes: [
          "Auto-hide puts the window away ten seconds after you stop using it. Moving the pointer back cancels it.",
          "Headless Mode: no window, no notifications, a greyed tray icon, and it starts with Windows and connects. Opening the helper again turns it off.",
        ],
      },
      {
        title: "Troubleshooting",
        notes: [
          "Diagnostics copies a full report of the app, game, mods, connection, MR and recent log, to paste into a Discord bug report.",
          "Test Notification fires an achievement card after a short wait, so you can switch to the game first. Five presses show every rarity.",
        ],
      },
      {
        title: "Where the ratings come from",
        notes: [
          "Ratings come from a feed published once a minute rather than from Wavu directly, so one request serves every player. Your own page is read when you connect, and at most once every ten minutes after your own matches.",
        ],
      },
    ],
  },
];

async function copy(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.style.position = "fixed";
    ta.style.opacity = "0";
    document.body.appendChild(ta);
    ta.select();
    const ok = document.execCommand("copy");
    ta.remove();
    return ok;
  }
}

export function SettingsPane({
  settings,
  flip,
}: {
  settings: Settings | null;
  flip: (key: SettingKey, on: boolean) => void;
}) {
  return (
    <ScrollBox className="ratings settings-list">
      {SECTIONS.map(([title, items]) => (
        <div key={title}>
          <div className="group-head">
            <span className="group-word">{title}</span>
            <span className="faint">&middot; {items.length}</span>
          </div>
          {items.map(({ key, label, field, sub }) => {
            const on = Boolean(settings?.[field]);
            return (
              <div className="row setting-row" key={key}>
                <button
                  className={`switch ${on ? "on" : ""}`}
                  title={on ? "Disable" : "Enable"}
                  aria-label={label}
                  aria-pressed={on}
                  disabled={settings === null}
                  onClick={(e) => {
                    e.stopPropagation();
                    flip(key, !on);
                  }}
                />
                <span className={`dot ${on ? "on" : "off"}`} />
                <span className="name">
                  {label}
                  {sub && <span className="setting-sub">{sub}</span>}
                </span>
              </div>
            );
          })}
        </div>
      ))}
      <Troubleshooting />
      <Changelog />
    </ScrollBox>
  );
}

function Changelog() {
  const [open, setOpen] = useState<string | null>(null);
  const top = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (open === null) return;
    const id = requestAnimationFrame(() => top.current?.scrollIntoView({ block: "start", behavior: "smooth" }));
    return () => cancelAnimationFrame(id);
  }, [open]);

  return (
    <div ref={top}>
      <div className="group-head">
        <span className="group-word">Changelog</span>
        <span className="faint">&middot; {CHANGELOG.length}</span>
      </div>
      <div className="changelog">
        {CHANGELOG.map((rel) => {
          const shown = open === rel.version;
          return (
            <div className="changelog-entry" key={rel.version}>
              <button
                className="changelog-head"
                aria-expanded={shown}
                onClick={() => setOpen(shown ? null : rel.version)}
              >
                <span>
                  Version {rel.version} - {rel.date}
                </span>
                <span className={`changelog-chev ${shown ? "open" : ""}`}>&rsaquo;</span>
              </button>
              {shown && (
                <div className="changelog-body">
                  <p className="changelog-lead">{rel.lead}</p>
                  {rel.sections.map((s) => (
                    <div className="changelog-part" key={s.title}>
                      <span className="changelog-part-head">{s.title}</span>
                      <ul className="changelog-notes">
                        {s.notes.map((n, i) => (
                          <li key={i}>{n}</li>
                        ))}
                      </ul>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function Troubleshooting() {
  const [status, setStatus] = useState<TroubleshootStatus | null>(null);
  const [copied, setCopied] = useState(false);
  const [testIn, setTestIn] = useState(0);

  const refresh = () =>
    invoke<TroubleshootStatus>("troubleshoot_status")
      .then(setStatus)
      .catch(() => {});
  useEffect(() => {
    refresh();
  }, []);
  useEffect(() => {
    if (testIn <= 0) return;
    const t = setTimeout(() => setTestIn((n) => Math.max(0, n - 1)), 1000);
    return () => clearTimeout(t);
  }, [testIn]);

  const fireTest = () =>
    invoke<number>("test_notification")
      .then((secs) => setTestIn(secs))
      .catch(() => {});

  const modFound = (status?.mod_files.length ?? 0) > 0;

  const copyDiagnostics = async () => {
    const text = await invoke<string>("diagnostics").catch(() => "");
    if (text && (await copy(text))) {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div>
      <div className="group-head">
        <span className="group-word">Troubleshooting</span>
        <span className="faint">&middot; 4</span>
      </div>

      <div className="row setting-row tool-row">
        <span className="name">
          Open Mod Folder
          <span className="setting-sub">Check where you've placed the mod at</span>
          {status !== null && !modFound && (
            <span className="setting-sub report-result bad">
              Not found in the game's Paks folder. The HUD can't show anything until the mod is installed.
            </span>
          )}
        </span>
        <Button
          variant="secondary"
          size="compact"
          onClick={() => {
            invoke("open_mod_folder").catch(() => {});
            refresh();
          }}
        >
          Open Folder
        </Button>
      </div>

      <div className="row setting-row tool-row">
        <span className="name">
          Wavu Wank Table
          <span className="setting-sub">
            Opens the save folder holding MuRating.sav. This disconnects from Wavu Wank until you reconnect.
          </span>
        </span>
        <Button variant="secondary" size="compact" onClick={() => invoke("open_table_folder").catch(() => {})}>
          Open Folder
        </Button>
      </div>

      <div className="row setting-row tool-row">
        <span className="name">
          Test Notification
          <span className="setting-sub">
            {testIn > 0
              ? `Firing in ${testIn}… switch to Tekken now.`
              : "Fires an achievement card after a short wait, so you can switch to the game first. Five presses show every rarity."}
          </span>
        </span>
        <Button variant="secondary" size="compact" disabled={testIn > 0} onClick={fireTest}>
          {testIn > 0 ? String(testIn) : "Test"}
        </Button>
      </div>

      <div className="row setting-row tool-row">
        <span className="name">
          Diagnostics
          <span className="setting-sub">
            Copies a full report of the app, game, mods, connection, MR and recent log to paste in a Discord bug report.
          </span>
        </span>
        <Button variant="secondary" size="compact" onClick={copyDiagnostics}>
          {copied ? "Copied" : "Copy"}
        </Button>
      </div>

    </div>
  );
}
