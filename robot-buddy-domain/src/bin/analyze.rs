//! Is it working? Reads exported sessions and prints the attempt report.
//!
//! Export from the game's parent settings ("Export session"), as often as you
//! like — each export carries the whole saved attempt history, and records
//! that appear in more than one export are counted once.
//!
//! Usage: cargo run -p robot-buddy-domain --bin analyze -- exports/*.json

use std::collections::HashSet;
use std::env;
use std::fs;

use robot_buddy_domain::learning::attempt_analysis::analyze;
use robot_buddy_domain::learning::attempt_log::AttemptRecord;

fn main() {
    let paths: Vec<String> = env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: analyze <export.json>...");
        std::process::exit(2);
    }

    let mut seen = HashSet::new();
    let mut records = Vec::new();
    for path in &paths {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("skipping {path}: {e}");
                continue;
            }
        };
        let json: serde_json::Value = match serde_json::from_str(&text) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("skipping {path}: not JSON ({e})");
                continue;
            }
        };
        let Some(history) = json.get("attemptHistory").and_then(|h| h.as_array()) else {
            eprintln!("skipping {path}: no attemptHistory (exported before the attempt log existed?)");
            continue;
        };
        let mut added = 0;
        for item in history {
            match serde_json::from_value::<AttemptRecord>(item.clone()) {
                Ok(r) => {
                    if seen.insert(r.key()) {
                        records.push(r);
                        added += 1;
                    }
                }
                Err(e) => eprintln!("{path}: skipping a record ({e})"),
            }
        }
        eprintln!("{path}: {} records, {added} new", history.len());
    }

    print!("{}", analyze(&records));
}
