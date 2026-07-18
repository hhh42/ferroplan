//! IPC-5 qualitative-preferences metric locks (the 0.7 trajectory target).
//!
//! These instances carry PDDL3 `(:constraints ...)` trajectory PREFERENCES
//! (always / sometime / at-most-once / sometime-before, all soft) on top of
//! soft goals. `Options::default()` routes them through the constraint gate
//! (monitor compilation, `constraints.rs`) into the PDDL3 metric optimizer;
//! lower metric is better.
//!
//! Unlike the simple-preferences suite, the verifier is authoritative on
//! EVERY domain here: the metrics are pure weighted `(is-violated ...)`
//! sums, and `constraints::expand` grounds the formula-level quantifiers, so
//! `verify::verify`'s trajectory folds score each constraint preference
//! exactly. reported == verified is asserted per instance, not sampled.

use ferroplan::{solve, Options};
use std::fs;

fn base() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../benchmarks/ipc/qualpref")
}

/// Solve with defaults; return (reported metric, plan steps).
fn run(domain: &str, problem: &str) -> (f64, Vec<(String, Vec<String>)>) {
    let d = fs::read_to_string(format!("{}/{domain}/domain.pddl", base())).unwrap();
    let p = fs::read_to_string(format!("{}/{domain}/{problem}.pddl", base())).unwrap();
    let sol = solve(&d, &p, &Options::default()).unwrap();
    assert!(sol.solved, "{domain}/{problem} should solve");
    let plan = sol.plan.expect("plan");
    let m = plan.metric.expect("metric reported");
    let steps = plan
        .steps
        .iter()
        .map(|s| (s.action.clone(), s.args.clone()))
        .collect();
    (m, steps)
}

#[test]
#[ignore = "heavy IPC qualitative-preferences solve; opt-in via --include-ignored"]
fn qual_reported_equals_verified_everywhere() {
    // The 0.7 oracle gate: on every qualitative domain's p01, the reported
    // metric must equal the independent verifier's trajectory-replay metric,
    // and the plan must meet the hard goal with no HARD constraint violated
    // (this suite's constraints are all soft — constraints_met pins that the
    // fold machinery agrees nothing hard was smuggled in).
    for d in ["openstacks", "rovers", "storage", "tpp", "trucks"] {
        let dom = fs::read_to_string(format!("{}/{d}/domain.pddl", base())).unwrap();
        let prob = fs::read_to_string(format!("{}/{d}/p01.pddl", base())).unwrap();
        let (reported, steps) = run(d, "p01");
        let v = ferroplan::verify::verify(&dom, &prob, &steps).unwrap();
        assert!(v.hard_goal_met, "{d}/p01 plan must meet the hard goal");
        assert!(v.constraints_met, "{d}/p01: {:?}", v.constraint_failures);
        assert!(
            (v.metric - reported).abs() < 1e-6,
            "{d}/p01 reported {reported} != independently verified {}",
            v.metric
        );
    }
}

#[test]
#[ignore = "heavy IPC qualitative-preferences solve; opt-in via --include-ignored"]
fn ipc5_qual_metric_no_regression() {
    // p01 snapshot ceilings (must not regress upward). Locked 2026-07-16 from
    // the Phase-2 defaults sweep (see benchmarks/ipc5-qualitative-scoreboard.md).
    // 1e-6 slack: summed f64 metrics carry accumulation noise.
    for (d, ceiling) in [
        ("openstacks", 66.0),
        ("rovers", 86.64633),
        ("storage", 0.0),
        ("tpp", 24.0),
        ("trucks", 0.0),
    ] {
        let (m, _) = run(d, "p01");
        assert!(
            m <= ceiling + 1e-6,
            "{d}/p01 metric {m} regressed above {ceiling}"
        );
    }
}

#[test]
#[ignore = "heavy IPC qualitative-preferences solve; opt-in via --include-ignored"]
fn storage_p03_survives_the_quadratic_forall() {
    // The static-simplification sentinel: p03's quadratic forall-preference
    // (crates² × storeareas²) expands to 1,554 constraint instances; without
    // the constraint-side drop in `constraints::compile`, monitor grounding
    // OOMs a 15 GB container. Locks coverage AND quality (60, verified ==
    // reported when locked 2026-07-16; ceiling with slack for search shifts,
    // not for a coverage loss).
    let (m, steps) = run("storage", "p03");
    assert!(
        m <= 60.0 + 1e-6,
        "storage/p03 metric {m} regressed above 60"
    );
    let dom = fs::read_to_string(format!("{}/storage/domain.pddl", base())).unwrap();
    let prob = fs::read_to_string(format!("{}/storage/p03.pddl", base())).unwrap();
    let v = ferroplan::verify::verify(&dom, &prob, &steps).unwrap();
    assert!(
        (v.metric - m).abs() < 1e-6,
        "storage/p03 reported {m} != verified {}",
        v.metric
    );
}

#[test]
#[ignore = "heavy IPC qualitative-preferences solve; opt-in via --include-ignored"]
fn storage_p05_defaults_holds_the_no_espc_metric() {
    // 0.8 Phase 3 (docs/roadmap-0.8.md): the shared monitor block no longer
    // feeds ESPC's deadline-pair detection, so monitor-artifact-only tasks
    // take the closure path on PURE DEFAULTS — the behavior the 0.7 board
    // documented per-row as `FF_NO_ESPC=1` (storage p05 completed at 47
    // there; on defaults it exit-137'd inside one ESPC monolithic pass).
    // Locked 2026-07-18 from the Phase-3 defaults run: metric 47, no env,
    // reported == verified. Ceiling may re-derive with a dated reason for
    // search shifts, never for a coverage loss.
    let dom = fs::read_to_string(format!("{}/storage/domain.pddl", base())).unwrap();
    let prob = fs::read_to_string(format!("{}/storage/p05.pddl", base())).unwrap();
    let (reported, steps) = run("storage", "p05");
    assert!(
        reported <= 47.0 + 1e-6,
        "storage/p05 defaults metric regressed: {reported} (ceiling 47)"
    );
    let v = ferroplan::verify::verify(&dom, &prob, &steps).unwrap();
    assert!(v.hard_goal_met && v.constraints_met);
    assert!(
        (v.metric - reported).abs() < 1e-6,
        "reported {reported} != verified {}",
        v.metric
    );
}
