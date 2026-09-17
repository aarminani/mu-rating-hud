use std::process::ExitCode;

use murating_core::paths;
use murating_core::slot::{self, DisplayMode, SlotData};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |f: &str| args.iter().any(|a| a == f);
    let val = |f: &str| {
        args.iter()
            .position(|a| a == f)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };

    let dry = has("-n") || has("--dry-run");
    let account_mode = has("--account");
    let payload_only = has("--payload-only");
    let mr: i32 = val("--mr")
        .and_then(|s| s.parse().ok())
        .unwrap_or(2304);
    let payload = val("--payload").unwrap_or_else(|| format!("MR OK {mr}"));

    let root = paths::saves_root();
    if !root.is_dir() {
        eprintln!("ABORT: no SaveGames directory at {}", root.display());
        return ExitCode::FAILURE;
    }

    let (dir, where_) = if account_mode {
        match paths::active_account(&root) {
            Ok(d) => {
                let w = format!(
                    "account {} — DIAGNOSTIC, the game does not read here",
                    paths::account_id(&d)
                );
                (d, w)
            }
            Err(e) => {
                eprintln!("ABORT: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        let who = paths::active_account(&root)
            .map(|d| paths::account_id(&d))
            .unwrap_or_else(|_| "unknown".into());
        (
            paths::slot_dir(),
            format!("SaveGames root (active account {who})"),
        )
    };

    let blob = if payload_only {
        slot::payload_only_bytes(&payload)
    } else {
        SlotData {
            owner_tekken_id: "2yh7ByTerD8a".into(),
            written_at: now_unix(),
            self_mr: mr,
            self_chara: 37,
            self_by_chara: vec![(37, 2248), (33, 2108)],
            candidates: vec![],
            prev_mr: mr - 8,
            change_id: 1,
            display_mode: DisplayMode::ShowBoth,
            payload: payload.clone(),
        }
        .to_bytes()
    };

    println!("{}", "=".repeat(70));
    println!("target   {}", dir.join(slot::SLOT_FILE).display());
    println!("where    {where_}");
    println!("shape    {}", if payload_only { "Payload only (proven)" } else { "full schema" });
    println!("payload  {payload:?}");
    println!("size     {} bytes, header {:?}", blob.len(), &blob[0..4]);

    if dry {
        println!("\ndry run - nothing written");
        return ExitCode::SUCCESS;
    }

    match slot::write_slot(&dir, &blob) {
        Ok(p) => {
            println!("\nwritten. {} bytes to {}", blob.len(), p.display());
            println!("{}", "=".repeat(70));
            println!("Launch, load any match, read the badge.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("\nABORT: {e}");
            ExitCode::FAILURE
        }
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
