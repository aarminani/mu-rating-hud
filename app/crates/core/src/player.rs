use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct Rating {
    pub character: String,
    pub mu: i32,
    pub sigma: Option<i32>,
    pub games: Option<i32>,
    pub last_seen: Option<i64>,
    pub group: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PlayerError {
    #[error("no rating block on the page, Wavu's markup may have changed")]
    NoRatings,
    #[error("player not found on Wavu")]
    NotFound,
    #[error("network: {0}")]
    Net(String),
}

fn between<'a>(hay: &'a str, from: usize, open: &str, close: &str) -> Option<(&'a str, usize)> {
    let s = hay.get(from..)?;
    let a = s.find(open)? + open.len();
    let rest = s.get(a..)?;
    let b = rest.find(close)?;
    Some((&rest[..b], from + a + b + close.len()))
}

fn text_of<'a>(block: &'a str, class: &str) -> Option<&'a str> {
    let marker = format!("class=\"{class}\">");
    let i = block.find(&marker)? + marker.len();
    let rest = &block[i..];
    let j = rest.find("</div>")?;
    Some(rest[..j].trim())
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn first_int(s: &str) -> Option<i32> {
    let digits: String = s
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

pub fn parse_ratings(html: &str) -> Result<Vec<Rating>, PlayerError> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    let mut group = String::new();

    while let Some(i) = html[cursor..].find("class=\"").map(|p| cursor + p) {
        let tail = &html[i..];
        if tail.starts_with("class=\"rating-group\"") {
            if let Some(label) = between(html, i, "class=\"label\">", "</div>").map(|(t, _)| t) {
                group = unescape(label.trim());
            }
            cursor = i + "class=\"rating-group\"".len();
            continue;
        }
        if tail.starts_with("class=\"rating\">") {
            let end = html[i..]
                .find("class=\"rating\">")
                .map(|_| i + "class=\"rating\">".len())
                .unwrap_or(i);
            let next = html[end..]
                .find("class=\"rating\">")
                .map(|p| end + p)
                .unwrap_or(html.len());
            let block = &html[i..next];

            if let (Some(ch), Some(mu_txt)) = (text_of(block, "char"), text_of(block, "mu")) {
                if let Some(mu) = first_int(mu_txt) {
                    out.push(Rating {
                        character: ch.to_string(),
                        mu,
                        sigma: text_of(block, "sigma").and_then(first_int),
                        games: text_of(block, "games").and_then(first_int),
                        last_seen: block
                            .find("printDate(")
                            .and_then(|p| {
                                let r = &block[p + "printDate(".len()..];
                                r.find(')').map(|q| &r[..q])
                            })
                            .and_then(|s| s.trim().parse::<i64>().ok()),
                        group: group.clone(),
                    });
                }
            }
            cursor = end;
            continue;
        }
        cursor = i + "class=\"".len();
    }

    if out.is_empty() {
        return Err(PlayerError::NoRatings);
    }
    Ok(out)
}

pub fn parse_name(html: &str) -> Option<String> {
    let i = html.find("<title>")? + "<title>".len();
    let rest = &html[i..];
    let j = rest.find("</title>")?;
    let title = rest[..j].trim();
    let name = title
        .rsplit_once("Wavu Wank")
        .map(|(before, _)| before)
        .unwrap_or(title)
        .trim()
        .trim_end_matches(['\u{00b7}', '\u{2022}', '|', '\u{2013}', '\u{2014}', '-'])
        .trim()
        .to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

pub fn current<'a>(ratings: &'a [Rating]) -> Option<&'a Rating> {
    ratings.iter().max_by_key(|r| r.last_seen.unwrap_or(i64::MIN))
}

pub fn best<'a>(ratings: &'a [Rating]) -> Option<&'a Rating> {
    ratings
        .iter()
        .max_by_key(|r| (r.mu, r.last_seen.unwrap_or(i64::MIN)))
}

pub fn parse_steam_id(html: &str) -> Option<String> {
    const LINK: &str = "steamcommunity.com/profiles/";
    let i = html.find(LINK)? + LINK.len();
    let id: String = html[i..].chars().take_while(|c| c.is_ascii_digit()).collect();
    (id.len() == 17).then_some(id)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryRow {
    pub battle_at: i64,
    pub left_id: String,
    pub left_char: String,
    pub right_id: String,
    pub right_char: String,
    pub left_name: String,
    pub right_name: String,
    pub left_rating: Option<i32>,
    pub left_change: Option<i32>,
    pub right_rating: Option<i32>,
    pub right_change: Option<i32>,
}

pub fn parse_history(html: &str) -> Vec<HistoryRow> {
    const CELL: &str = "class=\"battle-at\">";
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(p) = html[from..].find(CELL) {
        let start = from + p + CELL.len();
        from = start;
        let rest = &html[start..];
        let row = &rest[..rest.find("</tr>").unwrap_or(rest.len())];
        let cell = &row[..row.find("</td>").unwrap_or(row.len())];
        let Some(at) = cell
            .find("printDateTime(")
            .map(|q| &cell[q + "printDateTime(".len()..])
            .and_then(|a| a.find(')').map(|e| &a[..e]))
            .and_then(|s| s.trim().parse::<i64>().ok())
        else {
            continue;
        };
        type Side = (String, String, String, Option<i32>, Option<i32>);
        let side = |class: &str| -> Option<Side> {
            let marker = format!("class=\"{class}\">");
            let i = row.find(&marker)? + marker.len();
            let td = &row[i..];
            let td = &td[..td.find("</td>").unwrap_or(td.len())];
            let h = td.find("href=\"/player/")? + "href=\"/player/".len();
            let id = &td[h..];
            let name = id
                .find('>')
                .map(|g| &id[g + 1..])
                .map(|n| &n[..n.find('<').unwrap_or(n.len())])
                .unwrap_or("");
            let id = &id[..id.find('"')?];
            let c = td.find("class=\"char\">")? + "class=\"char\">".len();
            let ch = &td[c..];
            let ch = &ch[..ch.find('<')?];
            let (rating, change) = match td.find("class=\"rating\">") {
                Some(r) => {
                    let cell = &td[r + "class=\"rating\">".len()..];
                    let rating = cell[..cell.find('<').unwrap_or(cell.len())].trim().parse().ok();
                    let change = cell
                        .find("<span")
                        .and_then(|s| cell[s..].find('>').map(|g| &cell[s + g + 1..]))
                        .and_then(|v| v[..v.find('<').unwrap_or(v.len())].trim().replace('\u{2212}', "-").parse().ok());
                    (rating, change)
                }
                None => (None, None),
            };
            Some((crate::table::tekken_id(id), unescape(ch.trim()), unescape(name.trim()), rating, change))
        };
        if let (Some(l), Some(r)) = (side("left"), side("right")) {
            out.push(HistoryRow {
                battle_at: at,
                left_id: l.0,
                left_char: l.1,
                left_name: l.2,
                left_rating: l.3,
                left_change: l.4,
                right_id: r.0,
                right_char: r.1,
                right_name: r.2,
                right_rating: r.3,
                right_change: r.4,
            });
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WavuCharacter {
    pub slug: String,
    pub name: String,
}

pub fn parse_roster(html: &str) -> Vec<WavuCharacter> {
    const OPEN: &str = "<select name=\"opp_char\">";
    const OPTION: &str = "<option value=\"";
    let Some(start) = html.find(OPEN).map(|i| i + OPEN.len()) else {
        return Vec::new();
    };
    let block = &html[start..];
    let block = &block[..block.find("</select>").unwrap_or(block.len())];
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(p) = block[from..].find(OPTION) {
        let v = from + p + OPTION.len();
        let Some(q) = block[v..].find('"') else { break };
        let slug = &block[v..v + q];
        let Some(gt) = block[v + q..].find('>') else { break };
        let text = &block[v + q + gt + 1..];
        let text = &text[..text.find('<').unwrap_or(text.len())];
        from = v + q;
        let name = unescape(&text.split_whitespace().collect::<Vec<_>>().join(" "));
        if !slug.is_empty() && !name.is_empty() {
            out.push(WavuCharacter { slug: slug.to_string(), name });
        }
    }
    out
}

pub fn parse_battle_times(html: &str) -> Vec<i64> {
    const CELL: &str = "class=\"battle-at\">";
    const CALL: &str = "printDateTime(";
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(p) = html[from..].find(CELL) {
        let start = from + p + CELL.len();
        let rest = &html[start..];
        let cell = &rest[..rest.find("</td>").unwrap_or(rest.len())];
        if let Some(q) = cell.find(CALL) {
            let arg = &cell[q + CALL.len()..];
            if let Some(end) = arg.find(')') {
                if let Ok(t) = arg[..end].trim().parse::<i64>() {
                    out.push(t);
                }
            }
        }
        from = start;
    }
    out
}

pub fn fetch(base: &str, tekken_id: &str, user_agent: &str) -> Result<String, PlayerError> {
    let url = format!("{}/player/{}", base.trim_end_matches('/'), tekken_id);
    let resp = ureq::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(30))
        .build()
        .get(&url)
        .set("Accept-Encoding", "gzip")
        .call();

    match resp {
        Ok(r) => r.into_string().map_err(|e| PlayerError::Net(e.to_string())),
        Err(ureq::Error::Status(404, _)) => Err(PlayerError::NotFound),
        Err(e) => Err(PlayerError::Net(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = include_str!("../tests/fixtures/player_ratings.html");

    const HISTORY: &str = r#"<tbody>
<tr> <td class="battle-at"><time><noscript>13 Sep 26 1:08</noscript><script>printDateTime(1789261702)</script></time></td> <td class="left"> <span class="player"> <a href="/player/aaaaaaaaaaaa">Player A</a> </span> <span class="char">Yoshimitsu</span> <span class="rating"> 2244 <span class="win"> +5 </span> </span> </td> <td class="result"> 3-1 </td> <td class="right"> <span class="rating"> 2067 <span class="lose"> -6 </span> </span> <span class="char">Lili</span> <span class="player"> <a href="/player/bbbbbbbbbbbb">Player B</a> </span> </td> </tr>
<tr> <td class="battle-at"><time><noscript>13 Sep 26 1:05</noscript><script>printDateTime(1789261542)</script></time></td> <td class="left"> <span class="rating"> 2238 </span> </td> </tr>
<tr> <td class="battle-at"><time><noscript>no script</noscript></time></td> <td class="left"> <script>printDateTime(1111111111)</script> </td> </tr>
</tbody>"#;

    #[test]
    fn roster_comes_from_the_opponent_filter() {
        let roster = parse_roster(include_str!("../tests/fixtures/opp_char_select.html"));
        assert_eq!(roster.len(), 42, "the whole ranked cast, not 'All characters'");
        assert_eq!(roster[0], WavuCharacter { slug: "alisa".into(), name: "Alisa".into() });
        assert!(roster.contains(&WavuCharacter { slug: "ak".into(), name: "Armor King".into() }));
        assert!(roster.contains(&WavuCharacter { slug: "miary".into(), name: "Miary Zo".into() }));
        assert!(roster.contains(&WavuCharacter { slug: "jack".into(), name: "Jack-8".into() }));
    }

    #[test]
    fn roster_ignores_the_players_own_filter() {
        let html = r#"<select name="char"> <option value="">All characters</option> <option value="leroy" > Leroy (2376) </option> </select>"#;
        assert!(parse_roster(html).is_empty());
        assert!(parse_roster("<html></html>").is_empty());
    }

    #[test]
    fn steam_id_comes_from_the_header_link() {
        let header = r#"<span class="platform"> <svg class="platform-icon"></svg> <a href="https://steamcommunity.com/profiles/76561198087654321">76561198087654321</a> </span> <span class="region">"#;
        assert_eq!(parse_steam_id(header).as_deref(), Some("76561198087654321"));
        assert_eq!(parse_steam_id(r#"<span class="platform">psn</span>"#), None);
    }

    #[test]
    fn history_rows_carry_both_players_and_characters() {
        let rows = parse_history(HISTORY);
        assert_eq!(rows, vec![HistoryRow {
            battle_at: 1789261702,
            left_id: "aaaaaaaaaaaa".into(),
            left_char: "Yoshimitsu".into(),
            right_id: "bbbbbbbbbbbb".into(),
            right_char: "Lili".into(),
            left_name: "Player A".into(),
            right_name: "Player B".into(),
            left_rating: Some(2244),
            left_change: Some(5),
            right_rating: Some(2067),
            right_change: Some(-6),
        }], "the half row and the timeless row are skipped");
    }

    #[test]
    fn battle_times_come_from_the_match_history() {
        assert_eq!(parse_battle_times(HISTORY), vec![1789261702, 1789261542]);
        assert!(parse_battle_times(PAGE).iter().all(|&t| t > 1_700_000_000));
    }

    #[test]
    fn parses_the_real_page() {
        let rs = parse_ratings(PAGE).expect("ratings parse");
        assert!(rs.len() >= 4, "only {} ratings", rs.len());

        let anna = rs.iter().find(|r| r.character == "Anna").expect("Anna present");
        assert_eq!(anna.mu, 2248);
        assert_eq!(anna.sigma, Some(75));
        assert_eq!(anna.games, Some(2010));
        assert!(anna.group.starts_with("Leaderboard"), "got {:?}", anna.group);
        assert!(anna.last_seen.unwrap() > 1_700_000_000);
    }

    #[test]
    fn emits_each_rating_exactly_once() {
        let rs = parse_ratings(PAGE).unwrap();
        assert_eq!(rs.len(), 18, "got {:?}", rs.iter().map(|r| &r.character).collect::<Vec<_>>());

        let mut seen = std::collections::HashSet::new();
        for r in &rs {
            assert!(
                seen.insert((r.character.clone(), r.group.clone())),
                "{} appears twice in {}",
                r.character,
                r.group
            );
        }
    }

    #[test]
    fn match_history_is_not_mistaken_for_a_rating() {
        let rs = parse_ratings(PAGE).unwrap();
        assert!(
            rs.iter().all(|r| !r.character.is_empty() && r.mu > 500),
            "a history row leaked in: {:?}",
            rs.iter().filter(|r| r.mu <= 500).collect::<Vec<_>>()
        );
    }

    #[test]
    fn keeps_characters_apart() {
        let rs = parse_ratings(PAGE).unwrap();
        let mu = |n: &str| rs.iter().find(|r| r.character == n).map(|r| r.mu);
        assert_eq!(mu("Anna"), Some(2248));
        assert_eq!(mu("Nina"), Some(2205));
        assert_eq!(mu("Lili"), Some(2092));
        assert_eq!(mu("Bob"), Some(1637));
    }

    #[test]
    fn current_is_most_recent_not_highest() {
        let rs = parse_ratings(PAGE).unwrap();
        let cur = current(&rs).expect("a current character");
        assert_eq!(cur.character, "Lili");
        assert_ne!(cur.mu, 2248, "must not pick the best character");
    }

    #[test]
    fn unqualified_group_is_labelled() {
        let rs = parse_ratings(PAGE).unwrap();
        assert!(
            rs.iter().any(|r| r.group.starts_with("Unqualified")),
            "groups seen: {:?}",
            rs.iter().map(|r| &r.group).collect::<Vec<_>>()
        );
    }

    #[test]
    fn best_is_highest_not_most_recent() {
        let rs = parse_ratings(PAGE).unwrap();
        let b = best(&rs).expect("a best character");
        assert_eq!(b.character, "Anna");
        assert_eq!(b.mu, 2248);
        assert_ne!(b.character, current(&rs).unwrap().character);
    }

    #[test]
    fn name_comes_off_the_title() {
        assert_eq!(
            parse_name("<head><title>MRmani \u{00b7} Wavu Wank</title></head>").as_deref(),
            Some("MRmani")
        );
        assert_eq!(
            parse_name("<title>Steel\u{200b}Mantis \u{00b7} Wavu Wank</title>").as_deref(),
            Some("Steel\u{200b}Mantis")
        );
        assert_eq!(parse_name("<html></html>"), None);
    }

    #[test]
    fn a_redesign_fails_loudly() {
        assert!(matches!(
            parse_ratings("<html><body><p>2248</p></body></html>"),
            Err(PlayerError::NoRatings)
        ));
    }
}
