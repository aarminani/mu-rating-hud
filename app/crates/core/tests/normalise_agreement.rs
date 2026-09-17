use murating_core::table;

const FIXTURE: &str = include_str!("fixtures/normalise.json");

fn pairs(section: &str) -> Vec<(String, String)> {
    let doc: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    doc[section]
        .as_array()
        .expect("section is an array")
        .iter()
        .map(|row| {
            (
                row[0].as_str().unwrap().to_string(),
                row[1].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn key_agrees_with_python() {
    let mut bad = Vec::new();
    for (input, expected) in pairs("key") {
        let got = table::key(&input);
        if got != expected {
            bad.push(format!("key({input:?}): rust {got:?} != python {expected:?}"));
        }
    }
    assert!(bad.is_empty(), "{} disagreement(s):\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn char_key_agrees_with_python() {
    let mut bad = Vec::new();
    for (input, expected) in pairs("char_key") {
        let got = table::char_key(&input);
        if got != expected {
            bad.push(format!("char_key({input:?}): rust {got:?} != python {expected:?}"));
        }
    }
    assert!(bad.is_empty(), "{} disagreement(s):\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn tekken_id_agrees_with_python() {
    let mut bad = Vec::new();
    for (input, expected) in pairs("tekken_id") {
        let got = table::tekken_id(&input);
        if got != expected {
            bad.push(format!("tekken_id({input:?}): rust {got:?} != python {expected:?}"));
        }
    }
    assert!(bad.is_empty(), "{} disagreement(s):\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn the_fixture_actually_covers_the_hazards() {
    let ks = pairs("key");
    assert!(ks.iter().any(|(i, _)| i.contains('\u{200b}')), "zero-width case missing");
    assert!(ks.iter().any(|(i, _)| i.contains('\u{ff2d}')), "full-width case missing");
    assert!(ks.iter().any(|(i, _)| i.contains('\u{00df}')), "casefold case missing");
    assert!(ks.len() >= 20, "fixture shrank to {} rows", ks.len());
}
