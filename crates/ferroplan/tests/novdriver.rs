//! The partitioned h-free driver (0.22 Phase 5B, docs/roadmap-0.22.md —
//! THE CENTERPIECE). Four levers, smallest-first, each behind its own
//! hatch; these fixtures pin the separations that justify each lever and
//! keep the RED shapes on the record permanently (the ladder_wall.rs
//! convention: the hatched leg IS the recorded negative).
//!
//! - `stagelock`: R-partition vs goal-count cells. Goal count is FLAT (one
//!   goal fact) across m toggle-lock stages, so a goal-count-only cell
//!   exhausts width-1 novelty in stage 0 and BFS-grinds a 2^k lawn per
//!   stage; partitioning by achieved-R re-arms novelty at every stage.
//! - `pairlock`: width 2 vs width 1. The key state (p ∧ qn) carries ZERO
//!   new atoms — p seen at the root, qn seen on the chain — so width-1
//!   ranks it behind a u-toggle lawn of equally non-novel states;
//!   the (p, qn) R-PAIR is new, and the novel-2 rank pops it ahead.
//! - `childsnack_mini`: the negative control, EXCLUDED from the pot up
//!   front (symmetry's constituency, Phase 6): the driver must NOT solve
//!   it inside the rung's cap, so the centerpiece's referees stay
//!   undiluted.

use ferroplan::novelty;
use ferroplan::packed::PackedTask;

fn ground(dom: &str, prb: &str) -> PackedTask {
    let d = ferroplan::parser::parse_domain(dom).unwrap();
    let p = ferroplan::parser::parse_problem(prb).unwrap();
    ferroplan::ground::ground_task(&d, &p, 1).unwrap()
}

/// Replay `ops` from the initial state; every op must be applicable in
/// sequence and the final state must satisfy the goal.
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

/// m stages of a k-toggle lock: stage s advances only when every toggle
/// matches the stage parity (even: all on; odd: all off), and the single
/// goal fact sits behind the last stage — goal count is 1 everywhere, so
/// only the R machinery can tell the stages apart.
fn stagelock(k: usize, m: usize) -> (String, String) {
    let mut preds = String::from(" (gw)");
    for s in 0..=m {
        preds.push_str(&format!(" (c{s})"));
    }
    for i in 0..k {
        preds.push_str(&format!(" (on{i}) (off{i})"));
    }
    let mut acts = String::new();
    for i in 0..k {
        acts.push_str(&format!(
            "(:action set{i} :parameters () :precondition (off{i}) :effect (and (on{i}) (not (off{i}))))\n\
             (:action unset{i} :parameters () :precondition (on{i}) :effect (and (off{i}) (not (on{i}))))\n"
        ));
    }
    for s in 0..m {
        let pat: String = (0..k)
            .map(|i| {
                if s % 2 == 0 {
                    format!(" (on{i})")
                } else {
                    format!(" (off{i})")
                }
            })
            .collect();
        acts.push_str(&format!(
            "(:action adv{s} :parameters () :precondition (and (c{s}){pat}) :effect (and (c{}) (not (c{s}))))\n",
            s + 1
        ));
    }
    acts.push_str(&format!(
        "(:action win :parameters () :precondition (c{m}) :effect (gw))\n"
    ));
    let offs: String = (0..k).map(|i| format!(" (off{i})")).collect();
    (
        format!("(define (domain stagelock) (:predicates{preds}) {acts})"),
        format!("(define (problem sl) (:domain stagelock) (:init (c0){offs}) (:goal (gw)))"),
    )
}

/// The pair conjunction: (p) is true at the root and consumed by the only
/// op that opens the q-chain; re-establishing it AFTER the chain (fix
/// keeps qn) creates a state with no new atom but a new (p, qn) R-pair.
/// u free toggles supply the equally-non-novel lawn that buries the state
/// under width 1.
fn pairlock(u: usize, n: usize) -> (String, String) {
    let mut preds = String::from(" (gw) (p)");
    for j in 0..=n {
        preds.push_str(&format!(" (q{j})"));
    }
    for i in 0..u {
        preds.push_str(&format!(" (tf{i}) (tn{i})"));
    }
    let mut acts = String::from(
        "(:action brk :parameters () :precondition (p) :effect (and (q0) (not (p))))\n",
    );
    for j in 0..n {
        acts.push_str(&format!(
            "(:action step{j} :parameters () :precondition (q{j}) :effect (and (q{}) (not (q{j}))))\n",
            j + 1
        ));
    }
    acts.push_str(&format!(
        "(:action fix :parameters () :precondition (q{n}) :effect (p))\n\
         (:action win :parameters () :precondition (and (p) (q{n})) :effect (gw))\n"
    ));
    for i in 0..u {
        acts.push_str(&format!(
            "(:action flip{i} :parameters () :precondition (tf{i}) :effect (and (tn{i}) (not (tf{i}))))\n\
             (:action flop{i} :parameters () :precondition (tn{i}) :effect (and (tf{i}) (not (tn{i}))))\n"
        ));
    }
    let tfs: String = (0..u).map(|i| format!(" (tf{i})")).collect();
    (
        format!("(define (domain pairlock) (:predicates{preds}) {acts})"),
        format!("(define (problem pl) (:domain pairlock) (:init (p){tfs}) (:goal (gw)))"),
    )
}

/// The IPC child-snack shape, inline (kitchen → sandwich → tray → table),
/// with `nc` children (`na` of them gluten-allergic), exactly-sufficient
/// gluten-free supplies, and interchangeable everything — the factorial
/// core three engines deep at 0.21.
fn childsnack_mini(nc: usize, na: usize, trays: usize) -> (String, String) {
    let dom = "(define (domain child-snack)
      (:requirements :typing)
      (:types child bread-portion content-portion sandwich tray place)
      (:predicates (at_kitchen_bread ?b - bread-portion)
                   (at_kitchen_content ?c - content-portion)
                   (at_kitchen_sandwich ?s - sandwich)
                   (no_gluten_bread ?b - bread-portion)
                   (no_gluten_content ?c - content-portion)
                   (ontray ?s - sandwich ?t - tray)
                   (no_gluten_sandwich ?s - sandwich)
                   (allergic_gluten ?c - child)
                   (not_allergic_gluten ?c - child)
                   (served ?c - child)
                   (waiting ?c - child ?p - place)
                   (at ?t - tray ?p - place)
                   (notexist ?s - sandwich))
      (:action make_sandwich_no_gluten
        :parameters (?s - sandwich ?b - bread-portion ?c - content-portion)
        :precondition (and (at_kitchen_bread ?b) (at_kitchen_content ?c)
                           (no_gluten_bread ?b) (no_gluten_content ?c) (notexist ?s))
        :effect (and (not (at_kitchen_bread ?b)) (not (at_kitchen_content ?c))
                     (at_kitchen_sandwich ?s) (no_gluten_sandwich ?s) (not (notexist ?s))))
      (:action make_sandwich
        :parameters (?s - sandwich ?b - bread-portion ?c - content-portion)
        :precondition (and (at_kitchen_bread ?b) (at_kitchen_content ?c) (notexist ?s))
        :effect (and (not (at_kitchen_bread ?b)) (not (at_kitchen_content ?c))
                     (at_kitchen_sandwich ?s) (not (notexist ?s))))
      (:action put_on_tray
        :parameters (?s - sandwich ?t - tray)
        :precondition (and (at_kitchen_sandwich ?s) (at ?t kitchen))
        :effect (and (not (at_kitchen_sandwich ?s)) (ontray ?s ?t)))
      (:action serve_sandwich_no_gluten
        :parameters (?s - sandwich ?c - child ?t - tray ?p - place)
        :precondition (and (allergic_gluten ?c) (ontray ?s ?t) (waiting ?c ?p)
                           (no_gluten_sandwich ?s) (at ?t ?p))
        :effect (and (not (ontray ?s ?t)) (served ?c)))
      (:action serve_sandwich
        :parameters (?s - sandwich ?c - child ?t - tray ?p - place)
        :precondition (and (not_allergic_gluten ?c) (waiting ?c ?p) (ontray ?s ?t) (at ?t ?p))
        :effect (and (not (ontray ?s ?t)) (served ?c)))
      (:action move_tray
        :parameters (?t - tray ?p1 - place ?p2 - place)
        :precondition (at ?t ?p1)
        :effect (and (not (at ?t ?p1)) (at ?t ?p2))))";
    let mut objs = String::new();
    for i in 1..=nc {
        objs.push_str(&format!(" child{i}"));
    }
    objs.push_str(" - child");
    for i in 1..=nc {
        objs.push_str(&format!(" bread{i}"));
    }
    objs.push_str(" - bread-portion");
    for i in 1..=nc {
        objs.push_str(&format!(" content{i}"));
    }
    objs.push_str(" - content-portion");
    for i in 1..=trays {
        objs.push_str(&format!(" tray{i}"));
    }
    objs.push_str(" - tray");
    for i in 1..=nc {
        objs.push_str(&format!(" sandw{i}"));
    }
    objs.push_str(" - sandwich kitchen table1 table2 table3 - place");
    let mut init = String::new();
    for i in 1..=trays {
        init.push_str(&format!(" (at tray{i} kitchen)"));
    }
    for i in 1..=nc {
        init.push_str(&format!(
            " (at_kitchen_bread bread{i}) (at_kitchen_content content{i}) (notexist sandw{i})"
        ));
    }
    // Exactly-sufficient gluten-free supplies for the allergic block.
    for i in 1..=na {
        init.push_str(&format!(
            " (no_gluten_bread bread{i}) (no_gluten_content content{i})"
        ));
    }
    for i in 1..=nc {
        let table = 1 + (i % 3);
        if i <= na {
            init.push_str(&format!(" (allergic_gluten child{i})"));
        } else {
            init.push_str(&format!(" (not_allergic_gluten child{i})"));
        }
        init.push_str(&format!(" (waiting child{i} table{table})"));
    }
    let goals: String = (1..=nc).map(|i| format!(" (served child{i})")).collect();
    (
        dom.to_string(),
        format!(
            "(define (problem cs-mini) (:domain child-snack) (:objects{objs}) (:init{init}) (:goal (and{goals})))"
        ),
    )
}

// ---------------------------------------------------------------------------
// The RED records (permanent, the ladder_wall.rs convention): today's h-free
// rung — novelty-LIGHT, goal-count cells, width 1 — caps on both shapes.
// These legs pin the mechanism the driver's levers exist to fix.
// ---------------------------------------------------------------------------

#[test]
fn light_rung_caps_on_stagelock_the_red_record() {
    let (dom, prb) = stagelock(12, 4);
    let task = ground(&dom, &prb);
    assert!(
        novelty::search_light(&task, 3_000, &[]).is_none(),
        "goal-count width-1 was expected to cap on the stage lock (the RED shape)"
    );
}

#[test]
fn light_rung_caps_on_pairlock_the_red_record() {
    let (dom, prb) = pairlock(40, 10);
    let task = ground(&dom, &prb);
    assert!(
        novelty::search_light(&task, 700, &[]).is_none(),
        "width-1 was expected to cap on the pair conjunction (the RED shape)"
    );
}
