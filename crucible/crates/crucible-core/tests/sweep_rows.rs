//! What a measured row says, against a planner that does exactly what it is
//! told.
//!
//! Each test is a classification the standings depend on, and each was got
//! wrong at least once in the Python this replaces. The rows here are checked
//! as BYTES, through the same writer that produces a board raw, because the
//! shape of a row -- which keys are present, which are null -- is as
//! load-bearing as its values.

use crucible_core::corpus::Instance;
use crucible_core::exec::Ctl;
use crucible_core::platform;
use crucible_core::sweep::{self, BoardCfg, Engine};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;

fn cfg(timeout: u64) -> BoardCfg {
    BoardCfg {
        timeout_secs: timeout,
        mode: None,
        jobs: 2,
        threads: 1,
        mem_gb: 0.0,
        env: BTreeMap::new(),
        extra_args: vec![],
    }
}

fn inst() -> Instance {
    // The files never have to exist: fakeff does not read them, and the
    // measurement path does not either.
    Instance {
        label: "7".into(),
        label_is_int: true,
        domain: PathBuf::from("/dev/null"),
        problem: PathBuf::from("/dev/null"),
    }
}

fn run(env: &[(&str, &str)], timeout: u64) -> crucible_core::sweep::Measured {
    for (k, v) in env {
        std::env::set_var(k, v);
    }
    let engine = Engine {
        path: PathBuf::from(env!("CARGO_BIN_EXE_fakeff")),
        ver: "ff 0.26.0".into(),
        blake3: String::new(),
    };
    let mut c = cfg(timeout);
    // fakeff reads its instructions from the environment, and the sweep
    // deliberately SCRUBS the environment -- so they are passed the way a real
    // board's hatches are: declared on the board.
    for (k, v) in env {
        c.env.insert((*k).into(), (*v).into());
    }
    let (_tx, rx) = mpsc::channel::<Ctl>();
    let out = sweep::measure(
        &engine,
        &c,
        "ipc-2014",
        "barman-sequential-satisficing",
        &inst(),
        None,
        &std::env::temp_dir().join("crucible-test-plans"),
        &platform::host(),
        &rx,
        None,
    );
    for (k, _) in env {
        std::env::remove_var(k);
    }
    out
}

fn bytes(m: &crucible_core::sweep::Measured) -> String {
    let mut s = String::new();
    crucible_publish::write_row(&m.row, &mut s);
    s
}

/// A solved row carries `makespan`, even when the plan is not temporal and the
/// value is null. An unsolved row omits the key ENTIRELY -- it enters the
/// Python record via `rec.update()` on the solved branch only, which is why it
/// lands last and why no unsolved row in the whole corpus has it.
#[test]
fn a_solved_row_carries_makespan_and_an_unsolved_one_omits_it() {
    let solved = run(&[("FAKEFF_SOLVED", "1")], 30);
    assert!(solved.row.solved);
    let b = bytes(&solved);
    assert!(b.contains(r#""makespan": null"#), "{b}");
    assert!(b.ends_with(r#""makespan": null}"#), "makespan is LAST: {b}");

    let unsolved = run(&[("FAKEFF_SOLVED", "0")], 30);
    assert!(!unsolved.row.solved);
    let b = bytes(&unsolved);
    assert!(
        !b.contains("makespan"),
        "an unsolved row has no makespan key: {b}"
    );
}

/// NEVER CLASSIFY ON THE EXIT CODE. `ff` exits 1 for "goal simplifies to TRUE,
/// the empty plan solves it", so a trivially-solved problem looks like a
/// failure from the outside. The `--json` `solved` field is the verdict.
#[test]
fn a_solved_run_that_exits_nonzero_is_still_solved() {
    let m = run(&[("FAKEFF_SOLVED", "1"), ("FAKEFF_EXIT", "1")], 30);
    assert!(
        m.row.solved,
        "the JSON said solved; the exit code is not a verdict"
    );
    assert!(m.row.notes.is_none() || !m.row.note_text().starts_with("engine-exit"));
}

/// A deadline kill records the BUDGET as the time, as an INTEGER -- which is
/// why `time` is polymorphic in every raw on this box, and why the standings'
/// timeout class lands exactly on the 90% line rather than just under it.
#[test]
fn a_timeout_records_the_budget_as_an_integer() {
    let m = run(&[("FAKEFF_SLEEP_MS", "60000")], 1);
    let b = bytes(&m);
    assert!(b.contains(r#""time": 1,"#), "an integer, not 1.0: {b}");
    assert!(!m.row.solved);
}

/// The 0.24 label-hygiene note. The engine's own narration says the byte target
/// was raised past the declared model, so the row says the cap was
/// self-inflicted rather than merely hit.
#[test]
fn a_self_inflicted_mem_cap_says_so() {
    let m = run(
        &[
            ("FAKEFF_SOLVED", "0"),
            ("FAKEFF_EXIT", "1"),
            (
                "FAKEFF_STDERR",
                "allocation of 8589934592 bytes failed; node byte target raised",
            ),
        ],
        30,
    );
    assert_eq!(
        m.row.note_text(),
        "mem-cap (self-inflicted: node byte target raised)",
        "the engine's own narration says the target was raised past the \
         declared model, and the label must say so"
    );

    // Without the narration it is an ordinary mem-cap.
    let plain = run(
        &[
            ("FAKEFF_SOLVED", "0"),
            ("FAKEFF_EXIT", "1"),
            ("FAKEFF_STDERR", "allocation of 8589934592 bytes failed"),
        ],
        30,
    );
    assert_eq!(plain.row.note_text(), "mem-cap");
}

/// An engine that produced no JSON and exited nonzero is a real error, and the
/// row names the exit rather than pretending the search merely failed.
#[test]
fn no_json_and_a_nonzero_exit_is_a_named_engine_exit() {
    let m = run(&[("FAKEFF_EXIT", "3"), ("FAKEFF_NO_JSON", "1")], 30);
    assert_eq!(m.row.note_text(), "engine-exit-3");
    assert!(!m.row.solved);
    assert!(
        m.row.time_secs().is_some(),
        "elapsed is recorded for these too"
    );

    // A clean "searched and found nothing" JSON verdict is NOT an engine error,
    // however the process exited. That distinction is the difference between a
    // search loss and a bug.
    let searched = run(&[("FAKEFF_SOLVED", "0")], 30);
    assert!(!searched.row.note_text().starts_with("engine-exit"));
}

/// Elapsed is recorded for UNSOLVED rows. Before 0.20 it was not, so a
/// graceful engine exit at the armed wall left time=None and the standings
/// classed it engine-reject -- maintenance-2014's "8 rejects" were ordinary
/// timeouts wearing that costume.
#[test]
fn unsolved_rows_carry_an_honest_clock() {
    let m = run(&[("FAKEFF_SOLVED", "0"), ("FAKEFF_SLEEP_MS", "300")], 30);
    let t = m.row.time_secs().expect("unsolved rows record elapsed");
    assert!(t >= 0.25, "got {t}");
}

/// Without a validator, `val` is NULL -- validation UNAVAILABLE. Never false.
/// Reading that as a rejection is the incident that made the table read
/// fifteen instances light.
#[test]
fn no_validator_means_unavailable_not_rejected() {
    let m = run(&[("FAKEFF_SOLVED", "1")], 30);
    assert_eq!(m.row.val, None);
    assert!(bytes(&m).contains(r#""val": null"#));
}

/// Every row carries the tuple the resume gate compares. A row missing any of
/// them can never be reused -- fail-closed, by construction.
#[test]
fn every_row_is_stamped_with_its_run_parameters() {
    let m = run(&[("FAKEFF_SOLVED", "1")], 45);
    let b = bytes(&m);
    for stamp in [
        r#""budget": 45"#,
        r#""ver": "ff 0.26.0""#,
        r#""mode": "auto""#,
        r#""jobs": 2"#,
        r#""threads": "1""#,
    ] {
        assert!(b.contains(stamp), "missing {stamp} in {b}");
    }
    assert!(b.contains(r#""start_ts""#) && b.contains(r#""end_ts""#));
}

/// The row is byte-identical in SHAPE to what the Python writes: the same keys,
/// in the same order, with the same spacing.
#[test]
fn the_row_has_the_shape_every_committed_board_has() {
    let m = run(&[("FAKEFF_SOLVED", "1")], 60);
    let b = bytes(&m);
    let keys: Vec<&str> = b
        .trim_matches(['{', '}'])
        .split(", \"")
        .map(|p| p.split("\":").next().unwrap().trim_matches('"'))
        .collect();
    assert_eq!(
        keys,
        [
            "ipc", "variant", "instance", "solved", "time", "metric", "length", "val", "notes",
            "budget", "ver", "mode", "jobs", "threads", "start_ts", "end_ts", "makespan"
        ],
        "got {b}"
    );
}
