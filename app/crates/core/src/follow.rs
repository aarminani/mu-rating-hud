#[derive(Debug, Clone, PartialEq)]
pub struct Paired {
    pub tekken_id: String,
    pub steam_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Want {
    Setup,
    Waiting { expected: Option<String> },
    Unpaired,
    Follow(String),
}

pub struct Inputs<'a> {
    pub accounts: &'a [Paired],
    pub game_running: bool,
    pub signed_in: Option<&'a str>,
    pub following: Option<&'a str>,
    pub pinned: Option<&'a str>,
    pub last_followed: Option<&'a str>,
}

pub fn want(i: &Inputs) -> Want {
    if i.accounts.is_empty() {
        return Want::Setup;
    }
    let paired = i
        .signed_in
        .and_then(|s| i.accounts.iter().find(|a| a.steam_id.as_deref() == Some(s)));
    let known = |id: &str| i.accounts.iter().any(|a| a.tekken_id == id);
    let fallback = i
        .following
        .filter(|id| known(id))
        .or(i.last_followed.filter(|id| known(id)))
        .map(str::to_string);

    if !i.game_running {
        return match i.following.filter(|id| known(id)) {
            Some(id) => Want::Follow(id.to_string()),
            None => Want::Waiting {
                expected: paired.map(|a| a.tekken_id.clone()).or(fallback),
            },
        };
    }
    if let Some(pin) = i.pinned.filter(|id| known(id)) {
        return Want::Follow(pin.to_string());
    }
    match (i.signed_in, paired) {
        (_, Some(a)) => Want::Follow(a.tekken_id.clone()),
        (Some(_), None) => Want::Unpaired,
        (None, None) => match fallback {
            Some(id) => Want::Follow(id),
            None => Want::Follow(i.accounts[0].tekken_id.clone()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accounts() -> Vec<Paired> {
        vec![
            Paired { tekken_id: "main".into(), steam_id: Some("111".into()) },
            Paired { tekken_id: "alt".into(), steam_id: Some("222".into()) },
            Paired { tekken_id: "psn".into(), steam_id: None },
        ]
    }

    fn run(game: bool, signed_in: Option<&str>, following: Option<&str>, pinned: Option<&str>, last: Option<&str>) -> Want {
        let a = accounts();
        want(&Inputs { accounts: &a, game_running: game, signed_in, following, pinned, last_followed: last })
    }

    #[test]
    fn table() {
        use Want::*;
        let f = |s: &str| Follow(s.to_string());
        let cases: Vec<(&str, Want, Want)> = vec![
            ("game closed, nobody followed: wait for the signed-in account",
                run(false, Some("222"), None, None, Some("main")), Waiting { expected: Some("alt".into()) }),
            ("game closed, unpaired Steam: wait, expecting the last followed",
                run(false, Some("999"), None, None, Some("main")), Waiting { expected: Some("main".into()) }),
            ("game closed while following: stay",
                run(false, Some("222"), Some("main"), None, None), f("main")),
            ("game running, paired: follow it",
                run(true, Some("222"), None, None, None), f("alt")),
            ("game running, Steam switched while following: follow the new one",
                run(true, Some("222"), Some("main"), None, None), f("alt")),
            ("manual pick sticks",
                run(true, Some("111"), Some("psn"), Some("psn"), None), f("psn")),
            ("unpaired Steam account",
                run(true, Some("999"), Some("main"), None, None), Unpaired),
            ("nobody signed in: last followed",
                run(true, None, None, None, Some("alt")), f("alt")),
            ("nobody signed in, no history: first",
                run(true, None, None, None, None), f("main")),
            ("a removed account is not followed",
                run(false, None, Some("gone"), None, Some("gone")), Waiting { expected: None }),
        ];
        for (why, got, expected) in cases {
            assert_eq!(got, expected, "{why}");
        }
        let none: Vec<Paired> = Vec::new();
        assert_eq!(
            want(&Inputs { accounts: &none, game_running: true, signed_in: Some("111"), following: None, pinned: None, last_followed: None }),
            Setup
        );
    }
}
