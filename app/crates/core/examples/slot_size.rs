use murating_core::feed::{self, Extras, Store};
use murating_core::replays::Record;

fn main() {
    let path = std::env::args().nth(1).expect("records.json");
    let text = std::fs::read_to_string(&path).expect("read records");
    let value: serde_json::Value = serde_json::from_str(&text).expect("json");
    let list = value.get("records").cloned().unwrap_or(value);
    let recs: Vec<Record> = serde_json::from_value(list).expect("records");
    let now = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| recs.iter().map(|r| r.battle_at).max().unwrap_or(0));
    let span = now - recs.iter().map(|r| r.battle_at).min().unwrap_or(now);
    println!("{} records over {:.1} h, now {now}", recs.len(), span as f64 / 3600.0);

    let mut store = Store::new("nobody-local");
    store.merge(&recs);
    let extras = Extras {
        codes: (0..64).map(|id| (id, format!("c{id:02}"))).collect(),
        ..Extras::default()
    };
    let built = feed::slot_bytes(&store, now, "2000 MR", &extras);
    println!(
        "capped at {} MB: {} KB, replay rows {} ({} min), names {} ({} h)",
        feed::MAX_SLOT_BYTES / (1024 * 1024),
        built.bytes.len() / 1024,
        built.rows,
        built.horizon / 60,
        built.names,
        built.name_horizon / 3600
    );
}
