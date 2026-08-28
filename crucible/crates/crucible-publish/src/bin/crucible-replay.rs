//! Expose the pure layer so the Python oracle stays runnable against it.
//!
//! Every transform crucible ports from `benchmarks/standings.py` and
//! `benchmarks/ipc67.py` is a pure function of bytes already on disk. That is
//! what makes the port checkable: the same inputs go to both implementations
//! and the outputs are diffed, over the whole historical corpus, without
//! invoking the planner once.
//!
//! If these transforms were reachable only from inside the daemon, the 44,000
//! rows of sweep history on this box would be unusable as an oracle. They are
//! not a debug convenience -- they are the gate. See
//! `benchmarks/crucible-differential.py`.
//!
//!   crucible-replay classify  --raw FILE --budget N [--val-map FILE]
//!   crucible-replay coverage  --raw FILE --budget N [--val-map FILE]
//!   crucible-replay roundtrip --raw FILE

use crucible_publish::{parse_rows, write_row, Referee, ValUnavailable};

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1).cloned())
}

fn val_map(path: Option<String>) -> ValUnavailable {
    let Some(p) = path else {
        return ValUnavailable::default();
    };
    let Ok(src) = std::fs::read_to_string(&p) else {
        // Matching standings.py: a missing map is an empty map, not an error.
        return ValUnavailable::default();
    };
    let doc: serde_json::Value = serde_json::from_str(&src).expect("val map is JSON");
    match doc["unavailable"].as_object() {
        Some(o) => ValUnavailable::new(o.keys().cloned()),
        None => ValUnavailable::default(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    let raw = arg("--raw").ok_or("--raw FILE is required")?;

    // Byte round-trip: parse every line and re-emit it. A board raw is compared
    // against the oracle's byte for byte, so anything the writer cannot
    // reproduce exactly would read as noise across the whole differential and
    // hide the real drift underneath it.
    if cmd == "roundtrip" {
        let src = std::fs::read_to_string(&raw)?;
        let rows = parse_rows(&src, &raw)?;
        let mut lines = src.lines().filter(|l| !l.trim().is_empty());
        let mut checked = 0usize;
        for r in &rows {
            let want = lines.next().ok_or("fewer source lines than parsed rows")?;
            let mut got = String::new();
            write_row(r, &mut got);
            if got != want {
                let at = want
                    .char_indices()
                    .zip(got.char_indices())
                    .find(|((_, a), (_, b))| a != b)
                    .map(|((i, _), _)| i)
                    .unwrap_or_else(|| want.len().min(got.len()));
                eprintln!(
                    "MISMATCH {raw} row {}\n  first differs at byte {at}\n  want: {}\n  got:  {}",
                    checked + 1,
                    &want[at.saturating_sub(30)..(at + 60).min(want.len())],
                    &got[at.saturating_sub(30)..(at + 60).min(got.len())]
                );
                std::process::exit(1);
            }
            checked += 1;
        }
        println!("{checked}");
        return Ok(());
    }

    let budget: f64 = arg("--budget").ok_or("--budget N is required")?.parse()?;
    let referee = Referee::new(val_map(arg("--val-map")));

    let src = std::fs::read_to_string(&raw)?;
    let rows = parse_rows(&src, &raw)?;

    match cmd.as_str() {
        "classify" => {
            // One line per row, in file order, so a diff points at the row.
            for r in &rows {
                println!(
                    "{}\t{}\t{}\t{}",
                    r.ipc.as_deref().unwrap_or(""),
                    r.variant,
                    r.instance,
                    referee.classify(r, budget)
                );
            }
        }
        "coverage" => {
            let c = referee.coverage(&rows, budget);
            println!("{}\t{}\t{}", c.solved, c.total, c.failure_classes());
        }
        other => return Err(format!("unknown command {other:?}").into()),
    }
    Ok(())
}
