use ferroplan::{
    decompose_production, parse_production, solve_ppddl_production, solve_production,
    trace_production, validate_plan_production, Options, OutcomeClass, ProbabilisticOptions,
    ProductionLimits, ProductionSession, ValidationStatus,
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
fn bounded_parse_accepts_valid_input_and_refuses_malformed_or_oversized_input() {
    let valid = parse_production(DOMAIN, 1024 * 1024, Some("parse-valid"));
    assert_eq!(valid.outcome, OutcomeClass::Solved);
    assert_eq!(valid.validation, ValidationStatus::Valid);
    assert_eq!(valid.authority, "evidence_only");

    let malformed = parse_production("(define (domain broken)", 1024, None);
    assert_eq!(malformed.outcome, OutcomeClass::Refused);
    assert_eq!(
        malformed.error.as_ref().map(|error| error.code.as_str()),
        Some("FP_PARSE")
    );

    let oversized = parse_production(DOMAIN, 8, None);
    assert_eq!(oversized.outcome, OutcomeClass::Refused);
    assert_eq!(
        oversized.error.as_ref().map(|error| error.code.as_str()),
        Some("FP_LIMIT_INPUT")
    );
}

#[test]
fn bounded_trace_replays_an_independently_validated_candidate() {
    let solve = solve_production(
        DOMAIN,
        PROBLEM,
        &Options::default(),
        &ProductionLimits::default(),
        Some("trace-source"),
    );
    assert_eq!(solve.outcome, OutcomeClass::Solved);
    let plan = solve.payload.unwrap().plan.unwrap();
    let steps: Vec<(String, Vec<String>)> = plan
        .steps
        .iter()
        .map(|step| (step.action.clone(), step.args.clone()))
        .collect();
    let traced = trace_production(
        DOMAIN,
        PROBLEM,
        &steps,
        &ProductionLimits::default(),
        Some("trace-replay"),
    );
    assert_eq!(traced.outcome, OutcomeClass::Solved);
    assert_eq!(traced.validation, ValidationStatus::Valid);
    let snapshots = traced.payload.unwrap();
    assert_eq!(snapshots.len(), steps.len() + 1);
    assert!(snapshots
        .last()
        .unwrap()
        .facts
        .iter()
        .any(|fact| fact == "(DONE)"));

    let invalid = trace_production(
        DOMAIN,
        PROBLEM,
        &[("MISSING".to_string(), Vec::new())],
        &ProductionLimits::default(),
        None,
    );
    assert_eq!(invalid.outcome, OutcomeClass::Refused);
    assert_eq!(invalid.validation, ValidationStatus::Failed);
}

#[test]
fn production_session_is_budgeted_deterministic_and_replayable() {
    let session = ProductionSession::new(
        DOMAIN,
        PROBLEM,
        &Options::default(),
        ProductionLimits::default(),
    )
    .unwrap();
    let first = session.replan(1_000, Some(64), Some("session-first"));
    let second = session.replan(1_000, Some(64), Some("session-second"));
    assert_eq!(first.outcome, OutcomeClass::Solved);
    assert_eq!(second.outcome, OutcomeClass::Solved);
    assert_eq!(first.validation, ValidationStatus::Valid);
    assert_eq!(
        serde_json::to_value(&first.payload).unwrap(),
        serde_json::to_value(&second.payload).unwrap()
    );
    assert!(session.world_bytes() > 0);
    assert!(session.mind_bytes() > 0);

    let refused = session.replan(0, Some(64), None);
    assert_eq!(refused.outcome, OutcomeClass::Refused);
    assert_eq!(
        refused.error.as_ref().map(|error| error.code.as_str()),
        Some("FP_LIMIT_SEARCH")
    );
}

#[test]
fn production_decomposition_returns_a_valid_stitched_candidate() {
    let decomposition = decompose_production(
        DOMAIN,
        PROBLEM,
        &Options::default(),
        &ProductionLimits::default(),
        Some("decompose"),
    );
    assert_eq!(decomposition.outcome, OutcomeClass::Solved);
    assert_eq!(decomposition.validation, ValidationStatus::Valid);
    let plan = decomposition.payload.unwrap().plan.unwrap();
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
    let validation = validate_plan_production(
        DOMAIN,
        PROBLEM,
        &text,
        1024 * 1024,
        1024 * 1024,
        Some("decompose-validation"),
    );
    assert_eq!(validation.outcome, OutcomeClass::Solved);
    assert_eq!(validation.validation, ValidationStatus::Valid);
}

#[test]
fn production_ppddl_policy_is_bounded_validated_and_replayable() {
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
    let first = solve_ppddl_production(
        RETRY_DOMAIN,
        RETRY_PROBLEM,
        &options,
        1024 * 1024,
        4 * 1024 * 1024,
        Some("ppddl-first"),
    );
    let second = solve_ppddl_production(
        RETRY_DOMAIN,
        RETRY_PROBLEM,
        &options,
        1024 * 1024,
        4 * 1024 * 1024,
        Some("ppddl-second"),
    );
    assert_eq!(first.outcome, OutcomeClass::Solved);
    assert_eq!(first.validation, ValidationStatus::Valid);
    assert_eq!(second.outcome, OutcomeClass::Solved);
    assert_eq!(
        serde_json::to_value(&first.payload).unwrap(),
        serde_json::to_value(&second.payload).unwrap()
    );

    let mut excessive = options;
    excessive.max_states = usize::MAX;
    let refused = solve_ppddl_production(
        RETRY_DOMAIN,
        RETRY_PROBLEM,
        &excessive,
        1024 * 1024,
        4 * 1024 * 1024,
        None,
    );
    assert_eq!(refused.outcome, OutcomeClass::Refused);
    assert_eq!(
        refused.error.as_ref().map(|error| error.code.as_str()),
        Some("FP_LIMIT_SEARCH")
    );
}
