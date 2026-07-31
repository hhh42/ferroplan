#[test]
fn oneof_is_uniform_and_seeded_simulation_is_replayable() {
    let domain = r#"
    (define (domain oneof)
      (:requirements :strips :probabilistic-effects)
      (:predicates (a) (b) (c))
      (:action choose :parameters () :precondition (and)
        :effect (oneof (a) (b) (c))))
    "#;
    let problem = r#"
    (define (problem oneof-p)
      (:domain oneof)
      (:init)
      (:goal (a)))
    "#;
    let solution = solve_ppddl(domain, problem, &finite(1)).unwrap();
    assert_eq!(solution.policy[0].outcomes.len(), 3);
    let first = simulate_ppddl(domain, problem, &finite(1), 4_000, 93).unwrap();
    let second = simulate_ppddl(domain, problem, &finite(1), 4_000, 93).unwrap();
    assert_eq!(first.reached_goal, second.reached_goal);
    assert!((first.goal_rate - (1.0 / 3.0)).abs() < 0.04);
}

#[test]
fn transition_and_ground_expression_goal_rewards_compose() {
    let domain = r#"
    (define (domain reward)
      (:requirements :strips :mdp :numeric-fluents)
      (:predicates (done))
      (:functions (bonus))
      (:action finish
        :parameters ()
        :precondition (and)
        :effect (and (done) (increase (reward) 2))))
    "#;
    let problem = r#"
    (define (problem reward-p)
      (:domain reward)
      (:init (= (bonus) 5))
      (:goal (done))
      (:goal-reward (+ (bonus) 3))
      (:metric maximize (reward)))
    "#;
    let solution = solve_ppddl(domain, problem, &finite(1)).unwrap();
    assert_eq!(
        solution.objective,
        ProbabilisticObjective::MaximizeExpectedReward
    );
    assert!((solution.initial_value - 10.0).abs() < 1e-9);
}

#[test]
fn terminal_numeric_metric_selects_the_higher_expected_value() {
    let domain = r#"
    (define (domain score)
      (:requirements :strips :probabilistic-effects :numeric-fluents)
      (:predicates (done))
      (:functions (score))
      (:action safe :parameters () :precondition (and)
        :effect (and (done) (assign (score) 4)))
      (:action risky :parameters () :precondition (and)
        :effect (probabilistic
          0.5 (and (done) (assign (score) 10))
          0.5 (and (done) (assign (score) 0)))))
    "#;
    let problem = r#"
    (define (problem score-p)
      (:domain score)
      (:init (= (score) 0))
      (:goal (done))
      (:metric maximize (score)))
    "#;
    let solution = solve_ppddl(domain, problem, &finite(1)).unwrap();
    assert_eq!(solution.initial_action.as_deref(), Some("RISKY"));
    assert!((solution.initial_value - 5.0).abs() < 1e-9);
}

#[test]
fn minimizing_numeric_metric_selects_the_lower_expected_value() {
    let domain = r#"
    (define (domain cost)
      (:requirements :strips :numeric-fluents)
      (:predicates (done))
      (:functions (cost))
      (:action cheap :parameters () :precondition (and)
        :effect (and (done) (assign (cost) 2)))
      (:action expensive :parameters () :precondition (and)
        :effect (and (done) (assign (cost) 7))))
    "#;
    let problem = r#"
    (define (problem cost-p)
      (:domain cost)
      (:init (= (cost) 0))
      (:goal (done))
      (:metric minimize (cost)))
    "#;
    let solution = solve_ppddl(domain, problem, &finite(1)).unwrap();
    assert_eq!(solution.objective, ProbabilisticObjective::MinimizeExpectedMetric);
    assert_eq!(solution.initial_action.as_deref(), Some("CHEAP"));
    assert!((solution.initial_value - 2.0).abs() < 1e-9);
}

#[test]
fn total_time_metric_is_layer_aware() {
    let domain = r#"
    (define (domain timing)
      (:requirements :strips :negative-preconditions)
      (:predicates (done))
      (:action finish :parameters () :precondition (not (done)) :effect (done))
      (:action wait :parameters () :precondition (not (done)) :effect (and)))
    "#;
    let minimize_problem = r#"
    (define (problem timing-min)
      (:domain timing)
      (:init)
      (:goal (done))
      (:metric minimize total-time))
    "#;
    let minimize = solve_ppddl(domain, minimize_problem, &finite(2)).unwrap();
    assert_eq!(minimize.initial_action.as_deref(), Some("FINISH"));
    assert!((minimize.initial_value - 1.0).abs() < 1e-9);

    let maximize_problem = minimize_problem.replace("minimize", "maximize");
    let maximize = solve_ppddl(domain, &maximize_problem, &finite(2)).unwrap();
    assert_eq!(maximize.initial_action.as_deref(), Some("WAIT"));
    assert!((maximize.initial_value - 2.0).abs() < 1e-9);
}

#[test]
fn total_time_goal_reward_is_awarded_at_goal_entry_time() {
    let domain = r#"
    (define (domain timed-reward)
      (:requirements :strips :negative-preconditions :rewards)
      (:predicates (done))
      (:action finish :parameters () :precondition (not (done)) :effect (done))
      (:action wait :parameters () :precondition (not (done)) :effect (and)))
    "#;
    let problem = r#"
    (define (problem timed-reward-p)
      (:domain timed-reward)
      (:init (= (reward) 0))
      (:goal (done))
      (:goal-reward total-time)
      (:metric maximize (reward)))
    "#;
    let solution = solve_ppddl(domain, problem, &finite(2)).unwrap();
    assert_eq!(solution.initial_action.as_deref(), Some("WAIT"));
    assert!((solution.initial_value - 2.0).abs() < 1e-9);

    let infinite = ProbabilisticOptions {
        horizon: None,
        discount: 0.9,
        ..Default::default()
    };
    assert!(matches!(
        solve_ppddl(domain, problem, &infinite),
        Err(PpddlError::InvalidOptions(_))
    ));
}
