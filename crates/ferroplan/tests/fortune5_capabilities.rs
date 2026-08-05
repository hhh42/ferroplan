use ferroplan::{
    decompose, parse, solve, solve_ppddl, trace, validate_ppddl_policy, Options,
    ProbabilisticOptions, Session,
};

const DOMAIN: &str = r#"
(define (domain smoke)
  (:requirements :strips)
  (:predicates (done))
  (:action finish :parameters () :precondition (and) :effect (done)))
"#;

const PROBLEM: &str = r#"
(define (problem smoke-p)
  (:domain smoke)
  (:init)
  (:goal (done)))
"#;

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

#[test]
fn parse_accepts_valid_input_and_refuses_malformed_input() {
    let valid = parse(DOMAIN);
    assert!(valid.ok, "{:?}", valid.error);
    let invalid = parse("(define (domain broken)");
    assert!(!invalid.ok);
    assert!(invalid.error.is_some());
}

#[test]
fn trace_replays_the_solved_candidate_from_the_declared_initial_state() {
    let solution = solve(DOMAIN, PROBLEM, &Options::default()).unwrap();
    let plan = solution.plan.expect("solved plan");
    let steps: Vec<(String, Vec<String>)> = plan
        .steps
        .iter()
        .map(|step| (step.action.clone(), step.args.clone()))
        .collect();
    let snapshots = trace(DOMAIN, PROBLEM, &steps).unwrap();
    assert_eq!(snapshots.len(), steps.len() + 1);
    assert!(snapshots
        .last()
        .unwrap()
        .facts
        .iter()
        .any(|fact| fact == "(DONE)"));
}

#[test]
fn session_budget_is_deterministic_and_replayable() {
    let session = Session::new(DOMAIN, PROBLEM, &Options::default()).unwrap();
    let first = session.replan_budgeted(1_000, Some(64));
    let second = session.replan_budgeted(1_000, Some(64));
    assert!(first.solved && second.solved);
    assert_eq!(
        serde_json::to_value(&first.plan).unwrap(),
        serde_json::to_value(&second.plan).unwrap()
    );
    let plan = first.plan.as_ref().unwrap();
    assert!(session.plan_still_valid(plan, 0));
    assert!(session.world_bytes() > 0);
    assert!(session.mind_bytes() > 0);
}

#[test]
fn decomposition_returns_a_valid_stitched_candidate() {
    let decomposition = decompose(DOMAIN, PROBLEM, &Options::default()).unwrap();
    assert!(decomposition.solved, "{:?}", decomposition.notes);
    let plan = decomposition.plan.expect("stitched plan");
    let text = plan
        .steps
        .iter()
        .map(|step| {
            let args = if step.args.is_empty() {
                String::new()
            } else {
                format!(" {}", step.args.join(" "))
            };
            format!("step {}: {}{args}", step.index, step.action)
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(matches!(
        ferroplan::plan::validate_plan(DOMAIN, PROBLEM, &text).unwrap(),
        ferroplan::plan::Validity::Valid
    ));
}

#[test]
fn ppddl_policy_is_bounded_validated_and_seed_replayable() {
    let options = ProbabilisticOptions {
        horizon: Some(2),
        max_states: 128,
        max_transitions: 1_024,
        max_policy_entries: 128,
        max_value_cells: 1_024,
        simulation_max_steps: 32,
        threads: 1,
        ..Default::default()
    };
    let solution = solve_ppddl(RETRY_DOMAIN, RETRY_PROBLEM, &options).unwrap();
    let validation =
        validate_ppddl_policy(RETRY_DOMAIN, RETRY_PROBLEM, &options, &solution).unwrap();
    assert!(validation.valid, "{:?}", validation.errors);
    let first = ferroplan::simulate_ppddl(RETRY_DOMAIN, RETRY_PROBLEM, &options, 1_000, 7)
        .unwrap();
    let second = ferroplan::simulate_ppddl(RETRY_DOMAIN, RETRY_PROBLEM, &options, 1_000, 7)
        .unwrap();
    assert_eq!(first.reached_goal, second.reached_goal);
    assert_eq!(first.average_reward, second.average_reward);
    assert_eq!(first.average_discounted_reward, second.average_discounted_reward);
    assert_eq!(first.average_steps, second.average_steps);
}