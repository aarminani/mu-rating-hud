use serde::Serialize;

use crate::characters;
use crate::feed::OwnMatch;
use crate::player::HistoryRow;
use crate::table;

#[derive(Debug, Clone, PartialEq)]
pub struct OwnResult {
    pub battle_at: i64,
    pub character: Option<String>,
    pub opponent_name: String,
    pub opponent_character: Option<String>,
    pub mu_before: i32,
    pub change: i32,
    pub won: bool,
    pub rounds_own: i32,
    pub rounds_opp: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub at: i64,
    pub character: String,
    pub after: i32,
    pub change: i32,
    pub won: bool,
    pub opponent_name: String,
    pub opponent_character: Option<String>,
}

pub const TREND_LEN: usize = 20;
pub const LAST_LEN: usize = 8;

pub fn points(own_id: &str, results: &[OwnResult], history: &[HistoryRow]) -> Vec<Point> {
    let own_id = table::tekken_id(own_id);
    let ours = |row: &HistoryRow| our_side(&own_id, row);
    let cell_is_after = cell_is_after(&own_id, results, history);

    let mut out: Vec<Point> = Vec::new();
    for r in results {
        let row_char = history
            .iter()
            .find(|h| h.battle_at == r.battle_at)
            .and_then(|h| ours(h).map(|o| (o.character, o.opponent_character)));
        let Some(character) = r.character.clone().or_else(|| row_char.as_ref().map(|c| c.0.clone())) else {
            continue;
        };
        out.push(Point {
            at: r.battle_at,
            character,
            after: r.mu_before + r.change,
            change: r.change,
            won: r.won,
            opponent_name: r.opponent_name.clone(),
            opponent_character: r.opponent_character.clone().or_else(|| row_char.map(|c| c.1)),
        });
    }
    for row in history {
        if results.iter().any(|r| r.battle_at == row.battle_at) {
            continue;
        }
        let Some(o) = ours(row) else { continue };
        out.push(Point {
            at: row.battle_at,
            character: o.character,
            after: if cell_is_after { o.cell } else { o.cell + o.change },
            change: o.change,
            won: o.change > 0,
            opponent_name: o.opponent_name,
            opponent_character: Some(o.opponent_character),
        });
    }
    out.sort_by_key(|p| p.at);
    out.dedup_by_key(|p| p.at);
    out
}

struct OurSide {
    character: String,
    cell: i32,
    change: i32,
    opponent_id: String,
    opponent_name: String,
    opponent_character: String,
    opponent_cell: Option<i32>,
    opponent_change: Option<i32>,
}

fn our_side(own_id: &str, row: &HistoryRow) -> Option<OurSide> {
    let mine_left = row.left_id == own_id;
    if !mine_left && row.right_id != own_id {
        return None;
    }
    Some(if mine_left {
        OurSide {
            character: row.left_char.clone(),
            cell: row.left_rating?,
            change: row.left_change.unwrap_or(0),
            opponent_id: row.right_id.clone(),
            opponent_name: row.right_name.clone(),
            opponent_character: row.right_char.clone(),
            opponent_cell: row.right_rating,
            opponent_change: row.right_change,
        }
    } else {
        OurSide {
            character: row.right_char.clone(),
            cell: row.right_rating?,
            change: row.right_change.unwrap_or(0),
            opponent_id: row.left_id.clone(),
            opponent_name: row.left_name.clone(),
            opponent_character: row.left_char.clone(),
            opponent_cell: row.left_rating,
            opponent_change: row.left_change,
        }
    })
}

fn cell_is_after(own_id: &str, results: &[OwnResult], history: &[HistoryRow]) -> bool {
    let (mut before_votes, mut after_votes) = (0, 0);
    for row in history {
        let Some(o) = our_side(own_id, row) else { continue };
        if o.change == 0 {
            continue;
        }
        if let Some(r) = results.iter().find(|r| r.battle_at == row.battle_at) {
            if o.cell == r.mu_before {
                before_votes += 1;
            } else if o.cell == r.mu_before + r.change {
                after_votes += 1;
            }
        }
    }
    after_votes > before_votes
}

pub fn history_matches(own_id: &str, matches: &[OwnMatch], history: &[HistoryRow]) -> Vec<OwnMatch> {
    let own_id = table::tekken_id(own_id);
    let results: Vec<OwnResult> = matches.iter().map(OwnMatch::to_result).collect();
    let cell_after = cell_is_after(&own_id, &results, history);
    let mut out = Vec::new();
    for row in history {
        if matches.iter().any(|m| m.battle_at == row.battle_at) {
            continue;
        }
        let Some(o) = our_side(&own_id, row) else { continue };
        let mu_before = if cell_after { o.cell - o.change } else { o.cell };
        out.push(OwnMatch {
            battle_at: row.battle_at,
            battle_id: None,
            chara_id: None,
            character: Some(o.character),
            opponent_id: o.opponent_id,
            opponent_name: o.opponent_name,
            opponent_chara_id: None,
            opponent_character: Some(o.opponent_character),
            mu_before,
            change: Some(o.change),
            opponent_mu_before: match (cell_after, o.opponent_cell, o.opponent_change) {
                (false, cell, _) => cell,
                (true, Some(cell), Some(change)) => Some(cell - change),
                (true, _, _) => None,
            },
            won: o.change > 0,
            rounds_own: None,
            rounds_opp: None,
            rank: None,
            opponent_rank: None,
            region: None,
            opponent_region: None,
            power: None,
        });
    }
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LastBattle {
    pub at: i64,
    pub opponent_name: String,
    pub opponent_character: Option<String>,
    pub change: i32,
    pub won: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Trend {
    pub character: String,
    pub short: String,
    pub points: Vec<i32>,
    pub change_over: i32,
    pub mu: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Goal {
    pub character: String,
    pub mu: i32,
    pub target: i32,
    pub to_go: i32,
    pub progress: f64,
    pub wins_est: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionSummary {
    pub since: i64,
    pub net: i32,
    pub wins: u32,
    pub losses: u32,
    pub last8: Vec<bool>,
    pub streak: i32,
    pub last: Option<LastBattle>,
    pub trend: Option<Trend>,
    pub goal: Option<Goal>,
}

pub fn summarize(points: &[Point], since: i64, fallback: Option<(&str, i32)>) -> SessionSummary {
    let session: Vec<&Point> = points.iter().filter(|p| p.at >= since).collect();
    let net = session.iter().map(|p| p.change).sum();
    let wins = session.iter().filter(|p| p.won).count() as u32;
    let losses = session.len() as u32 - wins;
    let last8 = session.iter().rev().take(LAST_LEN).rev().map(|p| p.won).collect();
    let mut streak = 0i32;
    if let Some(first) = session.last() {
        let won = first.won;
        let n = session.iter().rev().take_while(|p| p.won == won).count() as i32;
        streak = if won { n } else { -n };
    }
    let last = points.last().map(|p| LastBattle {
        at: p.at,
        opponent_name: p.opponent_name.clone(),
        opponent_character: p.opponent_character.clone(),
        change: p.change,
        won: p.won,
    });

    let key = points.last().map(|p| table::char_key(&p.character));
    let on_char: Vec<&Point> = match &key {
        Some(k) => points.iter().filter(|p| &table::char_key(&p.character) == k).collect(),
        None => Vec::new(),
    };
    let trend = on_char.last().map(|latest| {
        let recent: Vec<&&Point> = on_char.iter().rev().take(TREND_LEN).collect::<Vec<_>>().into_iter().rev().collect();
        Trend {
            character: characters::full_name(&latest.character),
            short: latest.character.clone(),
            points: recent.iter().map(|p| p.after).collect(),
            change_over: recent.iter().map(|p| p.change).sum(),
            mu: latest.after,
        }
    });

    let current = on_char
        .last()
        .map(|p| (p.character.clone(), p.after))
        .or_else(|| fallback.map(|(c, mu)| (c.to_string(), mu)));
    let goal = current.map(|(character, mu)| {
        let target = (mu.div_euclid(100) + 1) * 100;
        let to_go = target - mu;
        let gains: Vec<i32> = on_char.iter().rev().filter(|p| p.won && p.change > 0).take(TREND_LEN).map(|p| p.change).collect();
        let wins_est = if gains.is_empty() {
            None
        } else {
            let mean = gains.iter().sum::<i32>() as f64 / gains.len() as f64;
            Some((to_go as f64 / mean).ceil().max(1.0) as u32)
        };
        Goal {
            character: characters::full_name(&character),
            mu,
            target,
            to_go,
            progress: (mu - (target - 100)) as f64 / 100.0,
            wins_est,
        }
    });

    SessionSummary { since, net, wins, losses, last8, streak, last, trend, goal }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "aaaaaaaaaaaa";

    fn result(at: i64, ch: &str, before: i32, change: i32) -> OwnResult {
        OwnResult {
            battle_at: at,
            character: Some(ch.into()),
            opponent_name: "Opp".into(),
            opponent_character: Some("Jin".into()),
            mu_before: before,
            change,
            won: change > 0,
            rounds_own: 3,
            rounds_opp: 1,
        }
    }

    fn row(at: i64, ch: &str, cell: i32, change: i32) -> HistoryRow {
        HistoryRow {
            battle_at: at,
            left_id: ME.into(),
            left_char: ch.into(),
            left_name: "Me".into(),
            left_rating: Some(cell),
            left_change: Some(change),
            right_id: "bbbbbbbbbbbb".into(),
            right_char: "Lili".into(),
            right_name: "Other".into(),
            right_rating: Some(2000),
            right_change: Some(-change),
        }
    }

    #[test]
    fn history_cell_is_calibrated_against_records() {
        let results = [result(300, "Anna", 2240, 8)];
        let history = [row(300, "Anna", 2240, 8), row(200, "Anna", 2230, 10)];
        let p = points(ME, &results, &history);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].after, 2240, "before-convention row: cell + change");
        assert_eq!(p[1].after, 2248, "the record wins for the shared battle");

        let history = [row(300, "Anna", 2248, 8), row(200, "Anna", 2240, 10)];
        let p = points(ME, &results, &history);
        assert_eq!(p[0].after, 2240, "after-convention row: the cell itself");
    }

    #[test]
    fn summary_counts_only_the_session() {
        let results: Vec<OwnResult> = vec![
            result(100, "Anna", 2200, 10),
            result(200, "Anna", 2210, 9),
            result(300, "Anna", 2219, -7),
            result(400, "Anna", 2212, 8),
            result(500, "Anna", 2220, 6),
        ];
        let p = points(ME, &results, &[]);
        let s = summarize(&p, 150, None);
        assert_eq!(s.net, 9 - 7 + 8 + 6);
        assert_eq!((s.wins, s.losses), (3, 1));
        assert_eq!(s.last8, vec![true, false, true, true]);
        assert_eq!(s.streak, 2);
        assert_eq!(s.last.as_ref().map(|l| l.change), Some(6));
        let t = s.trend.unwrap();
        assert_eq!(t.points, vec![2210, 2219, 2212, 2220, 2226]);
        assert_eq!(t.mu, 2226);
        assert_eq!(t.character, "Anna Williams");
        let g = s.goal.unwrap();
        assert_eq!((g.target, g.to_go), (2300, 74));
        assert_eq!(g.wins_est, Some(9));
    }

    #[test]
    fn loss_streak_is_negative_and_empty_session_is_zero() {
        let results = vec![result(200, "Anna", 2210, -9), result(300, "Anna", 2201, -7)];
        let p = points(ME, &results, &[]);
        assert_eq!(summarize(&p, 0, None).streak, -2);
        let s = summarize(&p, 1000, None);
        assert_eq!((s.net, s.wins, s.losses, s.streak), (0, 0, 0, 0));
        assert!(s.trend.is_some(), "the trend is not limited to the session");
    }

    #[test]
    fn goal_falls_back_to_the_page_without_battles() {
        let s = summarize(&[], 0, Some(("Nina", 2205)));
        assert!(s.trend.is_none());
        let g = s.goal.unwrap();
        assert_eq!((g.target, g.to_go, g.wins_est), (2300, 95, None));
        assert_eq!(g.character, "Nina Williams");
    }

    #[test]
    fn trend_follows_the_last_played_character() {
        let results = vec![result(100, "Anna", 2200, 10), result(200, "Lili", 2090, 5)];
        let s = summarize(&points(ME, &results, &[]), 0, None);
        assert_eq!(s.trend.unwrap().short, "Lili");
    }
}
