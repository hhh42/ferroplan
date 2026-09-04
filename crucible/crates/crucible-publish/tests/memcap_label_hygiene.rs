//! FORMERLY A LIVE BUG IN THE PUBLISHED TABLE, pinned on purpose -- and now
//! the record that its fix was deliberate.
//!
//! `ipc67.py:490-494` (0.24 "label hygiene") emits
//! `"mem-cap (self-inflicted: node byte target raised)"` when the refill
//! re-entry raised the node byte target past the declared model.
//! `standings.py:262` matched `"mem-cap"` by EXACT EQUALITY, so the labelled
//! variant fell past the mem-cap and spawn-fail tests, past the timeout line,
//! and landed in `early-exit` -- the class the 0.20 refill loop exists to
//! empty, and therefore the one column the refill loop is refereed by.
//!
//! Seven rows of the 0.25 table were misfiled that way (the fixture here);
//! by the 0.26 cut the candidate had produced far more of the labelled note,
//! and the fix moved SIXTY rows early-exit -> mem-cap across five boards
//! (2023-numeric 56, one each on 2026-numeric, 2014-mco-t4, 2014-mco-t8,
//! propositional). Coverage untouched; attribution corrected.
//!
//! WHY IT WAS PINNED RATHER THAN FIXED: a port that changes a number cannot
//! prove it is a port. crucible shipped bug-compatible so byte-parity against
//! the oracle was demonstrable (0.26 cut, 2026-09-04: parity before AND after
//! the fix, which landed in `standings.py` and `referee.rs` the same day).
//! See docs/roadmap-0.26.md, Phase 2 and Phase 6.
//!
//! Nothing was deleted when the fix landed; the expectations below flipped.

mod common;
use common::incident;
use crucible_publish::{Class, Referee};

#[test]
fn the_labelled_variant_is_mem_cap() {
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
            Class::MemCap,
            "{}/{} carries the labelled note and is mem-cap, not early-exit \
             (the exact-equality bug, fixed 2026-09-04)",
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

/// The exact size of the misattribution on the 0.25 fixture, as one number:
/// none of the seven is early-exit any more, all seven are mem-cap.
#[test]
fn the_movement_the_fix_caused_is_seven_rows_on_this_fixture() {
    let rows = incident("memcap-self-inflicted");
    let referee = Referee::default();
    let misfiled = rows
        .iter()
        .filter(|r| referee.classify(r, 60.0) == Class::EarlyExit)
        .count();
    let memcap = rows
        .iter()
        .filter(|r| referee.classify(r, 60.0) == Class::MemCap)
        .count();
    assert_eq!((misfiled, memcap), (0, 7), "-7 early-exit / +7 mem-cap");
}
