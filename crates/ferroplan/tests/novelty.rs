//! Novelty rung (0.17 Phase 3): the width-1 BFWS-style bounded search must
//! produce VALID plans, be deterministic across thread counts, and behave
//! as a pure-addition rung (the ladder with it solves everything the
//! ladder without it solves).
//!
//! Fixture: `benchmarks/bench/catalog-120-consume` — the depth-capped
//! big-catalog fixture (one gather rule, one make rule, catalog as init
//! data, consumable inputs; see benchmarks/bench/gen_catalog.py and the
//! fixture-design lesson recorded in docs/landscape-2026.md).

use ferroplan::ground::{ground, Outcome};
use ferroplan::{novelty, parser};

fn fixture() -> ferroplan::packed::PackedTask {
    let base = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../benchmarks/bench/catalog-120-consume"
    );
    let d = parser::parse_domain(&std::fs::read_to_string(format!("{base}/domain.pddl")).unwrap())
        .unwrap();
    let p =
        parser::parse_problem(&std::fs::read_to_string(format!("{base}/problem.pddl")).unwrap())
            .unwrap();
    match ground(&d, &p, 1) {
        Outcome::Task(t) => t,
        other => panic!("expected task, got {:?}", std::mem::discriminant(&other)),
    }
}

/// Replay `ops` from the initial state; every op must be applicable in
/// sequence and the final state must satisfy the goal.
fn assert_valid(task: &ferroplan::packed::PackedTask, ops: &[usize]) {
    let mut s = task.initial();
    for (i, &oi) in ops.iter().enumerate() {
        assert!(
            task.op_applicable(oi, &s),
            "step {i} ({}) inapplicable",
            task.op_display[oi]
        );
        s = task.apply(oi, &s);
    }
    assert!(
        task.goal_met_with(&s, &task.goal_pos, &task.goal_num),
        "goal unmet after replay"
    );
}

#[test]
fn novelty_rung_solves_the_catalog_fixture_with_a_valid_plan() {
    let task = fixture();
    let (ops, evaluated) = novelty::search(&task, 1, 500_000, &[], None)
        .expect("novelty rung should solve catalog-120");
    assert!(evaluated > 0);
    assert_valid(&task, &ops);
}

#[test]
fn novelty_rung_is_deterministic_across_thread_counts() {
    let task = fixture();
    let (p1, _) = novelty::search(&task, 1, 500_000, &[], None).expect("t1 solve");
    let (p4, _) = novelty::search(&task, 4, 500_000, &[], None).expect("t4 solve");
    assert_eq!(p1, p4, "plan must be identical at any thread count");
}
