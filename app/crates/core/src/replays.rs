use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::time::Duration;

use serde::Deserialize;

pub const WINDOW_SECS: i64 = 700;

pub const GOD_OF_DESTRUCTION: i32 = 29;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Record {
    pub battle_at: i64,
    pub battle_id: Option<String>,
    pub p1_polaris_id: Option<String>,
    pub p2_polaris_id: Option<String>,
    pub p1_chara_id: Option<i32>,
    pub p2_chara_id: Option<i32>,
    pub p1_name: Option<String>,
    pub p1_power: Option<i64>,
    pub p1_rank: Option<i32>,
    pub p1_rating_before: Option<i32>,
    pub p1_rating_change: Option<i32>,
    pub p2_name: Option<String>,
    pub p2_power: Option<i64>,
    pub p2_rank: Option<i32>,
    pub p2_rating_before: Option<i32>,
    pub p2_rating_change: Option<i32>,
    pub winner: Option<i32>,
    pub p1_rounds: Option<i32>,
    pub p2_rounds: Option<i32>,
    pub p1_region_id: Option<i32>,
    pub p2_region_id: Option<i32>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("network: {0}")]
    Net(String),
    #[error("unexpected response: {0}")]
    Body(String),
}

pub fn fetch_window(base: &str, before: Option<i64>, user_agent: &str) -> Result<Vec<Record>, ReplayError> {
    let mut url = format!("{}/api/replays", base.trim_end_matches('/'));
    if let Some(b) = before {
        url.push_str(&format!("?before={b}"));
    }
    let resp = ureq::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(60))
        .build()
        .get(&url)
        .set("Accept-Encoding", "gzip")
        .call()
        .map_err(|e| ReplayError::Net(e.to_string()))?;
    let mut text = String::new();
    resp.into_reader()
        .read_to_string(&mut text)
        .map_err(|e| ReplayError::Net(e.to_string()))?;
    serde_json::from_str(&text).map_err(|e| ReplayError::Body(e.to_string()))
}

pub fn key(name: &str, power: i64) -> String {
    format!("{name}|{power}")
}

#[derive(Debug, Default)]
pub struct Table {
    rows: BTreeMap<String, (i32, Option<i32>)>,
    ambiguous: BTreeSet<String>,
    pub sides_seen: usize,
    pub below_gate: usize,
    pub unrated: usize,
}

impl Table {
    pub fn add_records(&mut self, records: &[Record]) {
        for r in records {
            self.add_record(r);
        }
    }

    pub fn add_record(&mut self, r: &Record) {
        self.add_side(r.p1_name.as_deref(), r.p1_power, r.p1_rank, r.p1_rating_before, r.p1_rating_change);
        self.add_side(r.p2_name.as_deref(), r.p2_power, r.p2_rank, r.p2_rating_before, r.p2_rating_change);
    }

    fn add_side(&mut self, name: Option<&str>, power: Option<i64>, rank: Option<i32>,
                before: Option<i32>, change: Option<i32>) {
        self.sides_seen += 1;
        let (Some(name), Some(power)) = (name, power) else { return };
        if name.is_empty() {
            return;
        }
        if rank.unwrap_or(0) < GOD_OF_DESTRUCTION {
            self.below_gate += 1;
            return;
        }
        let Some(before) = before else {
            self.unrated += 1;
            return;
        };
        let k = key(name, power);
        if self.ambiguous.contains(&k) {
            return;
        }
        match self.rows.get(&k) {
            None => {
                self.rows.insert(k, (before, change));
            }
            Some(&existing) if existing == (before, change) => {}
            Some(_) => {
                self.rows.remove(&k);
                self.ambiguous.insert(k);
            }
        }
    }

    pub fn get(&self, k: &str) -> Option<(i32, Option<i32>)> {
        self.rows.get(k).copied()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn ambiguous_count(&self) -> usize {
        self.ambiguous.len()
    }

    pub fn mr_entries(&self) -> Vec<(String, String)> {
        self.rows.iter().map(|(k, (b, _))| (k.clone(), format!("{b} MR"))).collect()
    }

    pub fn delta_entries(&self) -> Vec<(String, i32)> {
        self.rows
            .iter()
            .filter_map(|(k, (_, c))| c.map(|c| (k.clone(), c)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(t: i64, a: (&str, i64, i32, Option<i32>, Option<i32>), b: (&str, i64, i32, Option<i32>, Option<i32>)) -> Record {
        Record {
            battle_at: t,
            p1_name: Some(a.0.into()), p1_power: Some(a.1), p1_rank: Some(a.2),
            p1_rating_before: a.3, p1_rating_change: a.4,
            p2_name: Some(b.0.into()), p2_power: Some(b.1), p2_rank: Some(b.2),
            p2_rating_before: b.3, p2_rating_change: b.4,
            ..Default::default()
        }
    }

    #[test]
    fn the_three_verified_battles_each_get_their_own_row() {
        let mut t = Table::default();
        t.add_records(&[
            rec(1789211410, ("MRmani", 407329, 33, Some(2119), Some(-17)), ("TidalRonin", 368338, 31, Some(1962), Some(16))),
            rec(1789211635, ("MRmani", 407330, 33, Some(2103), Some(7)), ("TidalRonin", 368339, 31, Some(1978), Some(-8))),
            rec(1789211804, ("MRmani", 407811, 33, Some(2111), Some(7)), ("TidalRonin", 368100, 31, Some(1971), Some(-8))),
        ]);
        assert_eq!(t.get("MRmani|407329"), Some((2119, Some(-17))));
        assert_eq!(t.get("MRmani|407330"), Some((2103, Some(7))));
        assert_eq!(t.get("MRmani|407811"), Some((2111, Some(7))));
        assert_eq!(t.get("TidalRonin|368100"), Some((1971, Some(-8))));
        assert_eq!(t.len(), 6);
        assert_eq!(t.ambiguous_count(), 0);
    }

    #[test]
    fn below_god_of_destruction_and_unrated_sides_are_left_out() {
        let mut t = Table::default();
        t.add_records(&[rec(1, ("Low", 300000, 28, Some(1800), Some(5)), ("NoMu", 400000, 30, None, None))]);
        assert!(t.is_empty());
        assert_eq!(t.below_gate, 1);
        assert_eq!(t.unrated, 1);
    }

    #[test]
    fn the_same_battle_twice_is_not_a_collision() {
        let mut t = Table::default();
        let r = rec(5, ("A", 500000, 30, Some(2000), Some(4)), ("B", 510000, 30, Some(2010), Some(-4)));
        t.add_records(&[r.clone(), r]);
        assert_eq!(t.len(), 2);
        assert_eq!(t.ambiguous_count(), 0);
    }

    #[test]
    fn two_ratings_for_one_key_drops_the_key_rather_than_guessing() {
        let mut t = Table::default();
        t.add_records(&[
            rec(5, ("A", 500000, 30, Some(2000), Some(4)), ("B", 510000, 30, Some(2010), Some(-4))),
            rec(9, ("A", 500000, 30, Some(2040), Some(1)), ("C", 520000, 30, Some(1990), Some(-1))),
            rec(12, ("A", 500000, 30, Some(2077), Some(2)), ("D", 530000, 30, Some(1980), Some(-2))),
        ]);
        assert_eq!(t.get("A|500000"), None, "never guess between two ratings");
        assert_eq!(t.ambiguous_count(), 1);
        assert!(t.get("B|510000").is_some() && t.get("D|530000").is_some());
    }

    #[test]
    fn entries_are_the_display_text_and_the_signed_change() {
        let mut t = Table::default();
        t.add_records(&[rec(1, ("MRmani", 407811, 33, Some(2111), Some(7)), ("TidalRonin", 368100, 31, Some(1971), None))]);
        assert!(t.mr_entries().contains(&("MRmani|407811".into(), "2111 MR".into())));
        assert_eq!(t.delta_entries(), vec![("MRmani|407811".to_string(), 7)]);
    }

    #[test]
    fn parses_a_real_record_shape() {
        let json = r#"[{"battle_at":1789211804,"battle_id":"x","battle_type":2,"game_version":30202,
            "p1_area_id":3,"p1_chara_id":14,"p1_lang":"en","p1_name":"MRmani","p1_polaris_id":"xxxxxxxxxxxx",
            "p1_power":407811,"p1_rank":33,"p1_rating_before":2111,"p1_rating_change":7,"p1_region_id":3,
            "p1_rounds":3,"p1_user_id":1,"p2_area_id":3,"p2_chara_id":1,"p2_lang":"en","p2_name":"TidalRonin",
            "p2_polaris_id":"x","p2_power":368100,"p2_rank":31,"p2_rating_before":1971,"p2_rating_change":-8,
            "p2_region_id":3,"p2_rounds":1,"p2_user_id":2,"stage_id":100,"winner":1}]"#;
        let recs: Vec<Record> = serde_json::from_str(json).unwrap();
        let mut t = Table::default();
        t.add_records(&recs);
        assert_eq!(t.get("MRmani|407811"), Some((2111, Some(7))));
        assert_eq!(recs[0].battle_id.as_deref(), Some("x"));
        assert_eq!(recs[0].p1_polaris_id.as_deref(), Some("xxxxxxxxxxxx"));
    }
}
