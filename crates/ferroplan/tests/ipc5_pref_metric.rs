//! IPC-5 preference-track metric regression guard (the ESPC / openstacks target).
//! `Options::default()` (Mode::Auto) routes these soft-goal domains to the PDDL3
//! metric optimizer; lower metric is better.

use ferroplan::{solve, Options};
use std::fs;

fn metric(domain: &str, problem: &str) -> f64 {
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../benchmarks/ipc/pref");
    let d = fs::read_to_string(format!("{base}/{domain}/domain.pddl")).unwrap();
    let p = fs::read_to_string(format!("{base}/{domain}/{problem}.pddl")).unwrap();
    let sol = solve(&d, &p, &Options::default()).unwrap();
    assert!(sol.solved, "{domain}/{problem} should solve");
    sol.plan.and_then(|pl| pl.metric).expect("metric reported")
}

#[test]
#[ignore = "heavy IPC pref-metric solve; opt-in via --include-ignored (CI runs these in release)"]
fn openstacks_beats_all_forgo_floor() {
    // before satisfaction-guidance ferroplan emitted the all-forgo plan (metric 70)
    let m = metric("openstacks", "p01");
    assert!(
        m < 70.0,
        "openstacks/p01 metric {m} should beat the 70 floor"
    );
}

#[test]
#[ignore = "heavy IPC pref-metric solve; opt-in via --include-ignored (CI runs these in release)"]
fn default_metric_matches_independent_verifier() {
    // W1 oracle: every default plan must be valid (hard goal met), and where the
    // independent verifier is authoritative the reported metric must equal the
    // replay-scored metric. Since 0.7 the verifier grounds inner quantifiers
    // (e.g. tpp's p4A `(forall (?m) ...)`) before scoring, so it is exact on
    // every domain whose metric is a pure weighted is-violated sum. rovers
    // stays validity-only: its metric folds a monotone numeric term
    // (`sum-traverse-cost`) the preference-only verifier doesn't recompute.
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../benchmarks/ipc/pref");
    for (d, exact) in [
        ("openstacks", true),
        ("tpp", true),
        ("storage", true),
        ("trucks", true),
        ("pathways", true),
        ("rovers", false),
    ] {
        let dom = fs::read_to_string(format!("{base}/{d}/domain.pddl")).unwrap();
        let prob = fs::read_to_string(format!("{base}/{d}/p01.pddl")).unwrap();
        let sol = solve(&dom, &prob, &Options::default()).unwrap();
        assert!(sol.solved, "{d}/p01 should solve");
        let plan = sol.plan.expect("plan");
        let reported = plan.metric.expect("metric");
        let steps: Vec<(String, Vec<String>)> = plan
            .steps
            .iter()
            .map(|s| (s.action.clone(), s.args.clone()))
            .collect();
        let v = ferroplan::verify::verify(&dom, &prob, &steps).unwrap();
        assert!(v.hard_goal_met, "{d}/p01 plan must meet the hard goal");
        if exact {
            assert!(
                (v.metric - reported).abs() < 1e-6,
                "{d}/p01 reported {reported} != independently verified {}",
                v.metric
            );
        }
    }
}

#[test]
#[ignore = "heavy IPC pref-metric solve; opt-in via --include-ignored (CI runs these in release)"]
fn ipc5_pref_metric_no_regression() {
    // p01 snapshot ceilings (must not regress upward). Re-derived 2026-07 for
    // the exact-closure optimizer + static simplification + barrier-free
    // guidance: tpp ties SGPlan5 (16), storage beats it (3 vs 5), openstacks'
    // default dropped 63 -> 49 (the opt-in FF_ESPC row is separate, see
    // tests/espc.rs). rovers rides the legacy folded-metric path, unchanged.
    // openstacks re-locked 49 -> 42 (2026-07, budget-escalating retry),
    // then 42 -> 23 (2026-07, anytime sweeps + the diversified restart ladder),
    // then 23 -> 19 (0.5: ESPC graduated to default-on, deterministic budget).
    for (d, ceiling) in [
        ("openstacks", 19.0),
        ("tpp", 16.0),
        ("storage", 3.0),
        ("trucks", 0.0),
        // rovers is MetricSimplePreferences: weighted is-violated + the monotone
        // (sum-traverse-cost) numeric term, which compile() now folds into
        // total-cost (previously dropped -> a bogus 0). Re-locked 935.3 ->
        // 811.3 (0.5: folded metrics route through the closure optimizer,
        // whose anytime sweeps tie SGPlan5 here).
        ("rovers", 811.3),
        ("pathways", 2.0),
    ] {
        let m = metric(d, "p01");
        // 1e-6 slack: summed f64 metric values carry accumulation noise
        // (rovers reports 811.3000000000001 for the exact 811.3).
        assert!(
            m <= ceiling + 1e-6,
            "{d}/p01 metric {m} regressed above {ceiling}"
        );
    }
}

#[test]
#[ignore = "heavy IPC pref-metric solve; opt-in via --include-ignored (CI runs these in release)"]
fn storage_p03_covered_with_sane_metric() {
    // storage p03 compiles to 1601 preference instances and produced NOTHING at
    // any timeout before static simplification + the exact-closure optimizer.
    // Locks both coverage and quality (6 beats SGPlan5's 14; ceiling with slack
    // for future search shifts, not for a coverage loss).
    let m = metric("storage", "p03");
    assert!(m <= 14.0, "storage/p03 metric {m} regressed above 14");
}
