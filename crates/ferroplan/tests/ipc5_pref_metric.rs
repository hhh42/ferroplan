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
fn openstacks_beats_all_forgo_floor() {
    // before satisfaction-guidance ferroplan emitted the all-forgo plan (metric 70)
    let m = metric("openstacks", "p01");
    assert!(
        m < 70.0,
        "openstacks/p01 metric {m} should beat the 70 floor"
    );
}

#[test]
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
