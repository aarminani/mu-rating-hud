export type Probe = {
  autostart: boolean;
  saves_root_exists: boolean;
  account: string | null;
  slot_exists: boolean;
  slot_is_ours: boolean;
  build_id: number | null;
};

export type CharRating = {
  character: string;
  mu: number;
  games: number | null;
  last_seen: number | null;
  group: string;
  change: number | null;
  sigma: number | null;
};

export type Settings = {
  mr_enabled: boolean;
  auto_hide: boolean;
  start_with_windows: boolean;
  hide_notifications: boolean;
  auto_connect: boolean;
  fighter_select: boolean;
  pager_auto_scroll: boolean;
  over_game: boolean;
  headless: boolean;
};

export type SettingKey =
  | "mr"
  | "autohide"
  | "autostart"
  | "notifications"
  | "autoconnect"
  | "fighter"
  | "pagerscroll"
  | "overgame"
  | "headless";

export type RatingsUpdate = {
  character: string;
  mu: number;
  ratings: CharRating[];
};

export type Connected = {
  tekken_id: string;
  name: string;
  character: string;
  mu: number;
  recent_character: string;
  recent_mu: number;
  ratings: CharRating[];
  wrote: string;
};

export type AccountRow = {
  tekken_id: string;
  name: string;
  paired: boolean;
  signed_in: boolean;
  active: boolean;
  character: string | null;
  mu: number | null;
};

export type FollowState =
  | { at: "setup" }
  | { at: "manual" }
  | { at: "paused" }
  | { at: "waiting"; expected: string | null }
  | { at: "unpaired" }
  | { at: "connecting"; tekken_id: string; name: string }
  | { at: "following"; me: Connected; manual: boolean }
  | { at: "failed"; tekken_id: string; name: string; error: string; retry_at: number | null };

export type TroubleshootStatus = {
  mod_files: { path: string; bytes: number }[];
  mod_folder: string | null;
  report_configured: boolean;
  report_cooldown_secs: number;
};

export type ReportSent = {
  sent: boolean;
  saved: string | null;
  cooldown_secs: number;
};

export type ConnectProgress = {
  tekken_id: string;
  state: "reading" | "done" | "failed";
  name?: string;
  error?: string;
};

export type FeedItem =
  | {
      kind: "battle";
      at: number;
      won: boolean;
      change: number;
      mu_after: number;
      character: string | null;
      opponent_name: string;
      opponent_character: string | null;
      opponent_mu_before: number | null;
      opponent_change: number | null;
      rounds_own: number | null;
      rounds_opp: number | null;
    }
  | {
      kind: "award";
      at: number;
      id: string;
      entry_kind: "milestone" | "achievement";
      rarity: number;
      title: string;
      text: string;
      emphasis: string | null;
    };

export type AchievementRow = {
  id: string;
  kind: "milestone" | "achievement";
  section: string;
  title: string;
  condition: string;
  rarity: number;
  earned: boolean;
  first_at: number | null;
  last_at: number | null;
  times: number;
  tier: number;
  tier_max: number;
  progress: string | null;
  current: number | null;
  target: number | null;
};

export type Achievements = {
  tekken_id: string;
  feed: FeedItem[];
  rows: AchievementRow[];
  sections: { name: string; earned: number; total: number }[];
  earned: number;
  total: number;
  unread: number;
  unread_rarity: number;
};

export type SessionSummary = {
  since: number;
  net: number;
  wins: number;
  losses: number;
  last8: boolean[];
  streak: number;
  last: {
    at: number;
    opponent_name: string;
    opponent_character: string | null;
    change: number;
    won: boolean;
  } | null;
  trend: {
    character: string;
    short: string;
    points: number[];
    change_over: number;
    mu: number;
  } | null;
  goal: {
    character: string;
    mu: number;
    target: number;
    to_go: number;
    progress: number;
    wins_est: number | null;
  } | null;
};
