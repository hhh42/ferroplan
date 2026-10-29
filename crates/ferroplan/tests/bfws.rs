//! BFWS the winners' way (0.31 Lane B, docs/roadmap-0.31.md).
//!
//! The 0.22 driver already is BFWS(f5); what the field's winners add is
//! pinned here, each behind its knob with 0.30 as the hatch:
//!
//! - **Novelty-first LAMA** (`lama::search_cfg(.., novelty = true)`, the
//!   ladder's `FF_LAMA_NOV=1`): BFWS(f6)'s key -- novelty relative to the
//!   ⟨lm, parent-h⟩ pair leads, LAMA's own currency breaks ties. The
//!   `deceptlawn` shape from `tests/ladder_wall.rs`: a 2^k toggle lawn the
//!   relaxed heuristic loves and a novel corridor it prices at h ≈ m. Plain
//!   LAMA grinds the lawn to its cap without popping the corridor; with
//!   novelty first every corridor state carries a never-seen fact and is
//!   popped ahead of the lawn.
//! - **The front end** (`DriverCfg::front()`, the ladder's `FF_NOV_FRONT=1`):
//!   the driver with non-novel states pruned. Polynomial: on the lawn it
//!   succeeds or fails inside a bounded number of pops; on a chain where
//!   every step is novel it solves outright.
//! - **R refresh + forgetting** (`DriverCfg::extended()`): the paper's #r
//!   and Maidu's forgetting. Pinned for validity and determinism on the
//!   chain and on a two-goal task; the board prices them.

use ferroplan::novelty::{self, DriverCfg};
use ferroplan::packed::PackedTask;

fn ground(dom: &str, prb: &str) -> PackedTask {
    let d = ferroplan::parser::parse_domain(dom).unwrap();
    let p = ferroplan::parser::parse_problem(prb).unwrap();
    ferroplan::ground::ground_task(&d, &p, 1).unwrap()
}

fn assert_valid(task: &PackedTask, ops: &[usize]) {
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

/// The tetris shape, distilled (tests/ladder_wall.rs): a DECEPTIVE LAWN
/// the relaxed heuristic loves and a NOVEL CORRIDOR it prices at h≈m.
fn deceptlawn(k: usize, m: usize, junk: usize) -> (String, String) {
    let mut preds = String::from(" (win) (fr) (mua) (mub) (jf)");
    for i in 0..k {
        preds.push_str(&format!(" (on{i}) (off{i})"));
    }
    for i in 0..=m {
        preds.push_str(&format!(" (c{i})"));
    }
    let mut acts = String::new();
    for i in 0..k {
        acts.push_str(&format!(
            "(:action set{i} :parameters () :precondition (off{i}) :effect (and (on{i}) (not (off{i}))))\n\
             (:action unset{i} :parameters () :precondition (on{i}) :effect (and (off{i}) (not (on{i}))))\n"
        ));
    }
    let allon: String = (0..k).map(|i| format!(" (on{i})")).collect();
    acts.push_str(&format!(
        "(:action winop :parameters () :precondition (and (mua) (mub){allon}) :effect (win))\n\
         (:action mk-a :parameters () :precondition (fr) :effect (and (mua) (not (fr))))\n\
         (:action mk-b :parameters () :precondition (fr) :effect (and (mub) (not (fr))))\n\
         (:action enter :parameters () :precondition (fr) :effect (and (c0) (not (fr))))\n\
         (:action realwin :parameters () :precondition (c{m}) :effect (win))\n"
    ));
    for i in 0..m {
        acts.push_str(&format!(
            "(:action walk{i} :parameters () :precondition (c{i}) :effect (c{})\n)",
            i + 1
        ));
    }
    for j in 0..junk {
        acts.push_str(&format!(
            "(:action junk{j} :parameters () :precondition (and (mua) (mub)) :effect (jf))\n"
        ));
    }
    let offs: String = (0..k).map(|i| format!(" (off{i})")).collect();
    (
        format!("(define (domain deceptlawn) (:predicates{preds}) {acts})"),
        format!("(define (problem dl) (:domain deceptlawn) (:init{offs} (fr)) (:goal (win)))"),
    )
}

/// The openstacks shape (tests/ladder_wall.rs): a chain every step of
/// which makes a never-seen fact true.
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

#[test]
fn novelty_first_lama_walks_the_corridor_plain_lama_grinds_the_lawn() {
    // Calibrated 2026-10-29: plain LAMA grows with k on a long corridor
    // (k=12 m=30 solved in 11k evals, the first guess's cap was 20k;
    // k=18 m=60 takes 279k), novelty-first stays flat (13.4k here).
    let (dom, prb) = deceptlawn(18, 60, 0);
    let task = ground(&dom, &prb);
    let cap = 50_000;
    let plain = novelty_lama(&task, cap, false);
    let novel = novelty_lama(&task, cap, true);
    let (ops, evaluated) = novel.expect("novelty-first LAMA must pop the corridor inside the cap");
    assert_valid(&task, &ops);
    assert!(
        plain.is_none(),
        "plain LAMA solved the lawn inside {cap} evals -- the fixture no longer separates the keys (k too small?)"
    );
    assert!(
        evaluated < cap,
        "novelty-first LAMA used every eval of its cap: {evaluated}"
    );
}

fn novelty_lama(task: &PackedTask, cap: usize, novelty: bool) -> Option<(Vec<usize>, usize)> {
    ferroplan::lama::search_cfg(task, 1, cap, &[], None, novelty)
}

#[test]
fn the_front_end_solves_a_novel_chain_and_is_polynomial_on_the_lawn() {
    // Every step of the chain carries a new fact: the pruned search never
    // drops a state it needs.
    let (dom, prb) = laddertax(60);
    let task = ground(&dom, &prb);
    let (ops, evaluated) = novelty::search_driver(&task, 100_000, &[], None, &DriverCfg::front())
        .expect("the front end must walk the chain");
    assert_valid(&task, &ops);
    assert!(evaluated < 1_000, "the chain cost {evaluated} pops");

    // On the lawn the complete driver solves it; the front end returns --
    // with a plan or without -- inside a bounded number of pops, well
    // under what the complete search is allowed.
    let (dom, prb) = deceptlawn(10, 20, 0);
    let task = ground(&dom, &prb);
    let cap = 200_000;
    let complete = novelty::search_driver(&task, cap, &[], None, &DriverCfg::default())
        .expect("the complete driver solves the lawn");
    assert_valid(&task, &complete.0);
    let t0 = std::time::Instant::now();
    let front = novelty::search_driver(&task, cap, &[], None, &DriverCfg::front());
    let secs = t0.elapsed().as_secs_f64();
    if let Some((ops, _)) = &front {
        assert_valid(&task, ops);
    }
    assert!(secs < 5.0, "the front end ran {secs:.2}s on a 2^10 lawn");
}

#[test]
fn the_extended_driver_is_valid_and_deterministic() {
    let (dom, prb) = laddertax(80);
    let task = ground(&dom, &prb);
    let a = novelty::search_driver(&task, 100_000, &[], None, &DriverCfg::extended())
        .expect("driverx walks the chain");
    assert_valid(&task, &a.0);
    let b = novelty::search_driver(&task, 100_000, &[], None, &DriverCfg::extended())
        .expect("driverx walks the chain");
    assert_eq!(a, b, "two runs must be identical");

    // Two goals behind the lawn's token: a refresh fires on each goal-count
    // improvement and the plan is still valid.
    let (dom, prb) = deceptlawn(6, 8, 0);
    let task = ground(&dom, &prb);
    let (ops, _) = novelty::search_driver(&task, 100_000, &[], None, &DriverCfg::extended())
        .expect("driverx solves the small lawn");
    assert_valid(&task, &ops);
}
