//! A stand-in for `ff` that does exactly what a test tells it to.
//!
//! The runner's hard parts -- wall enforcement, the RSS watchdog, spawn retry,
//! process-group kills, SIGSTOP/SIGCONT accounting, orphan reaping, and
//! surviving `kill -9` with zero lost work -- are all about what happens
//! AROUND a child process, not inside it. Testing them against the real
//! planner would mean a 6,584-instance gitignored corpus, minutes per case,
//! and no way at all to ask for a process that balloons to 8 GiB or fails to
//! fork. So the tests drive this instead, and the real planner is exercised by
//! the paired differential sweep, which is the only place it belongs.
//!
//! Every knob is an environment variable so a test can set it per-spawn
//! without touching the argv the runner builds:
//!
//!   FAKEFF_SLEEP_MS      run this long before exiting (default 0)
//!   FAKEFF_SOLVED        1 = emit a solved Solution, 0 = unsolved (default 1)
//!   FAKEFF_RSS_MB        balloon to this many MiB and hold, to trip the cap
//!   FAKEFF_EXIT          exit with this code instead of the natural one
//!   FAKEFF_STDOUT_BYTES  pad stdout to this size -- a 400-step plan exceeds
//!                        the 64 KiB pipe buffer, and a supervisor that waits
//!                        without draining deadlocks there. This is how that
//!                        gets tested without a 400-step plan.
//!   FAKEFF_NOTES         a note string for the Solution
//!   FAKEFF_IGNORE_TERM   swallow SIGTERM, so kill escalation has something to
//!                        escalate against
//!   FAKEFF_METRIC        plan metric (float); omit for null
//!   FAKEFF_MAKESPAN      plan makespan (float); omit for null
//!   FAKEFF_STDERR        write this to stderr -- how the engine's own
//!                        narration (e.g. "node byte target raised") is put in
//!                        front of the classifier
//!   FAKEFF_NO_JSON       emit NOTHING on stdout, so the "no verdict and a
//!                        nonzero exit" branch has something to classify
//!
//! It lives as a bin target of `crucible-core` rather than its own crate so
//! that `cargo test -p crucible-core` always builds it and the integration
//! tests can find it through `CARGO_BIN_EXE_fakeff` instead of guessing a path.
//!
//! It also honours the same `--version` and argv shape the runner passes, so
//! the capability probe and the version gate can be exercised end to end.

use std::io::Write;

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok()
}

fn env_num<T: std::str::FromStr>(k: &str) -> Option<T> {
    env(k).and_then(|v| v.parse().ok())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version") {
        println!(
            "ff {}",
            env("FAKEFF_VERSION").unwrap_or("0.0.0-fake".into())
        );
        return;
    }
    if args.iter().any(|a| a == "--help") {
        // The backfill capability probe greps this for the --mode value list.
        println!("Usage: fakeff -o DOMAIN -f PROBLEM --json");
        println!("      --mode <auto|ff|partition|pddl3|temporal|portfolio|optimal|sat>");
        return;
    }

    if env("FAKEFF_IGNORE_TERM").is_some() {
        // SAFETY: installing SIG_IGN for SIGTERM is async-signal-safe and is
        // the whole point -- the test needs a child that survives a polite ask.
        unsafe {
            libc::signal(libc::SIGTERM, libc::SIG_IGN);
        }
    }

    // Balloon first, so a memory cap trips before any sleep elapses.
    let _ballast: Option<Vec<u8>> = env_num::<usize>("FAKEFF_RSS_MB").map(|mb| {
        let mut v = vec![0u8; mb << 20];
        // Touch every page or the pages are never resident and the RSS
        // watchdog -- which measures resident bytes, not address space --
        // correctly sees nothing.
        for i in (0..v.len()).step_by(4096) {
            v[i] = 1;
        }
        v
    });

    let solved = env("FAKEFF_SOLVED").map(|v| v != "0").unwrap_or(true);
    let notes = env("FAKEFF_NOTES");

    if let Some(ms) = env_num::<u64>("FAKEFF_SLEEP_MS") {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    let mut plan = String::from("null");
    if solved {
        let metric = env("FAKEFF_METRIC").unwrap_or("null".into());
        let makespan = env("FAKEFF_MAKESPAN").unwrap_or("null".into());
        plan = format!(
            r#"{{"steps":[{{"index":0,"action":"noop","args":[],"time":null,"duration":null}}],"length":1,"metric":{metric},"makespan":{makespan}}}"#
        );
    }
    let notes_json = match &notes {
        Some(n) => format!("[{:?}]", n),
        None => "[]".to_string(),
    };
    let mut out = format!(
        r#"{{"solved":{solved},"mode":"auto","plan":{plan},"statistics":{{"grounded_facts":1,"grounded_actions":1,"evaluated_states":1,"threads":1}},"notes":{notes_json}}}"#
    );

    // Pad AFTER the JSON so the document still parses: a supervisor that reads
    // only the first pipe-buffer's worth would still see valid JSON and pass
    // for the wrong reason. Trailing whitespace keeps it honest.
    if let Some(n) = env_num::<usize>("FAKEFF_STDOUT_BYTES") {
        if n > out.len() {
            out.push_str(&" ".repeat(n - out.len()));
        }
    }

    if let Some(e) = env("FAKEFF_STDERR") {
        let _ = writeln!(std::io::stderr(), "{e}");
    }

    if env("FAKEFF_NO_JSON").is_none() {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(out.as_bytes());
        let _ = lock.write_all(b"\n");
        let _ = lock.flush();
    }

    let code = env_num::<i32>("FAKEFF_EXIT").unwrap_or(if solved { 0 } else { 1 });
    std::process::exit(code);
}
