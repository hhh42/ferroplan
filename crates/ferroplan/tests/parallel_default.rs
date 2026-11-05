//! The parallel rung portfolio's DEFAULT (0.31 Lane A′, the mco read of
//! 2026-11-04; `parallel.rs`). Its own test binary because the gate reads
//! the environment and the other portfolio fixtures set it.
//!
//! Three pins: auto threads (`Options::threads == 0`, the library's and
//! the CLI's default) keeps the batch-split ladder -- no portfolio note,
//! the plan and eval count of `FF_PARALLEL_RUNGS=0`; an explicit
//! `threads: 2` runs the portfolio and says so; `FF_PARALLEL_RUNGS=0`
//! turns an explicit request back into the ladder.

use ferroplan::{solve, Options};

fn bench() -> (String, String) {
    let base = format!("{}/../../benchmarks/bench", env!("CARGO_MANIFEST_DIR"));
    (
        std::fs::read_to_string(format!("{base}/visitgrid-domain.pddl")).unwrap(),
        std::fs::read_to_string(format!("{base}/visitgrid-7x7.pddl")).unwrap(),
    )
}

fn ran_portfolio(threads: usize) -> (bool, usize) {
    let (dom, prb) = bench();
    let opts = Options {
        threads,
        ..Options::default()
    };
    let s = solve(&dom, &prb, &opts).expect("solve");
    assert!(s.solved, "visitgrid-7x7 must solve");
    (
        s.notes.iter().any(|n| n.starts_with("parallel rungs:")),
        s.statistics.evaluated_states,
    )
}

#[test]
fn auto_threads_keep_the_ladder_and_explicit_threads_run_the_portfolio() {
    std::env::remove_var("FF_PARALLEL_RUNGS");
    let (auto_ran, auto_evals) = ran_portfolio(0);
    assert!(!auto_ran, "auto threads must keep the batch-split ladder");
    std::env::set_var("FF_PARALLEL_RUNGS", "0");
    let (_, hatch_evals) = ran_portfolio(0);
    assert_eq!(
        auto_evals, hatch_evals,
        "auto threads are byte-identical to the hatch"
    );
    let (hatch_ran, _) = ran_portfolio(2);
    assert!(
        !hatch_ran,
        "FF_PARALLEL_RUNGS=0 turns an explicit request back into the ladder"
    );
    std::env::remove_var("FF_PARALLEL_RUNGS");
    let (two_ran, _) = ran_portfolio(2);
    assert!(
        two_ran,
        "an explicit threads: 2 runs the portfolio by default"
    );
}
