//! The linear numeric-subgoaling charge (0.19 Phase 3): sailing-shaped
//! goals — linear combinations over several fluents — get a repetition
//! gradient where the bare-fluent path saw a plateau.

use ferroplan::{solve, Options};

/// Goal `(>= (+ (* 2 (x)) (y)) 12)` from x=y=0; one op adds x+=1,y+=1
/// (combo +3 per step) — exactly 4 steps. Without the linear charge the
/// relaxed h has no gradient on the combination (the old path handles
/// only bare-fluent-vs-literal); with it the solve is direct.
#[test]
fn linear_combo_goal_solves_with_gradient() {
    let dom = "(define (domain sail)
      (:requirements :typing :numeric-fluents)
      (:functions (x) (y))
      (:action row :parameters ()
        :precondition (<= (x) 100)
        :effect (and (increase (x) 1) (increase (y) 1))))";
    let prb = "(define (problem s1) (:domain sail)
      (:init (= (x) 0) (= (y) 0))
      (:goal (>= (+ (* 2 (x)) (y)) 12)))";
    let sol = solve(dom, prb, &Options::default()).unwrap();
    assert!(sol.solved);
    assert_eq!(sol.plan.unwrap().length, 4, "combo +3/step, gap 12");
}

/// The fluent-vs-fluent shape (`(>= (x) (d))` with static d) that killed
/// the bare path — rhs is not a literal. 5 steps of +2 against d=10.
#[test]
fn fluent_rhs_goal_gets_the_charge() {
    let dom = "(define (domain sail2)
      (:requirements :typing :numeric-fluents)
      (:functions (x) (d))
      (:action gain :parameters ()
        :precondition (<= (x) 100)
        :effect (increase (x) 2)))";
    let prb = "(define (problem s2) (:domain sail2)
      (:init (= (x) 0) (= (d) 10))
      (:goal (>= (x) (d))))";
    let sol = solve(dom, prb, &Options::default()).unwrap();
    assert!(sol.solved);
    assert_eq!(sol.plan.unwrap().length, 5);
}

/// The Eq refusal, repaired (0.21 Phase 3 probe rider c): the linear path
/// returned None on `CompOp::Eq` — block-grouping's ENTIRE goal shape
/// (`(= (x b1) (x b2))`), leaving h=0 flat. From a point value an Eq is
/// one-sided (raise if below, lower if above), so it charges toward zero
/// from cur's side. Four blocks on a 20-cell line meet in ~25 moves; a
/// charge-less search is blind Dijkstra over the 20^4 grid and caps.
#[test]
fn eq_goal_gets_the_charge() {
    let dom = "(define (domain bg)
      (:requirements :typing :numeric-fluents)
      (:types block - object)
      (:functions (x ?b - block) (max_x) (min_x))
      (:action right :parameters (?b - block)
        :precondition (<= (+ (x ?b) 1) (max_x))
        :effect (increase (x ?b) 1))
      (:action left :parameters (?b - block)
        :precondition (>= (- (x ?b) 1) (min_x))
        :effect (decrease (x ?b) 1)))";
    let prb = "(define (problem bg1) (:domain bg)
      (:objects b1 b2 b3 b4 - block)
      (:init (= (x b1) 20) (= (x b2) 17) (= (x b3) 3) (= (x b4) 9)
             (= (max_x) 20) (= (min_x) 1))
      (:goal (and (= (x b1) (x b2)) (= (x b1) (x b3)) (= (x b1) (x b4)))))";
    let opts = Options {
        max_evaluated: Some(50_000),
        threads: 1,
        ..Default::default()
    };
    let sol = solve(dom, prb, &opts).unwrap();
    assert!(sol.solved, "the Eq charge must give the meet a gradient");
}

/// The pre-existing bare shape stays byte-identical: same plan and the
/// same evaluated-state count with the linear path present (it never
/// runs where the bare path answers).
#[test]
fn bare_shape_unchanged() {
    let dom = "(define (domain bare)
      (:requirements :typing :numeric-fluents)
      (:functions (x))
      (:action gain :parameters ()
        :precondition (<= (x) 100)
        :effect (increase (x) 3)))";
    let prb = "(define (problem b1) (:domain bare)
      (:init (= (x) 0))
      (:goal (>= (x) 9)))";
    let sol = solve(dom, prb, &Options::default()).unwrap();
    assert!(sol.solved);
    assert_eq!(sol.plan.unwrap().length, 3);
}
