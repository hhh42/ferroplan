//! The goal-level temporal decomposer (`ferroplan::decompose`): split a temporal goal
//! into ordered, individually-solvable contracts, solve and stitch them, and expose
//! the inspectable breakdown. Self-contained domains (independent of the example
//! corpus).

use ferroplan::{decompose, Options};

// Two independent numeric deliverables, each from its own producer. The interaction
// partition makes each numeric goal its own contract, so a conjunctive goal splits.
const TWO_DELIVERABLES: &str = "
(define (domain mk)
  (:requirements :durative-actions :numeric-fluents)
  (:functions (a) (b))
  (:durative-action make-a :parameters () :duration (= ?duration 2)
    :condition () :effect (at end (increase (a) 1)))
  (:durative-action make-b :parameters () :duration (= ?duration 3)
    :condition () :effect (at end (increase (b) 1))))
";
const TWO_PROB: &str = "(define (problem p) (:domain mk)
  (:init (= (a) 0) (= (b) 0))
  (:goal (and (>= (a) 1) (>= (b) 1))))";

#[test]
fn splits_a_conjunctive_goal_into_contracts() {
    let d = decompose(TWO_DELIVERABLES, TWO_PROB, &Options::default()).expect("decompose runs");
    assert!(d.solved, "the goal is solved");
    assert!(!d.monolithic, "two independent deliverables should split");
    assert!(
        d.contracts.len() >= 2,
        "expected >= 2 contracts, got {}",
        d.contracts.len()
    );
    // every contract names a non-empty goal and carries a sub-plan
    for c in &d.contracts {
        assert!(
            !c.goal.is_empty(),
            "contract {} has a rendered goal",
            c.index
        );
        assert!(!c.steps.is_empty(), "contract {} has a sub-plan", c.index);
    }
    // contracts are offset sequentially along the stitched timeline
    let offsets: Vec<f64> = d.contracts.iter().map(|c| c.offset).collect();
    assert!(
        offsets.windows(2).all(|w| w[1] >= w[0]),
        "contract offsets are non-decreasing: {offsets:?}"
    );
    let plan = d.plan.expect("a stitched plan");
    assert!(plan.length >= 2, "stitched plan has all the steps");
}

#[test]
fn stitched_plan_validates_against_the_whole_goal() {
    use ferroplan::parser::{parse_domain, parse_problem};
    let d = decompose(TWO_DELIVERABLES, TWO_PROB, &Options::default()).expect("runs");
    let plan = d.plan.expect("a stitched plan");
    // Rebuild a temporal::TimedPlan from the API steps and validate independently.
    let dom = parse_domain(TWO_DELIVERABLES).unwrap();
    let prob = parse_problem(TWO_PROB).unwrap();
    let steps = plan
        .steps
        .iter()
        .map(|s| ferroplan::temporal::TimedStep {
            time: s.time.unwrap_or(0.0),
            action: if s.args.is_empty() {
                s.action.clone()
            } else {
                format!("{} {}", s.action, s.args.join(" "))
            },
            duration: s.duration,
        })
        .collect();
    let tp = ferroplan::temporal::TimedPlan {
        steps,
        makespan: plan.makespan.unwrap_or(0.0),
    };
    ferroplan::temporal::validate(&dom, &prob, &tp)
        .expect("the stitched decomposition validates against the original goal");
}

// A single linear accumulation is hand-whole-able (BORDERS): it must NOT be split.
const SINGLE: &str = "
(define (domain acc)
  (:requirements :durative-actions :numeric-fluents)
  (:functions (x))
  (:durative-action step :parameters () :duration (= ?duration 1)
    :condition () :effect (at end (increase (x) 1))))
";
const SINGLE_PROB: &str = "(define (problem p) (:domain acc) (:init (= (x) 0)) (:goal (>= (x) 3)))";

#[test]
fn single_goal_falls_back_to_one_monolithic_contract() {
    let d = decompose(SINGLE, SINGLE_PROB, &Options::default()).expect("runs");
    assert!(d.solved);
    assert!(d.monolithic, "a lone numeric goal is not split");
    assert_eq!(d.contracts.len(), 1, "exactly one (whole-goal) contract");
}
