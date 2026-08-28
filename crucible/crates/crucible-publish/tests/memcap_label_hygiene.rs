//! A LIVE BUG IN THE PUBLISHED TABLE, pinned on purpose.
//!
//! `ipc67.py:490-494` (0.24 "label hygiene") emits
//! `"mem-cap (self-inflicted: node byte target raised)"` when the refill
//! re-entry raised the node byte target past the declared model.
//! `standings.py:262` matches `"mem-cap"` by EXACT EQUALITY. The labelled
//! variant matches nothing, falls past the mem-cap and spawn-fail tests, past
//! the timeout line, and lands in `early-exit` -- the class the 0.20 refill
//! loop exists to empty, and therefore the one column the refill loop is
//! refereed by.
//!
//! Seven rows are misfiled in `benchmarks/ipc-standings.md` right now:
//!   line 61, 2023 numeric:    "6 early-exit, 1 mem-cap" -> should be "0 / 7"
//!   line 52, 2014 seq-mco t4: "2 early-exit, 1 mem-cap" -> should be "1 / 2"
//! Two more sit in ipc2014-mco-t8, swept and awaiting promotion.
//!
//! Same shape as the 0.20 audit's finding that maintenance-2014's "8 rejects"
//! were ordinary timeouts wearing that costume: a label changed on one side of
//! a two-file contract and the other side kept matching the old one.
//!
//! WHY IT IS PINNED RATHER THAN FIXED: a port that changes a number cannot
//! prove it is a port. crucible ships bug-compatible so byte-parity against the
//! oracle is demonstrable; the fix, its regenerated goldens and the recorded
//! -7 early-exit / +7 mem-cap movement are a SEPARATE change.
//! See docs/roadmap-0.26.md, Phase 0.
//!
//! WHEN THE FIX LANDS: flip the expectations below, and delete nothing --
//! this file becomes the record that the movement was deliberate.

mod common;
use common::incident;
use crucible_publish::{Class, Referee};

#[test]
fn the_labelled_variant_is_currently_misfiled_as_early_exit() {
    let rows = incident("memcap-self-inflicted");
    assert_eq!(rows.len(), 7, "the seven rows in the published table");

    let referee = Referee::default();
    for r in &rows {
        assert!(
            r.note_text().starts_with("mem-cap ("),
            "every fixture row carries the labelled note"
        );
        assert_eq!(
            referee.classify(r, 60.0),
            Class::EarlyExit,
            "PINNED BUG: {}/{} reads early-exit because the classifier \
             matches \"mem-cap\" exactly",
            r.variant,
            r.instance
        );
    }
}

/// The bare label still works, which is what makes the labelled one's failure
/// so quiet: the class never disappeared from the table, it just lost rows.
#[test]
fn the_bare_label_still_matches() {
    let r: crucible_publish::RawRow = serde_json::from_value(serde_json::json!({
        "variant": "v", "instance": 1, "solved": false, "time": 21.47,
        "notes": "mem-cap"
    }))
    .unwrap();
    assert_eq!(Referee::default().classify(&r, 60.0), Class::MemCap);
}

/// The exact size of the misattribution, as one number, so the fix commit can
/// assert the movement it causes rather than describing it.
#[test]
fn the_movement_the_fix_will_cause_is_seven_rows() {
    let rows = incident("memcap-self-inflicted");
    let referee = Referee::default();
    let misfiled = rows
        .iter()
        .filter(|r| referee.classify(r, 60.0) == Class::EarlyExit)
        .count();
    assert_eq!(
        misfiled, 7,
        "-7 early-exit / +7 mem-cap when the match widens"
    );
}
