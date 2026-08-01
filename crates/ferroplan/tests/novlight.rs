//! The novelty-LIGHT rung (0.20 Phase 3, docs/roadmap-0.20.md): IW(1) +
//! goal count with ZERO heuristic evaluations. The fixture pins the
//! SEPARATION that justifies the rung: on a visit-all-shaped grid the
//! light search walks to the goal in ~plan-length pops, while weighted
//! best-first — at ANY weight (the scoping probe swept 5/20/60) — cannot
//! finish within two orders of magnitude more evaluations (h_FF's
//! plateau defeats greed; only structural novelty makes progress).

fn grid_task() -> ferroplan::packed::PackedTask {
    let dom = include_str!("../../../benchmarks/bench/visitgrid-domain.pddl");
    let prb = include_str!("../../../benchmarks/bench/visitgrid-10x10.pddl");
    let d = ferroplan::parser::parse_domain(dom).unwrap();
    let p = ferroplan::parser::parse_problem(prb).unwrap();
    ferroplan::ground::ground_task(&d, &p, 1).unwrap()
}

#[test]
fn light_walks_the_grid_where_weighted_best_first_caps() {
    let task = grid_task();
    // The light rung: solved within a tight cap, ~plan-length pops.
    let r = ferroplan::novelty::search_light(&task, 5_000, &[]);
    let (ops, evaluated) = r.expect("light rung solves the 10x10 grid");
    assert!(!ops.is_empty());
    assert!(
        evaluated < 1_000,
        "light needed {evaluated} pops — the walk should be ~plan-length"
    );

    // The counterpart: weighted best-first under a 20x LARGER eval budget
    // still fails (the h_FF plateau — this is the mechanism the rung
    // exists for, and the reason it runs before LAMA in the ladder).
    let dom = include_str!("../../../benchmarks/bench/visitgrid-domain.pddl");
    let prb = include_str!("../../../benchmarks/bench/visitgrid-10x10.pddl");
    let opts = ferroplan::Options {
        search: ferroplan::Search::BestFirst,
        max_evaluated: Some(100_000),
        threads: 1,
        ..Default::default()
    };
    let sol = ferroplan::solve(dom, prb, &opts).unwrap();
    assert!(!sol.solved, "best-first should cap on the plateau");
}
