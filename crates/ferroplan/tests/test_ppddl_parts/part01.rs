use ferroplan::{
    parse_ppddl, simulate_ppddl, solve_ppddl, validate_ppddl_policy, PpddlError,
    ProbabilisticObjective, ProbabilisticOptions,
};

const RETRY_DOMAIN: &str = r#"
(define (domain retry)
  (:requirements :strips :negative-preconditions :probabilistic-effects)
  (:predicates (done))
  (:action attempt
    :parameters ()
    :precondition (not (done))
    :effect (probabilistic 0.25 (done))))
"#;

const RETRY_PROBLEM: &str = r#"
(define (problem retry-p)
  (:domain retry)
  (:init)
  (:goal (done)))
"#;

fn finite(horizon: usize) -> ProbabilisticOptions {
    ProbabilisticOptions {
        horizon: Some(horizon),
        ..Default::default()
    }
}

#[test]
fn parse_report_names_probabilistic_surface() {
    let report = parse_ppddl(RETRY_DOMAIN, RETRY_PROBLEM);
    assert!(report.ok, "{:?}", report.error);
    assert_eq!(report.domain.as_deref(), Some("retry"));
    assert_eq!(report.problem.as_deref(), Some("retry-p"));
    assert_eq!(report.probabilistic_actions, 1);
    assert_eq!(report.normalized_outcomes, 2);
    assert_eq!(report.initial_outcomes, 1);
    assert_eq!(report.goal_reward.as_deref(), Some("1"));
}

#[test]
fn implicit_noop_mass_is_preserved() {
    let solution = solve_ppddl(RETRY_DOMAIN, RETRY_PROBLEM, &finite(1)).unwrap();
    assert!((solution.initial_value - 0.25).abs() < 1e-9);
    assert_eq!(solution.policy.len(), 1);
    assert_eq!(solution.policy[0].outcomes.len(), 2);
    let mass: f64 = solution.policy[0]
        .outcomes
        .iter()
        .map(|outcome| outcome.probability)
        .sum();
    assert!((mass - 1.0).abs() < 1e-12);
}

#[test]
fn infinite_retry_converges_to_one() {
    let options = ProbabilisticOptions {
        horizon: None,
        epsilon: 1e-12,
        ..Default::default()
    };
    let solution = solve_ppddl(RETRY_DOMAIN, RETRY_PROBLEM, &options).unwrap();
    assert!(solution.statistics.converged);
    assert!((solution.initial_value - 1.0).abs() < 1e-9);
}

#[test]
fn nested_conjunctive_probabilities_form_cartesian_product() {
    let domain = r#"
    (define (domain product)
      (:requirements :strips :probabilistic-effects)
      (:predicates (a) (b) (c) (d) (done))
      (:action choose
        :parameters ()
        :precondition (and)
        :effect (and
          (done)
          (probabilistic 0.5 (a) 0.5 (b))
          (probabilistic 0.25 (c) 0.75 (d)))))
    "#;
    let problem = r#"
    (define (problem product-p)
      (:domain product)
      (:init)
      (:goal (done)))
    "#;
    let solution = solve_ppddl(domain, problem, &finite(1)).unwrap();
    assert_eq!(solution.policy[0].outcomes.len(), 4);
    let mut probabilities = solution.policy[0]
        .outcomes
        .iter()
        .map(|outcome| outcome.probability)
        .collect::<Vec<_>>();
    probabilities.sort_by(f64::total_cmp);
    assert_eq!(probabilities, vec![0.125, 0.125, 0.375, 0.375]);
}

#[test]
fn forall_probabilistic_effects_are_independent_per_binding() {
    let domain = r#"
    (define (domain quantified)
      (:requirements :strips :typing :probabilistic-effects)
      (:types item)
      (:predicates (marked ?x - item) (done))
      (:action mark
        :parameters ()
        :precondition (and)
        :effect (and
          (done)
          (forall (?x - item) (probabilistic 0.5 (marked ?x))))))
    "#;
    let problem = r#"
    (define (problem quantified-p)
      (:domain quantified)
      (:objects a b - item)
      (:init)
      (:goal (done)))
    "#;
    let solution = solve_ppddl(domain, problem, &finite(1)).unwrap();
    assert_eq!(solution.policy[0].outcomes.len(), 4);
    assert!(solution.policy[0]
        .outcomes
        .iter()
        .all(|outcome| (outcome.probability - 0.25).abs() < 1e-12));
}

#[test]
fn conditional_effects_read_the_source_state() {
    let domain = r#"
    (define (domain source-state)
      (:requirements :strips :conditional-effects :probabilistic-effects)
      (:predicates (p) (q) (done))
      (:action step
        :parameters ()
        :precondition (and)
        :effect (and
          (done)
          (probabilistic 0.5 (p))
          (when (p) (q)))))
    "#;
    let problem = r#"
    (define (problem source-state-p)
      (:domain source-state)
      (:init)
      (:goal (q)))
    "#;
    let solution = solve_ppddl(domain, problem, &finite(1)).unwrap();
    assert!((solution.initial_value - 0.0).abs() < 1e-12);
}

#[test]
fn probabilistic_initial_distribution_is_forced_before_policy() {
    let domain = r#"
    (define (domain initial)
      (:requirements :strips)
      (:predicates (heads)))
    "#;
    let problem = r#"
    (define (problem initial-p)
      (:domain initial)
      (:requirements :probabilistic-effects)
      (:init (probabilistic 0.4 (heads)))
      (:goal (heads)))
    "#;
    let solution = solve_ppddl(domain, problem, &finite(0)).unwrap();
    assert_eq!(solution.initial_distribution.len(), 2);
    assert!(solution.policy.is_empty());
    assert!((solution.initial_value - 0.4).abs() < 1e-9);
}
