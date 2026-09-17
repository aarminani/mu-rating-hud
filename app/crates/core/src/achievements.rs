

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::feed::{LiveRating, OwnMatch};
use crate::table::char_key;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Milestone,
    Achievement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Milestones,
    Match,
    Streaks,
    DailyStreaks,
    Session,
    Roster,
}

impl Section {
    pub fn label(self) -> &'static str {
        match self {
            Section::Milestones => "Milestones",
            Section::Match => "Match",
            Section::Streaks => "Streaks",
            Section::DailyStreaks => "Daily streaks",
            Section::Session => "Session",
            Section::Roster => "Roster",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Item {
    pub id: &'static str,
    pub kind: Kind,
    pub section: Section,
    pub title: &'static str,
    pub condition: &'static str,
    pub rarity: u8,
    pub tiers: &'static [i64],
    pub tier_rarities: &'static [u8],
}

const fn ms(id: &'static str, title: &'static str, condition: &'static str, rarity: u8) -> Item {
    Item { id, kind: Kind::Milestone, section: Section::Milestones, title, condition, rarity, tiers: &[], tier_rarities: &[] }
}

const fn tiered(id: &'static str, title: &'static str, condition: &'static str, tiers: &'static [i64]) -> Item {
    Item {
        id,
        kind: Kind::Milestone,
        section: Section::Milestones,
        title,
        condition,
        rarity: 5,
        tiers,
        tier_rarities: &[1, 2, 3, 4, 5],
    }
}

const fn ach(id: &'static str, section: Section, title: &'static str, condition: &'static str, rarity: u8) -> Item {
    Item { id, kind: Kind::Achievement, section, title, condition, rarity, tiers: &[], tier_rarities: &[] }
}

pub const GAMES_TIERS: &[i64] = &[500, 2_500, 10_000, 25_000, 75_000];
pub const TOTAL_GAMES_TIERS: &[i64] = &[2_500, 12_500, 50_000, 125_000, 375_000];
pub const POWER_TIERS: &[i64] = &[250_000, 375_000, 500_000, 625_000, 750_000];
pub const CHARS_TIERS: &[i64] = &[5, 10, 20, 30, 0];
pub const WINS_TIERS: &[i64] = &[250, 1_000, 2_500, 7_500, 20_000];

pub mod thresholds {
    pub const BIG_SWING: i32 = 25;
    pub const UPSETS: [(i32, &str); 4] = [(150, "A04"), (300, "A05"), (500, "A06"), (750, "A56")];
    pub const WIN_STREAKS: [(u32, &str); 6] =
        [(3, "A42"), (5, "A11"), (10, "A12"), (15, "A43"), (20, "A13"), (30, "A44")];
    pub const LOSS_RECOVERY: [(u32, &str); 3] = [(3, "A14"), (5, "A15"), (10, "A48")];
    pub const SWEEP_RUNS: [(u32, &str); 2] = [(3, "A16"), (5, "A45")];
    pub const SESSION_LENGTHS: [(u32, &str); 4] = [(10, "A17"), (30, "A18"), (75, "A19"), (150, "A40")];
    pub const SESSION_NETS: [(i32, &str); 4] = [(50, "A20"), (100, "A21"), (200, "A22"), (300, "A41")];
    pub const COMEBACKS: [(i32, &str); 2] = [(-30, "A23"), (-75, "A58")];
    pub const RIVAL_WINS: [(u32, &str); 2] = [(3, "A25"), (5, "A57")];
    pub const DAILY_STREAKS: [(u32, &str); 5] = [(3, "A49"), (7, "A50"), (14, "A51"), (30, "A52"), (100, "A53")];
    pub const WINNING_DAYS: [(u32, &str); 2] = [(3, "A54"), (7, "A55")];
    pub const RIVAL_PLAYS: u32 = 5;
    pub const SESSION_UPSETS: u32 = 3;
    pub const DOMINANT_MATCHES: u32 = 20;
    pub const DOMINANT_RATE: f64 = 0.75;
    pub const STAYED_POSITIVE: u32 = 50;
    pub const ROSTER_SHUFFLE: usize = 5;
    pub const SESSION_SWEEPS: u32 = 5;
    pub const SESSION_FINAL_ROUNDS: u32 = 5;
    pub const SESSION_OPPONENTS: usize = 20;
    pub const SESSION_PEAKS: usize = 2;
    pub const HIGHER_RANKED: usize = 5;
    pub const SESSION_REGIONS: usize = 3;
    pub const MAIN_WINS: u32 = 10;
    pub const SESSION_REVENGES: u32 = 3;
    pub const POCKET_GAMES: i32 = 100;
    pub const WELL_ROUNDED: usize = 20;
    pub const GLOBETROTTER_REGIONS: usize = 5;
    pub const PERFECT_START: usize = 5;
    pub const TILT_LOSSES: u32 = 3;
    pub const TILT_WINS: u32 = 3;
    pub const POCKET_PICKS: usize = 3;
    pub const GOD_OF_DESTRUCTION: i32 = 29;
}

use thresholds as T;

pub const CATALOG: &[Item] = &[
    ms("M01", "New Peak", "A character's MR goes above its saved peak", 1),
    Item { id: "M02", kind: Kind::Milestone, section: Section::Milestones, title: "Hundred",
        condition: "A character's peak crosses a hundred, from 1500 to 3000 MR", rarity: 5,
        tiers: &[1500, 1600, 1700, 1800, 1900, 2000, 2100, 2200, 2300, 2400, 2500, 2600, 2700, 2800, 2900, 3000],
        tier_rarities: &[1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 5] },
    ms("M03", "Qualified", "A character moves into the Leaderboard group", 3),
    tiered("M04", "Games Played", "A character passes 500 / 2,500 / 10,000 / 25,000 / 75,000 games", GAMES_TIERS),
    ms("M05", "New Main", "Your highest-rated character changes", 2),
    ms("M06", "God of Destruction", "A character's in-game rank reaches God of Destruction", 3),
    tiered("M07", "Tekken Prowess", "Tekken Prowess passes 250,000 / 375,000 / 500,000 / 625,000 / 750,000", POWER_TIERS),
    tiered("M08", "Total Games", "All characters' games together pass 2,500 / 12,500 / 50,000 / 125,000 / 375,000", TOTAL_GAMES_TIERS),
    tiered("M09", "Characters Played", "Rated characters reach 5 / 10 / 20 / 30 / the whole roster", CHARS_TIERS),
    tiered("M10", "Career Wins", "Wins tracked by the HUD pass 250 / 1,000 / 2,500 / 7,500 / 20,000", WINS_TIERS),
    ach("A01", Section::Match, "Clean Sweep", "Win, and the opponent took no rounds", 1),
    ach("A02", Section::Match, "Down to the Wire", "Win in the final round", 1),
    ach("A03", Section::Match, "Mirror Match", "Win with the same character as your opponent", 1),
    ach("A04", Section::Match, "Upset", "Beat someone 150+ MR above you", 2),
    ach("A05", Section::Match, "Giant Slayer", "Beat someone 300+ MR above you", 3),
    ach("A06", Section::Match, "David vs Goliath", "Beat someone 500+ MR above you", 4),
    ach("A56", Section::Match, "Dragon Slayer", "Beat someone 750+ MR above you", 5),
    ach("A07", Section::Match, "Big Swing", "Gain 25+ MR from one win", 2),
    ach("A08", Section::Match, "New Matchup", "First win ever against a character, with this character", 1),
    ach("A09", Section::Match, "Destroyer", "Beat a God of Destruction while your character is below it", 3),
    ach("A10", Section::Match, "Revenge", "Beat someone who beat you earlier this session", 2),
    ach("A61", Section::Match, "Night Owl", "Win a match between 00:00 and 05:00 local time", 1),
    ach("A62", Section::Match, "Early Bird", "Win a match between 05:00 and 08:00 local time", 1),
    ach("A64", Section::Match, "Pocket Monster", "Beat someone above your MR with a character under 100 games", 3),
    ach("A42", Section::Streaks, "Warm Streak", "3 wins in a row", 1),
    ach("A11", Section::Streaks, "Hot Streak", "5 wins in a row", 2),
    ach("A12", Section::Streaks, "On Fire", "10 wins in a row", 3),
    ach("A43", Section::Streaks, "Blazing", "15 wins in a row", 4),
    ach("A13", Section::Streaks, "Unstoppable", "20 wins in a row", 5),
    ach("A44", Section::Streaks, "Godlike", "30 wins in a row", 5),
    ach("A14", Section::Streaks, "Bounce Back", "Win after losing 3+ in a row", 1),
    ach("A15", Section::Streaks, "Iron Will", "Win after losing 5+ in a row", 2),
    ach("A48", Section::Streaks, "Unbreakable", "Win after losing 10+ in a row", 3),
    ach("A16", Section::Streaks, "Flawless Run", "3 clean sweeps in a row", 4),
    ach("A45", Section::Streaks, "Flawless Five", "5 clean sweeps in a row", 5),
    ach("A46", Section::Streaks, "Double Upset", "2 upsets in a row", 3),
    ach("A47", Section::Streaks, "Cliffhanger Run", "3 final-round wins in a row", 3),
    ach("A49", Section::DailyStreaks, "Regular", "Ranked matches on 3 days in a row", 1),
    ach("A50", Section::DailyStreaks, "Dedicated", "7 days in a row", 2),
    ach("A51", Section::DailyStreaks, "Devoted", "14 days in a row", 3),
    ach("A52", Section::DailyStreaks, "Relentless", "30 days in a row", 4),
    ach("A53", Section::DailyStreaks, "Ever-Present", "100 days in a row", 5),
    ach("A54", Section::DailyStreaks, "Consistent", "3 days in a row finishing up on MR", 3),
    ach("A55", Section::DailyStreaks, "Rock Solid", "7 days in a row finishing up on MR", 4),
    ach("A17", Section::Session, "Warmed Up", "10 matches this session", 1),
    ach("A18", Section::Session, "Grinder", "30 matches this session", 2),
    ach("A19", Section::Session, "Marathon", "75 matches this session", 3),
    ach("A40", Section::Session, "Iron Marathon", "150 matches this session", 4),
    ach("A20", Section::Session, "Good Session", "Session net reaches +50 MR", 2),
    ach("A21", Section::Session, "Big Session", "Session net reaches +100 MR", 3),
    ach("A22", Section::Session, "Career Session", "Session net reaches +200 MR", 4),
    ach("A41", Section::Session, "Legendary Session", "Session net reaches +300 MR", 5),
    ach("A23", Section::Session, "Comeback Kid", "Session net fell to \u{2212}30 or worse, then climbs above 0", 3),
    ach("A58", Section::Session, "Comeback Royale", "Session net fell to \u{2212}75 or worse, then climbs above 0", 4),
    ach("A24", Section::Session, "Rivalry", "Play the same opponent 5 times in a session", 1),
    ach("A25", Section::Session, "Rival Tamed", "Beat the same opponent 3 times in a session", 2),
    ach("A57", Section::Session, "Rival Swept", "Beat the same opponent 5 times in a session", 3),
    ach("A26", Section::Session, "Giant Hunter", "3 upsets in one session", 4),
    ach("A31", Section::Session, "Perfect Start", "Win the first 5 matches of a session", 2),
    ach("A32", Section::Session, "Dominant", "At 20+ matches, a session win rate of 75% or higher", 3),
    ach("A33", Section::Session, "Stayed Positive", "At 50+ matches, session net still above 0", 3),
    ach("A34", Section::Session, "Roster Shuffle", "Play 5 different characters in one session", 2),
    ach("A35", Section::Session, "Sweep Collector", "5 clean sweeps in one session", 3),
    ach("A36", Section::Session, "Nail-Biter Night", "5 final-round wins in one session", 2),
    ach("A37", Section::Session, "Tilt-Proof", "Lose 3 in a row, then win the next 3, in one session", 2),
    ach("A38", Section::Session, "Circuit Breaker", "Play 20 different opponents in one session", 2),
    ach("A39", Section::Session, "Double Peak", "New peaks on 2 different characters in one session", 3),
    ach("A59", Section::Session, "Ladder Climber", "Beat 5 different higher-ranked opponents in one session", 3),
    ach("A60", Section::Session, "Globetrotter Night", "Beat players from 3 different regions in one session", 2),
    ach("A63", Section::Session, "Main Event", "10 wins with your highest-rated character in one session", 2),
    ach("A65", Section::Session, "Revenge Tour", "3 revenges in one session", 3),
    ach("A27", Section::Roster, "Well Rounded", "Beat 20 different characters with one character", 3),
    ach("A28", Section::Roster, "Full Roster", "Beat every character with one character", 5),
    ach("A29", Section::Roster, "Pocket Picks", "Win with 3 different characters in one session", 2),
    ach("A30", Section::Roster, "Globetrotter", "Beat players from 5 different regions", 3),
];

pub fn item(id: &str) -> Option<&'static Item> {
    CATALOG.iter().find(|i| i.id == id)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Earned {
    pub first: i64,
    pub last: i64,
    pub count: u32,
    #[serde(default)]
    pub tier: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FeedItem {
    Battle {
        at: i64,
        won: bool,
        change: i32,
        mu_after: i32,
        character: Option<String>,
        opponent_name: String,
        opponent_character: Option<String>,
        opponent_mu_before: Option<i32>,
        opponent_change: Option<i32>,
        rounds_own: Option<i32>,
        rounds_opp: Option<i32>,
    },
    Award {
        at: i64,
        id: String,
        entry_kind: Kind,
        rarity: u8,
        title: String,
        text: String,
        emphasis: Option<String>,
    },
}

impl FeedItem {
    pub fn at(&self) -> i64 {
        match self {
            FeedItem::Battle { at, .. } | FeedItem::Award { at, .. } => *at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    pub id: String,
    pub kind: Kind,
    pub rarity: u8,
    pub title: String,
    pub text: String,
    pub emphasis: Option<String>,
    pub at: i64,
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub entries: Vec<Entry>,
    pub toast: Option<Entry>,
    pub changed: bool,
}

pub const FEED_CAP: usize = 500;
pub const SEEN_CAP: usize = 2000;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Progress {
    #[serde(default)]
    pub initialized: bool,
    #[serde(default)]
    pub seen: Vec<String>,
    #[serde(default)]
    pub seen_times: Vec<i64>,
    #[serde(default)]
    pub peaks: BTreeMap<String, i32>,
    #[serde(default)]
    pub hundreds: BTreeMap<String, i32>,
    #[serde(default)]
    pub games_tier: BTreeMap<String, u8>,
    #[serde(default)]
    pub qualified: BTreeSet<String>,
    #[serde(default)]
    pub god: BTreeSet<String>,
    #[serde(default)]
    pub power_tier: u8,
    #[serde(default)]
    pub total_games_tier: u8,
    #[serde(default)]
    pub chars_tier: u8,
    #[serde(default)]
    pub wins_tier: u8,
    #[serde(default)]
    pub career_wins: u32,
    #[serde(default)]
    pub career_battles: u32,
    #[serde(default)]
    pub main: Option<String>,
    #[serde(default)]
    pub main_name: Option<String>,
    #[serde(default)]
    pub beaten: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    pub regions: BTreeSet<i32>,
    #[serde(default)]
    pub win_streak: u32,
    #[serde(default)]
    pub loss_streak: u32,
    #[serde(default)]
    pub sweep_run: u32,
    #[serde(default)]
    pub upset_run: u32,
    #[serde(default)]
    pub final_round_run: u32,
    #[serde(default)]
    pub last_day: Option<i32>,
    #[serde(default)]
    pub day_streak: u32,
    #[serde(default)]
    pub day_net: i32,
    #[serde(default)]
    pub winning_day_streak: u32,
    #[serde(default)]
    pub scoped: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    pub earned: BTreeMap<String, Earned>,
    #[serde(default)]
    pub feed: Vec<FeedItem>,
    #[serde(default)]
    pub seen_at: i64,
}

#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub since: i64,
    pub matches: u32,
    pub wins: u32,
    pub losses: u32,
    pub net: i32,
    pub low: i32,
    pub first5: Vec<bool>,
    pub plays: HashMap<String, u32>,
    pub wins_by_opponent: HashMap<String, u32>,
    pub lost_to: HashSet<String>,
    pub opponents: HashSet<String>,
    pub chars_played: HashSet<i32>,
    pub chars_won: HashSet<i32>,
    pub upsets: u32,
    pub sweeps: u32,
    pub final_round_wins: u32,
    pub tilt_losses: u32,
    pub tilt_wins: u32,
    pub peak_chars: HashSet<String>,
    pub higher_ranked: HashSet<String>,
    pub regions_beaten: HashSet<i32>,
    pub main_wins: u32,
    pub revenges: u32,
    pub fired: HashSet<String>,
}

impl SessionState {
    pub fn new(since: i64) -> Self {
        Self { since, ..Default::default() }
    }
}

pub struct Inputs<'a> {
    pub matches: &'a [OwnMatch],
    pub ratings: &'a [LiveRating],
    pub roster_total: usize,
    pub session_since: i64,
    pub now: i64,
    pub utc_offset_min: i32,
}

struct Ctx {
    entries: Vec<Entry>,
    silent: bool,
    roster_total: usize,
}

fn local_day(at: i64, offset_min: i32) -> i32 {
    (at + i64::from(offset_min) * 60).div_euclid(86_400) as i32
}

fn local_hour(at: i64, offset_min: i32) -> i32 {
    ((at + i64::from(offset_min) * 60).rem_euclid(86_400) / 3_600) as i32
}

fn commas(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("\u{2212}{out}")
    } else {
        out
    }
}

fn signed(n: i32) -> String {
    if n < 0 {
        format!("\u{2212}{}", n.abs())
    } else {
        format!("+{n}")
    }
}

impl Ctx {
    fn fire(&mut self, p: &mut Progress, id: &str, at: i64, text: String, emphasis: Option<String>, tier: u8) {
        let Some(it) = item(id) else { return };
        let rarity = if tier > 0 && !it.tier_rarities.is_empty() {
            it.tier_rarities[(tier as usize - 1).min(it.tier_rarities.len() - 1)]
        } else {
            it.rarity
        };
        let e = p.earned.entry(id.to_string()).or_default();
        if e.count == 0 {
            e.first = at;
        }
        e.last = at;
        e.count += 1;
        e.tier = e.tier.max(tier);
        if self.silent {
            return;
        }
        self.entries.push(Entry {
            id: id.to_string(),
            kind: it.kind,
            rarity,
            title: it.title.to_string(),
            text,
            emphasis,
            at,
        });
    }
}

fn scoped_first(p: &mut Progress, id: &str, scope: &str) -> bool {
    p.scoped.entry(id.to_string()).or_default().insert(scope.to_string())
}

fn top_step<'a, N: PartialOrd + Copy>(steps: &'a [(N, &'a str)], value: N) -> Option<&'a str> {
    steps.iter().rev().find(|(need, _)| value >= *need).map(|(_, id)| *id)
}

pub fn update(p: &mut Progress, s: &mut SessionState, i: &Inputs) -> Outcome {
    if s.since != i.session_since {
        *s = SessionState::new(i.session_since);
    }
    let mut ctx = Ctx { entries: Vec::new(), silent: !p.initialized, roster_total: i.roster_total };
    let before = p.clone();

    if p.seen_times.is_empty() && !p.feed.is_empty() {
        p.seen_times = p
            .feed
            .iter()
            .filter(|f| matches!(f, FeedItem::Battle { .. }))
            .map(FeedItem::at)
            .collect();
        p.seen_times.sort_unstable();
    }

    let mut seen: HashSet<&str> = p.seen.iter().map(String::as_str).collect();
    let mut seen_times: HashSet<i64> = p.seen_times.iter().copied().collect();
    let mut fresh: Vec<&OwnMatch> = Vec::new();
    let mut fuller: Vec<&OwnMatch> = Vec::new();
    for m in i.matches {
        if m.change.is_none() {
            continue;
        }
        let key = battle_key(m);
        if seen.contains(key.as_str()) || seen_times.contains(&m.battle_at) {
            fuller.push(m);
            continue;
        }
        fresh.push(m);
        seen.insert(Box::leak(key.into_boxed_str()));
        seen_times.insert(m.battle_at);
    }
    fresh.sort_by_key(|m| m.battle_at);

    for m in fresh {
        p.seen.push(battle_key(m));
        p.seen_times.push(m.battle_at);
        battle(&mut ctx, p, s, m, i);
    }
    if p.seen.len() > SEEN_CAP {
        let drop = p.seen.len() - SEEN_CAP;
        p.seen.drain(0..drop);
    }
    if p.seen_times.len() > SEEN_CAP {
        let drop = p.seen_times.len() - SEEN_CAP;
        p.seen_times.drain(0..drop);
    }
    for m in fuller {
        enrich(p, m);
    }

    ratings_pass(&mut ctx, p, s, i);

    let entries = std::mem::take(&mut ctx.entries);
    for e in &entries {
        push_feed(p, FeedItem::Award {
            at: e.at,
            id: e.id.clone(),
            entry_kind: e.kind,
            rarity: e.rarity,
            title: e.title.clone(),
            text: e.text.clone(),
            emphasis: e.emphasis.clone(),
        });
    }
    p.initialized = true;
    let toast = pick_toast(&entries);
    let changed = *p != before;
    Outcome { entries, toast, changed }
}

fn battle_key(m: &OwnMatch) -> String {
    m.battle_id.clone().unwrap_or_else(|| format!("t{}", m.battle_at))
}

fn enrich(p: &mut Progress, m: &OwnMatch) -> bool {
    for f in p.feed.iter_mut() {
        let FeedItem::Battle {
            at, character, opponent_character, opponent_mu_before, opponent_change, rounds_own, rounds_opp, ..
        } = f
        else {
            continue;
        };
        if *at != m.battle_at {
            continue;
        }
        let mut moved = false;
        if rounds_own.is_none() && m.rounds_own.is_some() {
            *rounds_own = m.rounds_own;
            *rounds_opp = m.rounds_opp;
            moved = true;
        }
        if opponent_mu_before.is_none() && m.opponent_mu_before.is_some() {
            *opponent_mu_before = m.opponent_mu_before;
            *opponent_change = m.opponent_mu_before.map(|_| -m.change.unwrap_or(0));
            moved = true;
        }
        if opponent_character.is_none() && m.opponent_character.is_some() {
            opponent_character.clone_from(&m.opponent_character);
            moved = true;
        }
        if character.is_none() && m.character.is_some() {
            character.clone_from(&m.character);
            moved = true;
        }
        return moved;
    }
    false
}

fn push_feed(p: &mut Progress, item: FeedItem) {
    let at = item.at();
    let pos = p.feed.iter().position(|f| f.at() < at).unwrap_or(p.feed.len());
    p.feed.insert(pos, item);
    p.feed.truncate(FEED_CAP);
}

fn pick_toast(entries: &[Entry]) -> Option<Entry> {
    entries
        .iter()
        .max_by(|a, b| {
            a.rarity
                .cmp(&b.rarity)
                .then_with(|| matches!(a.kind, Kind::Milestone).cmp(&matches!(b.kind, Kind::Milestone)))
                .then_with(|| b.id.cmp(&a.id))
        })
        .cloned()
}

fn battle(ctx: &mut Ctx, p: &mut Progress, s: &mut SessionState, m: &OwnMatch, i: &Inputs) {
    let change = m.change.unwrap_or(0);
    let won = m.won;
    let at = m.battle_at;
    let in_session = at >= i.session_since;
    let sweep = won && m.rounds_opp == Some(0);
    let final_round = won && matches!((m.rounds_own, m.rounds_opp), (Some(a), Some(b)) if a == b + 1 && b > 0);
    let gap = m.opponent_mu_before.map(|o| o - m.mu_before).unwrap_or(0);
    let upset = won && gap >= T::UPSETS[0].0;

    p.career_battles += 1;
    if won {
        p.career_wins += 1;
    }

    let loss_run_before = p.loss_streak;
    if won {
        p.win_streak += 1;
        p.loss_streak = 0;
    } else {
        p.loss_streak += 1;
        p.win_streak = 0;
    }
    p.sweep_run = if sweep { p.sweep_run + 1 } else { 0 };
    p.upset_run = if upset { p.upset_run + 1 } else { 0 };
    p.final_round_run = if final_round { p.final_round_run + 1 } else { 0 };

    let day = local_day(at, i.utc_offset_min);
    if p.last_day != Some(day) {
        if let Some(prev) = p.last_day {
            if p.day_net > 0 {
                p.winning_day_streak += 1;
            } else {
                p.winning_day_streak = 0;
            }
            p.day_streak = if day == prev + 1 { p.day_streak + 1 } else { 1 };
        } else {
            p.day_streak = 1;
        }
        p.last_day = Some(day);
        p.day_net = 0;
    }
    p.day_net += change;

    let mut new_matchup = false;
    if won {
        if let (Some(mine), Some(theirs)) = (&m.character, &m.opponent_character) {
            let set = p.beaten.entry(char_key(mine)).or_default();
            new_matchup = set.insert(char_key(theirs));
        }
        if let Some(region) = m.opponent_region {
            p.regions.insert(region);
        }
    }

    if in_session {
        s.matches += 1;
        if won {
            s.wins += 1;
        } else {
            s.losses += 1;
        }
        s.net += change;
        s.low = s.low.min(s.net);
        if s.first5.len() < T::PERFECT_START {
            s.first5.push(won);
        }
        if !m.opponent_id.is_empty() {
            *s.plays.entry(m.opponent_id.clone()).or_default() += 1;
            s.opponents.insert(m.opponent_id.clone());
        }
        if let Some(c) = m.chara_id {
            s.chars_played.insert(c);
        }
        if won {
            if !m.opponent_id.is_empty() {
                *s.wins_by_opponent.entry(m.opponent_id.clone()).or_default() += 1;
            }
            if let Some(c) = m.chara_id {
                s.chars_won.insert(c);
            }
            if upset {
                s.upsets += 1;
            }
            if sweep {
                s.sweeps += 1;
            }
            if final_round {
                s.final_round_wins += 1;
            }
            if let (Some(mine), Some(theirs)) = (m.rank, m.opponent_rank) {
                if theirs > mine && !m.opponent_id.is_empty() {
                    s.higher_ranked.insert(m.opponent_id.clone());
                }
            }
            if let Some(region) = m.opponent_region {
                s.regions_beaten.insert(region);
            }
            if p.main.as_deref() == m.character.as_deref().map(char_key).as_deref() {
                s.main_wins += 1;
            }
            if s.lost_to.contains(&m.opponent_id) {
                s.revenges += 1;
                s.lost_to.remove(&m.opponent_id);
                fire_session(ctx, p, s, "A10", at, format!("Revenge on {}", m.opponent_name), Some(m.opponent_name.clone()), Some(m.opponent_id.clone()));
            }
            s.tilt_wins = if s.tilt_losses >= T::TILT_LOSSES { s.tilt_wins + 1 } else { 0 };
            s.tilt_losses = 0;
        } else {
            if !m.opponent_id.is_empty() {
                s.lost_to.insert(m.opponent_id.clone());
            }
            s.tilt_losses += 1;
            s.tilt_wins = 0;
        }
    }

    let who = m.opponent_name.clone();
    if won && in_session {
        if sweep {
            fire_session(ctx, p, s, "A01", at, format!("Didn't drop a round against {who}"), Some(who.clone()), None);
        }
        if final_round {
            fire_session(ctx, p, s, "A02", at, format!("Won the final round against {who}"), Some(who.clone()), None);
        }
        if m.chara_id.is_some() && m.chara_id == m.opponent_chara_id {
            let mine = m.character.clone().unwrap_or_default();
            fire_session(ctx, p, s, "A03", at, format!("Won the mirror match with {mine}"), Some(mine), None);
        }
        if let Some(id) = top_step(&T::UPSETS, gap) {
            let text = if id == "A04" {
                format!("Upset win against {who}")
            } else {
                format!("Beat a player {} MR above you, {who}", T::UPSETS.iter().find(|(_, i)| *i == id).unwrap().0)
            };
            fire_session(ctx, p, s, id, at, text, Some(who.clone()), None);
        }
        if change >= T::BIG_SWING {
            fire_session(ctx, p, s, "A07", at, format!("One win, {} MR", signed(change)), Some(format!("{} MR", signed(change))), None);
        }
        if let (Some(mine), Some(theirs)) = (m.rank, m.opponent_rank) {
            if theirs >= T::GOD_OF_DESTRUCTION && mine < T::GOD_OF_DESTRUCTION {
                fire_session(ctx, p, s, "A09", at, format!("Took down a God of Destruction, {who}"), Some(who.clone()), None);
            }
        }
        match local_hour(at, i.utc_offset_min) {
            0..=4 => fire_session(ctx, p, s, "A61", at, format!("Late-night win against {who}"), Some(who.clone()), None),
            5..=7 => fire_session(ctx, p, s, "A62", at, format!("Early-morning win against {who}"), Some(who.clone()), None),
            _ => {}
        }
        if gap > 0 {
            if let Some(name) = &m.character {
                let games = i
                    .ratings
                    .iter()
                    .find(|r| char_key(&r.rating.character) == char_key(name))
                    .and_then(|r| r.rating.games);
                if games.is_some_and(|g| g < T::POCKET_GAMES) {
                    fire_session(ctx, p, s, "A64", at, format!("Upset with a pocket pick, {name}"), Some(name.clone()), None);
                }
            }
        }
        if let Some(id) = top_step(&T::WIN_STREAKS, p.win_streak) {
            let n = T::WIN_STREAKS.iter().find(|(_, i)| *i == id).unwrap().0;
            if p.win_streak == n {
                fire_session(ctx, p, s, id, at, format!("Win streak of {n}"), Some(n.to_string()), None);
            }
        }
        if let Some(id) = top_step(&T::LOSS_RECOVERY, loss_run_before) {
            let n = T::LOSS_RECOVERY.iter().find(|(_, i)| *i == id).unwrap().0;
            let text = if n == 3 {
                format!("Snapped a losing streak against {who}")
            } else {
                format!("Broke a {n}-loss slide against {who}")
            };
            fire_session(ctx, p, s, id, at, text, Some(who.clone()), None);
        }
        if let Some(id) = top_step(&T::SWEEP_RUNS, p.sweep_run) {
            let n = T::SWEEP_RUNS.iter().find(|(_, i)| *i == id).unwrap().0;
            if p.sweep_run == n {
                let word = if n == 3 { "Three" } else { "Five" };
                fire_session(ctx, p, s, id, at, format!("{word} straight clean sweeps"), Some("clean sweeps".into()), None);
            }
        }
        if p.upset_run == 2 {
            fire_session(ctx, p, s, "A46", at, format!("Back-to-back upsets, {who}"), Some(who.clone()), None);
        }
        if p.final_round_run == 3 {
            fire_session(ctx, p, s, "A47", at, "Three final-round wins in a row".into(), Some("in a row".into()), None);
        }
        if new_matchup {
            let mine = m.character.clone().unwrap_or_default();
            let theirs = m.opponent_character.clone().unwrap_or_default();
            let scope = format!("{}|{}", char_key(&mine), char_key(&theirs));
            if scoped_first(p, "A08", &scope) {
                ctx.fire(p, "A08", at, format!("First win with {mine} against {theirs}"), Some(theirs), 0);
            }
            let key = char_key(&mine);
            let beaten = p.beaten.get(&key).map(BTreeSet::len).unwrap_or(0);
            if beaten >= T::WELL_ROUNDED && scoped_first(p, "A27", &key) {
                ctx.fire(p, "A27", at, format!("Beaten 20 characters with {mine}"), Some(mine.clone()), 0);
            }
            if ctx.roster_total > 0 && beaten >= ctx.roster_total && scoped_first(p, "A28", &key) {
                ctx.fire(p, "A28", at, format!("Beaten every character with {mine}"), Some(mine.clone()), 0);
            }
        }
        if p.regions.len() >= T::GLOBETROTTER_REGIONS && scoped_first(p, "A30", "once") {
            ctx.fire(p, "A30", at, "Beaten players from 5 regions".into(), Some("5 regions".into()), 0);
        }
    }

    if let Some(id) = top_step(&T::DAILY_STREAKS, p.day_streak) {
        let n = T::DAILY_STREAKS.iter().find(|(_, i)| *i == id).unwrap().0;
        let scope = format!("{}", day as i64 - i64::from(p.day_streak) + 1);
        if p.day_streak >= n && scoped_first(p, id, &scope) {
            ctx.fire(p, id, at, format!("{n} days in a row on ranked"), Some(format!("{n} days")), 0);
        }
    }
    if let Some(id) = top_step(&T::WINNING_DAYS, p.winning_day_streak) {
        let n = T::WINNING_DAYS.iter().find(|(_, i)| *i == id).unwrap().0;
        let scope = format!("{}", day as i64 - i64::from(p.winning_day_streak));
        if scoped_first(p, id, &scope) {
            ctx.fire(p, id, at, format!("{n} winning days in a row"), Some(format!("{n} winning days")), 0);
        }
    }

    if in_session {
        session_items(ctx, p, s, m, at);
    }

    push_feed(p, FeedItem::Battle {
        at,
        won,
        change,
        mu_after: m.mu_before + change,
        character: m.character.clone(),
        opponent_name: m.opponent_name.clone(),
        opponent_character: m.opponent_character.clone(),
        opponent_mu_before: m.opponent_mu_before,
        opponent_change: m.opponent_mu_before.map(|_| -change),
        rounds_own: m.rounds_own,
        rounds_opp: m.rounds_opp,
    });
}

fn fire_session(ctx: &mut Ctx, p: &mut Progress, s: &mut SessionState, id: &str, at: i64, text: String, emphasis: Option<String>, per: Option<String>) {
    let key = match &per {
        Some(x) => format!("{id}|{x}"),
        None => id.to_string(),
    };
    if !s.fired.insert(key) {
        return;
    }
    ctx.fire(p, id, at, text, emphasis, 0);
}

fn session_items(ctx: &mut Ctx, p: &mut Progress, s: &mut SessionState, m: &OwnMatch, at: i64) {
    let who = m.opponent_name.clone();
    if let Some(id) = top_step(&T::SESSION_LENGTHS, s.matches) {
        let n = T::SESSION_LENGTHS.iter().find(|(_, i)| *i == id).unwrap().0;
        fire_session(ctx, p, s, id, at, format!("{n} matches this session"), Some(format!("{n} matches")), None);
    }
    if let Some(id) = top_step(&T::SESSION_NETS, s.net) {
        let n = T::SESSION_NETS.iter().find(|(_, i)| *i == id).unwrap().0;
        fire_session(ctx, p, s, id, at, format!("Up +{n} MR this session"), Some(format!("+{n} MR")), None);
    }
    if s.net > 0 {
        let low = s.low;
        for (floor, id) in T::COMEBACKS.iter().rev() {
            if low <= *floor {
                let text = if *id == "A23" {
                    format!("Back in the green, {} MR", signed(s.net))
                } else {
                    format!("Back from \u{2212}75, now {} MR", signed(s.net))
                };
                fire_session(ctx, p, s, id, at, text, Some(format!("{} MR", signed(s.net))), None);
                break;
            }
        }
    }
    if !m.opponent_id.is_empty() {
        if s.plays.get(&m.opponent_id).copied().unwrap_or(0) >= T::RIVAL_PLAYS {
            fire_session(ctx, p, s, "A24", at, format!("Five matches with {who}"), Some(who.clone()), Some(m.opponent_id.clone()));
        }
        let wins = s.wins_by_opponent.get(&m.opponent_id).copied().unwrap_or(0);
        if let Some(id) = top_step(&T::RIVAL_WINS, wins) {
            let n = T::RIVAL_WINS.iter().find(|(_, i)| *i == id).unwrap().0;
            let word = if n == 3 { "Three" } else { "Five" };
            fire_session(ctx, p, s, id, at, format!("{word} wins over {who}"), Some(who.clone()), Some(m.opponent_id.clone()));
        }
    }
    if s.upsets >= T::SESSION_UPSETS {
        fire_session(ctx, p, s, "A26", at, "Three upsets this session".into(), Some("Three upsets".into()), None);
    }
    if s.first5.len() == T::PERFECT_START && s.first5.iter().all(|w| *w) {
        fire_session(ctx, p, s, "A31", at, "Session opened with 5 straight wins".into(), Some("5 straight wins".into()), None);
    }
    if s.matches >= T::DOMINANT_MATCHES && f64::from(s.wins) / f64::from(s.matches) >= T::DOMINANT_RATE {
        fire_session(ctx, p, s, "A32", at, format!("75% win rate over {} matches", s.matches), Some(format!("{} matches", s.matches)), None);
    }
    if s.matches >= T::STAYED_POSITIVE && s.net > 0 {
        fire_session(ctx, p, s, "A33", at, "Still in the green after 50 matches".into(), Some("50 matches".into()), None);
    }
    if s.chars_played.len() >= T::ROSTER_SHUFFLE {
        fire_session(ctx, p, s, "A34", at, "Played 5 characters this session".into(), Some("5 characters".into()), None);
    }
    if s.sweeps >= T::SESSION_SWEEPS {
        fire_session(ctx, p, s, "A35", at, "5 clean sweeps this session".into(), Some("5 clean sweeps".into()), None);
    }
    if s.final_round_wins >= T::SESSION_FINAL_ROUNDS {
        fire_session(ctx, p, s, "A36", at, "5 final-round wins this session".into(), Some("5 final-round wins".into()), None);
    }
    if s.tilt_wins >= T::TILT_WINS {
        fire_session(ctx, p, s, "A37", at, "Lost three, then won three straight".into(), Some("three straight".into()), None);
    }
    if s.opponents.len() >= T::SESSION_OPPONENTS {
        fire_session(ctx, p, s, "A38", at, "20 different opponents this session".into(), Some("20 different opponents".into()), None);
    }
    if s.higher_ranked.len() >= T::HIGHER_RANKED {
        fire_session(ctx, p, s, "A59", at, "Beaten 5 higher-ranked players this session".into(), Some("5 higher-ranked players".into()), None);
    }
    if s.regions_beaten.len() >= T::SESSION_REGIONS {
        fire_session(ctx, p, s, "A60", at, "Beaten players from 3 regions this session".into(), Some("3 regions".into()), None);
    }
    if s.main_wins >= T::MAIN_WINS {
        let main = p.main_name.clone().or_else(|| p.main.clone()).unwrap_or_default();
        fire_session(ctx, p, s, "A63", at, format!("10 wins this session with {main}"), Some(main), None);
    }
    if s.revenges >= T::SESSION_REVENGES {
        fire_session(ctx, p, s, "A65", at, "Three revenges this session".into(), Some("Three revenges".into()), None);
    }
    if s.chars_won.len() >= T::POCKET_PICKS {
        fire_session(ctx, p, s, "A29", at, "Won with 3 characters this session".into(), Some("3 characters".into()), None);
    }
}

fn ratings_pass(ctx: &mut Ctx, p: &mut Progress, s: &mut SessionState, i: &Inputs) {
    let at = i.now;
    let mut total_games: i64 = 0;
    for r in i.ratings {
        let name = r.rating.character.clone();
        let key = char_key(&name);
        total_games += i64::from(r.rating.games.unwrap_or(0));
        let mu = r.rating.mu;
        let peak = p.peaks.get(&key).copied().unwrap_or(i32::MIN);
        if mu > peak {
            p.peaks.insert(key.clone(), mu);
            if peak != i32::MIN && s.fired.insert(format!("M01|{key}")) {
                s.peak_chars.insert(key.clone());
                ctx.fire(p, "M01", at, format!("New peak on {name}, {mu} MR"), Some(format!("{mu} MR")), 0);
            }
            let hundred = (mu / 100) * 100;
            if (1500..=3000).contains(&hundred) && hundred > p.hundreds.get(&key).copied().unwrap_or(0) {
                p.hundreds.insert(key.clone(), hundred);
                let tier = item("M02")
                    .and_then(|it| it.tiers.iter().position(|t| *t == i64::from(hundred)))
                    .map(|n| n as u8 + 1)
                    .unwrap_or(1);
                ctx.fire(p, "M02", at, format!("{name} reached {hundred} MR"), Some(format!("{hundred} MR")), tier);
            }
        }
        if r.rating.group.starts_with("Leaderboard") && scoped_first(p, "M03", &key) {
            ctx.fire(p, "M03", at, format!("Qualified for the leaderboard with {name}"), Some(name.clone()), 0);
        }
        if let Some(games) = r.rating.games {
            let tier = tier_of(GAMES_TIERS, i64::from(games), ctx.roster_total);
            let have = p.games_tier.get(&key).copied().unwrap_or(0);
            if tier > have {
                p.games_tier.insert(key.clone(), tier);
                ctx.fire(p, "M04", at, format!("{} games on {name}", commas(GAMES_TIERS[tier as usize - 1])), Some(name.clone()), tier);
            }
        }
    }
    if s.peak_chars.len() >= T::SESSION_PEAKS && s.fired.insert("A39".into()) {
        ctx.fire(p, "A39", at, "New peaks on 2 characters this session".into(), Some("2 characters".into()), 0);
    }

    if let Some(best) = i.ratings.iter().max_by_key(|r| (r.rating.mu, r.rating.last_seen.unwrap_or(i64::MIN))) {
        let key = char_key(&best.rating.character);
        p.main_name = Some(best.rating.character.clone());
        if p.main.as_deref() != Some(key.as_str()) {
            let had_main = p.main.is_some();
            p.main = Some(key.clone());
            if had_main && s.fired.insert(format!("M05|{key}")) {
                ctx.fire(p, "M05", at, format!("Your highest rating is now {}", best.rating.character), Some(best.rating.character.clone()), 0);
            }
        }
    }

    for m in i.matches {
        if m.rank.is_some_and(|r| r >= T::GOD_OF_DESTRUCTION) {
            if let Some(name) = &m.character {
                let key = char_key(name);
                if p.god.insert(key) {
                    ctx.fire(p, "M06", m.battle_at, format!("God of Destruction with {name}"), Some(name.clone()), 0);
                }
            }
        }
    }

    let power = i.matches.iter().filter_map(|m| m.power).max();
    if let Some(power) = power {
        let tier = tier_of(POWER_TIERS, power, ctx.roster_total);
        if tier > p.power_tier {
            p.power_tier = tier;
            ctx.fire(p, "M07", at, format!("Tekken Prowess passed {}", commas(POWER_TIERS[tier as usize - 1])), Some(commas(POWER_TIERS[tier as usize - 1])), tier);
        }
    }
    let tier = tier_of(TOTAL_GAMES_TIERS, total_games, ctx.roster_total);
    if tier > p.total_games_tier {
        p.total_games_tier = tier;
        ctx.fire(p, "M08", at, format!("{} ranked games across all characters", commas(TOTAL_GAMES_TIERS[tier as usize - 1])), Some("all characters".into()), tier);
    }
    let rated = i.ratings.len() as i64;
    let tier = tier_of(CHARS_TIERS, rated, ctx.roster_total);
    if tier > p.chars_tier {
        p.chars_tier = tier;
        let step = chars_step(tier, ctx.roster_total);
        ctx.fire(p, "M09", at, format!("Rated on {step} characters"), Some(format!("{step} characters")), tier);
    }
    let tier = tier_of(WINS_TIERS, i64::from(p.career_wins), ctx.roster_total);
    if tier > p.wins_tier {
        p.wins_tier = tier;
        ctx.fire(p, "M10", at, format!("{} wins tracked by the HUD", commas(WINS_TIERS[tier as usize - 1])), Some(format!("{} wins", commas(WINS_TIERS[tier as usize - 1]))), tier);
    }
}

fn chars_step(tier: u8, roster_total: usize) -> i64 {
    let raw = CHARS_TIERS[(tier as usize - 1).min(CHARS_TIERS.len() - 1)];
    if raw == 0 {
        roster_total as i64
    } else {
        raw
    }
}

fn tier_of(tiers: &[i64], value: i64, roster_total: usize) -> u8 {
    let mut out = 0u8;
    for (n, step) in tiers.iter().enumerate() {
        let step = if *step == 0 { roster_total as i64 } else { *step };
        if step > 0 && value >= step {
            out = n as u8 + 1;
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Row {
    pub id: String,
    pub kind: Kind,
    pub section: String,
    pub title: String,
    pub condition: String,
    pub rarity: u8,
    pub earned: bool,
    pub first_at: Option<i64>,
    pub last_at: Option<i64>,
    pub times: u32,
    pub tier: u8,
    pub tier_max: u8,
    pub progress: Option<String>,
    pub current: Option<i64>,
    pub target: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Sections {
    pub name: String,
    pub earned: usize,
    pub total: usize,
}

pub fn collection(p: &Progress, s: &SessionState, i: &Inputs) -> (Vec<Row>, Vec<Sections>) {
    let rows: Vec<Row> = CATALOG
        .iter()
        .map(|it| {
            let e = p.earned.get(it.id);
            let (current, target) = progress_of(it, p, s, i);
            let rarity = match (e, it.tier_rarities.is_empty()) {
                (Some(e), false) if e.tier > 0 => it.tier_rarities[(e.tier as usize - 1).min(it.tier_rarities.len() - 1)],
                _ => it.rarity,
            };
            Row {
                id: it.id.to_string(),
                kind: it.kind,
                section: it.section.label().to_string(),
                title: it.title.to_string(),
                condition: it.condition.to_string(),
                rarity,
                earned: e.is_some(),
                first_at: e.map(|e| e.first),
                last_at: e.map(|e| e.last),
                times: e.map(|e| e.count).unwrap_or(0),
                tier: e.map(|e| e.tier).unwrap_or(0),
                tier_max: it.tiers.len() as u8,
                progress: match (current, target) {
                    (Some(c), Some(t)) => Some(format!("{} / {}", commas(c), commas(t))),
                    _ => None,
                },
                current,
                target,
            }
        })
        .collect();
    let mut sections: Vec<Sections> = Vec::new();
    for it in CATALOG {
        let name = it.section.label().to_string();
        let earned = usize::from(p.earned.contains_key(it.id));
        match sections.iter_mut().find(|s| s.name == name) {
            Some(s) => {
                s.total += 1;
                s.earned += earned;
            }
            None => sections.push(Sections { name, earned, total: 1 }),
        }
    }
    (rows, sections)
}

fn progress_of(it: &Item, p: &Progress, s: &SessionState, i: &Inputs) -> (Option<i64>, Option<i64>) {
    let next_tier = |tiers: &[i64], have: u8, value: i64| -> (Option<i64>, Option<i64>) {
        let idx = have as usize;
        if idx >= tiers.len() {
            return (Some(value), None);
        }
        let step = if tiers[idx] == 0 { i.roster_total as i64 } else { tiers[idx] };
        (Some(value), Some(step))
    };
    match it.id {
        "M04" => {
            let best = i
                .ratings
                .iter()
                .filter_map(|r| r.rating.games.map(|g| (i64::from(g), char_key(&r.rating.character))))
                .max();
            match best {
                Some((games, key)) => next_tier(GAMES_TIERS, p.games_tier.get(&key).copied().unwrap_or(0), games),
                None => (None, None),
            }
        }
        "M07" => next_tier(POWER_TIERS, p.power_tier, i.matches.iter().filter_map(|m| m.power).max().unwrap_or(0)),
        "M08" => next_tier(
            TOTAL_GAMES_TIERS,
            p.total_games_tier,
            i.ratings.iter().filter_map(|r| r.rating.games).map(i64::from).sum(),
        ),
        "M09" => next_tier(CHARS_TIERS, p.chars_tier, i.ratings.len() as i64),
        "M10" => next_tier(WINS_TIERS, p.wins_tier, i64::from(p.career_wins)),
        "A17" | "A18" | "A19" | "A40" => {
            let need = T::SESSION_LENGTHS.iter().find(|(_, id)| *id == it.id).unwrap().0;
            (Some(i64::from(s.matches)), Some(i64::from(need)))
        }
        "A20" | "A21" | "A22" | "A41" => {
            let need = T::SESSION_NETS.iter().find(|(_, id)| *id == it.id).unwrap().0;
            (Some(i64::from(s.net.max(0))), Some(i64::from(need)))
        }
        "A27" => {
            let best = p.beaten.values().map(BTreeSet::len).max().unwrap_or(0);
            (Some(best as i64), Some(T::WELL_ROUNDED as i64))
        }
        "A28" if i.roster_total > 0 => {
            let best = p.beaten.values().map(BTreeSet::len).max().unwrap_or(0);
            (Some(best as i64), Some(i.roster_total as i64))
        }
        "A30" => (Some(p.regions.len() as i64), Some(T::GLOBETROTTER_REGIONS as i64)),
        "A38" => (Some(s.opponents.len() as i64), Some(T::SESSION_OPPONENTS as i64)),
        "A49" | "A50" | "A51" | "A52" | "A53" => {
            let need = T::DAILY_STREAKS.iter().find(|(_, id)| *id == it.id).unwrap().0;
            (Some(i64::from(p.day_streak)), Some(i64::from(need)))
        }
        "A54" | "A55" => {
            let need = T::WINNING_DAYS.iter().find(|(_, id)| *id == it.id).unwrap().0;
            (Some(i64::from(p.winning_day_streak)), Some(i64::from(need)))
        }
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::Rating;

    fn m(at: i64, won: bool, change: i32, mu: i32, opp: &str) -> OwnMatch {
        OwnMatch {
            battle_at: at,
            battle_id: Some(format!("b{at}")),
            chara_id: Some(1),
            character: Some("Anna".into()),
            opponent_id: opp.to_string(),
            opponent_name: opp.to_string(),
            opponent_chara_id: Some(2),
            opponent_character: Some("Lili".into()),
            mu_before: mu,
            change: Some(change),
            opponent_mu_before: Some(mu),
            won,
            rounds_own: Some(if won { 3 } else { 1 }),
            rounds_opp: Some(if won { 1 } else { 3 }),
            rank: Some(30),
            opponent_rank: Some(30),
            region: Some(1),
            opponent_region: Some(2),
            power: Some(400_000),
        }
    }

    fn rating(name: &str, mu: i32, games: i32) -> LiveRating {
        LiveRating {
            rating: Rating { character: name.into(), mu, sigma: Some(70), games: Some(games), last_seen: Some(1), group: "Leaderboard".into() },
            change: None,
        }
    }

    fn inputs<'a>(matches: &'a [OwnMatch], ratings: &'a [LiveRating], since: i64) -> Inputs<'a> {
        Inputs { matches, ratings, roster_total: 42, session_since: since, now: 10_000, utc_offset_min: 0 }
    }

    #[test]
    fn the_catalog_is_whole() {
        assert_eq!(CATALOG.len(), 75, "10 milestones and 65 achievements");
        let ids: BTreeSet<&str> = CATALOG.iter().map(|i| i.id).collect();
        assert_eq!(ids.len(), CATALOG.len(), "no duplicate ids");
        for i in 1..=10 {
            assert!(ids.contains(format!("M{i:02}").as_str()), "M{i:02} present");
        }
        for i in 1..=65 {
            assert!(ids.contains(format!("A{i:02}").as_str()), "A{i:02} present");
        }
        assert!(CATALOG.iter().all(|i| (1..=5).contains(&i.rarity)));
    }

    #[test]
    fn the_first_pass_earns_everything_and_says_nothing() {
        let mut p = Progress::default();
        let mut s = SessionState::new(0);
        let matches: Vec<OwnMatch> = (0..6).map(|n| m(100 + n, true, 10, 2000, "rival")).collect();
        let ratings = [rating("Anna", 2100, 600)];
        let out = update(&mut p, &mut s, &inputs(&matches, &ratings, 0));
        assert!(out.entries.is_empty(), "silent");
        assert!(out.toast.is_none());
        assert_eq!(p.career_wins, 6, "counters still moved");
        assert!(p.earned.contains_key("A42"), "and the earnings are recorded");
        assert_eq!(p.feed.iter().filter(|f| matches!(f, FeedItem::Battle { .. })).count(), 6, "battles still reach the feed");

        let before = p.clone();
        let out = update(&mut p, &mut s, &inputs(&matches, &ratings, 0));
        assert!(out.entries.is_empty());
        assert_eq!(p.career_wins, before.career_wins, "battles are counted once");
    }

    #[test]
    fn a_win_streak_fires_its_highest_step_only() {
        let mut p = Progress { initialized: true, ..Default::default() };
        let mut s = SessionState::new(0);
        let ratings = [rating("Anna", 2000, 600)];
        let mut fired: Vec<String> = Vec::new();
        for n in 0..5 {
            let matches = [m(100 + n, true, 8, 2000, "rival")];
            let out = update(&mut p, &mut s, &inputs(&matches, &ratings, 0));
            fired.extend(out.entries.iter().map(|e| e.id.clone()));
        }
        assert!(fired.contains(&"A42".to_string()), "3 in a row");
        assert!(fired.contains(&"A11".to_string()), "5 in a row");
        assert_eq!(fired.iter().filter(|id| *id == "A42").count(), 1, "once per session");
    }

    #[test]
    fn an_upset_fires_only_the_biggest_rung() {
        let mut p = Progress { initialized: true, ..Default::default() };
        let mut s = SessionState::new(0);
        let mut big = m(100, true, 20, 2000, "giant");
        big.opponent_mu_before = Some(2000 + 800);
        let ratings = [rating("Anna", 2000, 600)];
        let out = update(&mut p, &mut s, &inputs(&[big], &ratings, 0));
        let ids: Vec<&str> = out.entries.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"A56"), "Dragon Slayer");
        assert!(!ids.contains(&"A04") && !ids.contains(&"A05") && !ids.contains(&"A06"), "and nothing below it");
        assert_eq!(out.toast.as_ref().unwrap().rarity, 5, "the rarest fires the toast");
    }

    #[test]
    fn milestones_come_off_the_ratings() {
        let mut p = Progress { initialized: true, ..Default::default() };
        let mut s = SessionState::new(0);
        update(&mut p, &mut s, &inputs(&[], &[rating("Anna", 2090, 9_000)], 0));
        let out = update(&mut p, &mut s, &inputs(&[], &[rating("Anna", 2105, 10_400)], 0));
        let ids: Vec<&str> = out.entries.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"M01"), "new peak");
        assert!(ids.contains(&"M02"), "crossed 2100");
        assert!(ids.contains(&"M04"), "10,000 games");
        assert_eq!(p.earned["M04"].tier, 3);
        assert_eq!(out.entries.iter().find(|e| e.id == "M02").unwrap().rarity, 3, "2100 is a Rare hundred");
    }

    #[test]
    fn the_directory_lists_everything_with_progress() {
        let mut p = Progress { initialized: true, ..Default::default() };
        let mut s = SessionState::new(0);
        let ratings = [rating("Anna", 2000, 600)];
        update(&mut p, &mut s, &inputs(&[m(100, true, 9, 2000, "rival")], &ratings, 0));
        let (rows, sections) = collection(&p, &s, &inputs(&[], &ratings, 0));
        assert_eq!(rows.len(), 75);
        assert_eq!(sections.iter().map(|s| s.total).sum::<usize>(), 75);
        assert_eq!(sections[0].name, "Milestones");
        let m10 = rows.iter().find(|r| r.id == "M10").unwrap();
        assert_eq!((m10.current, m10.target), (Some(1), Some(250)), "career wins count up to the first step");
        let a17 = rows.iter().find(|r| r.id == "A17").unwrap();
        assert_eq!(a17.progress.as_deref(), Some("1 / 10"));
    }

    #[test]
    fn days_in_a_row_earn_again_after_a_break() {
        let mut p = Progress { initialized: true, ..Default::default() };
        let mut s = SessionState::new(0);
        let ratings = [rating("Anna", 2000, 600)];
        let day = 86_400;
        for d in 0..3 {
            update(&mut p, &mut s, &inputs(&[m(d * day + 100, true, 5, 2000, "rival")], &ratings, 0));
        }
        assert_eq!(p.day_streak, 3);
        assert!(p.earned.contains_key("A49"));
        for d in 10..13 {
            update(&mut p, &mut s, &inputs(&[m(d * day + 100, true, 5, 2000, "rival")], &ratings, 0));
        }
        assert_eq!(p.earned["A49"].count, 2, "a broken streak can be earned again");
    }

    #[test]
    fn a_page_battle_is_not_counted_again_when_the_api_catches_up() {
        let ratings = [rating("Anna", 2000, 100)];
        let mut p = Progress::default();
        let mut s = SessionState::new(0);

        let mut page = m(5_000, true, 7, 2000, "rival");
        page.battle_id = None;
        update(&mut p, &mut s, &inputs(&[page], &ratings, 0));
        assert_eq!(p.career_battles, 1);

        let api = m(5_000, true, 7, 2000, "rival");
        update(&mut p, &mut s, &inputs(&[api], &ratings, 0));
        assert_eq!(p.career_battles, 1, "the battle is the same one");
        assert_eq!(
            p.feed.iter().filter(|f| matches!(f, FeedItem::Battle { .. })).count(),
            1,
            "and it has one feed row, not two"
        );
    }

    #[test]
    fn an_upgrade_seeds_the_times_from_the_feed() {
        let ratings = [rating("Anna", 2000, 100)];
        let mut p = Progress::default();
        let mut s = SessionState::new(0);
        update(&mut p, &mut s, &inputs(&[m(5_000, true, 7, 2000, "rival")], &ratings, 0));
        assert_eq!(p.career_battles, 1);

        p.seen_times.clear();

        let mut page = m(5_000, true, 7, 2000, "rival");
        page.battle_id = None;
        update(&mut p, &mut s, &inputs(&[page], &ratings, 0));
        assert_eq!(p.career_battles, 1, "an upgrade counts nothing twice");
    }

    #[test]
    fn a_record_fills_in_the_page_rows_missing_detail() {
        let ratings = [rating("Anna", 2000, 100)];
        let mut p = Progress::default();
        let mut s = SessionState::new(0);

        let mut page = m(5_000, true, 7, 2000, "rival");
        page.battle_id = None;
        page.rounds_own = None;
        page.rounds_opp = None;
        page.opponent_mu_before = None;
        update(&mut p, &mut s, &inputs(&[page], &ratings, 0));
        match &p.feed[0] {
            FeedItem::Battle { rounds_own, .. } => assert_eq!(*rounds_own, None, "the page had none to give"),
            _ => panic!("expected a battle row"),
        }

        update(&mut p, &mut s, &inputs(&[m(5_000, true, 7, 2000, "rival")], &ratings, 0));
        assert_eq!(p.career_battles, 1, "still one battle");
        match &p.feed[0] {
            FeedItem::Battle { rounds_own, rounds_opp, opponent_mu_before, .. } => {
                assert_eq!(*rounds_own, Some(3));
                assert_eq!(*rounds_opp, Some(1));
                assert_eq!(*opponent_mu_before, Some(2000));
            }
            _ => panic!("expected a battle row"),
        }
        assert_eq!(
            p.feed.iter().filter(|f| matches!(f, FeedItem::Battle { .. })).count(),
            1,
            "filled in, not added again"
        );
    }
}

