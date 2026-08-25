//! THE FIFTEEN INSTANCES LIGHT.
//!
//! VAL has several ways to refuse a domain -- "Parser failed", "Problem in
//! domain definition!", "Type problem in problem specification!" -- and it
//! emits every one of them BEFORE judging any plan. A `val: false` row on such
//! a domain is validation UNAVAILABLE, not a rejected plan.
//!
//! The 0.20 runner tested for the first signature only. So `data-network-2018`
//! (7 rows) and `factory-robot-2026` (8 rows) arrived in the raws as
//! `val: false`, `standings.py` drops those from coverage, and the published
//! table read **46/240 and 113/320 where the boards beside it said 53 and 121**
//! -- fifteen instances light, for a cycle, on a released record.
//!
//! Defends: standings.py:211-241 (`_load_val_unavailable`, `solved`),
//! ipc67.py:291-306 (`VAL_UNAVAILABLE_SIGNATURES`).
//!
//! The fixture is the actual fifteen rows, rescued from `benchmarks/air/`,
//! which `.gitignore` excludes -- before that rescue this test could not have
//! been written at all. See crucible/tests/fixtures/README.md.

mod common;
use common::{incident, real_val_map};
use crucible_publish::{Class, Referee, ValUnavailable};

#[test]
fn the_fifteen_are_coverage_when_the_map_is_read() {
    let rows = incident("val-unavailable-15");
    assert_eq!(rows.len(), 15, "the fixture is the incident: fifteen rows");

    let referee = Referee::new(real_val_map());
    let cov = referee.coverage(&rows, 60.0);

    assert_eq!(
        cov.solved, 15,
        "every one of these rows produced a plan; VAL merely could not read \
         the domain to judge it"
    );
    assert_eq!(
        cov.classes.get(&Class::ValRed),
        None,
        "none of them is a VAL-RED"
    );
}

#[test]
fn the_fifteen_vanish_when_the_map_is_not_read() {
    let rows = incident("val-unavailable-15");
    let blind = Referee::default();
    let cov = blind.coverage(&rows, 60.0);

    assert_eq!(
        cov.solved, 0,
        "this is the bug: fifteen solves become nothing"
    );
    assert_eq!(cov.classes[&Class::ValRed], 15);
}

/// The delta IS the incident. Stated as one number so a regression cannot hide
/// behind a partially-correct map.
#[test]
fn the_map_is_worth_exactly_fifteen_instances() {
    let rows = incident("val-unavailable-15");
    let with = Referee::new(real_val_map()).coverage(&rows, 60.0).solved;
    let without = Referee::default().coverage(&rows, 60.0).solved;
    assert_eq!(with - without, 15);
}

/// The other side of the line: a plan VAL read and REJECTED, on a domain VAL
/// ingests perfectly well. Same `solved: true, val: false` shape as the fifteen
/// -- only the map tells them apart, which is why both fixtures are kept.
#[test]
fn a_real_rejection_stays_a_rejection() {
    let rows = incident("val-red-map-analyzer");
    assert_eq!(rows.len(), 3);

    let referee = Referee::new(real_val_map());
    let cov = referee.coverage(&rows, 60.0);

    assert_eq!(cov.solved, 0, "VAL read these and said no");
    assert_eq!(cov.classes[&Class::ValRed], 3);
    assert_eq!(
        cov.failure_classes(),
        "3 VAL-RED",
        "a first-class signal, never lumped into search losses"
    );
}

/// markettrader's instances init undeclared fluents, so VAL's TYPECHECKER
/// refuses the PROBLEM -- "Type problem in problem specification!". 0.21 was
/// missing that signature and booked the board's only VAL-RED through the gap.
#[test]
fn the_typechecker_refusal_is_also_not_a_verdict() {
    let rows = incident("val-false-markettrader");
    assert_eq!(rows.len(), 1);

    assert_eq!(Referee::new(real_val_map()).coverage(&rows, 60.0).solved, 1);
    assert_eq!(Referee::default().coverage(&rows, 60.0).solved, 0);
}

/// The exemption is per (ipc, variant), so a domain that is unavailable in one
/// competition does not excuse a rejection in another.
#[test]
fn the_exemption_is_scoped_to_its_competition() {
    let rows = incident("val-red-map-analyzer");
    let wrong_competition = ValUnavailable::new(
        // The right variant name, the wrong ipc.
        ["ipc-2011/map-analyzer-temporal-satisficing".to_string()],
    );
    assert_eq!(
        Referee::new(wrong_competition).coverage(&rows, 60.0).solved,
        0,
        "a key that differs only in its competition must not excuse these rows"
    );
}
