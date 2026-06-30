//! ESPC (`FF_ESPC`) penalty-resolution loop: independent metric verification,
//! strict improvement over the default optimizer, no-regression floor, and
//! thread-count determinism.
//!
//! One sequential test on purpose: `FF_ESPC` is a process-global env toggle, and a
//! separate test binary running a single test means it can never race the other
//! suites. The determinism assertion uses p01, which terminates by the (fully
//! deterministic) stall/saddle conditions well within the wall-clock backstop, so
//! the per-search thread-independence carries through to the whole loop.

use ferroplan::{solve, Options, Plan};

fn read(inst: &str) -> (String, String) {
    let base = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../benchmarks/ipc/pref/openstacks"
    );
    (
        std::fs::read_to_string(format!("{base}/domain.pddl")).unwrap(),
        std::fs::read_to_string(format!("{base}/{inst}.pddl")).unwrap(),
    )
}

fn steps(plan: &Plan) -> Vec<(String, Vec<String>)> {
    plan.steps
        .iter()
        .map(|s| (s.action.clone(), s.args.clone()))
        .collect()
}

#[test]
#[ignore = "heavy IPC solve (~2-6 min): ESPC penalty-resolution loop on openstacks/p01. \
            Opt-in via `cargo test -- --include-ignored`; CI runs it in release."]
fn espc_openstacks_p01_improves_verifies_and_is_deterministic() {
    let (d, p) = read("p01");

    // Default path: the independent verifier must agree with the reported metric
    // (guards against crediting a metric the plan did not actually earn).
    let base = solve(&d, &p, &Options::default()).unwrap();
    let bplan = base.plan.expect("default plan");
    let bm = bplan.metric.expect("default metric");
    let bv = ferroplan::verify::verify(&d, &p, &steps(&bplan)).unwrap();
    assert!(bv.hard_goal_met, "default plan must meet the hard goal");
    assert!(
        (bv.metric - bm).abs() < 1e-6,
        "default reported {bm} != independently verified {}",
        bv.metric
    );

    // ESPC path at two thread counts (env set once, removed straight after).
    // Pin the wall-clock backstop high so termination is by the (deterministic)
    // stall/saddle conditions, not the timer — otherwise a slow debug build could
    // make the iteration count, and thus the plan, depend on thread count.
    std::env::set_var("FF_ESPC", "1");
    std::env::set_var("FF_ESPC_TIME_MS", "600000");
    let o1 = Options {
        threads: 1,
        ..Options::default()
    };
    let o8 = Options {
        threads: 8,
        ..Options::default()
    };
    let s1 = solve(&d, &p, &o1).unwrap();
    let s8 = solve(&d, &p, &o8).unwrap();
    std::env::remove_var("FF_ESPC");
    std::env::remove_var("FF_ESPC_TIME_MS");

    let p1 = s1.plan.expect("espc plan (t1)");
    let m1 = p1.metric.expect("espc metric");
    let v1 = ferroplan::verify::verify(&d, &p, &steps(&p1)).unwrap();
    assert!(v1.hard_goal_met, "espc plan must meet the hard goal");
    assert!(
        (v1.metric - m1).abs() < 1e-6,
        "espc reported {m1} != independently verified {}",
        v1.metric
    );
    assert!(
        m1 < bm,
        "espc metric {m1} must strictly beat the default {bm}"
    );
    assert!(
        m1 <= 46.0,
        "espc openstacks/p01 metric {m1} regressed above the locked 46"
    );

    // Thread count must change neither the plan nor the metric.
    let p8 = s8.plan.expect("espc plan (t8)");
    assert_eq!(
        steps(&p1),
        steps(&p8),
        "espc plan differs across thread counts"
    );
    assert_eq!(
        m1,
        p8.metric.expect("espc metric t8"),
        "espc metric differs across thread counts"
    );
}
