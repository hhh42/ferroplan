#[cfg(test)]
mod tests {
    use super::*;

    const COIN_DOMAIN: &str = r#"
    (define (domain coin)
      (:requirements :strips :negative-preconditions :probabilistic-effects)
      (:predicates (heads) (tails))
      (:action flip
        :parameters ()
        :precondition (not (heads))
        :effect (probabilistic 0.7 (heads) 0.3 (tails))))
    "#;
    const COIN_PROBLEM: &str = r#"
    (define (problem coin-p)
      (:domain coin)
      (:init)
      (:goal (heads)))
    "#;

    #[test]
    fn nested_probabilistic_effects_cross_product() {
        let effect = parse_sexp(
            "(and (probabilistic 0.5 (a) 0.5 (b)) (probabilistic 0.25 (c) 0.75 (d)))",
        )
        .unwrap();
        let universe = ObjectUniverse::default();
        let outcomes = expand_effect(&effect, &universe, 16).unwrap();
        assert_eq!(outcomes.len(), 4);
        assert!(
            (outcomes.iter().map(|entry| entry.0).sum::<f64>() - 1.0).abs()
                < 1e-12
        );
    }

    #[test]
    fn quantified_probabilistic_effects_are_independent() {
        let domain = parse_sexp(
            "(define (domain q) (:requirements :typing) (:types item) (:predicates (p ?x - item)))",
        )
        .unwrap();
        let problem = parse_sexp(
            "(define (problem q-p) (:domain q) (:objects a b - item) (:init) (:goal (and)))",
        )
        .unwrap();
        let universe = ObjectUniverse::from_documents(&domain, &problem).unwrap();
        let effect =
            parse_sexp("(forall (?x - item) (probabilistic 0.5 (p ?x)))").unwrap();
        let outcomes = expand_effect(&effect, &universe, 16).unwrap();
        assert_eq!(outcomes.len(), 4);
    }

    #[test]
    fn missing_probability_mass_is_noop() {
        let effect = parse_sexp("(probabilistic 0.25 (heads))").unwrap();
        let outcomes =
            expand_effect(&effect, &ObjectUniverse::default(), 16).unwrap();
        assert_eq!(outcomes.len(), 2);
        assert!(outcomes.iter().any(|(probability, effect)| {
            (*probability - 0.75).abs() < 1e-12 && effect.head() == Some("AND")
        }));
    }

    #[test]
    fn finite_horizon_coin_probability() {
        let options = ProbabilisticOptions {
            horizon: Some(1),
            ..Default::default()
        };
        let solution = solve_ppddl(COIN_DOMAIN, COIN_PROBLEM, &options).unwrap();
        assert!(solution.solved);
        assert!((solution.initial_value - 0.7).abs() < 1e-9);
        assert_eq!(solution.initial_action.as_deref(), Some("FLIP"));
    }

    #[test]
    fn probabilistic_initial_state_is_preserved() {
        let problem = r#"
        (define (problem coin-p)
          (:domain coin)
          (:init (probabilistic 0.4 (heads) 0.6 (tails)))
          (:goal (heads)))
        "#;
        let options = ProbabilisticOptions {
            horizon: Some(0),
            ..Default::default()
        };
        let solution = solve_ppddl(COIN_DOMAIN, problem, &options).unwrap();
        assert_eq!(solution.initial_distribution.len(), 2);
        assert!((solution.initial_value - 0.4).abs() < 1e-9);
    }

    #[test]
    fn infinite_retry_reaches_probability_one() {
        let options = ProbabilisticOptions {
            horizon: None,
            epsilon: 1e-12,
            ..Default::default()
        };
        let solution = solve_ppddl(COIN_DOMAIN, COIN_PROBLEM, &options).unwrap();
        assert!(solution.statistics.converged);
        assert!((solution.initial_value - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rewards_are_optimized() {
        let domain = r#"
        (define (domain gamble)
          (:requirements :strips :mdp)
          (:predicates (done))
          (:action safe
            :parameters ()
            :precondition (and)
            :effect (and (done) (increase (reward) 3)))
          (:action gamble
            :parameters ()
            :precondition (and)
            :effect (probabilistic
              0.5 (and (done) (increase (reward) 10))
              0.5 (and (done) (decrease (reward) 4)))))
        "#;
        let problem = r#"
        (define (problem gamble-p)
          (:domain gamble)
          (:init)
          (:goal (done)))
        "#;
        let options = ProbabilisticOptions {
            horizon: Some(1),
            ..Default::default()
        };
        let solution = solve_ppddl(domain, problem, &options).unwrap();
        assert_eq!(
            solution.objective,
            ProbabilisticObjective::MaximizeExpectedReward
        );
        assert!((solution.initial_value - 3.0).abs() < 1e-9);
        assert_eq!(solution.initial_action.as_deref(), Some("SAFE"));
    }

    #[test]
    fn reward_restrictions_are_enforced() {
        let domain = r#"
        (define (domain bad)
          (:requirements :strips :rewards)
          (:predicates (done))
          (:action bad
            :precondition (> (reward) 0)
            :effect (done)))
        "#;
        let problem = r#"
        (define (problem bad-p)
          (:domain bad)
          (:init (= (reward) 0))
          (:goal (done)))
        "#;
        assert!(matches!(
            solve_ppddl(domain, problem, &ProbabilisticOptions::default()),
            Err(PpddlError::RewardViolation(_))
        ));
    }

    #[test]
    fn reward_expression_fluents_remain_in_stochastic_state_keys() {
        let domain = r#"
        (define (domain reward-state)
          (:requirements :strips :mdp :numeric-fluents)
          (:predicates (done))
          (:functions (bonus))
          (:action collect
            :parameters ()
            :precondition (and)
            :effect (and (done) (increase (reward) (bonus)))))
        "#;
        let problem = r#"
        (define (problem reward-state-p)
          (:domain reward-state)
          (:init (probabilistic
            0.5 (assign (bonus) 1)
            0.5 (assign (bonus) 9)))
          (:goal (done))
          (:metric maximize (reward)))
        "#;
        let options = ProbabilisticOptions {
            horizon: Some(1),
            ..Default::default()
        };
        let solution = solve_ppddl(domain, problem, &options).unwrap();
        assert_eq!(solution.initial_distribution.len(), 2);
        assert!((solution.initial_value - 5.0).abs() < 1e-9);
    }

    #[test]
    fn goal_reward_accepts_ground_numeric_expressions() {
        let domain = r#"
        (define (domain bonus)
          (:requirements :strips :mdp :numeric-fluents)
          (:predicates (done))
          (:functions (bonus))
          (:action finish
            :parameters ()
            :precondition (and)
            :effect (done)))
        "#;
        let problem = r#"
        (define (problem bonus-p)
          (:domain bonus)
          (:init (= (bonus) 5))
          (:goal (done))
          (:goal-reward (+ (bonus) 2))
          (:metric maximize (reward)))
        "#;
        let options = ProbabilisticOptions {
            horizon: Some(1),
            ..Default::default()
        };
        let solution = solve_ppddl(domain, problem, &options).unwrap();
        assert!((solution.initial_value - 7.0).abs() < 1e-9);
    }

    #[test]
    fn initially_satisfied_goal_receives_goal_reward_once() {
        let domain = r#"
        (define (domain initial-goal)
          (:requirements :strips :mdp :numeric-fluents)
          (:predicates (done))
          (:functions (bonus)))
        "#;
        let problem = r#"
        (define (problem initial-goal-p)
          (:domain initial-goal)
          (:init (done) (= (bonus) 4))
          (:goal (done))
          (:goal-reward (bonus))
          (:metric maximize (reward)))
        "#;
        let options = ProbabilisticOptions {
            horizon: Some(0),
            ..Default::default()
        };
        let solution = solve_ppddl(domain, problem, &options).unwrap();
        assert!((solution.initial_value - 4.0).abs() < 1e-9);
        let simulation = simulate_ppddl(domain, problem, &options, 10, 1).unwrap();
        assert!((simulation.average_reward - 4.0).abs() < 1e-9);
    }

    #[test]
    fn terminal_metric_fluents_remain_in_stochastic_state_keys() {
        let domain = r#"
        (define (domain score)
          (:requirements :strips :probabilistic-effects :numeric-fluents)
          (:predicates (done))
          (:functions (score))
          (:action safe
            :parameters ()
            :precondition (and)
            :effect (and (done) (assign (score) 4)))
          (:action risky
            :parameters ()
            :precondition (and)
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
        let options = ProbabilisticOptions {
            horizon: Some(1),
            ..Default::default()
        };
        let solution = solve_ppddl(domain, problem, &options).unwrap();
        assert_eq!(
            solution.objective,
            ProbabilisticObjective::MaximizeExpectedMetric
        );
        assert_eq!(solution.initial_action.as_deref(), Some("RISKY"));
        assert!((solution.initial_value - 5.0).abs() < 1e-9);
    }

    #[test]
    fn problem_requirements_admit_probabilistic_initial_state() {
        let domain = r#"
        (define (domain initial-probability)
          (:requirements :strips)
          (:predicates (heads)))
        "#;
        let problem = r#"
        (define (problem initial-probability-p)
          (:domain initial-probability)
          (:requirements :probabilistic-effects)
          (:init (probabilistic 0.25 (heads)))
          (:goal (heads)))
        "#;
        let options = ProbabilisticOptions {
            horizon: Some(0),
            ..Default::default()
        };
        let solution = solve_ppddl(domain, problem, &options).unwrap();
        assert!((solution.initial_value - 0.25).abs() < 1e-9);
    }

    #[test]
    fn policy_validation_and_seeded_simulation() {
        let options = ProbabilisticOptions {
            horizon: Some(2),
            ..Default::default()
        };
        let solution = solve_ppddl(COIN_DOMAIN, COIN_PROBLEM, &options).unwrap();
        let validation =
            validate_ppddl_policy(COIN_DOMAIN, COIN_PROBLEM, &options, &solution)
                .unwrap();
        assert!(validation.valid, "{:?}", validation.errors);
        let first =
            simulate_ppddl(COIN_DOMAIN, COIN_PROBLEM, &options, 1_000, 42)
                .unwrap();
        let second =
            simulate_ppddl(COIN_DOMAIN, COIN_PROBLEM, &options, 1_000, 42)
                .unwrap();
        assert_eq!(first.reached_goal, second.reached_goal);
        assert!((first.goal_rate - 0.91).abs() < 0.05);
    }
}
