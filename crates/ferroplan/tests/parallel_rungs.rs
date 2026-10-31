//! THE PARALLEL RUNG PORTFOLIO (0.31 Lane A′, docs/roadmap-0.31.md;
//! `parallel.rs`): at `--threads N ≥ 2` under `FF_PARALLEL_RUNGS=1`, the
//! ladder runs as one worker and N−1 of its rungs run beside it with the
//! whole wall each; the first plan wins, the others are cancelled through
//! the per-call budget's flag.
//!
//! What these legs pin, and what they deliberately do not. They pin the
//! PLUMBING: the members and their order (the matrix's marginal order),
//! that the mode runs and says so in the notes, that one thread is the
//! ladder untouched, and that a withdrawn caller brings every worker home
//! promptly -- the scope must never hang on a loser. They do NOT pin which
//! member wins or how fast: that is a race on wall time by construction
//! (the one contract the mode breaks, named in the module header), and
//! the mco subset sweep is its judge, not a fixture on a box that may be
//! sweeping.
//!
//! In-process, not child processes: every leg wants the same env, and
//! `FF_TIME_LIMIT` is never set here -- every stop is the caller's own
//! budget, armed in code (tests/call_budget.rs's rule).

use ferroplan::parallel::members;
use ferroplan::{solve, Options};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// tests/ladder_wall.rs's openstacks shape: goals (ga) and (gb) each
/// consume the (one) token, which refills only after an L-step chain
/// walk. EHC's lookahead is L³-shaped; novelty-light and LAMA finish it
/// quickly. Any member can win it, which is the point of the first leg.
fn laddertax(l: usize) -> (String, String) {
    let mut preds = String::from(" (ga) (gb) (one)");
    for i in 0..=l {
        preds.push_str(&format!(" (p{i})"));
    }
    let mut acts = String::from(
        "(:action make-ga :parameters () :precondition (one) :effect (and (ga) (not (one))))\n\
         (:action make-gb :parameters () :precondition (one) :effect (and (gb) (not (one))))\n",
    );
    acts.push_str(&format!(
        "(:action refill :parameters () :precondition (p{l}) :effect (and (one) (p0) (not (p{l}))))\n"
    ));
    for i in 0..l {
        acts.push_str(&format!(
            "(:action step{i} :parameters () :precondition (p{i}) :effect (and (p{}) (not (p{i}))))\n",
            i + 1
        ));
    }
    (
        format!("(define (domain laddertax) (:predicates{preds}) {acts})"),
        "(define (problem lt) (:domain laddertax) (:init (one) (p0)) (:goal (and (ga) (gb))))"
            .into(),
    )
}

fn opts(threads: usize) -> Options {
    std::env::set_var("FF_PARALLEL_RUNGS", "1");
    Options {
        threads,
        max_evaluated: Some(2_000_000),
        ..Default::default()
    }
}

fn names(t: usize) -> Vec<&'static str> {
    members(t).iter().map(|m| m.name()).collect()
}

/// The members are the matrix's marginal order beside the ladder (the
/// 2026-10-31 read with Lane B's novelty-first LAMA in the pool): LAMA-
/// novelty (+50 cells), driver (+11), bfs (+4), then the unique-solve
/// rungs, plain LAMA last, the front end after. One thread is the ladder
/// alone; more threads than members run the members.
#[test]
fn the_members_follow_the_matrix_order() {
    assert_eq!(names(1), ["ladder"]);
    assert_eq!(names(2), ["ladder", "LAMA-novelty"]);
    assert_eq!(names(3), ["ladder", "LAMA-novelty", "novelty-driver"]);
    assert_eq!(
        names(4),
        ["ladder", "LAMA-novelty", "novelty-driver", "bfs"]
    );
    assert_eq!(names(8).len(), 8);
    assert_eq!(names(64), names(8));
}

/// The mode runs at two threads, solves, and the notes say so twice: the
/// portfolio's own line (workers, members, timing-dependent, who won) and
/// the ladder's `rung:` line routed through `parallel/<member>`.
#[test]
fn two_workers_solve_and_the_notes_say_so() {
    let (dom, prb) = laddertax(400);
    let sol = solve(&dom, &prb, &opts(2)).expect("solves");
    assert!(sol.solved, "{:?}", sol.notes);
    let plan = sol.plan.expect("a plan");
    assert!(!plan.steps.is_empty());
    assert!(
        sol.notes.iter().any(|n| n.starts_with(
            "parallel rungs: 2 workers (ladder, LAMA-novelty), timing-dependent; won by "
        )),
        "the portfolio must report itself:\n{:?}",
        sol.notes
    );
    assert!(
        sol.notes
            .iter()
            .any(|n| n.starts_with("rung: solved by parallel/")),
        "the rung line must route through the member:\n{:?}",
        sol.notes
    );
}

/// One thread is the ladder exactly as before: no workers, no portfolio
/// line, the plain `rung:` note. The opt-in env is set and must not matter.
#[test]
fn one_thread_is_the_ladder_untouched() {
    let (dom, prb) = laddertax(400);
    let sol = solve(&dom, &prb, &opts(1)).expect("solves");
    assert!(sol.solved, "{:?}", sol.notes);
    assert!(
        !sol.notes.iter().any(|n| n.contains("parallel")),
        "one thread must not run the portfolio:\n{:?}",
        sol.notes
    );
    assert!(
        sol.notes
            .iter()
            .any(|n| n.starts_with("rung: solved by ")
                && !n.starts_with("rung: solved by parallel/")),
        "{:?}",
        sol.notes
    );
}

/// A caller that withdraws brings every worker home: the call returns
/// promptly whether or not a member had already won, and an unsolved
/// return names the withdrawal, never unsolvability. Three workers, so
/// LAMA (which polls the flag only at its batch boundary) is among them.
#[test]
fn a_withdrawn_caller_brings_every_worker_home() {
    // Already withdrawn: nobody does any work worth the name.
    let (dom, prb) = laddertax(2500);
    let mut o = opts(3);
    o.should_continue = Some(Arc::new(AtomicBool::new(false)));
    let t0 = Instant::now();
    let sol = solve(&dom, &prb, &o).expect("a stop is a verdict, not an error");
    assert!(!sol.solved, "{:?}", sol.notes);
    assert!(
        sol.notes.iter().any(|n| n.contains("the caller withdrew")),
        "{:?}",
        sol.notes
    );
    assert!(
        !sol.notes.join(" ").to_lowercase().contains("unsolvable"),
        "a withdrawal is never a verdict:\n{:?}",
        sol.notes
    );
    if !cfg!(debug_assertions) {
        assert!(
            t0.elapsed().as_secs_f64() < 15.0,
            "{:.1} s",
            t0.elapsed().as_secs_f64()
        );
    }

    // Withdrawn mid-flight, from another thread. Whichever comes first --
    // a member's plan or the flag -- the call must come back; the scope
    // waiting on three workers is exactly what could hang.
    let flag = Arc::new(AtomicBool::new(true));
    let trigger = Arc::clone(&flag);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        trigger.store(false, Ordering::Relaxed);
    });
    let mut o = opts(3);
    o.should_continue = Some(flag);
    let t0 = Instant::now();
    let sol = solve(&dom, &prb, &o).expect("a stop is a verdict, not an error");
    if !sol.solved {
        assert!(
            sol.notes.iter().any(|n| n.contains("the caller withdrew")),
            "{:?}",
            sol.notes
        );
    }
    assert!(
        sol.notes
            .iter()
            .any(|n| n.starts_with("parallel rungs: 3 workers")),
        "{:?}",
        sol.notes
    );
    if !cfg!(debug_assertions) {
        assert!(
            t0.elapsed().as_secs_f64() < 20.0,
            "{:.1} s",
            t0.elapsed().as_secs_f64()
        );
    }
}
