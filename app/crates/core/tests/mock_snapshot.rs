use murating_core::achievements::{self as ach, Progress, SessionState};
use murating_core::feed::{LiveRating, OwnMatch};
use murating_core::player::Rating;

const DAY: i64 = 86_400;
const NOW: i64 = 1_757_980_800;

fn m(at: i64, chara: &str, mu: i32, ch: i32, opp: &str, opp_chara: &str, opp_mu: i32, ro: i32, rp: i32) -> OwnMatch {
    OwnMatch {
        battle_at: at,
        battle_id: Some(format!("b{at}")),
        chara_id: Some(1),
        character: Some(chara.into()),
        opponent_id: format!("id-{opp}"),
        opponent_name: opp.into(),
        opponent_chara_id: Some(2),
        opponent_character: Some(opp_chara.into()),
        mu_before: mu,
        change: Some(ch),
        opponent_mu_before: Some(opp_mu),
        won: ro > rp,
        rounds_own: Some(ro),
        rounds_opp: Some(rp),
        rank: Some(30),
        opponent_rank: Some(30),
        region: Some(1),
        opponent_region: Some(1),
        power: Some(280_000),
    }
}

#[test]
fn dump() {
    let mut mu = 1_952;
    let mut out: Vec<OwnMatch> = Vec::new();
    let opps = [
        ("MRmani", "Jin"),
        ("NeonJaguar", "Dragunov"),
        ("OmenStrike", "Feng"),
        ("PyreLion", "Bryan"),
        ("RuneBrawler", "Kazuya"),
        ("SolarRonin", "Law"),
        ("TempestFist", "Victor"),
        ("UmbralFox", "Hwoarang"),
    ];

    for day in 0i32..3 {
        let base = NOW - i64::from(2 - day) * DAY - 5 * 3_600;
        let n = [9, 7, 12][day as usize];
        for g in 0i32..n {
            let (name, oc) = opps[((day * 5 + g) % 8) as usize];
            let win = match day {
                0 => g % 3 != 2,
                1 => g < 2,
                _ => g != 3 && g != 4,
            };
            let ch = if win { 9 + (g % 4) } else { -(8 + (g % 5)) };
            let opp_mu = mu + [-140, 210, 15, -60, 320][(g % 5) as usize];
            let (ro, rp) = if win { (3, g % 3) } else { (g % 3, 3) };
            out.push(m(base + i64::from(g) * 420, "Kazuya", mu, ch, name, oc, opp_mu, ro, rp));
            mu += ch;
        }
    }

    let ratings = vec![
        LiveRating {
            rating: Rating {
                character: "Kazuya".into(),
                mu,
                sigma: Some(52),
                games: Some(1_284),
                last_seen: Some(NOW),
                group: "Leaderboard".into(),
            },
            change: Some(11),
        },
        LiveRating {
            rating: Rating {
                character: "Jin".into(),
                mu: 1_704,
                sigma: Some(88),
                games: Some(210),
                last_seen: Some(NOW - 6 * DAY),
                group: "Leaderboard".into(),
            },
            change: None,
        },
    ];

    let mut p = Progress::default();
    let mut s = SessionState::new(NOW - 5 * 3_600);
    for i in 1..=out.len() {
        let inputs = ach::Inputs {
            matches: &out[..i],
            ratings: &ratings,
            roster_total: 32,
            session_since: NOW - 5 * 3_600,
            now: out[i - 1].battle_at + 60,
            utc_offset_min: -300,
        };
        ach::update(&mut p, &mut s, &inputs);
    }

    let inputs = ach::Inputs {
        matches: &out,
        ratings: &ratings,
        roster_total: 32,
        session_since: NOW - 5 * 3_600,
        now: NOW,
        utc_offset_min: -300,
    };
    let (rows, sections) = ach::collection(&p, &s, &inputs);
    let earned = rows.iter().filter(|r| r.earned).count();
    let snap = serde_json::json!({
        "tekken_id": "MOCK-0001",
        "feed": p.feed,
        "rows": rows,
        "sections": sections,
        "earned": earned,
        "total": rows.len(),
        "unread": 3,
        "unread_rarity": 4,
    });
    std::fs::write(
        std::env::var("MOCK_OUT").unwrap_or_else(|_| "mock-achievements.json".into()),
        serde_json::to_string_pretty(&snap).unwrap(),
    )
    .unwrap();
    println!("feed {} rows {} earned {}", p.feed.len(), rows.len(), earned);
}
