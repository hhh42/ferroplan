#[test]
fn policy_validation_detects_transition_tampering() {
    let options = finite(2);
    let mut solution = solve_ppddl(RETRY_DOMAIN, RETRY_PROBLEM, &options).unwrap();
    let validation = validate_ppddl_policy(RETRY_DOMAIN, RETRY_PROBLEM, &options, &solution)
        .unwrap();
    assert!(validation.valid, "{:?}", validation.errors);

    solution.policy[0].outcomes[0].probability += 0.1;
    let invalid = validate_ppddl_policy(RETRY_DOMAIN, RETRY_PROBLEM, &options, &solution)
        .unwrap();
    assert!(!invalid.valid);
    assert!(!invalid.errors.is_empty());
}

#[test]
fn probability_and_resource_bounds_fail_closed() {
    let invalid = RETRY_DOMAIN.replace(
        "(probabilistic 0.25 (done))",
        "(probabilistic 0.75 (done) 0.75 (and))",
    );
    assert!(matches!(
        solve_ppddl(&invalid, RETRY_PROBLEM, &ProbabilisticOptions::default()),
        Err(PpddlError::InvalidProbability(_))
    ));

    let state_bound = ProbabilisticOptions {
        max_states: 1,
        horizon: Some(1),
        ..Default::default()
    };
    assert!(matches!(
        solve_ppddl(RETRY_DOMAIN, RETRY_PROBLEM, &state_bound),
        Err(PpddlError::StateLimit { limit: 1 })
    ));

    let value_bound = ProbabilisticOptions {
        horizon: Some(2),
        max_value_cells: 1,
        ..Default::default()
    };
    assert!(matches!(
        solve_ppddl(RETRY_DOMAIN, RETRY_PROBLEM, &value_bound),
        Err(PpddlError::ValueTableLimit { limit: 1 })
    ));

    let outcome_bound = ProbabilisticOptions {
        max_outcomes_per_action: 1,
        ..Default::default()
    };
    assert!(matches!(
        solve_ppddl(RETRY_DOMAIN, RETRY_PROBLEM, &outcome_bound),
        Err(PpddlError::OutcomeLimit { limit: 1, .. })
    ));
}

#[test]
fn reward_semantic_restrictions_fail_closed() {
    let domain = r#"
    (define (domain bad-reward)
      (:requirements :strips :rewards)
      (:predicates (done))
      (:action bad :parameters ()
        :precondition (> (reward) 0)
        :effect (done)))
    "#;
    let problem = r#"
    (define (problem bad-reward-p)
      (:domain bad-reward)
      (:init (= (reward) 0))
      (:goal (done)))
    "#;
    assert!(matches!(
        solve_ppddl(domain, problem, &ProbabilisticOptions::default()),
        Err(PpddlError::RewardViolation(_))
    ));
}

#[test]
fn infinite_expected_reward_requires_discounting() {
    let domain = r#"
    (define (domain reward-loop)
      (:requirements :strips :rewards)
      (:predicates (done))
      (:action collect :parameters () :precondition (and)
        :effect (increase (reward) 1)))
    "#;
    let problem = r#"
    (define (problem reward-loop-p)
      (:domain reward-loop)
      (:init (= (reward) 0))
      (:goal (done))
      (:metric maximize (reward)))
    "#;
    let options = ProbabilisticOptions {
        horizon: None,
        discount: 1.0,
        ..Default::default()
    };
    assert!(matches!(
        solve_ppddl(domain, problem, &options),
        Err(PpddlError::InvalidOptions(_))
    ));
}

#[test]
fn unsupported_temporal_and_derived_extensions_are_named() {
    let temporal = r#"
    (define (domain temporal-prob)
      (:requirements :strips :durative-actions :probabilistic-effects)
      (:predicates (done))
      (:durative-action wait
        :parameters ()
        :duration (= ?duration 1)
        :condition (and)
        :effect (at end (done))))
    "#;
    assert!(matches!(
        solve_ppddl(temporal, RETRY_PROBLEM, &ProbabilisticOptions::default()),
        Err(PpddlError::Unsupported(_)) | Err(PpddlError::Syntax(_))
    ));
}
