use caseless::default_case_fold_str;
use unicode_normalization::UnicodeNormalization;

const INVISIBLE: [char; 12] = [
    '\u{200B}', '\u{200C}', '\u{200D}', '\u{2060}', '\u{FEFF}',
    '\u{200E}', '\u{200F}',
    '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}',
];

fn strip_invisible(s: &str) -> String {
    s.chars().filter(|c| !INVISIBLE.contains(c)).collect()
}

pub fn key(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    let nfkc: String = name.nfkc().collect();
    let visible = strip_invisible(&nfkc);
    let collapsed = visible.split_whitespace().collect::<Vec<_>>().join(" ");
    default_case_fold_str(&collapsed)
}

pub fn char_key(character: &str) -> String {
    if character.is_empty() {
        return String::new();
    }
    let nfkc: String = character.nfkc().collect();
    let visible = strip_invisible(&nfkc);
    let alnum: String = visible.chars().filter(|c| c.is_alphanumeric()).collect();
    default_case_fold_str(&alnum)
}

pub fn tekken_id(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect()
}

pub fn looks_like_tekken_id(value: &str) -> bool {
    let bare = tekken_id(value);
    bare.len() == 12
        && bare.chars().all(|c| c.is_ascii_alphanumeric())
        && !bare.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_width_is_stripped() {
        assert_eq!(key("Steel\u{200b}Mantis"), "steelmantis");
        assert_eq!(key("SteelMantis"), "steelmantis");
        assert_ne!(key("Steel Mantis"), key("SteelMantis"), "a real space is not nothing");
    }

    #[test]
    fn full_width_latin_folds_to_ascii() {
        assert_eq!(key("\u{ff2d}\u{ff32}mani"), "mrmani");
    }

    #[test]
    fn case_and_whitespace_collapse() {
        assert_eq!(key("  MRmani  "), "mrmani");
        assert_eq!(key("MR   mani"), "mr mani");
    }

    #[test]
    fn casefold_is_not_lowercase() {
        assert_eq!(key("stra\u{00df}e"), "strasse");
    }

    #[test]
    fn hud_character_matches_wavu_character() {
        assert_eq!(char_key("DEVIL JIN"), char_key("Devil Jin"));
        assert_eq!(char_key("JACK-8"), char_key("Jack-8"));
        assert_eq!(char_key("Jack-8"), "jack8");
    }

    #[test]
    fn different_characters_stay_different() {
        assert_ne!(char_key("Jin"), char_key("Devil Jin"));
    }

    #[test]
    fn tekken_id_strips_hyphens_and_keeps_case() {
        assert_eq!(tekken_id("4Rh7-ai3D-3G2a"), "4Rh7ai3D3G2a");
        assert_eq!(tekken_id("2yh7ByTerD8a"), "2yh7ByTerD8a");
        assert_ne!(tekken_id("aB3456789012"), tekken_id("Ab3456789012"));
    }

    #[test]
    fn id_shape_rejects_the_near_misses() {
        assert!(looks_like_tekken_id("2yh7ByTerD8a"));
        assert!(looks_like_tekken_id("4Rh7-ai3D-3G2a"));
        assert!(!looks_like_tekken_id("2yh7ByTerD8"), "11 chars");
        assert!(!looks_like_tekken_id("123456789012"), "all digits is a score");
        assert!(!looks_like_tekken_id("MRmani"), "a name");
    }
}
