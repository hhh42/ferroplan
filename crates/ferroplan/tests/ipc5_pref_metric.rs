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
    // replay-scored metric. Exact-match is asserted only for openstacks — the ESPC
    // target, whose preferences are simple atomic deliveries the phase-1 verifier
    // scores exactly. The others are validity-only: rovers folds a monotone numeric
    // term the preference-only verifier doesn't recompute, and tpp/storage/trucks/
    // pathways have preference bodies with INNER quantifiers (e.g. tpp's p4A
    // `(forall (?m) ...)`) that the verifier evaluates best-effort (verify.rs:74-76),
    // so its count is not authoritative there.
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../benchmarks/ipc/pref");
    for (d, exact) in [
        ("openstacks", true),
        ("tpp", false),
        ("storage", false),
        ("trucks", false),
        ("pathways", false),
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
    // p01 snapshot ceilings with satisfaction-guidance (must not regress upward)
    for (d, ceiling) in [
        ("openstacks", 63.0),
        ("tpp", 21.0),
        ("storage", 8.0),
        ("trucks", 0.0),
        // rovers is MetricSimplePreferences: weighted is-violated + the monotone
        // (sum-traverse-cost) numeric term, which compile() now folds into
        // total-cost (previously dropped -> a bogus 0). 935.3 is the real metric.
        ("rovers", 935.3),
        ("pathways", 2.0),
    ] {
        let m = metric(d, "p01");
        assert!(m <= ceiling, "{d}/p01 metric {m} regressed above {ceiling}");
    }
}
