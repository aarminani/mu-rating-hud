use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::glicko;
use crate::replays::{Record, GOD_OF_DESTRUCTION};

pub const HORIZON: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub phi: f64,
    pub at: i64,
    pub seen: u32,
}

impl Entry {
    pub fn rd(&self) -> f64 {
        glicko::rd_of(self.phi)
    }

    pub fn settled(&self) -> bool {
        self.seen >= SETTLED_AFTER
    }
}

pub const SETTLED_AFTER: u32 = 1;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RdTable {
    entries: HashMap<String, Entry>,
}

fn key(player: &str, chara: i32) -> String {
    format!("{player}#{chara}")
}

impl RdTable {
    pub fn phi(&self, player: &str, chara: i32) -> f64 {
        self.entries
            .get(&key(player, chara))
            .filter(|e| e.settled())
            .map_or(glicko::phi_of(glicko::PRIOR_RD), |e| e.phi)
    }

    pub fn get(&self, player: &str, chara: i32) -> Option<&Entry> {
        self.entries.get(&key(player, chara))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn seed(&mut self, player: &str, chara: i32, rd: f64, at: i64) {
        let e = self.entries.entry(key(player, chara)).or_insert(Entry {
            phi: glicko::phi_of(rd),
            at,
            seen: SETTLED_AFTER,
        });
        if at >= e.at {
            e.phi = glicko::phi_of(rd);
            e.at = at;
            e.seen = e.seen.max(SETTLED_AFTER);
        }
    }

    pub fn observe(&mut self, r: &Record) {
        let (Some(p1), Some(p2)) = (r.p1_polaris_id.as_deref(), r.p2_polaris_id.as_deref()) else {
            return;
        };
        let (Some(c1), Some(c2)) = (r.p1_chara_id, r.p2_chara_id) else {
            return;
        };
        let (Some(m1), Some(m2)) = (r.p1_rating_before, r.p2_rating_before) else {
            return;
        };
        if r.p1_rank.unwrap_or(0) < GOD_OF_DESTRUCTION || r.p2_rank.unwrap_or(0) < GOD_OF_DESTRUCTION
        {
            return;
        }
        let (k1, k2) = (key(p1, c1), key(p2, c2));
        let phi1 = self.phi_raw(&k1);
        let phi2 = self.phi_raw(&k2);
        let (mu1, mu2) = (glicko::mu_of(m1), glicko::mu_of(m2));
        self.put(k1, glicko::propagate(phi1, phi2, mu1, mu2), r.battle_at);
        self.put(k2, glicko::propagate(phi2, phi1, mu2, mu1), r.battle_at);
    }

    fn phi_raw(&self, k: &str) -> f64 {
        self.entries
            .get(k)
            .map_or(glicko::phi_of(glicko::PRIOR_RD), |e| e.phi)
    }

    fn put(&mut self, k: String, phi: f64, at: i64) {
        let e = self.entries.entry(k).or_insert(Entry { phi, at, seen: 0 });
        if at >= e.at || e.seen == 0 {
            e.phi = phi;
            e.at = at;
            e.seen = e.seen.saturating_add(1);
        }
    }

    pub fn median_phi(&self) -> Option<f64> {
        let mut v: Vec<f64> = self
            .entries
            .values()
            .filter(|e| e.settled())
            .map(|e| e.phi)
            .collect();
        if v.is_empty() {
            return None;
        }
        v.sort_by(|a, b| a.partial_cmp(b).expect("phi is never NaN"));
        Some(v[v.len() / 2])
    }

    pub fn evict(&mut self, now: i64) {
        let floor = now - HORIZON;
        self.entries.retain(|_, e| e.at >= floor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn battle(p1: &str, c1: i32, m1: i32, p2: &str, c2: i32, m2: i32, at: i64) -> Record {
        Record {
            battle_at: at,
            battle_id: Some(format!("{at}")),
            p1_polaris_id: Some(p1.into()),
            p2_polaris_id: Some(p2.into()),
            p1_chara_id: Some(c1),
            p2_chara_id: Some(c2),
            p1_rating_before: Some(m1),
            p2_rating_before: Some(m2),
            p1_rating_change: Some(10),
            p2_rating_change: Some(-10),
            p1_rank: Some(29),
            p2_rank: Some(29),
            winner: Some(1),
            ..Default::default()
        }
    }

    #[test]
    fn propagation_settles_below_the_prior_for_a_regular_player() {
        let mut t = RdTable::default();
        for i in 0..60 {
            t.observe(&battle("me", 14, 2065, "them", 6, 2065, 1_000 + i));
        }
        let rd = t.get("me", 14).unwrap().rd();
        assert!(
            (55.0..65.0).contains(&rd),
            "expected a settled regular near the low 60s, got {rd:.1}"
        );
    }

    #[test]
    fn an_opponents_rd_is_the_same_whether_they_win_or_lose() {
        let mut won = RdTable::default();
        let mut lost = RdTable::default();
        for i in 0..30 {
            let mut w = battle("them", 6, 2100, "me", 14, 2065, 1_000 + i);
            w.winner = Some(1);
            let mut l = battle("them", 6, 2100, "me", 14, 2065, 1_000 + i);
            l.winner = Some(2);
            won.observe(&w);
            lost.observe(&l);
        }
        assert_eq!(
            won.get("them", 6).map(|e| e.phi),
            lost.get("them", 6).map(|e| e.phi),
            "the outcome moved a deviation - propagation must be result-blind"
        );
    }

    #[test]
    fn an_unseen_player_is_the_prior_and_one_battle_beats_it() {
        let mut t = RdTable::default();
        assert_eq!(
            t.phi("nobody", 14),
            glicko::phi_of(glicko::PRIOR_RD),
            "a player we have never seen must read as the prior"
        );
        t.observe(&battle("me", 14, 2065, "them", 6, 2200, 1_000));
        assert!(t.get("me", 14).unwrap().settled(), "one battle is enough to publish");
        assert_ne!(
            t.phi("me", 14),
            glicko::phi_of(glicko::PRIOR_RD),
            "one battle must actually move the published value off the prior"
        );
    }

    #[test]
    fn a_page_seed_is_trusted_immediately() {
        let mut t = RdTable::default();
        t.seed("me", 14, 63.0, 5_000);
        assert!(t.get("me", 14).unwrap().settled());
        assert!((glicko::rd_of(t.phi("me", 14)) - 63.0).abs() < 1e-9);
    }

    #[test]
    fn an_older_battle_does_not_overwrite_a_newer_seed() {
        let mut t = RdTable::default();
        t.seed("me", 14, 63.0, 5_000);
        let before = t.phi("me", 14);
        t.observe(&battle("me", 14, 2065, "them", 6, 2065, 4_000));
        assert_eq!(t.phi("me", 14), before, "a stale battle moved a fresh page seed");
    }

    #[test]
    fn below_god_of_destruction_is_not_tracked() {
        let mut t = RdTable::default();
        let mut r = battle("me", 14, 2065, "them", 6, 2065, 1_000);
        r.p2_rank = Some(28);
        t.observe(&r);
        assert!(t.is_empty(), "the badge never renders there, so we do not model it");
    }

    #[test]
    fn eviction_drops_only_the_stale() {
        let mut t = RdTable::default();
        t.observe(&battle("edge", 14, 2065, "a", 6, 2065, 1_000));
        t.observe(&battle("old", 14, 2065, "b", 6, 2065, 999));
        t.observe(&battle("new", 14, 2065, "c", 6, 2065, 1_000 + HORIZON));
        t.evict(1_000 + HORIZON);
        assert!(t.get("old", 14).is_none(), "past the horizon, should be gone");
        assert!(t.get("edge", 14).is_some(), "exactly at the horizon is still inside it");
        assert!(t.get("new", 14).is_some());
    }

    #[test]
    fn the_table_round_trips_through_json() {
        let mut t = RdTable::default();
        for i in 0..30 {
            t.observe(&battle("me", 14, 2065, "them", 6, 2070, 1_000 + i));
        }
        let json = serde_json::to_string(&t).unwrap();
        let back: RdTable = serde_json::from_str(&json).unwrap();
        assert_eq!(back.get("me", 14), t.get("me", 14));
        assert_eq!(back.len(), t.len());
    }
}
