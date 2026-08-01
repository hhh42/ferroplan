use ferroplan::parse_ppddl;

const DOMAIN: &str = include_str!("../../../examples/daily_agent_methods/domain.ppddl");
const PROBLEM: &str =
    include_str!("../../../examples/daily_agent_methods/problem-2026-07-31.ppddl");
const CATALOG: &str =
    include_str!("../../../examples/daily_agent_methods/method-catalog.json");

#[test]
fn daily_agent_method_corpus_is_admitted_by_the_ppddl_parser() {
    let report = parse_ppddl(DOMAIN, PROBLEM);

    assert!(report.ok, "daily agent PPDDL refused: {:?}", report.error);
    assert_eq!(report.domain.as_deref(), Some("daily-agent-methods"));
    assert_eq!(
        report.problem.as_deref(),
        Some("daily-agent-work-2026-07-31")
    );
    assert_eq!(report.probabilistic_actions, 1);
    assert!(report.normalized_outcomes >= 4);
    assert_eq!(report.initial_outcomes, 1);
}

#[test]
fn daily_agent_method_catalog_remains_bound_to_the_same_snapshot() {
    let catalog: serde_json::Value = serde_json::from_str(CATALOG).expect("catalog JSON");

    assert_eq!(catalog["schema"], "daily-agent-method-catalog/v1");
    assert_eq!(catalog["snapshot_date"], "2026-07-31");
    assert_eq!(catalog["timezone"], "America/Los_Angeles");
    assert_eq!(catalog["standing"], "PARTIAL_ALIVE");
    assert_eq!(catalog["patterns"].as_array().map(Vec::len), Some(20));
}

#[test]
fn probability_mass_above_one_is_refused() {
    let invalid_domain = r#"
    (define (domain invalid-daily-agent-probability)
      (:requirements :strips :probabilistic-effects)
      (:predicates (done) (queued))
      (:action external-edge
        :parameters ()
        :precondition (and)
        :effect (probabilistic
          0.75 (done)
          0.50 (queued))))
    "#;
    let invalid_problem = r#"
    (define (problem invalid-daily-agent-probability-p)
      (:domain invalid-daily-agent-probability)
      (:init)
      (:goal (done)))
    "#;

    let report = parse_ppddl(invalid_domain, invalid_problem);
    assert!(!report.ok, "probability mass above one was admitted");
    assert!(report.error.is_some());
}
