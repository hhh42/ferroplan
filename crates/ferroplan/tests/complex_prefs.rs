//! The complex-preferences entry (0.25 Phase 2): soft `(preference ...)`
//! trajectory constraints AND goal preferences on durative-action
//! (temporal) domains — the IPC-5 track ferroplan could never attempt
//! ("last of 3, until the feature ships", docs/ipc-rankings.md).
//!
//! The entry's shape (docs/roadmap-0.25.md): preferences never gate
//! VALIDITY. The router banks coverage first (soft constraints dropped,
//! goal preferences trivially true — exactly what grounding already does
//! with `Formula::Pref`), then chases QUALITY with every preference
//! HARDENED on the remaining wall (plans(hardened) ⊆ plans(banked), so
//! the chase can never lose the banked solve). Scoring is post-hoc and
//! independent of the search: the ORIGINAL preferences fold over the
//! plan's replayed state trajectory (the validate() machinery's `Fold`),
//! and the `:metric` is evaluated with the PDDL3 `(is-violated name)`
//! counts — one instance per (preference × outer forall binding).
//!
//! RED first: before the entry, both fixtures below died at the gate
//! with "PDDL3 preference (soft) trajectory constraints on
//! durative-action (temporal) domains are not yet enforced".

/// Two durative actions; `work` achieves the hard goal, `wave` exists
/// only to satisfy preferences. `(never-obtainable)` has no producer.
const DOMAIN: &str = "
(define (domain cp-mini)
  (:requirements :strips :durative-actions :constraints :preferences)
  (:predicates (fresh-work) (fresh-wave) (done) (waved) (never-obtainable))
  (:durative-action work
    :parameters ()
    :duration (= ?duration 5)
    :condition (at start (fresh-work))
    :effect (and (at start (not (fresh-work))) (at end (done))))
  (:durative-action wave
    :parameters ()
    :duration (= ?duration 2)
    :condition (at start (fresh-wave))
    :effect (and (at start (not (fresh-wave))) (at end (waved)))))
";

/// Every preference satisfiable: the quality chase (hardened tier) can
/// reach metric 0 — a plan that works AND waves.
const P_SATISFIABLE: &str = "
(define (problem cp-sat) (:domain cp-mini)
  (:init (fresh-work) (fresh-wave))
  (:goal (and (done) (preference gp (waved))))
  (:constraints (preference cp (sometime (waved))))
  (:metric minimize (+ (* 2 (is-violated gp)) (* 3 (is-violated cp)))))
";

/// One preference impossible: the hardened tier cannot solve, the banked
/// tier must still deliver the row, and the score must price exactly the
/// impossible preference (weight 5) — the satisfiable ones still count.
const P_MIXED: &str = "
(define (problem cp-mixed) (:domain cp-mini)
  (:init (fresh-work) (fresh-wave))
  (:goal (and (done) (preference gp (waved))))
  (:constraints (and (preference cp (sometime (waved)))
                     (preference cq (sometime (never-obtainable)))))
  (:metric minimize (+ (* 2 (is-violated gp))
                       (* 3 (is-violated cp))
                       (* 5 (is-violated cq)))))
";

fn solve(problem: &str) -> ferroplan::Solution {
    ferroplan::solve(DOMAIN, problem, &ferroplan::Options::default()).expect("solve runs")
}

#[test]
fn all_satisfiable_prefs_reach_metric_zero() {
    let sol = solve(P_SATISFIABLE);
    assert!(sol.solved, "the entry must solve: {:?}", sol.notes);
    let plan = sol.plan.expect("plan");
    assert_eq!(
        plan.metric,
        Some(0.0),
        "every preference is jointly satisfiable — the quality chase must \
         reach metric 0 (notes: {:?}, plan: {:?})",
        sol.notes,
        plan.steps
    );
    // The chase actually waved — the preference is satisfied by the plan,
    // not by bookkeeping.
    assert!(
        plan.steps
            .iter()
            .any(|s| s.action.contains("WAVE") || s.action.contains("wave")),
        "metric 0 requires the plan to wave: {:?}",
        plan.steps
    );
}

#[test]
fn impossible_pref_is_priced_not_fatal() {
    let sol = solve(P_MIXED);
    assert!(
        sol.solved,
        "an impossible preference must never cost the row (coverage banks \
         first): {:?}",
        sol.notes
    );
    let plan = sol.plan.expect("plan");
    assert_eq!(
        plan.metric,
        Some(5.0),
        "exactly the impossible preference (weight 5) is violated; gp and \
         cp are satisfiable and must be satisfied by the quality chase \
         (notes: {:?})",
        sol.notes
    );
}

/// The scorer prices what the PLAN did, not what was reachable: a plan
/// that never waves violates gp (2) and cp (3) — metric 5 — even though
/// both were satisfiable. Pinned through Mode::Temporal with the chase
/// disabled via the banked problem shape (no soft members to harden ==
/// no chase; here we instead pin the SCORER directly).
#[test]
fn scorer_prices_the_plan_not_the_reachable() {
    let d = ferroplan::parser::parse_domain(DOMAIN).unwrap();
    let p = ferroplan::parser::parse_problem(P_SATISFIABLE).unwrap();
    // A lazy plan: work only.
    let plan = ferroplan::temporal::TimedPlan {
        steps: vec![ferroplan::temporal::TimedStep {
            time: 0.0,
            action: "WORK".into(),
            duration: Some(5.0),
        }],
        makespan: 5.0,
    };
    let score = ferroplan::temporal::score_soft(&d, &p, &plan).expect("scored");
    assert_eq!(score.metric, Some(5.0), "gp (2) + cp (3): {score:?}");
    assert_eq!(
        score.violated.len(),
        2,
        "both preferences violated by the lazy plan: {score:?}"
    );
}
