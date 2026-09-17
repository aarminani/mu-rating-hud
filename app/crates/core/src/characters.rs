use crate::table::char_key;

const NAMES: &[(&str, &str)] = &[
    ("Alisa", "Alisa Bosconovitch"),
    ("Anna", "Anna Williams"),
    ("Armor King", "Armor King"),
    ("Asuka", "Asuka Kazama"),
    ("Azucena", "Azucena"),
    ("Bob", "Bob Richards"),
    ("Bryan", "Bryan Fury"),
    ("Claudio", "Claudio Serafino"),
    ("Clive", "Clive Rosfield"),
    ("Devil Jin", "Devil Jin"),
    ("Dragunov", "Sergei Dragunov"),
    ("Eddy", "Eddy Gordo"),
    ("Fahkumram", "Fahkumram"),
    ("Feng", "Feng Wei"),
    ("Heihachi", "Heihachi Mishima"),
    ("Hwoarang", "Hwoarang"),
    ("Jack-8", "Jack-8"),
    ("Jin", "Jin Kazama"),
    ("Jun", "Jun Kazama"),
    ("Kazuya", "Kazuya Mishima"),
    ("King", "King"),
    ("Kuma", "Kuma"),
    ("Kunimitsu", "Kunimitsu"),
    ("Lars", "Lars Alexandersson"),
    ("Law", "Marshall Law"),
    ("Lee", "Lee Chaolan"),
    ("Leo", "Leo Kliesen"),
    ("Leroy", "Leroy Smith"),
    ("Lidia", "Lidia Sobieska"),
    ("Lili", "Lili Rochefort"),
    ("Miary Zo", "Miary Zo"),
    ("Nina", "Nina Williams"),
    ("Panda", "Panda"),
    ("Paul", "Paul Phoenix"),
    ("Raven", "Raven"),
    ("Reina", "Reina"),
    ("Shaheen", "Shaheen"),
    ("Steve", "Steve Fox"),
    ("Victor", "Victor Chevalier"),
    ("Xiaoyu", "Ling Xiaoyu"),
    ("Yoshimitsu", "Yoshimitsu"),
    ("Zafina", "Zafina"),
];

const UPCOMING: &[(&str, &str)] = &[
    ("Roger Jr.", "Roger Jr."),
    ("Roger", "Roger Jr."),
];

pub fn roster() -> Vec<String> {
    NAMES.iter().map(|(_, full)| full.to_string()).collect()
}

pub fn full_name(short: &str) -> String {
    let k = char_key(short);
    NAMES
        .iter()
        .chain(UPCOMING)
        .find(|(wavu, _)| char_key(wavu) == k)
        .map(|(_, full)| full.to_string())
        .unwrap_or_else(|| short.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_the_ones_with_surnames() {
        assert_eq!(full_name("Anna"), "Anna Williams");
        assert_eq!(full_name("Dragunov"), "Sergei Dragunov");
        assert_eq!(full_name("Law"), "Marshall Law");
    }

    #[test]
    fn family_name_first_where_that_is_the_name() {
        assert_eq!(full_name("Xiaoyu"), "Ling Xiaoyu");
        assert_eq!(full_name("Feng"), "Feng Wei");
    }

    #[test]
    fn every_name_fits_the_column() {
        assert_eq!(full_name("Lili"), "Lili Rochefort");
        assert_eq!(full_name("Leo"), "Leo Kliesen");
        assert_eq!(full_name("Bob"), "Bob Richards");
        assert_eq!(full_name("Azucena"), "Azucena");
        for (_, full) in NAMES {
            assert!(full.len() <= 20, "{full:?} is {} chars", full.len());
        }
    }

    #[test]
    fn titles_and_designations_are_left_alone() {
        for n in ["King", "Armor King", "Jack-8", "Kuma", "Panda", "Devil Jin"] {
            assert_eq!(full_name(n), n, "{n} should not gain a surname");
        }
    }

    #[test]
    fn no_surname_revealed_stays_bare() {
        for n in ["Reina", "Raven", "Shaheen", "Zafina", "Kunimitsu", "Fahkumram"] {
            assert_eq!(full_name(n), n);
        }
    }

    #[test]
    fn the_hud_spelling_resolves_too() {
        assert_eq!(full_name("LILI"), "Lili Rochefort");
        assert_eq!(full_name("DEVIL JIN"), "Devil Jin");
        assert_eq!(full_name("JACK-8"), "Jack-8");
    }

    #[test]
    fn an_unknown_character_passes_through() {
        assert_eq!(full_name("Someone New"), "Someone New");
    }

    #[test]
    fn the_table_covers_the_whole_live_roster() {
        assert_eq!(NAMES.len(), 42);
    }

    #[test]
    fn an_announced_character_is_named_but_not_on_the_roster_yet() {
        assert_eq!(full_name("Roger"), "Roger Jr.");
        assert_eq!(full_name("ROGER JR."), "Roger Jr.");
        assert!(!roster().iter().any(|n| n.starts_with("Roger")));
    }
}
