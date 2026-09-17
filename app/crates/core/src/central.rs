use std::io::Read;
use std::time::Duration;

use serde::Deserialize;

use crate::replays::{Record, ReplayError};

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Latest {
    pub v: u32,
    pub minute: i64,
    pub newest_at: i64,
    #[serde(default)]
    pub minutes: Vec<i64>,
    #[serde(default)]
    pub hours: Vec<i64>,
}

pub enum Fetched {
    NotModified,
    Body { text: String, etag: Option<String> },
    Missing,
}

pub fn get(url: &str, user_agent: &str, etag: Option<&str>) -> Result<Fetched, ReplayError> {
    get_dated(url, user_agent, etag).map(|(f, _)| f)
}

pub fn get_dated(url: &str, user_agent: &str, etag: Option<&str>) -> Result<(Fetched, Option<i64>), ReplayError> {
    let agent = ureq::builder().user_agent(user_agent).timeout(Duration::from_secs(30)).build();
    let mut req = agent.get(url).set("Accept-Encoding", "gzip");
    if let Some(tag) = etag {
        req = req.set("If-None-Match", tag);
    }
    let date = |resp: &ureq::Response| resp.header("Date").and_then(http_date);
    match req.call() {
        Ok(resp) if resp.status() == 304 => Ok((Fetched::NotModified, date(&resp))),
        Ok(resp) => {
            let at = date(&resp);
            let etag = resp.header("ETag").map(str::to_string);
            let mut text = String::new();
            resp.into_reader().read_to_string(&mut text).map_err(|e| ReplayError::Net(e.to_string()))?;
            Ok((Fetched::Body { text, etag }, at))
        }
        Err(ureq::Error::Status(304, resp)) => Ok((Fetched::NotModified, date(&resp))),
        Err(ureq::Error::Status(404, resp)) => Ok((Fetched::Missing, date(&resp))),
        Err(e) => Err(ReplayError::Net(e.to_string())),
    }
}

pub fn http_date(s: &str) -> Option<i64> {
    let mut parts = s.split_whitespace().skip(1);
    let day: i64 = parts.next()?.parse().ok()?;
    let month = match parts.next()? {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year: i64 = parts.next()?.parse().ok()?;
    let mut hms = parts.next()?.split(':').map(|n| n.parse::<i64>().ok());
    let (h, m, sec) = (hms.next()??, hms.next()??, hms.next()??);
    Some(days_from_civil(year, month, day) * 86_400 + h * 3_600 + m * 60 + sec)
}

pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn parse_latest(text: &str) -> Result<Latest, ReplayError> {
    let l: Latest = serde_json::from_str(text).map_err(|e| ReplayError::Body(e.to_string()))?;
    if l.v != 1 {
        return Err(ReplayError::Body(format!("feed version {} not understood", l.v)));
    }
    Ok(l)
}

pub fn parse_records(text: &str) -> Result<Vec<Record>, ReplayError> {
    serde_json::from_str(text).map_err(|e| ReplayError::Body(e.to_string()))
}

pub fn get_records(url: &str, user_agent: &str, keep: impl FnMut(&Record) -> bool) -> Result<Option<Vec<Record>>, ReplayError> {
    let agent = ureq::builder().user_agent(user_agent).timeout(Duration::from_secs(30)).build();
    match agent.get(url).set("Accept-Encoding", "gzip").call() {
        Ok(resp) => read_records(std::io::BufReader::new(resp.into_reader()), keep).map(Some),
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(e) => Err(ReplayError::Net(e.to_string())),
    }
}

pub fn read_records(reader: impl Read, mut keep: impl FnMut(&Record) -> bool) -> Result<Vec<Record>, ReplayError> {
    use serde::de::{Deserializer as _, SeqAccess, Visitor};

    struct Kept<'a, F> {
        keep: &'a mut F,
        out: &'a mut Vec<Record>,
    }
    impl<'de, F: FnMut(&Record) -> bool> Visitor<'de> for Kept<'_, F> {
        type Value = ();
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an array of replay records")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
            while let Some(r) = seq.next_element::<Record>()? {
                if (self.keep)(&r) {
                    self.out.push(r);
                }
            }
            Ok(())
        }
    }

    let mut out = Vec::new();
    let mut de = serde_json::Deserializer::from_reader(reader);
    de.deserialize_seq(Kept { keep: &mut keep, out: &mut out }).map_err(|e| ReplayError::Body(e.to_string()))?;
    de.end().map_err(|e| ReplayError::Body(e.to_string()))?;
    Ok(out)
}

pub fn latest_url(base: &str) -> String {
    format!("{}/feed/latest.json", base.trim_end_matches('/'))
}

pub fn minute_url(base: &str, minute: i64) -> String {
    format!("{}/feed/min/{minute}.json", base.trim_end_matches('/'))
}

pub fn hour_url(base: &str, hour: i64) -> String {
    format!("{}/feed/hour/{hour}.json", base.trim_end_matches('/'))
}

pub fn hours_wanted(latest: &Latest, now: i64, horizon: i64, own_times: &[i64], own_horizon: i64) -> Vec<i64> {
    let mut out: Vec<i64> = latest
        .hours
        .iter()
        .copied()
        .filter(|h| (h + 1) * 3600 > now - horizon)
        .collect();
    for t in own_times {
        let h = t.div_euclid(3600);
        if *t > now - own_horizon && latest.hours.contains(&h) && !out.contains(&h) {
            out.push(h);
        }
    }
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_servers_date_header() {
        assert_eq!(http_date("Thu, 17 Sep 2026 14:39:59 GMT"), Some(1_789_655_999));
        assert_eq!(http_date("Thu, 01 Jan 1970 00:00:00 GMT"), Some(0));
        assert_eq!(http_date("Tue, 29 Feb 2028 12:00:00 GMT"), Some(1_835_438_400), "a leap day");
        assert_eq!(http_date("not a date"), None);
        assert_eq!(http_date("Thu, 17 Foo 2026 14:39:59 GMT"), None);
    }

    const LATEST: &str = r#"{"v":1,"minute":29825169,"newest_at":1789510103,"newest_ids":["FEAD"],"minutes":[29825167,29825169],"hours":[497084,497085,497086]}"#;

    #[test]
    fn latest_parses_and_ignores_the_cursor_fields() {
        let l = parse_latest(LATEST).unwrap();
        assert_eq!(l.minute, 29825169);
        assert_eq!(l.minutes, vec![29825167, 29825169]);
        assert_eq!(l.hours.len(), 3);
        assert!(parse_latest(r#"{"v":2,"minute":1,"newest_at":1}"#).is_err());
    }

    #[test]
    fn minute_files_read_as_records() {
        let text = r#"[{"battle_at":1789510102,"battle_id":"C7","winner":2,"p1_polaris_id":"483r85h7RR2Q","p1_chara_id":40,"p1_name":"KarateKa","p1_power":348346,"p1_rank":29,"p1_rounds":1,"p2_polaris_id":"5hgbTtBtRDdh","p2_chara_id":14,"p2_name":"Vengador117","p2_rank":30,"p2_rating_before":2001,"p2_rating_change":7,"p2_rounds":3}]"#;
        let r = parse_records(text).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].p1_rating_before, None, "null fields are dropped by the Worker and read as None");
        assert_eq!(r[0].p2_rating_change, Some(7));
        assert_eq!(r[0].winner, Some(2));
    }

    #[test]
    fn streamed_records_match_the_whole_string_parse_and_honour_the_filter() {
        let text = r#"[
            {"battle_at":3,"battle_id":"A","battle_type":2,"p1_rank":29,"p2_rank":12,"p1_rating_before":null,"stage_id":500},
            {"battle_at":2,"battle_id":"B","p1_rank":10,"p2_rank":11},
            {"battle_at":1,"battle_id":"C","p1_rank":30,"p2_rank":31,"p1_name":"ютуб юпюп"}
        ]"#;
        let all = read_records(text.as_bytes(), |_| true).unwrap();
        let whole = parse_records(text).unwrap();
        assert_eq!(all.len(), whole.len());
        assert!(all.iter().zip(&whole).all(|(a, b)| a.battle_id == b.battle_id && a.p1_name == b.p1_name));
        let god = read_records(text.as_bytes(), |r| r.p1_rank.unwrap_or(0) >= 29 || r.p2_rank.unwrap_or(0) >= 29).unwrap();
        assert_eq!(god.iter().map(|r| r.battle_id.as_deref().unwrap()).collect::<Vec<_>>(), vec!["A", "C"]);
        assert!(read_records(&b"[{\"battle_at\":1}"[..], |_| true).is_err(), "a truncated file must be an error");
        assert!(read_records(&b"[] trailing"[..], |_| true).is_err(), "junk after the array must be an error");
    }

    #[test]
    fn cold_start_takes_the_horizon_and_own_hours() {
        let l = Latest { v: 1, minute: 0, newest_at: 0, minutes: vec![], hours: (100..=125).collect() };
        let now = 126 * 3600 + 600;
        let got = hours_wanted(&l, now, 8 * 3600, &[104 * 3600 + 10, 90 * 3600], 24 * 3600);
        assert_eq!(got, vec![104, 118, 119, 120, 121, 122, 123, 124, 125]);
    }
}
