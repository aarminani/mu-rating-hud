use std::collections::{BTreeSet, HashMap};

use crate::glicko;
use crate::player::{HistoryRow, Rating};
use crate::replays::{Record, Table, GOD_OF_DESTRUCTION, WINDOW_SECS};
use crate::{slot, table};

pub const GLOBAL_HORIZON: i64 = 3 * 3600;

pub const OWN_HORIZON: i64 = 24 * 3600;

pub const SETTLE: i64 = 180;

pub const NAME_HORIZON: i64 = 8 * 3600;

pub const REPLAY_ROWS_HORIZON: i64 = 3600;

pub const MAX_SLOT_BYTES: usize = 4 * 1024 * 1024;

const MIN_NAME_HORIZON: i64 = 3600;

const TRIM_STEP: i64 = 1800;
const NAME_TRIM_STEP: i64 = 2 * 3600;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlayerLatest {
    pub at: i64,
    pub name: String,
    pub power: i64,
    pub mr: i32,
    #[serde(default)]
    pub chara: Option<i32>,
}

fn player_key(tekken_id: &str, chara: Option<i32>) -> String {
    format!("{tekken_id}#{}", chara.unwrap_or(-1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnBattle {
    pub battle_at: i64,
    pub chara_id: i32,
    pub rounds: i32,
    pub god: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnMatch {
    pub battle_at: i64,
    pub battle_id: Option<String>,
    pub chara_id: Option<i32>,
    pub character: Option<String>,
    pub opponent_id: String,
    pub opponent_name: String,
    pub opponent_chara_id: Option<i32>,
    pub opponent_character: Option<String>,
    pub mu_before: i32,
    pub change: Option<i32>,
    pub opponent_mu_before: Option<i32>,
    pub won: bool,
    pub rounds_own: Option<i32>,
    pub rounds_opp: Option<i32>,
    pub rank: Option<i32>,
    pub opponent_rank: Option<i32>,
    pub region: Option<i32>,
    pub opponent_region: Option<i32>,
    pub power: Option<i64>,
}

impl OwnMatch {
    pub fn to_result(&self) -> crate::session::OwnResult {
        crate::session::OwnResult {
            battle_at: self.battle_at,
            character: self.character.clone(),
            opponent_name: self.opponent_name.clone(),
            opponent_character: self.opponent_character.clone(),
            mu_before: self.mu_before,
            change: self.change.unwrap_or(0),
            won: self.won,
            rounds_own: self.rounds_own.unwrap_or(0),
            rounds_opp: self.rounds_opp.unwrap_or(0),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SavedPlayers {
    pub players: HashMap<String, PlayerLatest>,
    pub settled: Vec<i64>,
}

pub fn window_end(t: i64) -> i64 {
    let r = t.rem_euclid(WINDOW_SECS);
    if r == 0 {
        t
    } else {
        t - r + WINDOW_SECS
    }
}

pub fn is_live(now: i64, end: i64) -> bool {
    end > now - SETTLE
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    P1,
    P2,
}

fn side_of(own_id: &str, r: &Record) -> Option<Side> {
    if own_id.is_empty() {
        return None;
    }
    let is = |id: &Option<String>| id.as_deref().map(table::tekken_id).as_deref() == Some(own_id);
    if is(&r.p1_polaris_id) {
        Some(Side::P1)
    } else if is(&r.p2_polaris_id) {
        Some(Side::P2)
    } else {
        None
    }
}

fn identity(r: &Record) -> String {
    match &r.battle_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => format!(
            "{}|{}|{}|{}|{}",
            r.battle_at,
            r.p1_name.as_deref().unwrap_or(""),
            r.p1_power.unwrap_or(-1),
            r.p2_name.as_deref().unwrap_or(""),
            r.p2_power.unwrap_or(-1),
        ),
    }
}

pub struct Store {
    own_id: String,
    records: HashMap<String, Record>,
    settled: BTreeSet<i64>,
    chara: HashMap<i32, String>,
    players: HashMap<String, PlayerLatest>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveRating {
    pub rating: Rating,
    pub change: Option<i32>,
}

pub struct Built {
    pub bytes: Vec<u8>,
    pub horizon: i64,
    pub name_horizon: i64,
    pub rows: usize,
    pub names: usize,
}

impl Store {
    pub fn new(own_tekken_id: &str) -> Self {
        Self {
            own_id: table::tekken_id(own_tekken_id),
            records: HashMap::new(),
            settled: BTreeSet::new(),
            chara: HashMap::new(),
            players: HashMap::new(),
        }
    }

    fn note_players(&mut self, r: &Record) {
        let sides = [
            (&r.p1_polaris_id, &r.p1_name, r.p1_power, r.p1_rank, r.p1_rating_before, r.p1_rating_change),
            (&r.p2_polaris_id, &r.p2_name, r.p2_power, r.p2_rank, r.p2_rating_before, r.p2_rating_change),
        ];
        let sides = [
            (sides[0], r.p1_chara_id),
            (sides[1], r.p2_chara_id),
        ];
        for ((id, name, power, rank, before, change), chara) in sides {
            let (Some(id), Some(name), Some(power), Some(before)) = (id.as_deref(), name, power, before) else {
                continue;
            };
            if name.is_empty() || rank.unwrap_or(0) < GOD_OF_DESTRUCTION {
                continue;
            }
            let id = table::tekken_id(id);
            if id == self.own_id {
                continue;
            }
            let key = player_key(&id, chara);
            if self.players.get(&key).is_some_and(|p| p.at >= r.battle_at) {
                continue;
            }
            self.players.insert(key, PlayerLatest {
                at: r.battle_at,
                name: name.clone(),
                power,
                mr: before + change.unwrap_or(0),
                chara,
            });
        }
    }

    pub fn own_battles(&self) -> Vec<OwnBattle> {
        let mut out: Vec<OwnBattle> = self
            .records
            .values()
            .filter_map(|r| {
                let side = side_of(&self.own_id, r)?;
                let (chara_id, rank) = match side {
                    Side::P1 => (r.p1_chara_id, r.p1_rank),
                    Side::P2 => (r.p2_chara_id, r.p2_rank),
                };
                Some(OwnBattle {
                    battle_at: r.battle_at,
                    chara_id: chara_id?,
                    rounds: r.p1_rounds.unwrap_or(0) + r.p2_rounds.unwrap_or(0),
                    god: rank.unwrap_or(0) >= GOD_OF_DESTRUCTION,
                })
            })
            .collect();
        out.sort_by_key(|b| b.battle_at);
        out
    }

    pub fn own_matches(&self) -> Vec<OwnMatch> {
        let mut out: Vec<OwnMatch> = self
            .records
            .values()
            .filter_map(|r| {
                let side = side_of(&self.own_id, r)?;
                let p1 = side == Side::P1;
                let (mine, theirs) = if p1 {
                    (
                        (r.p1_chara_id, r.p1_rating_before, r.p1_rating_change, r.p1_rounds, r.p1_rank, r.p1_region_id, r.p1_power),
                        (&r.p2_polaris_id, &r.p2_name, r.p2_chara_id, r.p2_rating_before, r.p2_rounds, r.p2_rank, r.p2_region_id),
                    )
                } else {
                    (
                        (r.p2_chara_id, r.p2_rating_before, r.p2_rating_change, r.p2_rounds, r.p2_rank, r.p2_region_id, r.p2_power),
                        (&r.p1_polaris_id, &r.p1_name, r.p1_chara_id, r.p1_rating_before, r.p1_rounds, r.p1_rank, r.p1_region_id),
                    )
                };
                let mu_before = mine.1?;
                let change = mine.2;
                let won = match r.winner {
                    Some(w) => (w == 1) == p1,
                    None => change.unwrap_or(0) > 0,
                };
                Some(OwnMatch {
                    battle_at: r.battle_at,
                    battle_id: r.battle_id.clone(),
                    chara_id: mine.0,
                    character: mine.0.and_then(|c| self.chara.get(&c).cloned()),
                    opponent_id: theirs.0.as_deref().map(table::tekken_id).unwrap_or_default(),
                    opponent_name: theirs.1.clone().unwrap_or_default(),
                    opponent_chara_id: theirs.2,
                    opponent_character: theirs.2.and_then(|c| self.chara.get(&c).cloned()),
                    mu_before,
                    change,
                    opponent_mu_before: theirs.3,
                    won,
                    rounds_own: mine.3,
                    rounds_opp: theirs.4,
                    rank: mine.4,
                    opponent_rank: theirs.5,
                    region: mine.5,
                    opponent_region: theirs.6,
                    power: mine.6,
                })
            })
            .collect();
        out.sort_by_key(|b| b.battle_at);
        out
    }

    pub fn own_results(&self) -> Vec<crate::session::OwnResult> {
        self.own_matches().iter().map(OwnMatch::to_result).collect()
    }

    pub fn latest_own_inputs(&self) -> HashMap<i32, (i32, i32, bool)> {
        let mut out: HashMap<i32, (i64, (i32, i32, bool))> = HashMap::new();
        for r in self.records.values() {
            let Some(side) = side_of(&self.own_id, r) else { continue };
            let (chara, ours, theirs, we_are_p1) = match side {
                Side::P1 => (r.p1_chara_id, r.p1_rating_before, r.p2_rating_before, true),
                Side::P2 => (r.p2_chara_id, r.p2_rating_before, r.p1_rating_before, false),
            };
            let (Some(chara), Some(ours), Some(theirs), Some(winner)) =
                (chara, ours, theirs, r.winner)
            else {
                continue;
            };
            let won = (winner == 1) == we_are_p1;
            let slot = out.entry(chara).or_insert((i64::MIN, (0, 0, false)));
            if r.battle_at >= slot.0 {
                *slot = (r.battle_at, (ours, theirs, won));
            }
        }
        out.into_iter().map(|(k, (_, v))| (k, v)).collect()
    }

    pub fn saved_players(&self, now: i64) -> SavedPlayers {
        SavedPlayers {
            players: self.players.clone(),
            settled: self.settled.iter().copied().filter(|&e| e <= now - GLOBAL_HORIZON).collect(),
        }
    }

    pub fn restore_players(&mut self, saved: SavedPlayers, now: i64, own_times: &[i64]) {
        let own_windows: BTreeSet<i64> = own_times.iter().map(|&t| window_end(t)).collect();
        for (id, p) in saved.players {
            let Some((player, _)) = id.split_once('#') else { continue };
            if p.at <= now - NAME_HORIZON || player == self.own_id {
                continue;
            }
            if !self.players.get(&id).is_some_and(|cur| cur.at >= p.at) {
                self.players.insert(id, p);
            }
        }
        for e in saved.settled {
            if e <= now - GLOBAL_HORIZON && e > now - NAME_HORIZON && !own_windows.contains(&e) {
                self.settled.insert(e);
            }
        }
    }

    pub fn characters(&self) -> &HashMap<i32, String> {
        &self.chara
    }

    pub fn add_characters(&mut self, known: impl IntoIterator<Item = (i32, String)>) {
        self.chara.extend(known);
    }

    pub fn learn_characters(&mut self, history: &[HistoryRow]) -> usize {
        let mut by_time: HashMap<i64, Vec<&HistoryRow>> = HashMap::new();
        for row in history {
            by_time.entry(row.battle_at).or_default().push(row);
        }
        let mut learned = Vec::new();
        for r in self.records.values() {
            let Some(rows) = by_time.get(&r.battle_at) else { continue };
            for row in rows {
                for (pid, cid) in [(&r.p1_polaris_id, r.p1_chara_id), (&r.p2_polaris_id, r.p2_chara_id)] {
                    let (Some(pid), Some(cid)) = (pid.as_deref(), cid) else { continue };
                    let pid = table::tekken_id(pid);
                    if pid == row.left_id {
                        learned.push((cid, row.left_char.clone()));
                    } else if pid == row.right_id {
                        learned.push((cid, row.right_char.clone()));
                    }
                }
            }
        }
        let mut new = 0;
        for (cid, name) in learned {
            if self.chara.insert(cid, name).is_none() {
                new += 1;
            }
        }
        new
    }

    pub fn own_character_unknown(&self) -> bool {
        self.records.values().any(|r| match side_of(&self.own_id, r) {
            Some(Side::P1) => r.p1_chara_id.map_or(false, |c| !self.chara.contains_key(&c)),
            Some(Side::P2) => r.p2_chara_id.map_or(false, |c| !self.chara.contains_key(&c)),
            None => false,
        })
    }

    pub fn live_ratings(&self, page: &[Rating]) -> Vec<LiveRating> {
        let mut own: HashMap<String, Vec<(i64, i32, i32)>> = HashMap::new();
        for r in self.records.values() {
            let (cid, before, change) = match side_of(&self.own_id, r) {
                Some(Side::P1) => (r.p1_chara_id, r.p1_rating_before, r.p1_rating_change),
                Some(Side::P2) => (r.p2_chara_id, r.p2_rating_before, r.p2_rating_change),
                None => continue,
            };
            let (Some(cid), Some(before)) = (cid, before) else { continue };
            let Some(name) = self.chara.get(&cid) else { continue };
            own.entry(table::char_key(name)).or_default().push((r.battle_at, before, change.unwrap_or(0)));
        }
        for v in own.values_mut() {
            v.sort_unstable();
        }

        let mut out: Vec<LiveRating> = Vec::new();
        for base in page {
            let key = table::char_key(&base.character);
            let since = base.last_seen.unwrap_or(i64::MIN);
            let newer: Vec<(i64, i32, i32)> = own
                .get(&key)
                .map(|v| v.iter().filter(|(at, _, _)| *at > since).copied().collect())
                .unwrap_or_default();
            own.remove(&key);
            let mut rating = base.clone();
            let mut change = None;
            if let Some(&(at, before, delta)) = newer.last() {
                rating.mu = before + delta;
                rating.games = rating.games.map(|g| g + newer.len() as i32);
                rating.last_seen = Some(at);
                change = Some(delta);
            }
            out.push(LiveRating { rating, change });
        }
        let group = page.last().map(|r| r.group.clone()).unwrap_or_default();
        let mut fresh: Vec<(String, Vec<(i64, i32, i32)>)> = own.into_iter().collect();
        fresh.sort();
        for (key, battles) in fresh {
            let Some(&(at, before, delta)) = battles.last() else { continue };
            let name = self
                .chara
                .values()
                .find(|n| table::char_key(n) == key)
                .cloned()
                .unwrap_or(key);
            out.push(LiveRating {
                rating: Rating {
                    character: name,
                    mu: before + delta,
                    sigma: None,
                    games: Some(battles.len() as i32),
                    last_seen: Some(at),
                    group: group.clone(),
                },
                change: Some(delta),
            });
        }
        out
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn players_len(&self) -> usize {
        self.players.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn wants(&self, r: &Record) -> bool {
        self.wanted_record(r)
    }

    fn wanted_record(&self, r: &Record) -> bool {
        r.p1_rank.unwrap_or(0) >= GOD_OF_DESTRUCTION
            || r.p2_rank.unwrap_or(0) >= GOD_OF_DESTRUCTION
            || side_of(&self.own_id, r).is_some()
    }

    pub fn merge(&mut self, recs: &[Record]) -> usize {
        let mut added = 0;
        for r in recs {
            self.note_players(r);
            if !self.wanted_record(r) {
                continue;
            }
            let id = identity(r);
            if !self.records.contains_key(&id) {
                added += 1;
            }
            self.records.insert(id, r.clone());
        }
        added
    }

    pub fn merge_with_rd(&mut self, recs: &[Record], rd: &mut crate::rd::RdTable) -> usize {
        let mut fresh: Vec<&Record> = Vec::new();
        let mut added = 0;
        for r in recs {
            self.note_players(r);
            if !self.wanted_record(r) {
                continue;
            }
            let id = identity(r);
            let ratable = |r: &Record| r.p1_rating_before.is_some() && r.p2_rating_before.is_some();
            let newly_ratable = match self.records.get(&id) {
                None => ratable(r),
                Some(old) => !ratable(old) && ratable(r),
            };
            if !self.records.contains_key(&id) {
                added += 1;
            }
            if newly_ratable {
                fresh.push(r);
            }
            self.records.insert(id, r.clone());
        }
        fresh.sort_by_key(|r| r.battle_at);
        for r in fresh {
            rd.observe(r);
        }
        added
    }

    pub fn settle(&mut self, end: i64) {
        self.settled.insert(end);
    }

    pub fn wanted(&self, now: i64, own_times: &[i64]) -> Vec<i64> {
        let mut ends = BTreeSet::new();
        let floor = now - NAME_HORIZON;
        let mut e = window_end(now);
        while e > floor {
            ends.insert(e);
            e -= WINDOW_SECS;
        }
        for &t in own_times {
            if t > now - OWN_HORIZON && t <= now {
                ends.insert(window_end(t));
            }
        }
        ends.into_iter().rev().filter(|e| !self.settled.contains(e)).collect()
    }

    pub fn evict(&mut self, now: i64) {
        let global_floor = now - GLOBAL_HORIZON;
        let own_floor = now - OWN_HORIZON;
        let own_id = self.own_id.clone();
        self.records.retain(|_, r| {
            r.battle_at > global_floor || (r.battle_at > own_floor && side_of(&own_id, r).is_some())
        });
        self.settled.retain(|&e| e > own_floor);
        let name_floor = now - NAME_HORIZON;
        self.players.retain(|_, p| p.at > name_floor);
    }

    pub fn latest_own_mr(&self) -> Option<i32> {
        self.latest_own().map(|(_, mr)| mr)
    }

    pub fn latest_own(&self) -> Option<(i64, i32)> {
        self.records
            .values()
            .filter_map(|r| {
                let side = side_of(&self.own_id, r)?;
                let (before, change) = match side {
                    Side::P1 => (r.p1_rating_before, r.p1_rating_change),
                    Side::P2 => (r.p2_rating_before, r.p2_rating_change),
                };
                before.map(|b| (r.battle_at, b + change.unwrap_or(0)))
            })
            .max_by_key(|(at, _)| *at)
    }

    pub fn latest_own_at(&self) -> Option<i64> {
        self.records
            .values()
            .filter(|r| side_of(&self.own_id, r).is_some())
            .map(|r| r.battle_at)
            .max()
    }

    pub fn table(&self, now: i64, global_horizon: i64) -> Table {
        let mut t = Table::default();
        for r in self.in_horizon(now, global_horizon) {
            t.add_record(r);
        }
        t
    }

    fn in_horizon(&self, now: i64, global_horizon: i64) -> impl Iterator<Item = &Record> {
        let floor = now - global_horizon;
        let own_floor = now - OWN_HORIZON;
        self.records
            .values()
            .filter(move |r| r.battle_at > floor || (r.battle_at > own_floor && side_of(&self.own_id, r).is_some()))
    }

    pub fn latest_own_power(&self) -> Option<(String, i64)> {
        let (r, side) = self
            .records
            .values()
            .filter_map(|r| side_of(&self.own_id, r).map(|s| (r, s)))
            .max_by_key(|(r, _)| r.battle_at)?;
        let (name, power) = match side {
            Side::P1 => (&r.p1_name, r.p1_power),
            Side::P2 => (&r.p2_name, r.p2_power),
        };
        Some((name.clone()?, power?))
    }

    pub fn name_entries(
        &self,
        now: i64,
        name_horizon: i64,
        own: Option<&OwnName>,
        codes: &HashMap<i32, String>,
        rd: Option<&crate::rd::RdTable>,
    ) -> Vec<(String, String)> {
        let floor = now - name_horizon;
        let recent: Vec<(&str, &PlayerLatest)> = self
            .players
            .iter()
            .filter(|(_, p)| p.at > floor)
            .map(|(k, p)| (k.split_once('#').map_or(k.as_str(), |(id, _)| id), p))
            .collect();

        let mut latest: HashMap<&str, &PlayerLatest> = HashMap::new();
        for (id, p) in &recent {
            if latest.get(id).map_or(true, |cur| p.at > cur.at) {
                latest.insert(id, p);
            }
        }

        let mut by_key: HashMap<String, (String, Vec<(i64, String)>)> = HashMap::new();
        let mut rd_by_key: HashMap<String, (String, Vec<(i64, String)>)> = HashMap::new();
        let mut add = |key: String, candidate: (i64, String)| {
            by_key
                .entry(key.to_lowercase())
                .or_insert_with(|| (key, Vec::new()))
                .1
                .push(candidate);
        };
        let mut add_rd = |key: String, candidate: (i64, String)| {
            rd_by_key
                .entry(key.to_lowercase())
                .or_insert_with(|| (key, Vec::new()))
                .1
                .push(candidate);
        };
        let rd_floor = now - GLOBAL_HORIZON;
        let bucket = |rd: Option<&crate::rd::RdTable>, id: &str, chara: Option<i32>, at: i64| {
            if at <= rd_floor {
                return None;
            }
            let (rd, chara) = (rd?, chara?);
            let e = rd.get(id, chara).filter(|e| e.settled())?;
            Some(glicko::bucket_rd(e.rd()).to_string())
        };
        for (id, p) in latest.iter() {
            add(p.name.clone(), (p.power, format!("{} MR", p.mr)));
            if let Some(b) = bucket(rd, id, p.chara, p.at) {
                add_rd(p.name.clone(), (p.power, b));
            }
        }
        for (id, p) in &recent {
            let Some(code) = p.chara.and_then(|c| codes.get(&c)) else { continue };
            let power = latest.get(id).map_or(p.power, |l| l.power);
            add(format!("{}#{code}", p.name), (power, format!("{} MR", p.mr)));
            if let Some(b) = bucket(rd, id, p.chara, p.at) {
                add_rd(format!("{}#{code}", p.name), (power, b));
            }
        }

        if let Some(own) = own.filter(|o| !o.name.is_empty()) {
            let mut own_keys: Vec<(String, String)> = vec![(own.name.clone(), own.text.clone())];
            own_keys.extend(own.per_code.iter().map(|(code, text)| (format!("{}#{code}", own.name), text.clone())));
            for (key, text) in own_keys {
                let folded = key.to_lowercase();
                match own.power {
                    Some(p) => by_key.entry(folded).or_insert_with(|| (key, Vec::new())).1.push((p, text)),
                    None => {
                        by_key.insert(folded, (key, vec![(0, text)]));
                    }
                }
            }
        }

        let mut out: Vec<(String, String)> = by_key
            .into_values()
            .filter_map(|(name, mut candidates)| name_value(&mut candidates).map(|v| (name, v)))
            .collect();
        out.extend(rd_by_key.into_values().filter_map(|(name, candidates)| {
            let mut buckets = candidates.iter().map(|(_, b)| b.as_str());
            let first = buckets.next()?;
            buckets.all(|b| b == first).then(|| (format!("|rd#{name}"), first.to_string()))
        }));
        out.sort();
        out
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OwnName {
    pub name: String,
    pub power: Option<i64>,
    pub text: String,
    pub per_code: Vec<(String, String)>,
}

pub fn name_value(candidates: &mut [(i64, String)]) -> Option<String> {
    candidates.sort();
    if candidates.windows(2).any(|w| w[0].0 == w[1].0) {
        return None;
    }
    let mut parts = Vec::with_capacity(candidates.len());
    for (i, (power, text)) in candidates.iter().enumerate() {
        let from = if i == 0 { 0 } else { (candidates[i - 1].0 + power + 1) / 2 };
        parts.push(format!("{from}={text}"));
    }
    (!parts.is_empty()).then(|| parts.join(";"))
}

#[derive(Debug, Clone, Default)]
pub struct Extras {
    pub own: Option<OwnName>,
    pub tuning: Vec<(String, String)>,
    pub codes: HashMap<i32, String>,
    pub deltas: Vec<(String, i32, i32)>,
    pub bases: Vec<(String, i32)>,
    pub asofs: Vec<(String, i64)>,
    pub rds: Vec<(String, f64)>,
    pub opponent_phi: Option<f64>,
    pub rd: crate::rd::RdTable,
}

fn est_entries(extras: &Extras) -> Vec<(String, String)> {
    let prior = glicko::phi_of(glicko::PRIOR_RD);
    let opp = extras.opponent_phi.unwrap_or(prior);
    let mut out = Vec::new();
    let me_for_opp = match extras.rds.as_slice() {
        [(_, rd)] => glicko::phi_of(*rd),
        _ => prior,
    };
    for (gap, (w, l)) in glicko::table(opp, me_for_opp) {
        out.push((format!("|est@{gap}"), format!("{w}/{l}")));
    }
    for b in glicko::buckets() {
        for (gap, (w, l)) in glicko::table(glicko::phi_of(f64::from(b)), me_for_opp) {
            out.push((format!("|est@{b}@{gap}"), format!("{w}/{l}")));
        }
    }
    for (code, rd) in &extras.rds {
        out.push((format!("|me_rd#{code}"), format!("{rd:.1}")));
        for (gap, (w, l)) in glicko::table(glicko::phi_of(*rd), opp) {
            out.push((format!("|me_est#{code}@{gap}"), format!("{w}/{l}")));
        }
    }
    out
}

pub const ASOF_TIME_SPAN: i64 = 1 << 17;

pub fn pack_asof(chara_id: i32, battle_at: i64) -> Option<i64> {
    (0..128).contains(&chara_id).then(|| i64::from(chara_id) * ASOF_TIME_SPAN + battle_at.rem_euclid(ASOF_TIME_SPAN))
}

fn me_entries(extras: &Extras) -> Vec<(String, String)> {
    let Some(own) = extras.own.as_ref().filter(|o| !o.name.is_empty()) else {
        return Vec::new();
    };
    let mut out = vec![("|me".to_string(), own.name.clone())];
    let asofs: HashMap<&str, i64> = extras.asofs.iter().map(|(c, t)| (c.as_str(), *t)).collect();
    let chara_of: HashMap<&str, i32> = extras.codes.iter().map(|(id, c)| (c.as_str(), *id)).collect();
    for (code, base) in &extras.bases {
        out.push((format!("|me_from#{code}"), base.to_string()));
        out.push((format!("|me_base#{code}"), base.to_string()));
        let packed = asofs.get(code.as_str()).zip(chara_of.get(code.as_str())).and_then(|(at, id)| pack_asof(*id, *at));
        if let Some(v) = packed {
            out.push((format!("|me_asof#{code}"), v.to_string()));
        }
    }
    if let Some(power) = own.power {
        out.push(("|me_power".to_string(), power.to_string()));
    }
    out
}

pub fn slot_bytes(store: &Store, now: i64, payload: &str, extras: &Extras) -> Built {
    slot_bytes_capped(store, now, payload, extras, MAX_SLOT_BYTES)
}

fn slot_bytes_capped(store: &Store, now: i64, payload: &str, extras: &Extras, max: usize) -> Built {
    let mut fixed: Vec<(String, String)> = extras
        .tuning
        .iter()
        .map(|(k, v)| (format!("|{}", k.trim_start_matches('|')), v.clone()))
        .collect();
    fixed.extend(me_entries(extras));
    fixed.extend(est_entries(extras));
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::with_capacity(fixed.len());
    for (k, v) in fixed.into_iter().rev() {
        if seen.insert(k.to_lowercase()) {
            deduped.push((k, v));
        }
    }
    deduped.reverse();
    let fixed = deduped;
    let mut horizon = REPLAY_ROWS_HORIZON;
    let mut name_horizon = NAME_HORIZON;
    loop {
        let t = store.table(now, horizon);
        let mut entries = t.mr_entries();
        entries.extend(fixed.iter().cloned());
        let names =
            store.name_entries(now, name_horizon, extras.own.as_ref(), &extras.codes, Some(&extras.rd));
        let names_count = names.len();
        entries = merge_names(entries, names);
        let bytes = slot::replay_bytes(payload, &entries, &[]);
        if bytes.len() <= max || (horizon == 0 && name_horizon <= MIN_NAME_HORIZON) {
            return Built { bytes, horizon, name_horizon, rows: t.len(), names: names_count };
        }
        if horizon > 0 {
            horizon = (horizon - TRIM_STEP).max(0);
        } else {
            name_horizon = (name_horizon - NAME_TRIM_STEP).max(MIN_NAME_HORIZON);
        }
    }
}

fn merge_names(mut entries: Vec<(String, String)>, names: Vec<(String, String)>) -> Vec<(String, String)> {
    let taken: std::collections::HashSet<String> = entries.iter().map(|(k, _)| k.to_lowercase()).collect();
    entries.extend(names.into_iter().filter(|(k, _)| !k.is_empty() && !taken.contains(&k.to_lowercase())));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "2yh7ByTerD8a";
    const NOW: i64 = 1_789_262_000;

    fn battle(id: &str, at: i64, p1: (&str, &str, i64, i32, i32, i32), p2: (&str, &str, i64, i32, i32, i32)) -> Record {
        Record {
            battle_at: at,
            battle_id: Some(id.into()),
            p1_name: Some(p1.0.into()),
            p1_polaris_id: Some(p1.1.into()),
            p1_power: Some(p1.2),
            p1_rank: Some(p1.3),
            p1_rating_before: Some(p1.4),
            p1_rating_change: Some(p1.5),
            p2_name: Some(p2.0.into()),
            p2_polaris_id: Some(p2.1.into()),
            p2_power: Some(p2.2),
            p2_rank: Some(p2.3),
            p2_rating_before: Some(p2.4),
            p2_rating_change: Some(p2.5),
            ..Default::default()
        }
    }

    fn rating(ch: &str, mu: i32, games: i32, last_seen: i64) -> Rating {
        Rating {
            character: ch.into(),
            mu,
            sigma: Some(75),
            games: Some(games),
            last_seen: Some(last_seen),
            group: "Leaderboard".into(),
        }
    }

    #[test]
    fn character_names_are_learned_from_matching_history_rows() {
        let mut s = Store::new(ME);
        let mut r = battle("1", NOW - 60, ("Me", ME, 10, 30, 2248, 7), ("B", "bbbbbbbbbbbb", 11, 30, 2200, -7));
        r.p1_chara_id = Some(36);
        r.p2_chara_id = Some(14);
        s.merge(&[r]);
        let rows = vec![HistoryRow {
            battle_at: NOW - 60,
            left_id: ME.into(),
            left_char: "Anna".into(),
            right_id: "bbbbbbbbbbbb".into(),
            right_char: "Lili".into(),
            ..Default::default()
        }];
        assert_eq!(s.learn_characters(&rows), 2);
        assert_eq!(s.characters().get(&36).map(String::as_str), Some("Anna"));
        assert_eq!(s.characters().get(&14).map(String::as_str), Some("Lili"));
        assert_eq!(s.learn_characters(&rows), 0, "nothing new the second time");
    }

    #[test]
    fn live_ratings_apply_only_battles_newer_than_the_page() {
        let mut s = Store::new(ME);
        s.add_characters([(36, "Anna".to_string()), (14, "Lili".to_string()), (7, "Bryan".to_string())]);
        let page_seen = NOW - 3000;
        let mut old = battle("old", page_seen - 10, ("Me", ME, 1, 30, 2240, 8), ("X", "xxxxxxxxxxxx", 2, 30, 2000, -8));
        old.p1_chara_id = Some(36);
        let mut a1 = battle("a1", NOW - 600, ("Me", ME, 3, 30, 2248, 8), ("X", "xxxxxxxxxxxx", 4, 30, 2000, -8));
        a1.p1_chara_id = Some(36);
        let mut a2 = battle("a2", NOW - 300, ("X", "xxxxxxxxxxxx", 5, 30, 1990, 4), ("Me", ME, 6, 30, 2256, -4));
        a2.p2_chara_id = Some(36);
        let mut b1 = battle("b1", NOW - 200, ("Me", ME, 7, 30, 1700, 12), ("X", "xxxxxxxxxxxx", 8, 30, 1990, -12));
        b1.p1_chara_id = Some(7);
        s.merge(&[old, a1, a2, b1]);

        let page = vec![rating("Anna", 2248, 2010, page_seen), rating("Lili", 2092, 2373, page_seen)];
        let live = s.live_ratings(&page);
        assert_eq!(live[0].rating.mu, 2252, "2256 - 4 after the latest Anna battle");
        assert_eq!(live[0].rating.games, Some(2012), "two battles newer than the page");
        assert_eq!(live[0].change, Some(-4));
        assert_eq!(live[1].rating.mu, 2092);
        assert_eq!(live[1].change, None, "Lili untouched");
        assert_eq!(live[2].rating.character, "Bryan", "first Bryan battle appended");
        assert_eq!(live[2].rating.mu, 1712);
        assert!(!s.own_character_unknown());
    }

    #[test]
    fn windows_align_to_700_seconds() {
        assert_eq!(window_end(1400), 1400);
        assert_eq!(window_end(1401), 2100);
        assert_eq!(window_end(699), 700);
        assert_eq!(window_end(0), 0);
    }

    #[test]
    fn wanted_covers_the_name_horizon_newest_first_and_skips_settled() {
        let mut s = Store::new(ME);
        let w = s.wanted(NOW, &[]);
        assert_eq!(w[0], window_end(NOW));
        assert!(w.windows(2).all(|p| p[0] - p[1] == WINDOW_SECS));
        assert!(*w.last().unwrap() > NOW - NAME_HORIZON);
        assert!(*w.last().unwrap() - WINDOW_SECS <= NOW - NAME_HORIZON, "gap at the old end");

        s.settle(w[5]);
        assert!(!s.wanted(NOW, &[]).contains(&w[5]));
    }

    #[test]
    fn wanted_adds_own_battles_from_the_day_only() {
        let s = Store::new(ME);
        let twenty_h = NOW - 20 * 3600;
        let two_days = NOW - 48 * 3600;
        let w = s.wanted(NOW, &[twenty_h, two_days]);
        assert!(w.contains(&window_end(twenty_h)));
        assert!(!w.contains(&window_end(two_days)));
    }

    #[test]
    fn a_window_delivered_newest_first_still_folds_in_chronologically() {
        let mk = |i: i64| {
            let mut r = battle(
                &format!("b{i}"),
                NOW - 600 + i,
                ("A", "aaaaaaaaaaaa", 500_000, 30, 2000, 5),
                ("B", "bbbbbbbbbbbb", 510_000, 30, 1990, -5),
            );
            r.p1_chara_id = Some(14);
            r.p2_chara_id = Some(6);
            r
        };
        let forward: Vec<Record> = (0..10).map(mk).collect();
        let mut backward = forward.clone();
        backward.reverse();

        let (mut sf, mut rf) = (Store::new(ME), crate::rd::RdTable::default());
        sf.merge_with_rd(&forward, &mut rf);
        let (mut sb, mut rb) = (Store::new(ME), crate::rd::RdTable::default());
        sb.merge_with_rd(&backward, &mut rb);

        let f = rf.get("aaaaaaaaaaaa", 14).expect("forward entry");
        let b = rb.get("aaaaaaaaaaaa", 14).expect("newest-first entry");
        assert_eq!(f.seen, 10, "every battle must be folded in, not just the newest");
        assert_eq!(b.seen, f.seen, "delivery order changed how many battles counted");
        assert_eq!(b.phi, f.phi, "delivery order changed the resulting deviation");
        assert!(
            b.rd() < glicko::PRIOR_RD,
            "ten battles must move the deviation off the prior, got {:.2}",
            b.rd()
        );
    }

    #[test]
    fn a_battle_seen_twice_moves_the_deviation_once() {
        let mut s = Store::new(ME);
        let mut rd = crate::rd::RdTable::default();
        let mut r = battle(
            "a",
            NOW - 60,
            ("A", "aaaaaaaaaaaa", 500_000, 30, 2000, 5),
            ("B", "bbbbbbbbbbbb", 510_000, 30, 1990, -5),
        );
        r.p1_chara_id = Some(14);
        r.p2_chara_id = Some(6);
        s.merge_with_rd(std::slice::from_ref(&r), &mut rd);
        let once = rd.get("aaaaaaaaaaaa", 14).copied();
        assert!(once.is_some(), "the battle was folded in");
        s.merge_with_rd(std::slice::from_ref(&r), &mut rd);
        let twice = rd.get("aaaaaaaaaaaa", 14).copied();
        assert_eq!(once, twice, "the same battle moved the deviation a second time");
    }

    #[test]
    fn merge_dedupes_and_drops_what_the_badge_never_draws() {
        let mut s = Store::new(ME);
        let god = battle("a", NOW - 60, ("A", "aaaaaaaaaaaa", 500_000, 30, 2000, 5), ("B", "bbbbbbbbbbbb", 510_000, 20, 1500, -5));
        let low = battle("b", NOW - 60, ("C", "cccccccccccc", 300_000, 20, 1400, 5), ("D", "dddddddddddd", 310_000, 21, 1450, -5));
        let mine_low = battle("c", NOW - 60, ("Me", ME, 400_000, 25, 1700, 4), ("E", "eeeeeeeeeeee", 410_000, 24, 1690, -4));
        assert_eq!(s.merge(&[god.clone(), low, mine_low]), 2);
        assert_eq!(s.merge(&[god]), 0, "same battle again");
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn eviction_keeps_own_battles_for_a_day_and_others_for_three_hours() {
        let mut s = Store::new(ME);
        s.merge(&[
            battle("old-other", NOW - 4 * 3600, ("A", "aaaaaaaaaaaa", 1, 30, 2000, 1), ("B", "bbbbbbbbbbbb", 2, 30, 2000, -1)),
            battle("old-mine", NOW - 5 * 3600, ("Me", ME, 3, 30, 2100, 2), ("B", "bbbbbbbbbbbb", 4, 30, 2000, -2)),
            battle("ancient-mine", NOW - 25 * 3600, ("Me", ME, 5, 30, 2050, 2), ("B", "bbbbbbbbbbbb", 6, 30, 2000, -2)),
            battle("new-other", NOW - 600, ("A", "aaaaaaaaaaaa", 7, 30, 2001, 1), ("B", "bbbbbbbbbbbb", 8, 30, 2001, -1)),
        ]);
        s.evict(NOW);
        let t = s.table(NOW, GLOBAL_HORIZON);
        assert!(t.get("Me|3").is_some(), "own battle from 5 h ago kept");
        assert!(t.get("B|4").is_some(), "its opponent comes with it");
        assert!(t.get("A|1").is_none(), "others past three hours dropped");
        assert!(t.get("Me|5").is_none(), "own past a day dropped");
        assert!(t.get("A|7").is_some());
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn payload_is_the_mr_after_the_latest_own_battle_on_either_side() {
        let mut s = Store::new(ME);
        assert_eq!(s.latest_own_mr(), None);
        s.merge(&[
            battle("1", NOW - 900, ("Me", ME, 10, 30, 2248, 7), ("B", "bbbbbbbbbbbb", 11, 30, 2200, -7)),
            battle("2", NOW - 300, ("B", "bbbbbbbbbbbb", 12, 30, 2193, 9), ("Me", "2yh7-ByTe-rD8a", 13, 30, 2255, -9)),
        ]);
        assert_eq!(s.latest_own_mr(), Some(2246), "P2 side, dashed id, 2255 - 9");
    }

    fn badge_pick(value: &str, on_screen: f64) -> Option<String> {
        let mut shown = None;
        for part in value.split(';').filter(|p| !p.is_empty()) {
            if let Some((from, text)) = part.split_once('=') {
                if on_screen >= from.parse::<f64>().unwrap() {
                    shown = Some(text.to_string());
                }
            }
        }
        shown
    }

    #[test]
    fn a_shared_name_resolves_to_the_closest_prowess() {
        let mut c = vec![(500_000, "2300 MR".to_string()), (300_000, "1900 MR".to_string()), (420_000, "2100 MR".into())];
        let v = name_value(&mut c).unwrap();
        assert_eq!(v, "0=1900 MR;360000=2100 MR;460000=2300 MR");
        assert_eq!(badge_pick(&v, 250_000.0).as_deref(), Some("1900 MR"));
        assert_eq!(badge_pick(&v, 359_999.0).as_deref(), Some("1900 MR"));
        assert_eq!(badge_pick(&v, 361_000.0).as_deref(), Some("2100 MR"));
        assert_eq!(badge_pick(&v, 470_000.0).as_deref(), Some("2300 MR"));
        assert_eq!(badge_pick(&v, 9_000_000.0).as_deref(), Some("2300 MR"));
        assert_eq!(name_value(&mut [(1, "a".into())]).as_deref(), Some("0=a"), "one candidate owns everything");
        assert_eq!(name_value(&mut [(7, "a".into()), (7, "b".into())]), None, "same Prowess: never guess");
    }

    #[test]
    fn name_entries_hold_each_players_latest_mr_and_the_local_player() {
        let mut s = Store::new(ME);
        s.merge(&[
            battle("1", NOW - 900, ("Twin", "aaaaaaaaaaaa", 400_000, 30, 2000, 5), ("Solo", "bbbbbbbbbbbb", 350_000, 30, 1800, -5)),
            battle("2", NOW - 300, ("Twin", "aaaaaaaaaaaa", 400_100, 30, 2005, 6), ("Twin", "cccccccccccc", 600_000, 31, 2400, -6)),
            battle("3", NOW - 200, ("Low", "dddddddddddd", 300_000, 20, 1500, 1), ("Me", ME, 410_000, 30, 2240, 8)),
        ]);
        let own = OwnName { name: "Me".into(), power: Some(410_000), text: "2248 MR".into(), per_code: vec![] };
        let e: HashMap<String, String> = s.name_entries(NOW, GLOBAL_HORIZON, Some(&own), &HashMap::new(), None).into_iter().collect();
        assert_eq!(e.get("Solo").map(String::as_str), Some("0=1795 MR"));
        assert_eq!(e.get("Twin").map(String::as_str), Some("0=2011 MR;500050=2394 MR"), "latest battle each, split at the midpoint");
        assert_eq!(e.get("Me").map(String::as_str), Some("0=2248 MR"), "the payload, not a record");
        assert!(!e.contains_key("Low"), "below God of Destruction");

        let unknown = OwnName { power: None, ..own };
        let mut s2 = Store::new(ME);
        s2.merge(&[battle("4", NOW - 100, ("Me", "eeeeeeeeeeee", 420_000, 30, 2100, 1), ("X", "ffffffffffff", 1, 30, 1, 1))]);
        let e2: HashMap<String, String> = s2.name_entries(NOW, GLOBAL_HORIZON, Some(&unknown), &HashMap::new(), None).into_iter().collect();
        assert_eq!(e2.get("Me").map(String::as_str), Some("0=2248 MR"), "own Prowess unknown: own number wins the name");
    }

    #[test]
    fn each_character_gets_its_own_key_and_the_latest_any_character_stays() {
        let codes: HashMap<i32, String> = HashMap::from([(29, "jly".to_string()), (14, "hms".to_string())]);
        let mut s = Store::new(ME);
        let mut a = battle("1", NOW - 900, ("Rival", "aaaaaaaaaaaa", 500_000, 30, 2376, 4), ("X", "xxxxxxxxxxxx", 1, 30, 1, 1));
        a.p1_chara_id = Some(29);
        let mut b = battle("2", NOW - 300, ("Rival", "aaaaaaaaaaaa", 500_300, 30, 2092, -3), ("Y", "yyyyyyyyyyyy", 1, 30, 1, 1));
        b.p1_chara_id = Some(14);
        s.merge(&[a, b]);
        let e: HashMap<String, String> = s.name_entries(NOW, NAME_HORIZON, None, &codes, None).into_iter().collect();
        assert_eq!(e.get("Rival#jly").map(String::as_str), Some("0=2380 MR"), "Leroy's own MR");
        assert_eq!(e.get("Rival#hms").map(String::as_str), Some("0=2089 MR"));
        assert_eq!(e.get("Rival").map(String::as_str), Some("0=2089 MR"), "any character = the latest battle");

        let own = OwnName {
            name: "Me".into(),
            power: Some(410_000),
            text: "2092 MR".into(),
            per_code: vec![("hms".into(), "2092 MR".into()), ("kgr".into(), "2248 MR".into())],
        };
        let e: HashMap<String, String> = s.name_entries(NOW, NAME_HORIZON, Some(&own), &codes, None).into_iter().collect();
        assert_eq!(e.get("Me#kgr").map(String::as_str), Some("0=2248 MR"));
        assert_eq!(e.get("Me").map(String::as_str), Some("0=2092 MR"));
    }

    #[test]
    fn the_slot_says_who_the_local_player_is_and_what_changed() {
        let s = Store::new(ME);
        let own = OwnName { name: "Me".into(), power: Some(410_000), text: "2100 MR".into(), per_code: vec![("hms".into(), "2100 MR".into())] };
        let extras = Extras { own: Some(own.clone()), deltas: vec![("hms".into(), 2092, 2100)], ..Extras::default() };
        let b = slot_bytes(&s, NOW, "2100 MR", &extras).bytes;
        let has = |needle: &[u8]| b.windows(needle.len()).any(|w| w == needle);
        assert!(has(b"|me_power") && has(b"410000"));
        assert!(has(b"Me#hms"));
        assert!(!has(b"|me_delta"), "the load-time delta must not reach the slot");
        assert!(!has(b"2092=2100"), "and neither must its FROM=TO payload");

        let blind = Extras { own: Some(OwnName { power: None, ..own }), deltas: vec![("hms".into(), 2092, 2100)], ..Extras::default() };
        let b = slot_bytes(&s, NOW, "2100 MR", &blind).bytes;
        assert!(!b.windows(b"|me_power".len()).any(|w| w == b"|me_power"));
        assert!(b.windows(b"|me".len()).any(|w| w == b"|me"));
    }

    #[test]
    fn own_battles_carry_character_rounds_and_the_gate() {
        let mut s = Store::new(ME);
        let mut r = battle("1", NOW - 60, ("X", "xxxxxxxxxxxx", 1, 30, 2000, 5), ("Me", ME, 410_000, 28, 2092, 8));
        r.p2_chara_id = Some(14);
        r.p1_rounds = Some(3);
        r.p2_rounds = Some(2);
        s.merge(&[r]);
        assert_eq!(s.own_battles(), vec![OwnBattle { battle_at: NOW - 60, chara_id: 14, rounds: 5, god: false }]);
    }

    #[test]
    fn any_name_goes_in_unless_it_is_literally_another_key() {
        let entries = vec![("TidalRonin|368100".to_string(), "1971 MR".to_string()), ("|notice_y".into(), "230".into())];
        let names = vec![
            ("VØID|WraithHook".to_string(), "0=2100 MR".to_string()),
            ("ютуб юпюп".into(), "0=2300 MR".into()),
            ("x=y;z".into(), "0=1 MR".into()),
            ("  spaced  ".into(), "0=2 MR".into()),
            ("🔥ZERO\u{200b}WIDTH🔥".into(), "0=3 MR".into()),
            ("tidalronin|368100".into(), "0=4 MR".into()),
            ("|NOTICE_Y".into(), "0=5 MR".into()),
        ];
        let merged = merge_names(entries, names);
        let keys: Vec<&str> = merged.iter().map(|(k, _)| k.as_str()).collect();
        for k in ["VØID|WraithHook", "ютуб юпюп", "x=y;z", "  spaced  ", "🔥ZERO\u{200b}WIDTH🔥"] {
            assert!(keys.contains(&k), "{k:?} should be in");
        }
        assert_eq!(merged.len(), 7, "only the two exact collisions (ignoring case) are dropped");
    }

    #[test]
    fn names_differing_only_in_case_share_one_key_and_pick_by_prowess() {
        let mut s = Store::new(ME);
        s.merge(&[battle("1", NOW - 60, ("TidalRonin", "aaaaaaaaaaaa", 300_000, 30, 1900, 0), ("TIDALRONIN", "bbbbbbbbbbbb", 500_000, 30, 2300, 0))]);
        let e = s.name_entries(NOW, NAME_HORIZON, None, &HashMap::new(), None);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].1, "0=1900 MR;400000=2300 MR");
    }

    #[test]
    fn name_entries_outlive_the_battle_table_and_survive_a_restart() {
        assert!(
            NAME_HORIZON > GLOBAL_HORIZON,
            "the name fallback needs a window of its own beyond the battle table"
        );
        let mut s = Store::new(ME);
        let recent = NOW - 600;
        let fallback = NOW - (GLOBAL_HORIZON + 1800);
        s.merge(&[
            battle(
                "y",
                recent,
                ("ютуб юпюп", "3a2nqAtrj8bB", 541_147, 33, 2290, 8),
                ("Low", "cccccccccccc", 1, 10, 1, 1),
            ),
            battle(
                "z",
                fallback,
                ("Older", "dddddddddddd", 480_000, 33, 2000, -10),
                ("Low2", "eeeeeeeeeeee", 1, 10, 1, 1),
            ),
        ]);
        let e: HashMap<String, String> = s
            .name_entries(NOW, NAME_HORIZON, None, &HashMap::new(), None)
            .into_iter()
            .collect();
        assert_eq!(
            e.get("ютуб юпюп").map(String::as_str),
            Some("0=2298 MR"),
            "a GoD+ player inside the horizon is named"
        );
        assert!(!e.contains_key("Low"), "the badge never renders below GoD");
        assert_eq!(
            e.get("Older").map(String::as_str),
            Some("0=1990 MR"),
            "a player past the battle horizon is still named - this IS the fallback window"
        );
        let battles_only: HashMap<String, String> = s
            .name_entries(NOW, GLOBAL_HORIZON, None, &HashMap::new(), None)
            .into_iter()
            .collect();
        assert!(
            !battles_only.contains_key("Older"),
            "and would have been gone had the horizons stayed equal"
        );

        let saved = s.saved_players(NOW);
        let json = serde_json::to_string(&saved).unwrap();
        let mut fresh = Store::new(ME);
        fresh.restore_players(serde_json::from_str(&json).unwrap(), NOW, &[]);
        assert_eq!(
            fresh.name_entries(NOW, NAME_HORIZON, None, &HashMap::new(), None),
            s.name_entries(NOW, NAME_HORIZON, None, &HashMap::new(), None),
            "names did not survive the round trip"
        );

        let own = NOW - 12 * 3600;
        let mut mine = Store::new(ME);
        mine.restore_players(serde_json::from_str(&json).unwrap(), NOW, &[own]);
        assert!(
            mine.wanted(NOW, &[own]).contains(&window_end(own)),
            "a window with an own battle is refetched for the table"
        );

        let past = NOW + NAME_HORIZON + 600;
        s.evict(past);
        assert!(
            s.name_entries(past, NAME_HORIZON, None, &HashMap::new(), None).is_empty(),
            "gone once past the horizon"
        );
    }

    #[test]
    fn wanted_reaches_back_a_day_newest_first() {
        let s = Store::new(ME);
        let w = s.wanted(NOW, &[]);
        assert_eq!(w[0], window_end(NOW));
        assert!(*w.last().unwrap() > NOW - NAME_HORIZON);
        assert!(w.len() as i64 >= NAME_HORIZON / WINDOW_SECS);
    }

    #[test]
    fn latest_own_inputs_picks_the_newest_per_character_and_skips_gaps() {
        let mut s = Store::new(ME);
        let mut older = battle("a", NOW - 600, ("Me", ME, 400_000, 30, 2055, 5), ("Opp", "bbbbbbbbbbbb", 400_000, 30, 1835, -6));
        older.p1_chara_id = Some(14);
        older.winner = Some(1);
        let mut newer = battle("b", NOW - 60, ("Me", ME, 400_000, 30, 2060, 4), ("Opp", "bbbbbbbbbbbb", 400_000, 30, 1830, -5));
        newer.p1_chara_id = Some(14);
        newer.winner = Some(1);
        let mut other = battle("c", NOW - 30, ("Opp", "cccccccccccc", 400_000, 30, 1900, 9), ("Me", ME, 400_000, 30, 1500, -9));
        other.p2_chara_id = Some(33);
        other.winner = Some(1);
        let mut broken = battle("d", NOW - 10, ("Me", ME, 400_000, 30, 1700, 0), ("Opp", "dddddddddddd", 400_000, 30, 1700, 0));
        broken.p1_chara_id = Some(41);
        broken.winner = None;
        s.merge(&[older, newer, other, broken]);

        let got = s.latest_own_inputs();
        assert_eq!(got.get(&14), Some(&(2060, 1830, true)), "newest Lili battle, a win");
        assert_eq!(got.get(&33), Some(&(1500, 1900, false)), "we were P2 and lost");
        assert_eq!(got.get(&41), None, "a record with no winner is skipped");
    }

    #[test]
    fn tuning_and_names_go_into_the_slot() {
        let mut s = Store::new(ME);
        s.merge(&[battle("1", NOW - 60, ("A", "aaaaaaaaaaaa", 500_000, 30, 2000, 5), ("B", "bbbbbbbbbbbb", 510_000, 30, 1500, -5))]);
        let extras = Extras { tuning: vec![("notice_y".into(), "230".into())], ..Extras::default() };
        let b = slot_bytes(&s, NOW, "", &extras).bytes;
        let has = |needle: &[u8]| b.windows(needle.len()).any(|w| w == needle);
        assert!(has(b"|notice_y") && has(b"230"));
        assert!(has(b"0=2005 MR") && has(b"A|500000"));
        assert_eq!(s.latest_own_power(), None);
    }

    #[test]
    fn each_side_getting_its_own_rd_breaks_the_mirror() {
        let extras = Extras {
            rds: vec![("hms".into(), 63.0)],
            opponent_phi: Some(glicko::phi_of(75.0)),
            ..Default::default()
        };
        let e: HashMap<String, String> = est_entries(&extras).into_iter().collect();
        let mut differed = 0;
        for gap in (-600..=600).step_by(10) {
            let mine = e.get(&format!("|me_est#hms@{gap}")).expect("our table");
            let theirs = e.get(&format!("|est@{}", -gap)).expect("their table");
            let (mw, _) = mine.split_once('/').unwrap();
            let (_, tl) = theirs.split_once('/').unwrap();
            if mw.parse::<i32>().unwrap() != -tl.parse::<i32>().unwrap() {
                differed += 1;
            }
        }
        assert!(
            differed > 40,
            "only {differed} of 121 gaps differed - the two sides are still mirroring"
        );
    }

    #[test]
    fn with_no_known_rd_the_tables_are_the_shipped_prior_table() {
        let e: HashMap<String, String> = est_entries(&Extras::default()).into_iter().collect();
        for (gap, w, l) in [(-600, 1, -24), (-100, 9, -15), (0, 12, -12), (100, 15, -9)] {
            assert_eq!(
                e.get(&format!("|est@{gap}")).map(String::as_str),
                Some(format!("{w}/{l}").as_str()),
                "gap {gap} no longer matches the table the badge was proven with"
            );
        }
    }

    #[test]
    fn a_live_est_table_wins_against_a_stale_tuning_override() {
        let s = Store::new(ME);
        let extras = Extras {
            own: Some(OwnName { name: "MRmani".into(), power: None, ..Default::default() }),
            tuning: vec![("est@0".into(), "99/-99".into())],
            rds: vec![("hms".into(), 63.0)],
            opponent_phi: Some(glicko::phi_of(75.0)),
            ..Default::default()
        };
        let built = slot_bytes(&s, NOW, "2065 MR", &extras);
        let hay = built.bytes.windows(6).any(|w| w == b"99/-99");
        assert!(!hay, "a stale tuning table survived and would pin the badge's deltas");
    }

    #[test]
    fn our_own_rd_is_published_for_the_badge_to_read() {
        let extras = Extras { rds: vec![("hms".into(), 63.0)], ..Default::default() };
        let e: HashMap<String, String> = est_entries(&extras).into_iter().collect();
        assert_eq!(e.get("|me_rd#hms").map(String::as_str), Some("63.0"));
    }

    #[test]
    fn from_equals_base_and_asof_says_what_base_is_current_to() {
        let extras = Extras {
            own: Some(OwnName { name: "MRmani".into(), power: None, ..Default::default() }),
            codes: HashMap::from([(32, "hms".to_string()), (7, "usi".to_string())]),
            bases: vec![("hms".into(), 2038), ("usi".into(), 1635)],
            asofs: vec![("hms".into(), 1_789_574_714)],
            ..Default::default()
        };
        let e: HashMap<String, String> = me_entries(&extras).into_iter().collect();
        assert_eq!(e.get("|me_base#hms").map(String::as_str), Some("2038"));
        assert_eq!(e.get("|me_from#hms"), e.get("|me_base#hms"), "the old badge must see no correction");
        assert_eq!(e.get("|me_from#usi"), e.get("|me_base#usi"));
        let packed: i64 = e["|me_asof#hms"].parse().unwrap();
        assert_eq!(packed, 32 * 131_072 + 1_789_574_714 % 131_072);
        assert_eq!(e.get("|me_asof#usi"), None);
    }

    #[test]
    fn packed_asof_is_float_exact_and_tells_characters_and_battles_apart() {
        let t = 1_789_574_714;
        for id in [0, 32, 47, 127] {
            let v = pack_asof(id, t).unwrap();
            assert!(v < 1 << 24, "id {id} packs to {v}, beyond a float's exact integers");
            assert_eq!((v as f32) as i64, v, "id {id} does not survive the cvar");
            assert_eq!(v / ASOF_TIME_SPAN, i64::from(id), "character bits lost");
        }
        assert_ne!(pack_asof(32, t), pack_asof(32, t + 180), "the next battle must change it");
        assert_ne!(pack_asof(32, t), pack_asof(7, t), "another character must not match");
        assert_eq!(pack_asof(128, t), None);
        assert_eq!(pack_asof(-1, t), None);
    }

    #[test]
    fn base_is_published_page_exact_and_from_is_not() {
        let s = Store::new(ME);
        let extras = Extras {
            own: Some(OwnName { name: "MRmani".into(), power: None, ..Default::default() }),
            tuning: vec![("me_base#hms".into(), "2065".into())],
            deltas: vec![("hms".into(), 2054, 2065)],
            bases: vec![("hms".into(), 2064)],
            ..Extras::default()
        };
        let b = slot_bytes(&s, NOW, "", &extras).bytes;
        let has = |needle: &[u8]| b.windows(needle.len()).any(|w| w == needle);
        assert!(has(b"|me_base#hms"), "base key missing");
        assert!(has(b"|me_from#hms"), "from key missing - then_4 would write nothing");
        assert!(has(b"2064"), "page-exact value missing");
        assert!(!has(b"2065"), "stale tuning value survived and would pin the badge");
        assert!(!has(b"2054"), "a delta.rs guess reached the slot");
        assert!(!has(b"|me_power"));
        assert!(!has(b"|me_delta#hms"));
    }

    #[test]
    fn the_cap_trims_others_first_and_never_own_rows() {
        let mut s = Store::new(ME);
        let mut recs = Vec::new();
        for k in 0..400 {
            let at = NOW - 60 - k * 25;
            recs.push(battle(&format!("o{k}"), at,
                (&format!("Other{k}"), "aaaaaaaaaaaa", 100_000 + k, 30, 2000, 1),
                (&format!("Rival{k}"), "bbbbbbbbbbbb", 200_000 + k, 30, 2000, -1)));
        }
        recs.push(battle("mine", NOW - 10_000, ("Me", ME, 999_999, 30, 2248, 3), ("Opp", "cccccccccccc", 888_888, 30, 2100, -3)));
        s.merge(&recs);

        let none = Extras::default();
        let full = slot_bytes_capped(&s, NOW, "2251 MR", &none, usize::MAX);
        assert_eq!(full.horizon, REPLAY_ROWS_HORIZON);
        assert_eq!(full.name_horizon, NAME_HORIZON);
        let capped = slot_bytes_capped(&s, NOW, "2251 MR", &none, full.bytes.len() * 2 / 3);
        assert!(capped.horizon < REPLAY_ROWS_HORIZON);
        assert!(capped.rows < full.rows);
        assert!(s.table(NOW, capped.horizon).get("Me|999999").is_some());
        assert!(capped.bytes.windows(b"Me|999999".len()).any(|w| w == b"Me|999999"));
    }

    #[test]
    fn the_cap_keeps_names_before_replay_rows() {
        let mut s = Store::new(ME);
        let mut recs = Vec::new();
        for k in 0..960 {
            let at = NOW - 20 - k * 30;
            recs.push(battle(&format!("n{k}"), at,
                (&format!("Old{k}"), &format!("{k:012}"), 100_000 + k, 30, 2000, 1),
                (&format!("Foe{k}"), &format!("f{k:011}"), 200_000 + k, 31, 2100, -1)));
        }
        s.merge(&recs);
        let none = Extras::default();
        let full = slot_bytes_capped(&s, NOW, "", &none, usize::MAX);
        let rows_gone = slot_bytes_capped(&s, NOW, "", &none, full.bytes.len() - 1);
        assert!(rows_gone.horizon < REPLAY_ROWS_HORIZON, "rows must be trimmed first");
        assert_eq!(rows_gone.name_horizon, NAME_HORIZON, "names must not be touched while rows remain");
        let tiny = slot_bytes_capped(&s, NOW, "", &none, 1);
        assert_eq!(tiny.horizon, 0);
        assert_eq!(tiny.name_horizon, MIN_NAME_HORIZON, "names stop at an hour");
    }
}
