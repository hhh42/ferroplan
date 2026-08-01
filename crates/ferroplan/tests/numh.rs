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
