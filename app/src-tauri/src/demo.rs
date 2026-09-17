use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use murating_core::achievements::{self as ach, Progress, SessionState};
use murating_core::feed::{LiveRating, OwnMatch};
use murating_core::player::Rating;
use murating_core::{characters, session, table};

use crate::accounts::AccountRow;
use crate::commands::{self, Announce, Connected};
use crate::{feed, follow, toast, tray};

const NAME: &str = "HeavenlyUzumaki";
const MAIN: &str = "Reina";
const MAIN_MU: i32 = 2515;

const VANITY: [(&str, &str, &str, i32); 4] = [
    ("8Rq2nWx4kT7e", "Eshan", "Kazuya", 2400),
    ("3mVb9Lp6Hs2d", "MewGlider", "Lili", 2360),
    ("6tYc1Kz8Qw5f", "Old Lady Relax", "Nina", 2539),
    ("9hNd4Js7Ga3r", "AyoMickey", "Leo", 2404),
];

const READ_TIME: Duration = Duration::from_millis(1400);

static GEN: AtomicU64 = AtomicU64::new(0);
static FIRST_LOOK: std::sync::Mutex<Option<crate::achievements::Snapshot>> = std::sync::Mutex::new(None);
static AUTOSTART: AtomicBool = AtomicBool::new(false);

pub fn autostart() -> bool {
    AUTOSTART.load(Ordering::SeqCst)
}

pub fn set_autostart(on: bool) {
    AUTOSTART.store(on, Ordering::SeqCst);
}

fn vanity(id: &str) -> Option<&'static (&'static str, &'static str, &'static str, i32)> {
    VANITY.iter().find(|v| v.0 == id)
}

fn file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join("preview.json"))
}

fn saved_id(app: &AppHandle) -> Option<String> {
    file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("tekken_id").and_then(|t| t.as_str()).map(str::to_string))
}

fn save_id(app: &AppHandle, id: Option<&str>) {
    let Some(p) = file(app) else { return };
    match id {
        Some(id) => {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&p, serde_json::json!({ "tekken_id": id }).to_string());
        }
        None => {
            let _ = std::fs::remove_file(&p);
        }
    }
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || match saved_id(&app) {
        Some(id) => {
            follow::set_state(&app, follow::FollowState::Connecting { tekken_id: id.clone(), name: NAME.into() });
            std::thread::sleep(Duration::from_millis(900));
            let _ = activate(&app, &id, Announce::Established);
        }
        None => follow::set_state(&app, follow::FollowState::Setup),
    });
}

pub fn rows(app: &AppHandle) -> Vec<AccountRow> {
    let following = tray::current_tekken_id(app);
    let row = |id: &str, name: &str, character: &str, mu: i32| {
        let on = following.as_deref() == Some(id);
        AccountRow {
            tekken_id: id.to_string(),
            name: name.to_string(),
            paired: true,
            signed_in: on,
            active: on,
            character: Some(character.to_string()),
            mu: Some(mu),
        }
    };
    let mut out: Vec<AccountRow> = VANITY.iter().map(|(id, name, c, mu)| row(id, name, c, *mu)).collect();
    if let Some(real) = saved_id(app) {
        out.insert(1, row(&real, NAME, MAIN, MAIN_MU));
    }
    out
}

pub fn connect_all(app: &AppHandle, entered: &[String], prefer: Option<String>) -> Result<Connected, String> {
    let _ = prefer;
    let mut ids: Vec<String> = Vec::new();
    for raw in entered {
        let id = table::tekken_id(raw);
        if id.is_empty() || ids.contains(&id) {
            continue;
        }
        if !table::looks_like_tekken_id(&id) {
            return Err(format!(
                "{id:?} is not a Tekken ID, it should be 12 letters and digits, as shown in the game's Replay menu"
            ));
        }
        ids.push(id);
    }
    let saved = saved_id(app);
    let Some(real) = ids.iter().find(|id| vanity(id).is_none()).cloned().or_else(|| saved.clone()) else {
        return Err("Enter a Tekken ID.".to_string());
    };
    for id in &ids {
        if let Some((_, name, _, _)) = vanity(id) {
            commands::progress(app, id, "done", Some(name.to_string()), None);
        }
    }
    if saved.as_deref() != Some(real.as_str()) {
        commands::progress(app, &real, "reading", None, None);
        std::thread::sleep(READ_TIME);
        save_id(app, Some(&real));
    }
    commands::progress(app, &real, "done", Some(NAME.to_string()), None);
    activate(app, &real, Announce::Established)
}

pub fn activate(app: &AppHandle, id: &str, announce: Announce) -> Result<Connected, String> {
    if vanity(id).is_some() {
        std::thread::sleep(Duration::from_millis(700));
        return match follow::state() {
            follow::FollowState::Following { me, .. } => Ok(me),
            _ => Err("That account is only on the accounts page in the preview build.".to_string()),
        };
    }
    let gen = GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let now = feed::now();
    let profile = profile(id, now, crate::roster::full_roster_len(app));

    tray::set_account(app, NAME, id, &format!("{MAIN_MU} MR"));
    match announce {
        Announce::Established => toast::established(app, NAME),
        Announce::Switched => toast::switched(app, NAME),
        Announce::Quiet => {}
    }
    if let Ok(mut first) = FIRST_LOOK.lock() {
        *first = Some(profile.achievements.clone());
    }
    crate::achievements::put(app, profile.achievements);
    feed::set_session(app, id, profile.session.clone());
    let mut diag = profile.diag;
    for c in &mut diag.characters {
        c.code = crate::roster::code_for_name(app, &c.character);
    }
    feed::set_diag_feed(diag);
    crate::diag::net_ok(crate::diag::Endpoint::Wavu, 412, None);
    crate::diag::net_ok(crate::diag::Endpoint::Feed, 88, Some(now));
    feed::polled(app, now);
    feed::central_at(app, now - 45);
    follow::set_following(app, profile.connected.clone());
    let _ = app.emit("session:update", profile.session);
    let _ = app.emit("achievements:changed", ());
    crate::accounts::changed(app);
    ticker(app, id, gen);
    Ok(profile.connected)
}

pub fn window_opened(app: &AppHandle) {
    let Some(first) = FIRST_LOOK.lock().ok().and_then(|f| f.clone()) else { return };
    if tray::current_tekken_id(app).as_deref() != Some(first.tekken_id.as_str()) {
        return;
    }
    crate::achievements::put(app, first);
    let _ = app.emit("achievements:changed", ());
}

fn ticker(app: &AppHandle, id: &str, gen: u64) {
    let app = app.clone();
    let id = id.to_string();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(2500));
        if GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        if let Some(s) = feed::session(&app) {
            let _ = app.emit("session:update", s);
        }
        let _ = app.emit("achievements:changed", ());
        tick_forever(&app, &id, gen);
    });
}

fn tick_forever(app: &AppHandle, id: &str, gen: u64) {
    loop {
        for _ in 0..60 {
            std::thread::sleep(Duration::from_secs(1));
            if GEN.load(Ordering::SeqCst) != gen {
                return;
            }
        }
        if tray::current_tekken_id(app).as_deref() != Some(id) {
            continue;
        }
        let now = feed::now();
        crate::diag::net_ok(crate::diag::Endpoint::Feed, 70 + (now % 40) as u64, Some(now));
        feed::polled(app, now);
        feed::central_at(app, now - 45);
    }
}

pub fn disconnect(app: &AppHandle) {
    GEN.fetch_add(1, Ordering::SeqCst);
    save_id(app, None);
    tray::clear_account(app);
    feed::clear_polled();
    follow::set_state(app, follow::FollowState::Setup);
    crate::accounts::changed(app);
}

pub fn resume(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || match saved_id(&app) {
        Some(id) => {
            follow::set_state(&app, follow::FollowState::Connecting { tekken_id: id.clone(), name: NAME.into() });
            std::thread::sleep(Duration::from_millis(700));
            let _ = activate(&app, &id, Announce::Quiet);
        }
        None => follow::set_state(&app, follow::FollowState::Setup),
    });
}

pub struct Profile {
    pub connected: Connected,
    pub session: session::SessionSummary,
    pub achievements: crate::achievements::Snapshot,
    pub diag: feed::DiagFeed,
}

const DAY: i64 = 86_400;

struct Day {
    ago: i64,
    start: i64,
    script: &'static str,
}

const HISTORY: [Day; 5] = [
    Day { ago: 21, start: 10, script: "WSLWWLFWL" },
    Day { ago: 19, start: -20, script: "LWLlWLWLW" },
    Day { ago: 17, start: 25, script: "WWLSWLWFL" },
    Day { ago: 14, start: 0, script: "LWlWLWLLW" },
    Day { ago: 12, start: -35, script: "LWLWlWLFL" },
];

const WEEK: [Day; 8] = [
    Day { ago: 7, start: 5, script: "WLSWFLWLWWLW" },
    Day { ago: 6, start: -25, script: "LWFLWSLWLW" },
    Day { ago: 5, start: 30, script: "WLWSWLlWL" },
    Day { ago: 4, start: -10, script: "LWlLWFLWLSL" },
    Day { ago: 3, start: 15, script: "WFLWLWWLSSS" },
    Day { ago: 2, start: -30, script: "SSLWFLWWLWLl" },
    Day { ago: 1, start: 20, script: "WLlWLFLWL" },
    Day { ago: 0, start: 0, script: "LWSlLFWWSWLlWFWS" },
];

const OPP_CHARS: [&str; 24] = [
    "Jin", "Kazuya", "Bryan", "Dragunov", "King", "Law", "Feng", "Paul", "Hwoarang", "Asuka", "Lili", "Steve",
    "Claudio", "Victor", "Azucena", "Raven", "Nina", "Devil Jin", "Lee", "Xiaoyu", "Leroy", "Lars", "Alisa",
    "Yoshimitsu",
];

const OPPONENTS: [&str; 36] = [
    "EmberKnuckle", "IronTempest", "AshenFist", "CobaltRaven", "VoidTiger", "GraniteWolf",
    "NovaStrike", "RazorMonk", "ObsidianKick", "SableFang", "TitanPalm", "CinderLotus",
    "FrostJackal", "HollowCrane", "LunarBrawler", "MagmaHeel", "OnyxViper", "QuartzOni",
    "RiftBoxer", "SteelTalon", "UmbraKnee", "VenomCrane", "WraithHook", "AzureFang",
    "BlightDragon", "CrimsonMonk", "DuskHammer", "EchoDragon", "FeralKnight", "GloomRider",
    "HexKnuckle", "InfernoHeel", "JadeTyphoon", "KarmaFang", "LotusBreaker", "MidnightOni",
];

const GAPS: [i32; 15] = [-40, 55, -95, 20, 110, -60, 35, -120, 80, -15, 65, -80, 5, 95, -35];

const REINA_ID: i32 = 100;

fn change_for(won: bool, gap: i32) -> i32 {
    let expected = 1.0 / (1.0 + 10f64.powf(f64::from(gap) / 400.0));
    let raw = if won { 22.0 * (1.0 - expected) } else { -22.0 * expected };
    let n = raw.round() as i32;
    if won {
        n.clamp(4, 24)
    } else {
        n.clamp(-24, -4)
    }
}

struct Planned {
    at: i64,
    won: bool,
    rounds: (i32, i32),
    gap: i32,
    opponent: usize,
    day: usize,
}

const fn day_2026(doy: i64) -> i64 {
    1_767_225_600 + doy * DAY + 19 * 3_600
}
const fn aug(d: i64) -> i64 {
    day_2026(212 + d - 1)
}
const fn sep(d: i64) -> i64 {
    day_2026(243 + d - 1)
}

fn ratings(reina_mu: i32, reina_seen: i64, reina_change: Option<i32>) -> Vec<LiveRating> {
    let r = |character: &str, mu: i32, sigma: i32, games: i32, last_seen: i64, group: &str, change: Option<i32>| LiveRating {
        rating: Rating {
            character: character.to_string(),
            mu,
            sigma: Some(sigma),
            games: Some(games),
            last_seen: Some(last_seen),
            group: group.to_string(),
        },
        change,
    };
    const LEADERBOARD: &str = "Leaderboard (σ² < 75)";
    const UNQUALIFIED: &str = "Unqualified (σ² < 110)";
    const PROVISIONAL: &str = "Provisional (σ² ≥ 110)";
    vec![
        r(MAIN, reina_mu, 68, 7_598, reina_seen, LEADERBOARD, reina_change),
        r("Dragunov", 2_019, 73, 410, sep(15), LEADERBOARD, None),
        r("Devil Jin", 2_242, 86, 2_221, sep(11), UNQUALIFIED, None),
        r("Kazuya", 2_224, 80, 4_686, aug(31), UNQUALIFIED, None),
        r("Miary Zo", 2_167, 103, 362, aug(27), UNQUALIFIED, None),
        r("Lidia", 2_165, 102, 1_908, aug(8), UNQUALIFIED, None),
        r("Heihachi", 2_096, 87, 2_972, sep(1), UNQUALIFIED, None),
        r("Armor King", 2_074, 80, 276, aug(1), UNQUALIFIED, None),
        r("Jin", 2_011, 81, 321, aug(26), UNQUALIFIED, None),
        r("Kunimitsu", 2_117, 112, 20, sep(10), PROVISIONAL, None),
        r("Fahkumram", 2_095, 121, 279, aug(18), PROVISIONAL, None),
    ]
}

pub fn profile(tekken_id: &str, now: i64, roster_total: usize) -> Profile {
    let spacing = |j: usize, d: usize| 380 + ((j + d) as i64 * 47) % 120;
    let today_index = HISTORY.len() + WEEK.len() - 1;
    let today = WEEK[WEEK.len() - 1].script.len();
    let last_at = now - 140;
    let today_start = last_at - (1..today).map(|j| spacing(j, today_index)).sum::<i64>();
    let mut offset_min = ((19 * 3_600 - today_start.rem_euclid(DAY)).rem_euclid(DAY) / 60) as i32;
    if offset_min > 720 {
        offset_min -= 1_440;
    }

    let mut planned: Vec<Planned> = Vec::new();
    let mut wins_so_far = 0usize;
    let mut next_loss = OPPONENTS.len() - 1;
    for (d, day) in HISTORY.iter().chain(WEEK.iter()).enumerate() {
        let mut at = today_start - day.ago * DAY + day.start * 60;
        let week = d >= HISTORY.len();
        for (j, c) in day.script.chars().enumerate() {
            if j > 0 {
                at += spacing(j, d);
            }
            let won = matches!(c, 'S' | 'F' | 'W');
            let rounds = match c {
                'S' => (3, 0),
                'F' => (3, 2),
                'W' => (3, 1),
                'l' => (2, 3),
                _ => ((j % 2) as i32, 3),
            };
            let gap = GAPS[(d * 4 + j) % GAPS.len()];
            let opponent = if !week {
                if won {
                    wins_so_far += 1;
                    wins_so_far - 1
                } else {
                    next_loss = if next_loss == 24 { OPPONENTS.len() - 1 } else { next_loss - 1 };
                    next_loss
                }
            } else {
                (d * 13 + j) % OPPONENTS.len()
            };
            planned.push(Planned { at, won, rounds, gap, opponent, day: d });
        }
    }

    let changes: Vec<i32> = planned.iter().map(|m| change_for(m.won, m.gap)).collect();
    let mut mu = MAIN_MU - changes.iter().sum::<i32>();
    let matches: Vec<OwnMatch> = planned
        .iter()
        .zip(&changes)
        .map(|(m, &change)| {
            let before = mu;
            mu += change;
            let chara = OPP_CHARS[m.opponent % OPP_CHARS.len()];
            OwnMatch {
                battle_at: m.at,
                battle_id: Some(format!("preview-{}", m.at)),
                chara_id: Some(REINA_ID),
                character: Some(MAIN.to_string()),
                opponent_id: format!("preview-opp-{}", m.opponent),
                opponent_name: OPPONENTS[m.opponent].to_string(),
                opponent_chara_id: Some(m.opponent as i32 % OPP_CHARS.len() as i32 + 1),
                opponent_character: Some(chara.to_string()),
                mu_before: before,
                change: Some(change),
                opponent_mu_before: Some(before + m.gap),
                won: m.won,
                rounds_own: Some(m.rounds.0),
                rounds_opp: Some(m.rounds.1),
                rank: Some(31),
                opponent_rank: Some(29 + (m.opponent % 3) as i32),
                region: Some(1),
                opponent_region: Some(if m.opponent % 5 == 0 { 2 } else { 1 }),
                power: Some(391_208),
            }
        })
        .collect();

    let session_start = |d: usize| planned.iter().find(|m| m.day == d).map(|m| m.at - 300).unwrap_or(now);
    let mut p = Progress::default();
    let mut s = SessionState::new(i64::MAX);
    let history_len = HISTORY.iter().map(|d| d.script.len()).sum::<usize>();

    let old_end = &matches[history_len - 1];
    let old_ratings = ratings(old_end.mu_before + changes[history_len - 1], old_end.battle_at, None);
    ach::update(
        &mut p,
        &mut s,
        &ach::Inputs {
            matches: &matches[..history_len],
            ratings: &old_ratings,
            roster_total,
            session_since: i64::MAX,
            now: old_end.battle_at + 60,
            utc_offset_min: offset_min,
        },
    );
    for i in history_len..matches.len() {
        let m = &matches[i];
        let after = m.mu_before + changes[i];
        let live = ratings(after, m.battle_at, Some(changes[i]));
        ach::update(
            &mut p,
            &mut s,
            &ach::Inputs {
                matches: &matches[..=i],
                ratings: &live,
                roster_total,
                session_since: session_start(planned[i].day),
                now: m.battle_at + 60,
                utc_offset_min: offset_min,
            },
        );
    }

    let today_since = session_start(HISTORY.len() + WEEK.len() - 1);
    p.seen_at = today_since - 1;
    let last = matches.last().expect("the week has matches");
    let live = ratings(MAIN_MU, last.battle_at, last.change);
    let inputs = ach::Inputs {
        matches: &matches,
        ratings: &live,
        roster_total,
        session_since: today_since,
        now,
        utc_offset_min: offset_min,
    };
    let achievements = crate::achievements::build(tekken_id, &p, &s, &inputs);

    let results: Vec<session::OwnResult> = matches.iter().map(OwnMatch::to_result).collect();
    let points = session::points(tekken_id, &results, &[]);
    let summary = session::summarize(&points, today_since, Some((MAIN, MAIN_MU)));

    let connected = Connected {
        tekken_id: tekken_id.to_string(),
        name: NAME.to_string(),
        character: characters::full_name(MAIN),
        mu: MAIN_MU,
        recent_character: characters::full_name(MAIN),
        recent_mu: MAIN_MU,
        eligible: true,
        ratings: commands::char_ratings(&live),
        wrote: String::new(),
    };
    let diag = feed::DiagFeed {
        characters: live
            .iter()
            .map(|l| feed::DiagCharacter {
                character: l.rating.character.clone(),
                code: None,
                page_mu: Some(l.rating.mu),
                base: l.rating.mu,
                asof: l.rating.last_seen,
                rd: l.rating.sigma.map(f64::from),
            })
            .collect(),
        recent_matches: matches
            .iter()
            .rev()
            .take(5)
            .rev()
            .map(|m| feed::DiagMatch {
                at: m.battle_at,
                character: m.character.clone(),
                rated: true,
                change: m.change,
                mr_after: m.change.map(|c| m.mu_before + c),
            })
            .collect(),
        records: 48_210,
        players: 11_342,
        rd_entries: 11_342,
        page_fetched_at: Some(now),
        central_hours_loaded: 8,
        central_minutes_merged: 23,
        ..feed::DiagFeed::default()
    };
    Profile { connected, session: summary, achievements, diag }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_account_is_what_was_asked_for() {
        for now in [1_789_000_000_i64, feed::now()] {
            let p = profile("2yh7ByTerD8a", now, 42);
            let a = &p.achievements;
            let earned: Vec<(String, u8)> =
                a.rows.iter().filter(|r| r.earned).map(|r| (r.id.clone(), r.rarity)).collect();
            println!("earned {} of {}", a.earned, a.total);
            for r in a.rows.iter().filter(|r| r.earned) {
                println!("  {} {:<22} rarity {} x{} {:?}", r.id, r.title, r.rarity, r.times, r.progress);
            }
            println!("session {:+} ({}-{}), streak {}, last8 {:?}", p.session.net, p.session.wins, p.session.losses, p.session.streak, p.session.last8);
            println!("trend {:?}", p.session.trend);
            println!("goal {:?}", p.session.goal);
            println!("unread {} (rarity {})", a.unread, a.unread_rarity);
            for f in a.feed.iter().take(60) {
                match f {
                    ach::FeedItem::Award { at, title, text, rarity, .. } => {
                        println!("  {:>6} min  * {title} (r{rarity}): {text}", (now - at) / 60)
                    }
                    ach::FeedItem::Battle { at, won, change, mu_after, opponent_name, opponent_character, rounds_own, rounds_opp, .. } => {
                        println!(
                            "  {:>6} min  {} {:+} -> {mu_after} vs {opponent_name} ({}) {}-{}",
                            (now - at) / 60,
                            if *won { "W" } else { "L" },
                            change,
                            opponent_character.as_deref().unwrap_or("?"),
                            rounds_own.unwrap_or(0),
                            rounds_opp.unwrap_or(0)
                        )
                    }
                }
            }

            let want: &[(&str, u8)] = &[
                ("M01", 1), ("M02", 4), ("M03", 3), ("M04", 2), ("M06", 3), ("M07", 2), ("M08", 2),
                ("M09", 2), ("A01", 1), ("A02", 1), ("A42", 1), ("A11", 2), ("A16", 4), ("A45", 5),
                ("A49", 1), ("A50", 2), ("A54", 3), ("A17", 1), ("A20", 2), ("A63", 2),
            ];
            let mut got = earned.clone();
            got.sort();
            let mut expect: Vec<(String, u8)> = want.iter().map(|(id, r)| (id.to_string(), *r)).collect();
            expect.sort();
            assert_eq!(got, expect, "earned set moved");
            assert_eq!((a.earned, a.total), (20, 75));
            assert_eq!((p.session.wins, p.session.losses), (11, 5));
            assert_eq!(p.session.streak, 4);
            assert_eq!(p.connected.mu, MAIN_MU);
            assert_eq!(p.session.trend.as_ref().map(|t| t.mu), Some(MAIN_MU));
            assert!(a.unread > 0);
        }
    }
}
