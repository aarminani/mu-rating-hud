use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const BIAS: i64 = 30;
pub const INTRO: i64 = 20;
pub const ROUND: i64 = 28;
pub const MIN_MATCH: i64 = 45;
pub const LOAD_GAP: i64 = 60;
pub const SESSION_GAP: i64 = 600;
pub const DEFAULT_OFFSET: i64 = 5;
pub const MATCH_SAVE: i64 = 60;
pub const CONFIRM: i64 = 240;
const LOG_KEEP: i64 = 3 * 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shown {
    pub mr: i32,
    pub at: i64,
    #[serde(default)]
    pub from_save: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Guess(i64),
    Record(Option<i64>),
}

#[derive(Debug, Clone, PartialEq)]
struct Pending {
    save: i64,
    code: String,
    before: Option<Shown>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ledger {
    pub shown: BTreeMap<String, Shown>,
    pub last_code: Option<String>,
    pub offset: Option<i64>,
    pub battles: Vec<i64>,
    #[serde(skip)]
    log: Vec<(i64, BTreeMap<String, i32>)>,
    #[serde(skip)]
    ends: Vec<i64>,
    #[serde(skip)]
    pending: Vec<Pending>,
}

impl Ledger {
    pub fn wrote(&mut self, t: i64, current: &BTreeMap<String, i32>) {
        if self.log.last().map(|(_, m)| m) != Some(current) {
            self.log.push((t, current.clone()));
        }
        let floor = t - LOG_KEEP;
        if self.log.len() > 1 {
            let keep_from = self.log.iter().rposition(|(at, _)| *at <= floor).unwrap_or(0);
            self.log.drain(..keep_from);
        }
    }

    fn value_at(&self, code: &str, t: i64) -> Option<(i32, i64)> {
        let entry = self.log.iter().rev().find(|(at, _)| *at <= t).or_else(|| self.log.first())?;
        entry.1.get(code).map(|mr| (*mr, t.max(entry.0)))
    }

    fn set_shown(&mut self, code: &str, at: i64, source: Source) {
        let Some((mr, sampled)) = self.value_at(code, at) else { return };
        let from_save = match source {
            Source::Guess(w) => Some(w),
            Source::Record(_) => None,
        };
        let replaces_guess = match (source, self.shown.get(code)) {
            (Source::Record(Some(w)), Some(prev)) => prev.from_save == Some(w) && !self.ends.iter().any(|&e| e > w),
            _ => false,
        };
        match self.shown.get(code) {
            Some(prev) if prev.at > sampled && !replaces_guess => {}
            _ => {
                self.shown.insert(code.to_string(), Shown { mr, at: sampled, from_save });
            }
        }
    }

    fn previous_end(&self, t: i64) -> Option<i64> {
        self.ends.iter().copied().filter(|&w| w < t && t - w <= SESSION_GAP).max()
    }

    fn latest_load(&self, end: i64) -> i64 {
        let own = end - MIN_MATCH;
        match self.previous_end(end - MIN_MATCH) {
            Some(prev) => own.min(prev + LOAD_GAP),
            None => own,
        }
    }

    pub fn replay_saved(&mut self, t: i64) {
        let loaded = self.latest_load(t);
        self.ends.push(t);
        if let Some(code) = self.last_code.clone() {
            let before = self.shown.get(&code).copied();
            self.set_shown(&code, loaded + BIAS, Source::Guess(t));
            self.pending.push(Pending { save: t, code, before });
        }
    }

    pub fn settle_due(&self, now: i64) -> bool {
        self.pending.iter().any(|p| now - p.save > CONFIRM)
    }

    pub fn settle(&mut self, now: i64) -> bool {
        let (expired, keep): (Vec<Pending>, Vec<Pending>) =
            std::mem::take(&mut self.pending).into_iter().partition(|p| now - p.save > CONFIRM);
        self.pending = keep;
        let mut changed = false;
        for p in expired {
            self.ends.retain(|&w| w != p.save);
            if self.shown.get(&p.code).map(|s| s.from_save) == Some(Some(p.save)) {
                match p.before {
                    Some(b) => self.shown.insert(p.code, b),
                    None => self.shown.remove(&p.code),
                };
                changed = true;
            }
        }
        changed
    }

    pub fn own_battle(&mut self, battle_at: i64, code: &str, rounds: i32, god: bool) {
        if self.battles.contains(&battle_at) {
            return;
        }
        self.battles.push(battle_at);
        if self.battles.len() > 200 {
            self.battles.remove(0);
        }
        if let Some(w) = self.ends.iter().copied().filter(|w| (battle_at - w).abs() <= 60).min_by_key(|w| (battle_at - w - DEFAULT_OFFSET).abs()) {
            self.offset = Some(battle_at - w);
        }
        if self.battles.iter().any(|&b| b > battle_at) {
            return;
        }
        self.last_code = Some(code.to_string());
        if !god {
            self.shown.remove(code);
            return;
        }
        let offset = self.offset.unwrap_or(DEFAULT_OFFSET);
        let end = battle_at - offset;
        let by_rounds = end - INTRO - ROUND * i64::from(rounds.max(1));
        let loaded = by_rounds.min(self.latest_load(end));
        let matched = self.ends.iter().copied().filter(|w| (end - w).abs() <= MATCH_SAVE).min_by_key(|w| (end - w).abs());
        if let Some(w) = matched {
            self.pending.retain(|p| p.save != w);
        }
        self.set_shown(code, loaded + BIAS, Source::Record(matched));
    }

    pub fn deltas(&self, current: &BTreeMap<String, i32>) -> Vec<(String, i32, i32)> {
        current
            .iter()
            .filter_map(|(code, &to)| {
                let from = self.shown.get(code)?.mr;
                (from != to).then(|| (code.clone(), from, to))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(pairs: &[(&str, i32)]) -> BTreeMap<String, i32> {
        pairs.iter().map(|(c, v)| (c.to_string(), *v)).collect()
    }

    const T: i64 = 1_789_300_000;

    #[test]
    fn a_result_shows_once_at_the_next_match() {
        let mut l = Ledger::default();
        l.wrote(T, &m(&[("hms", 2092)]));
        l.replay_saved(T + 200);
        l.own_battle(T + 205, "hms", 4, true);
        l.wrote(T + 290, &m(&[("hms", 2100)]));
        assert_eq!(l.deltas(&m(&[("hms", 2100)])), vec![("hms".into(), 2092, 2100)], "match 2 shows (+8)");

        l.replay_saved(T + 560);
        assert!(l.deltas(&m(&[("hms", 2100)])).is_empty(), "consumed at once: match 3 must not repeat it");
        l.own_battle(T + 565, "hms", 3, true);
        assert!(l.deltas(&m(&[("hms", 2100)])).is_empty());
    }

    #[test]
    fn late_results_combine() {
        let mut l = Ledger::default();
        l.wrote(T, &m(&[("hms", 2092)]));
        l.replay_saved(T + 200);
        l.own_battle(T + 205, "hms", 4, true);
        l.wrote(T + 370, &m(&[("hms", 2097)]));
        l.replay_saved(T + 380);
        l.own_battle(T + 385, "hms", 3, true);
        l.wrote(T + 470, &m(&[("hms", 2093)]));
        assert_eq!(l.deltas(&m(&[("hms", 2093)])), vec![("hms".into(), 2092, 2093)], "reads (+1)");
    }

    #[test]
    fn a_result_close_to_a_match_start_is_skipped_never_repeated() {
        let mut l = Ledger::default();
        l.wrote(T, &m(&[("hms", 2092)]));
        l.replay_saved(T + 200);
        l.own_battle(T + 205, "hms", 4, true);
        l.wrote(T + 240, &m(&[("hms", 2097)]));
        l.replay_saved(T + 380);
        l.own_battle(T + 385, "hms", 3, true);
        l.wrote(T + 470, &m(&[("hms", 2093)]));
        assert_eq!(l.deltas(&m(&[("hms", 2093)])), vec![("hms".into(), 2097, 2093)]);
    }

    #[test]
    fn a_switched_character_shows_no_delta_until_played() {
        let mut l = Ledger::default();
        l.wrote(T, &m(&[("hms", 2092), ("kgr", 2248)]));
        l.replay_saved(T + 200);
        l.own_battle(T + 205, "hms", 4, true);
        l.wrote(T + 290, &m(&[("hms", 2100), ("kgr", 2248)]));
        assert!(l.deltas(&m(&[("hms", 2100), ("kgr", 2255)])).iter().all(|(c, ..)| c != "kgr"), "Anna never shown yet");
    }

    #[test]
    fn below_god_of_destruction_the_number_was_never_seen() {
        let mut l = Ledger::default();
        l.wrote(T, &m(&[("hms", 2092)]));
        l.own_battle(T + 205, "hms", 4, true);
        l.wrote(T + 290, &m(&[("hms", 2050)]));
        l.own_battle(T + 505, "hms", 3, false);
        assert!(l.deltas(&m(&[("hms", 2060)])).is_empty());
    }

    fn at(m_: i64, s: i64) -> i64 {
        T + m_ * 60 + s
    }

    fn two_back_to_back() -> Ledger {
        let mut l = Ledger::default();
        l.wrote(at(0, 0), &m(&[("hms", 2092)]));
        l.replay_saved(at(7, 9));
        l.own_battle(at(7, 14), "hms", 5, true);
        l.wrote(at(8, 40), &m(&[("hms", 2080)]));
        l.replay_saved(at(10, 46));
        l
    }

    #[test]
    fn spec_1_the_record_replaces_its_matchs_guess() {
        let mut l = two_back_to_back();
        l.own_battle(at(10, 51), "hms", 4, true);
        assert_eq!(l.deltas(&m(&[("hms", 2080)])), vec![("hms".into(), 2092, 2080)]);
        assert_eq!(l.shown["hms"].from_save, None, "record-derived");
    }

    #[test]
    fn spec_2_no_repeat() {
        let mut l = two_back_to_back();
        l.own_battle(at(10, 51), "hms", 4, true);
        l.replay_saved(at(13, 30));
        assert!(l.deltas(&m(&[("hms", 2080)])).is_empty());
    }

    #[test]
    fn spec_3_a_newer_match_already_ended() {
        let mut l = two_back_to_back();
        l.replay_saved(at(13, 30));
        let later_guess = l.shown["hms"];
        assert_eq!(later_guess.from_save, Some(at(13, 30)));
        l.own_battle(at(10, 51), "hms", 4, true);
        assert_eq!(l.shown["hms"], later_guess, "the record does not override");
    }

    #[test]
    fn spec_4_no_matched_save_is_forward_only() {
        let mut l = Ledger::default();
        l.wrote(at(0, 0), &m(&[("hms", 2092)]));
        l.own_battle(at(0, 30), "hms", 1, true);
        l.wrote(at(20, 0), &m(&[("hms", 2080)]));
        l.replay_saved(at(30, 0));
        let guess = l.shown["hms"];
        l.own_battle(at(31, 30), "hms", 9, true);
        assert_eq!(l.shown["hms"], guess);
    }

    #[test]
    fn a_newer_save_blocks_an_older_matchs_record() {
        let mut l = Ledger::default();
        l.wrote(T, &m(&[("hms", 2092)]));
        l.own_battle(T + 100, "hms", 1, true);
        l.wrote(T + 150, &m(&[("hms", 2100)]));
        l.replay_saved(T + 400);
        l.replay_saved(T + 600);
        l.own_battle(T + 405, "hms", 9, true);
        assert_eq!(l.shown["hms"].mr, 2100);
    }

    #[test]
    fn the_matching_record_replaces_its_saves_rough_guess() {
        let mut l = Ledger::default();
        l.wrote(-300, &m(&[("hms", 2092)]));
        l.own_battle(5, "hms", 5, true);
        l.wrote(91, &m(&[("hms", 2080)]));
        l.replay_saved(217);
        assert!(l.deltas(&m(&[("hms", 2080)])).is_empty(), "the rough guess drops it");
        l.own_battle(222, "hms", 5, true);
        assert_eq!(l.deltas(&m(&[("hms", 2080)])), vec![("hms".into(), 2092, 2080)], "restored by the record");
    }

    #[test]
    fn opening_a_replay_does_not_eat_a_pending_delta() {
        let mut l = Ledger::default();
        l.wrote(T, &m(&[("hms", 2092)]));
        l.replay_saved(T + 200);
        l.own_battle(T + 205, "hms", 4, true);
        l.wrote(T + 290, &m(&[("hms", 2080)]));
        assert!(!l.settle(T + 500), "match 1's save was confirmed");
        l.replay_saved(T + 1500);
        assert!(l.deltas(&m(&[("hms", 2080)])).is_empty(), "provisionally consumed");
        assert!(!l.settle(T + 1500 + CONFIRM), "not yet");
        assert!(l.settle(T + 1500 + CONFIRM + 1));
        assert_eq!(l.deltas(&m(&[("hms", 2080)])), vec![("hms".into(), 2092, 2080)], "back after CONFIRM");
    }

    #[test]
    fn bug_2026_09_13_lili_back_to_back() {
        let mut l = Ledger::default();
        l.wrote(-600, &m(&[("hms", 2092)]));
        l.replay_saved(0);
        l.settle(2);
        l.own_battle(5, "hms", 4, true);
        l.wrote(91, &m(&[("hms", 2080)]));
        l.settle(120);
        l.replay_saved(217);
        l.settle(219);
        let owed = vec![("hms".to_string(), 2092, 2080)];
        assert_eq!(l.deltas(&m(&[("hms", 2080)])), owed, "match 3 opens 2092 MR (-12)");
        l.own_battle(222, "hms", 3, true);
        l.settle(380);
        assert_eq!(l.deltas(&m(&[("hms", 2080)])), owed, "still owed after the record");
        l.replay_saved(420);
        assert!(l.deltas(&m(&[("hms", 2080)])).is_empty(), "match 4 must not repeat -12");
        l.own_battle(425, "hms", 3, true);
        assert!(l.deltas(&m(&[("hms", 2080)])).is_empty(), "nor after match 3's record");
        l.settle(700);
        assert!(l.deltas(&m(&[("hms", 2080)])).is_empty(), "nor after settling");
    }

    #[test]
    fn back_to_back_matches_show_the_result_at_the_match_after_next() {
        let mut l = Ledger::default();
        l.wrote(-300, &m(&[("hms", 2092)]));
        l.replay_saved(0);
        l.own_battle(5, "hms", 5, true);
        l.wrote(120, &m(&[("hms", 2080)]));
        assert_eq!(l.deltas(&m(&[("hms", 2080)])), vec![("hms".into(), 2092, 2080)]);
        l.replay_saved(217);
        assert_eq!(l.deltas(&m(&[("hms", 2080)])), vec![("hms".into(), 2092, 2080)], "not dropped");
        l.own_battle(220, "hms", 4, true);
        assert_eq!(l.deltas(&m(&[("hms", 2080)])), vec![("hms".into(), 2092, 2080)], "match 3 opens 2092 MR (-12)");
        l.wrote(350, &m(&[("hms", 2061)]));
        l.replay_saved(346 + 60);
        assert_eq!(l.deltas(&m(&[("hms", 2061)])), vec![("hms".into(), 2080, 2061)], "then (-19), never -12 again");
    }

    #[test]
    fn the_offset_is_learned_from_the_nearest_replay_save() {
        let mut l = Ledger::default();
        l.wrote(T, &m(&[("hms", 2092)]));
        l.replay_saved(T + 1000);
        l.own_battle(T + 1012, "hms", 3, true);
        assert_eq!(l.offset, Some(12));
    }

    #[test]
    fn it_saves_and_loads_without_the_log() {
        let mut l = Ledger::default();
        l.wrote(T, &m(&[("hms", 2092)]));
        l.own_battle(T + 205, "hms", 4, true);
        let back: Ledger = serde_json::from_str(&serde_json::to_string(&l).unwrap()).unwrap();
        assert_eq!(back.shown, l.shown);
        assert_eq!(back.last_code, l.last_code);
        assert!(back.log.is_empty());
    }
}
