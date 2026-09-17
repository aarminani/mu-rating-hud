use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use murating_core::replays::{self, Table, WINDOW_SECS};
use murating_core::{paths, slot};

const BASE: &str = "https://wank.wavu.wiki";
const UA: &str = concat!("MuRatingHelper/", env!("CARGO_PKG_VERSION"), " (+https://tekkenresourcehub.com; Tekken 8 MR badge)");
const REQUEST_CAP: usize = 40;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let all = |f: &str| -> Vec<String> {
        args.windows(2).filter(|w| w[0] == f).map(|w| w[1].clone()).collect()
    };
    let one = |f: &str| all(f).into_iter().next();
    let dry = args.iter().any(|a| a == "-n" || a == "--dry-run");

    let Some(payload) = one("--payload") else {
        eprintln!("--payload is required (the text the shipping badge draws, e.g. \"2248 MR\")");
        return ExitCode::from(2);
    };

    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let mut ranges: Vec<(i64, i64)> = Vec::new();
    if let Some(h) = one("--hours").and_then(|s| s.parse::<f64>().ok()) {
        ranges.push((now - (h * 3600.0) as i64, now));
    }
    for r in all("--range") {
        let parts: Vec<i64> = r.split('-').filter_map(|p| p.trim().parse().ok()).collect();
        if parts.len() == 2 && parts[0] < parts[1] {
            ranges.push((parts[0], parts[1]));
        } else {
            eprintln!("ignoring bad --range {r:?} (want FROM-TO unix seconds)");
        }
    }
    if ranges.is_empty() {
        eprintln!("give --hours N and/or --range FROM-TO");
        return ExitCode::from(2);
    }

    let mut table = Table::default();
    let mut requests = 0usize;
    for (from, to) in &ranges {
        let mut before = *to;
        loop {
            if requests >= REQUEST_CAP {
                eprintln!("request cap ({REQUEST_CAP}) reached - stopping early");
                break;
            }
            if requests > 0 {
                std::thread::sleep(Duration::from_secs(1));
            }
            match replays::fetch_window(BASE, Some(before), UA) {
                Ok(recs) => {
                    requests += 1;
                    table.add_records(&recs);
                    println!("  window ending {before}: {} records, table {} rows", recs.len(), table.len());
                }
                Err(e) => {
                    eprintln!("fetch failed for window ending {before}: {e}");
                    return ExitCode::from(1);
                }
            }
            let lower = before - WINDOW_SECS;
            if lower <= *from {
                break;
            }
            before = lower;
        }
    }

    let mr = table.mr_entries();
    let delta = if args.iter().any(|a| a == "--with-delta") { table.delta_entries() } else { Vec::new() };
    let bytes = slot::replay_bytes(&payload, &mr, &delta);
    let max_mb: f64 = one("--max-mb").and_then(|s| s.parse().ok()).unwrap_or(4.0);

    println!("requests       {requests}");
    println!("sides seen     {}", table.sides_seen);
    println!("below GoD      {}", table.below_gate);
    println!("unrated        {}", table.unrated);
    println!("rows           {}", table.len());
    println!("ambiguous keys {} (dropped, never guessed)", table.ambiguous_count());
    println!("slot bytes     {} ({:.2} MB)", bytes.len(), bytes.len() as f64 / 1_048_576.0);
    for k in all("--check") {
        match table.get(&k) {
            Some((b, c)) => println!("check {k:<28} -> {b} MR  change {}", c.map(|c| format!("{c:+}")).unwrap_or("?".into())),
            None => println!("check {k:<28} -> NOT FOUND"),
        }
    }

    if dry {
        println!("dry run: nothing written");
        return ExitCode::SUCCESS;
    }
    if bytes.len() as f64 / 1_048_576.0 > max_mb {
        eprintln!("slot would be {:.2} MB, over --max-mb {max_mb} - not written", bytes.len() as f64 / 1_048_576.0);
        return ExitCode::from(1);
    }
    match slot::write_slot(&paths::slot_dir(), &bytes) {
        Ok(p) => {
            println!("wrote {}", p.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("write refused: {e}");
            ExitCode::from(1)
        }
    }
}
