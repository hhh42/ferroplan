use ferroplan::{
    explain_production, solve_production, Options, OutcomeClass, ProductionLimits,
    ValidationStatus,
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

#[test]
fn explanation_requires_an_independently_validated_plan() {
    let solved = solve_production(
        DOMAIN,
        PROBLEM,
        &Options::default(),
        &ProductionLimits::default(),
        Some("explain-source"),
    );
    assert_eq!(solved.outcome, OutcomeClass::Solved);
    let plan = solved.payload.unwrap().plan.unwrap();

    let explained = explain_production(
        DOMAIN,
        PROBLEM,
        &plan,
        &ProductionLimits::default(),
        Some("explain-valid"),
    );
    assert_eq!(explained.outcome, OutcomeClass::Solved);
    assert_eq!(explained.validation, ValidationStatus::Valid);
    assert_eq!(explained.authority, "evidence_only");
    assert!(explained.payload.is_some());

    let mut invalid = plan;
    invalid.steps[0].action = "missing-action".to_string();
    let refused = explain_production(
        DOMAIN,
        PROBLEM,
        &invalid,
        &ProductionLimits::default(),
        Some("explain-invalid"),
    );
    assert_eq!(refused.outcome, OutcomeClass::Refused);
    assert_eq!(refused.validation, ValidationStatus::Failed);
    assert_eq!(
        refused.error.as_ref().map(|error| error.code.as_str()),
        Some("FP_VALIDATION")
    );
}

#[test]
fn deterministic_parallel_workers_preserve_the_candidate_plan() {
    let limits = ProductionLimits {
        max_workers: 4,
        ..ProductionLimits::default()
    };
    let one = solve_production(
        DOMAIN,
        PROBLEM,
        &Options {
            threads: 1,
            ..Options::default()
        },
        &limits,
        Some("parallel-one"),
    );
    let two = solve_production(
        DOMAIN,
        PROBLEM,
        &Options {
            threads: 2,
            ..Options::default()
        },
        &limits,
        Some("parallel-two"),
    );

    assert_eq!(one.outcome, OutcomeClass::Solved);
    assert_eq!(two.outcome, OutcomeClass::Solved);
    assert_eq!(one.validation, ValidationStatus::Valid);
    assert_eq!(two.validation, ValidationStatus::Valid);
    assert_eq!(
        serde_json::to_value(one.payload.as_ref().unwrap().plan.as_ref()).unwrap(),
        serde_json::to_value(two.payload.as_ref().unwrap().plan.as_ref()).unwrap()
    );

    let refused = solve_production(
        DOMAIN,
        PROBLEM,
        &Options {
            threads: 5,
            ..Options::default()
        },
        &limits,
        None,
    );
    assert_eq!(refused.outcome, OutcomeClass::Refused);
    assert_eq!(
        refused.error.as_ref().map(|error| error.code.as_str()),
        Some("FP_LIMIT_WORKERS")
    );
}
