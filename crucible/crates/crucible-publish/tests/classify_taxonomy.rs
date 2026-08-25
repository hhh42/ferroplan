//! The failure-class taxonomy, boundary by boundary.
//!
//! Every line in `classify()` was moved at least once for a recorded reason.
//! This file holds one test per reason, so moving one back fails loudly rather
//! than quietly re-attributing rows in a published table.
//!
//! Defends: standings.py:243-293 and :403-426. The three budget-stamp cases are
//! ported verbatim from benchmarks/test_standings.py::ClassifyBudgetStamp --
//! the oracle's own tests, kept identical so the two implementations are pinned
//! to the same examples.

mod common;
use common::{incident, real_val_map};
use crucible_publish::{Class, RawRow, Referee};

fn row(json: serde_json::Value) -> RawRow {
    serde_json::from_value(json).expect("fixture row parses")
}

/// An unsolved row with nothing but an elapsed time.
fn bare(time: f64) -> serde_json::Value {
    serde_json::json!({"variant": "v", "instance": 1, "solved": false, "time": time})
}

// ---------------------------------------------------------------------------
// The tier move: a row's own budget stamp beats the registry.
// ---------------------------------------------------------------------------

/// Ported verbatim from test_standings.py. A registry-side budget is a LIE the
/// moment one board's raws span two tiers.
#[test]
fn stamped_row_ignores_registry_budget() {
    let r = row(serde_json::json!({
        "variant": "v", "instance": 1, "solved": false, "time": 29.5, "budget": 30
    }));
    // Registry says 60 (post-flip): the 30 s-stamped wall-exit must still read
    // timeout, not early-exit.
    assert_eq!(Referee::default().classify(&r, 60.0), Class::Timeout);
}

#[test]
fn unstamped_row_uses_registry_budget() {
    let r = row(bare(29.5));
    assert_eq!(Referee::default().classify(&r, 30.0), Class::Timeout);
    assert_eq!(Referee::default().classify(&r, 60.0), Class::EarlyExit);
}

#[test]
fn sixty_second_stamp_under_lagging_registry() {
    // The deferral window itself: registry still 30, raw already 60.
    let r = row(serde_json::json!({
        "variant": "v", "instance": 1, "solved": false, "time": 58.9, "budget": 60
    }));
    assert_eq!(Referee::default().classify(&r, 30.0), Class::Timeout);
}

/// Python's `r.get("budget") or budget` is FALSY, so a stamp of 0 falls back
/// exactly as `null` does. A truthiness test ported as `is_some()` would read
/// a zero budget as real and class every row a timeout.
#[test]
fn a_zero_budget_stamp_falls_back_like_a_missing_one() {
    let r = row(serde_json::json!({
        "variant": "v", "instance": 1, "solved": false, "time": 29.5, "budget": 0
    }));
    assert_eq!(Referee::default().classify(&r, 60.0), Class::EarlyExit);
    assert_eq!(Referee::default().budget_for(&r, 60.0), 60.0);
}

// ---------------------------------------------------------------------------
// The timeout line: 90, not 95.
// ---------------------------------------------------------------------------

/// The refill loop refuses a new round below 10% of an armed wall, so a
/// graceful exit in [90%, 100%] is a spent wall by construction. At 95 the ten
/// rows in [90%, 95%) booked as early-exit were a DEFINITIONAL gap between the
/// two lines, not give-ups -- the 0.21 decode's boundary-sliver finding.
#[test]
fn the_timeout_line_is_ninety_percent_of_budget() {
    let r = Referee::default();
    for budget in [30.0_f64, 60.0, 300.0] {
        let line = budget * 0.90;
        assert_eq!(
            r.classify(&row(bare(line - 0.001)), budget),
            Class::EarlyExit,
            "just under the line at budget {budget}"
        );
        assert_eq!(
            r.classify(&row(bare(line)), budget),
            Class::Timeout,
            "the line itself is a timeout (>=, not >) at budget {budget}"
        );
        // The sliver that used to book as early-exit at a 95% line.
        assert_eq!(
            r.classify(&row(bare(budget * 0.94)), budget),
            Class::Timeout,
            "the [90%, 95%) sliver at budget {budget}"
        );
        assert_eq!(r.classify(&row(bare(budget)), budget), Class::Timeout);
    }
}

// ---------------------------------------------------------------------------
// Precedence. Each adjacent pair, because the ORDER is load-bearing.
// ---------------------------------------------------------------------------

/// A named engine mechanism that ALSO spent its wall is a TIMEOUT. The
/// mechanism test sits after the timeout test on purpose -- the docstring
/// includes "graceful engine exits AT an armed FF_TIME_LIMIT wall".
#[test]
fn a_named_mechanism_at_the_wall_is_still_a_timeout() {
    let r = Referee::default();
    let at_wall = row(serde_json::json!({
        "variant": "v", "instance": 1, "solved": false, "time": 59.0,
        "notes": ["unsolvable at grounding: goal fact is unreachable"]
    }));
    assert_eq!(r.classify(&at_wall, 60.0), Class::Timeout);

    let early = row(serde_json::json!({
        "variant": "v", "instance": 1, "solved": false, "time": 0.01,
        "notes": ["unsolvable at grounding: goal fact is unreachable"]
    }));
    assert_eq!(r.classify(&early, 60.0), Class::EngineRejectOrError);
}

/// Pre-0.20 rows only: elapsed was not recorded for unsolved rows, so a
/// graceful exit at the armed wall was indistinguishable from a true reject.
/// The 0.20 audit showed maintenance-2014's "8 rejects" were wall-exit
/// timeouts. This branch precedes the timeout test and empties as boards
/// re-sweep.
#[test]
fn a_row_with_no_elapsed_is_a_legacy_reject() {
    let r = row(serde_json::json!({"variant": "v", "instance": 1, "solved": false}));
    assert_eq!(
        Referee::default().classify(&r, 60.0),
        Class::EngineRejectOrError
    );
}

/// A `val: false` row on a VAL-unavailable domain is SOLVED, so `is_solved`
/// must be tested BEFORE the VAL-RED branch. Swapping them turns all fifteen
/// rows of the ingest-refusal incident into loud false alarms.
#[test]
fn solved_is_tested_before_val_red() {
    let rows = incident("val-unavailable-15");
    let referee = Referee::new(real_val_map());
    for r in &rows {
        assert_eq!(referee.classify(r, 60.0), Class::Solved);
    }
}

// ---------------------------------------------------------------------------
// Polymorphic notes.
// ---------------------------------------------------------------------------

/// Engine notes are a JSON list; runner-stamped classes are a bare string.
/// A one-element list joins to that element exactly, so `["mem-cap"]` still
/// matches; a two-element list matches nothing.
#[test]
fn notes_join_to_one_text() {
    let r = Referee::default();
    let as_list = row(serde_json::json!({
        "variant": "v", "instance": 1, "solved": false, "time": 1.0,
        "notes": ["mem-cap"]
    }));
    assert_eq!(r.classify(&as_list, 60.0), Class::MemCap);

    let two = row(serde_json::json!({
        "variant": "v", "instance": 1, "solved": false, "time": 1.0,
        "notes": ["mem-cap", "and something else"]
    }));
    assert_eq!(r.classify(&two, 60.0), Class::EarlyExit);
}

/// `notes: null` and `notes: []` are indistinguishable -- Python reaches the
/// mechanism tests through `notes or ""`, and an empty list is falsy.
#[test]
fn an_empty_note_list_is_the_same_as_none() {
    let r = Referee::default();
    let empty = row(serde_json::json!({
        "variant": "v", "instance": 1, "solved": false, "time": 1.0, "notes": []
    }));
    let null = row(bare(1.0));
    assert_eq!(r.classify(&empty, 60.0), r.classify(&null, 60.0));
    assert_eq!(empty.note_text(), "");
}

// ---------------------------------------------------------------------------
// Rendering order and the attestation line.
// ---------------------------------------------------------------------------

/// `sorted(cls.items())` sorts by the LABEL STRING, and 'V' (0x56) sorts below
/// 'e' (0x65) -- so VAL-RED comes first. A sort keyed on a lowercased label, or
/// on declaration order, would silently reorder every failure cell in the table.
#[test]
fn failure_classes_render_in_codepoint_order() {
    let mut rows = vec![];
    for (i, v) in [
        serde_json::json!({"solved": true,  "val": false}), // VAL-RED
        serde_json::json!({"solved": false, "time": 1.0, "notes": "mem-cap"}), // mem-cap
        serde_json::json!({"solved": false, "time": 1.0, "notes": "spawn-fail"}), // spawn-fail
        serde_json::json!({"solved": false, "time": 59.0}), // timeout
        serde_json::json!({"solved": false, "time": 1.0}),  // early-exit
        serde_json::json!({"solved": false}),               // engine-reject
    ]
    .into_iter()
    .enumerate()
    {
        let mut o = v.as_object().unwrap().clone();
        o.insert("variant".into(), "v".into());
        o.insert("instance".into(), (i as u64).into());
        rows.push(row(serde_json::Value::Object(o)));
    }
    let cov = Referee::default().coverage(&rows, 60.0);
    assert_eq!(
        cov.failure_classes(),
        "1 VAL-RED, 1 early-exit, 1 engine-reject/error, 1 mem-cap, \
         1 spawn-fail, 1 timeout"
    );
}

/// The attestation gap, named per board: a solved row with `val: null` was
/// judged by NO external referee. 71 quiet rows at the 0.24 audit is how this
/// line got here.
#[test]
fn unattested_solves_are_named_and_appended_last() {
    let rows = vec![
        row(serde_json::json!({"variant":"v","instance":1,"solved":true,"val":null})),
        row(serde_json::json!({"variant":"v","instance":2,"solved":false,"time":59.0})),
    ];
    let cov = Referee::default().coverage(&rows, 60.0);
    assert_eq!(cov.solved, 1);
    assert_eq!(cov.unattested, 1);
    assert_eq!(
        cov.failure_classes(),
        "1 timeout, 1 solved VAL-unavailable \
         (engine-oracle only; see benchmarks/val-availability.py)"
    );
}

/// The note stands alone when there are no failures at all.
#[test]
fn the_attestation_note_stands_alone_on_a_clean_board() {
    let rows = vec![row(
        serde_json::json!({"variant":"v","instance":1,"solved":true,"val":null}),
    )];
    assert_eq!(
        Referee::default().coverage(&rows, 60.0).failure_classes(),
        "1 solved VAL-unavailable \
         (engine-oracle only; see benchmarks/val-availability.py)"
    );
}

/// A board with nothing to report says so.
#[test]
fn a_spotless_board_reports_none() {
    let rows = vec![row(
        serde_json::json!({"variant":"v","instance":1,"solved":true,"val":true}),
    )];
    assert_eq!(
        Referee::default().coverage(&rows, 60.0).failure_classes(),
        "none"
    );
}
