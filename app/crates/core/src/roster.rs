use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::characters;
use crate::player::WavuCharacter;
use crate::table::char_key;
use crate::utoc::{self, ScanError};

pub const RANKED: &[(&str, &str, &str)] = &[
    ("aml", "Jun", "jun"),
    ("ant", "Jin", "jin"),
    ("bbn", "Raven", "raven"),
    ("bee", "Heihachi", "heihachi"),
    ("bsn", "Steve", "steve"),
    ("cat", "Azucena", "azu"),
    ("cbr", "Lidia", "lidia"),
    ("ccn", "Jack-8", "jack"),
    ("cht", "Bryan", "bryan"),
    ("cml", "Yoshimitsu", "yoshi"),
    ("crw", "Zafina", "zafina"),
    ("ctr", "Claudio", "claudio"),
    ("der", "Asuka", "asuka"),
    ("dog", "Eddy", "eddy"),
    ("ghp", "Leo", "leo"),
    ("grf", "Paul", "paul"),
    ("grl", "Kazuya", "kazuya"),
    ("hms", "Lili", "lili"),
    ("hrs", "Shaheen", "shaheen"),
    ("jly", "Leroy", "leroy"),
    ("kal", "Nina", "nina"),
    ("ker", "Kunimitsu", "kuni"),
    ("kgr", "Anna", "anna"),
    ("klw", "Feng", "feng"),
    ("kmd", "Dragunov", "drag"),
    ("knk", "Armor King", "ak"),
    ("lon", "Victor", "victor"),
    ("lzd", "Lars", "lars"),
    ("mnt", "Alisa", "alisa"),
    ("okm", "Clive", "clive"),
    ("pgn", "King", "king"),
    ("pig", "Law", "law"),
    ("rat", "Xiaoyu", "xiaoyu"),
    ("rbt", "Kuma", "kuma"),
    ("snk", "Hwoarang", "hwo"),
    ("swl", "Devil Jin", "dj"),
    ("tgr", "Fahkumram", "fahk"),
    ("ttr", "Panda", "panda"),
    ("usi", "Bob", "bob"),
    ("wkz", "Miary Zo", "miary"),
    ("wlf", "Lee", "lee"),
    ("zbr", "Reina", "reina"),
];

pub const CHARA_IDS: &[(i32, &str)] = &[
    (0, "grf"), (1, "pig"), (2, "pgn"), (3, "cml"), (4, "snk"), (5, "rat"), (6, "ant"),
    (7, "cht"), (8, "grl"), (9, "bsn"), (10, "ccn"), (11, "der"), (12, "swl"), (13, "klw"),
    (14, "hms"), (15, "kmd"), (16, "ghp"), (17, "lzd"), (18, "mnt"), (19, "ctr"), (20, "hrs"),
    (21, "kal"), (22, "wlf"), (23, "rbt"), (24, "ttr"), (28, "crw"), (29, "jly"), (32, "aml"),
    (33, "zbr"), (34, "cat"), (35, "lon"), (36, "bbn"), (38, "dog"), (39, "cbr"), (40, "bee"),
    (41, "okm"), (42, "kgr"), (43, "tgr"), (44, "knk"), (45, "wkz"), (46, "ker"), (47, "usi"),
];

pub const FORMS: &[&str] = &[
    "bee3", "got", "grl2", "grl4", "swl4", "xxa", "xxb", "xxc", "xxd", "xxe", "xxf", "xxg",
];

const NAME_ART_DIR: &str = "/UI/Rep_Texture/HUD_Character_Name/";
const NAME_ART_PREFIX: &str = "T_UI_HUD_Character_Name_";
const ITEMS_DIR: &str = "/UI/Rep_Texture/CUS_CH_Item/";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameScan {
    pub name_art: BTreeSet<String>,
    pub playable: BTreeSet<String>,
}

pub fn scan_paths<'a>(paths: impl IntoIterator<Item = &'a str>) -> GameScan {
    let mut name_art = BTreeSet::new();
    let mut items = BTreeSet::new();
    for path in paths {
        if let Some(i) = path.find(NAME_ART_DIR) {
            let file = &path[i + NAME_ART_DIR.len()..];
            if let Some(code) = file
                .strip_prefix(NAME_ART_PREFIX)
                .and_then(|f| f.strip_suffix(".uasset"))
                .filter(|c| !c.is_empty() && !c.contains('/'))
            {
                name_art.insert(code.to_ascii_lowercase());
            }
        } else if let Some(i) = path.find(ITEMS_DIR) {
            let rest = &path[i + ITEMS_DIR.len()..];
            if let Some((code, _)) = rest.split_once('/') {
                items.insert(code.to_ascii_lowercase());
            }
        }
    }
    let playable = name_art.intersection(&items).cloned().collect();
    GameScan { name_art, playable }
}

fn is_stock_container(file_name: &str) -> bool {
    let Some(rest) = file_name.strip_prefix("pakchunk") else { return false };
    let Some(rest) = rest.strip_suffix(".utoc") else { return false };
    let Some((chunk, layer)) = rest.split_once("-Windows") else { return false };
    if chunk.is_empty() || !chunk.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    match layer {
        "" => true,
        l => l
            .strip_prefix('_')
            .and_then(|l| l.strip_suffix("_P"))
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())),
    }
}

pub fn scan_game(install_dir: &Path) -> Result<GameScan, ScanError> {
    let paks = install_dir.join("Polaris").join("Content").join("Paks");
    let mut all = Vec::new();
    for entry in std::fs::read_dir(&paks)?.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !is_stock_container(name) || !entry.path().is_file() {
            continue;
        }
        if let Ok(info) = utoc::parse_file(&entry.path()) {
            all.extend(info.asset_paths);
        }
    }
    let scan = scan_paths(all.iter().map(String::as_str));
    if scan.name_art.is_empty() {
        return Err(ScanError::Malformed("no character name art in the game's containers"));
    }
    Ok(scan)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Learned {
    pub playable: bool,
    pub wavu: Option<WavuCharacter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub code: String,
    pub wavu: WavuCharacter,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Roster {
    pub build: Option<u64>,
    pub learned: BTreeMap<String, Learned>,
    pub wavu: Vec<WavuCharacter>,
}

fn is_baked(code: &str) -> bool {
    RANKED.iter().any(|(c, _, _)| *c == code) || FORMS.contains(&code)
}

impl Roster {
    pub fn needs_scan(&self, build: Option<u64>) -> bool {
        build.is_some() && self.build != build
    }

    pub fn observe_game(&mut self, build: Option<u64>, scan: &GameScan) -> Vec<String> {
        let mut added = Vec::new();
        for code in &scan.name_art {
            if is_baked(code) {
                continue;
            }
            let playable = scan.playable.contains(code);
            match self.learned.get_mut(code) {
                Some(l) => l.playable |= playable,
                None => {
                    self.learned.insert(code.clone(), Learned { playable, wavu: None });
                    added.push(code.clone());
                }
            }
        }
        self.build = build;
        added
    }

    pub fn observe_wavu(&mut self, roster: &[WavuCharacter]) -> bool {
        if roster.is_empty() || roster == self.wavu.as_slice() {
            return false;
        }
        self.wavu = roster.to_vec();
        true
    }

    pub fn waiting_codes(&self) -> Vec<String> {
        self.learned
            .iter()
            .filter(|(_, l)| l.playable && l.wavu.is_none())
            .map(|(c, _)| c.clone())
            .collect()
    }

    pub fn waiting_names(&self) -> Vec<WavuCharacter> {
        let mut slugs: HashSet<String> = HashSet::new();
        let mut names: HashSet<String> = HashSet::new();
        for (_, name, slug) in RANKED {
            slugs.insert(slug.to_string());
            names.insert(char_key(name));
        }
        for w in self.learned.values().filter_map(|l| l.wavu.as_ref()) {
            slugs.insert(w.slug.clone());
            names.insert(char_key(&w.name));
        }
        self.wavu
            .iter()
            .filter(|w| !slugs.contains(&w.slug) && !names.contains(&char_key(&w.name)))
            .cloned()
            .collect()
    }

    pub fn pair(&mut self) -> Option<Match> {
        let codes = self.waiting_codes();
        let names = self.waiting_names();
        let ([code], [wavu]) = (codes.as_slice(), names.as_slice()) else {
            return None;
        };
        self.learned.get_mut(code)?.wavu = Some(wavu.clone());
        Some(Match { code: code.clone(), wavu: wavu.clone() })
    }

    pub fn wavu_name(&self, code: &str) -> Option<String> {
        let code = code.to_ascii_lowercase();
        RANKED
            .iter()
            .find(|(c, _, _)| *c == code)
            .map(|(_, n, _)| n.to_string())
            .or_else(|| self.learned.get(&code)?.wavu.as_ref().map(|w| w.name.clone()))
    }

    pub fn code(&self, wavu_name: &str) -> Option<String> {
        let k = char_key(wavu_name);
        RANKED
            .iter()
            .find(|(_, n, _)| char_key(n) == k)
            .map(|(c, _, _)| c.to_string())
            .or_else(|| {
                self.learned
                    .iter()
                    .find(|(_, l)| l.wavu.as_ref().is_some_and(|w| char_key(&w.name) == k))
                    .map(|(c, _)| c.clone())
            })
    }

    pub fn code_for_chara(&self, chara_id: i32, learned: &std::collections::HashMap<i32, String>) -> Option<String> {
        CHARA_IDS
            .iter()
            .find(|(id, _)| *id == chara_id)
            .map(|(_, c)| c.to_string())
            .or_else(|| learned.get(&chara_id).and_then(|name| self.code(name)))
    }

    pub fn full_roster(&self) -> Vec<String> {
        if self.wavu.is_empty() {
            return characters::roster();
        }
        self.wavu.iter().map(|w| characters::full_name(&w.name)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::parse_roster;

    fn wavu(slug: &str, name: &str) -> WavuCharacter {
        WavuCharacter { slug: slug.into(), name: name.into() }
    }

    fn today() -> Vec<WavuCharacter> {
        parse_roster(include_str!("../tests/fixtures/opp_char_select.html"))
    }

    fn stock_paths() -> Vec<String> {
        let mut p = Vec::new();
        for (code, _, _) in RANKED {
            p.push(format!("Polaris/Content/UI/Rep_Texture/HUD_Character_Name/T_UI_HUD_Character_Name_{code}.uasset"));
            p.push(format!("Polaris/Content/UI/Rep_Texture/CUS_CH_Item/{code}/T_UI_CUS_{code}_0001.uasset"));
        }
        for code in FORMS {
            p.push(format!("Polaris/Content/UI/Rep_Texture/HUD_Character_Name/T_UI_HUD_Character_Name_{code}.uasset"));
        }
        p.push("Polaris/Content/UI/Rep_Texture/CUS_CH_Item/cmn/T_UI_CUS_cmn_0001.uasset".into());
        p.push("Polaris/Content/UI/Rep_Texture/HUD_Character_Name/T_UI_HUD_Character_Name_jly.ubulk".into());
        p
    }

    fn scan(extra: &[&str]) -> GameScan {
        let mut paths = stock_paths();
        paths.extend(extra.iter().map(|s| s.to_string()));
        scan_paths(paths.iter().map(String::as_str))
    }

    const ROGER_ART: &str = "Polaris/Content/UI/Rep_Texture/HUD_Character_Name/T_UI_HUD_Character_Name_rgj.uasset";
    const ROGER_ITEMS: &str = "Polaris/Content/UI/Rep_Texture/CUS_CH_Item/rgj/T_UI_CUS_rgj_0001.uasset";

    #[test]
    fn the_table_is_todays_wavu_roster() {
        let r = Roster { wavu: today(), ..Roster::default() };
        assert_eq!(RANKED.len(), 42);
        assert!(r.waiting_names().is_empty(), "unmatched: {:?}", r.waiting_names());
        for w in today() {
            assert!(
                RANKED.iter().any(|(_, n, s)| *n == w.name && *s == w.slug),
                "{w:?} is spelled differently in the table"
            );
        }
    }

    #[test]
    fn every_ranked_code_has_exactly_one_wavu_id() {
        assert_eq!(CHARA_IDS.len(), RANKED.len());
        for (code, name, _) in RANKED {
            let ids: Vec<i32> = CHARA_IDS.iter().filter(|(_, c)| c == code).map(|(i, _)| *i).collect();
            assert_eq!(ids.len(), 1, "{name} ({code})");
        }
        let r = Roster::default();
        let none = std::collections::HashMap::new();
        assert_eq!(r.code_for_chara(29, &none).as_deref(), Some("jly"), "Leroy");
        assert_eq!(r.code_for_chara(47, &none).as_deref(), Some("usi"), "Bob");
        assert_eq!(r.code_for_chara(14, &none).as_deref(), Some("hms"), "Lili");
    }

    #[test]
    fn a_new_fighters_id_comes_through_its_learned_name() {
        let mut r = Roster { wavu: today(), ..Roster::default() };
        r.wavu.push(wavu("roger", "Roger Jr."));
        r.observe_game(Some(25300000), &scan(&[ROGER_ART, ROGER_ITEMS]));
        r.pair();
        let learned = std::collections::HashMap::from([(48, "Roger Jr.".to_string())]);
        assert_eq!(r.code_for_chara(48, &learned).as_deref(), Some("rgj"));
        assert_eq!(r.code_for_chara(49, &learned), None);
    }

    #[test]
    fn a_stock_scan_finds_nothing_new() {
        let s = scan(&[]);
        assert_eq!(s.name_art.len(), 54);
        assert_eq!(s.playable.len(), 42, "cmn has items but no name art");
        let mut r = Roster::default();
        assert!(r.observe_game(Some(25221600), &s).is_empty());
        assert!(r.learned.is_empty());
        assert!(!r.needs_scan(Some(25221600)));
        assert!(r.needs_scan(Some(25300000)));
        assert!(!r.needs_scan(None), "no game installed is not a new build");
    }

    #[test]
    fn a_new_fighter_pairs_with_wavus_new_character() {
        let mut r = Roster { wavu: today(), ..Roster::default() };
        assert_eq!(r.observe_game(Some(25300000), &scan(&[ROGER_ART, ROGER_ITEMS])), vec!["rgj"]);
        assert_eq!(r.waiting_codes(), vec!["rgj"]);
        assert_eq!(r.pair(), None, "nothing to pair with until Wavu has him");
        assert_eq!(r.wavu_name("rgj"), None);

        let mut later = today();
        later.push(wavu("roger", "Roger Jr."));
        assert!(r.observe_wavu(&later));
        assert_eq!(r.pair(), Some(Match { code: "rgj".into(), wavu: wavu("roger", "Roger Jr.") }));
        assert_eq!(r.wavu_name("rgj").as_deref(), Some("Roger Jr."));
        assert_eq!(r.wavu_name("RGJ").as_deref(), Some("Roger Jr."));
        assert_eq!(r.code("Roger Jr.").as_deref(), Some("rgj"));
        assert!(r.waiting_codes().is_empty() && r.waiting_names().is_empty());
        assert_eq!(r.pair(), None, "a match is made once");
    }

    #[test]
    fn a_new_story_form_is_never_a_fighter() {
        let form = "Polaris/Content/UI/Rep_Texture/HUD_Character_Name/T_UI_HUD_Character_Name_xxh.uasset";
        let mut r = Roster { wavu: today(), ..Roster::default() };
        r.wavu.push(wavu("roger", "Roger Jr."));
        assert_eq!(r.observe_game(Some(25300000), &scan(&[form])), vec!["xxh"]);
        assert!(r.waiting_codes().is_empty(), "no items, so not a fighter");
        assert_eq!(r.pair(), None, "Roger Jr. must not be given the story form's code");
    }

    #[test]
    fn a_form_released_alongside_a_fighter_does_not_block_the_match() {
        let form = "Polaris/Content/UI/Rep_Texture/HUD_Character_Name/T_UI_HUD_Character_Name_xxh.uasset";
        let mut r = Roster { wavu: today(), ..Roster::default() };
        r.wavu.push(wavu("roger", "Roger Jr."));
        r.observe_game(Some(25300000), &scan(&[form, ROGER_ART, ROGER_ITEMS]));
        assert_eq!(r.pair().map(|m| m.code).as_deref(), Some("rgj"));
    }

    #[test]
    fn two_new_fighters_are_never_guessed_between() {
        let (art2, items2) = (
            "Polaris/Content/UI/Rep_Texture/HUD_Character_Name/T_UI_HUD_Character_Name_zzz.uasset",
            "Polaris/Content/UI/Rep_Texture/CUS_CH_Item/zzz/T_UI_CUS_zzz_0001.uasset",
        );
        let mut r = Roster { wavu: today(), ..Roster::default() };
        r.wavu.push(wavu("roger", "Roger Jr."));
        r.observe_game(Some(25300000), &scan(&[ROGER_ART, ROGER_ITEMS, art2, items2]));
        assert_eq!(r.waiting_codes(), vec!["rgj", "zzz"]);
        assert_eq!(r.pair(), None);
        r.wavu.push(wavu("new", "Someone New"));
        assert_eq!(r.pair(), None, "two and two is still a guess");
    }

    #[test]
    fn art_ahead_of_items_becomes_a_fighter_on_the_next_build() {
        let mut r = Roster { wavu: today(), ..Roster::default() };
        r.wavu.push(wavu("roger", "Roger Jr."));
        r.observe_game(Some(25300000), &scan(&[ROGER_ART]));
        assert_eq!(r.pair(), None, "art alone is not enough");
        assert!(r.observe_game(Some(25400000), &scan(&[ROGER_ART, ROGER_ITEMS])).is_empty());
        assert_eq!(r.pair().map(|m| m.code).as_deref(), Some("rgj"));
    }

    #[test]
    fn wavu_respelling_a_character_is_not_a_new_one() {
        let mut respelled = today();
        let drag = respelled.iter_mut().find(|w| w.slug == "drag").unwrap();
        drag.name = "Sergei Dragunov".into();
        let mut r = Roster { wavu: respelled, ..Roster::default() };
        r.observe_game(Some(25300000), &scan(&[ROGER_ART, ROGER_ITEMS]));
        assert!(r.waiting_names().is_empty(), "same slug, same character");
        assert_eq!(r.pair(), None, "Roger Jr.'s code must not go to Dragunov");
    }

    #[test]
    fn an_empty_page_keeps_the_last_roster() {
        let mut r = Roster { wavu: today(), ..Roster::default() };
        assert!(!r.observe_wavu(&[]));
        assert_eq!(r.wavu.len(), 42);
        assert!(!r.observe_wavu(&today()), "unchanged");
    }

    #[test]
    fn the_full_roster_follows_wavu() {
        let mut r = Roster::default();
        assert_eq!(r.full_roster().len(), 42, "the built-in list before Wavu is read");
        let mut later = today();
        later.push(wavu("roger", "Roger Jr."));
        r.observe_wavu(&later);
        let full = r.full_roster();
        assert_eq!(full.len(), 43);
        assert!(full.contains(&"Anna Williams".to_string()));
        assert!(full.contains(&"Roger Jr.".to_string()));
    }

    #[test]
    fn baked_lookups_work_both_ways() {
        let r = Roster::default();
        assert_eq!(r.wavu_name("jly").as_deref(), Some("Leroy"));
        assert_eq!(r.code("JACK-8").as_deref(), Some("ccn"));
        assert_eq!(r.code("Armor King").as_deref(), Some("knk"));
        assert_eq!(r.wavu_name("got"), None, "Azazel is not ranked");
    }

    #[test]
    fn it_survives_a_round_trip_through_json() {
        let mut r = Roster { wavu: today(), ..Roster::default() };
        r.wavu.push(wavu("roger", "Roger Jr."));
        r.observe_game(Some(25300000), &scan(&[ROGER_ART, ROGER_ITEMS]));
        r.pair();
        let back: Roster = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
        let old: Roster = serde_json::from_str("{}").unwrap();
        assert_eq!(old, Roster::default(), "a file from an older helper still loads");
    }

    #[test]
    fn only_the_games_own_containers_are_read() {
        for stock in ["pakchunk300-Windows.utoc", "pakchunk300-Windows_0_P.utoc", "pakchunk0-Windows_1_P.utoc"] {
            assert!(is_stock_container(stock), "{stock}");
        }
        for other in ["global.utoc", "pakchunk300-Windows.ucas", "MuRating_P.utoc", "pakchunk-Windows.utoc", "pakchunk99-mods_P.utoc", "pakchunk1-Windows_P.utoc"] {
            assert!(!is_stock_container(other), "{other}");
        }
    }

    #[test]
    #[ignore = "reads the installed game"]
    fn the_installed_game_matches_the_table() {
        let dir = crate::game::install_dir().expect("Tekken 8 installed");
        let s = scan_game(&dir).expect("scan");
        let mut r = Roster::default();
        assert_eq!(r.observe_game(crate::game::build_id(), &s), Vec::<String>::new(), "new codes in this build");
        assert_eq!(s.name_art.len(), 54);
        assert_eq!(s.playable.len(), 42);
    }
}
