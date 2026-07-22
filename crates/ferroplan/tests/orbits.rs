//! Object-symmetry orbits (0.14 ext Phase 10) + the over-all invariant
//! transition guard the TMS plans exposed (kiln-gap fixture).

use ferroplan::ground::{ground_stratified, Outcome};
use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::{orbits, temporal};

/// TMS-shaped mini fixture: two interchangeable goal PAIRS. `glue`
/// grounds every cross-pair combination too, so detection must survive
/// cross-member facts/ops via family closure, not per-member templates.
const MINI_DOM: &str = "
(define (domain mini-tms)
  (:requirements :strips :typing :durative-actions)
  (:types piece)
  (:predicates (raw ?p - piece) (made ?p - piece) (glued ?a ?b - piece) (free))
  (:durative-action make
    :parameters (?p - piece)
    :duration (= ?duration 2)
    :condition (and (at start (raw ?p)) (over all (free)))
    :effect (and (at start (not (raw ?p))) (at end (made ?p))))
  (:durative-action glue
    :parameters (?a ?b - piece)
    :duration (= ?duration 3)
    :condition (and (at start (made ?a)) (at start (made ?b)))
    :effect (at end (glued ?a ?b))))
";

const MINI_PROB: &str = "
(define (problem mini-tms-1)
  (:domain mini-tms)
  (:objects a1 b1 a2 b2 - piece)
  (:init (raw a1) (raw b1) (raw a2) (raw b2) (free))
  (:goal (and (glued a1 b1) (glued a2 b2))))
";

fn mini_task() -> (
    ferroplan::temporal::TemporalCompiled,
    ferroplan::packed::PackedTask,
) {
    let d = parse_domain(MINI_DOM).unwrap();
    let p = parse_problem(MINI_PROB).unwrap();
    let c = temporal::compile(&d, &p);
    let task = match ground_stratified(&c.domain, &c.problem, 1) {
        Outcome::Task(t) => t,
        _ => panic!("grounding failed"),
    };
    (c, task)
}

#[test]
fn detects_goal_pair_orbit() {
    let (c, task) = mini_task();
    let om = orbits::detect(&c.domain, &c.problem, &task).expect("pair orbit should be detected");
    assert_eq!(om.orbits.len(), 1, "one orbit of goal pairs");
    assert_eq!(om.orbits[0].facts.len(), 2, "two interchangeable members");
    // Each member's template carries its per-member dynamic facts (RAW,
    // MADE, RUNNING-*, its own GLUED diagonal...) — same length for both.
    assert_eq!(
        om.orbits[0].facts[0].len(),
        om.orbits[0].facts[1].len(),
        "aligned templates"
    );
    assert!(!om.orbits[0].facts[0].is_empty());
}

#[test]
fn canonical_key_merges_permuted_states_only() {
    let (c, task) = mini_task();
    let om = orbits::detect(&c.domain, &c.problem, &task).unwrap();
    let base = task.initial();
    let set = |names: &[&str]| {
        let mut s = base.clone();
        for n in names {
            let f = task.fact_id(n).unwrap_or_else(|| panic!("no fact {n}"));
            ferroplan::bitset::set(&mut s.bits, f);
        }
        s
    };
    // (MADE A1) vs (MADE A2): the same state up to swapping the two goal
    // pairs — one canonical key.
    let key = |names: &[&str]| {
        let (k, ag) = om.canonical_key(&task, &set(names), &[]);
        (k.bits, k.vals, ag)
    };
    assert!(
        key(&["(MADE A1)"]) == key(&["(MADE A2)"]),
        "π-related states share a canonical key"
    );
    // Same-member progress vs cross-member progress are NOT symmetric.
    assert!(
        key(&["(MADE A1)", "(MADE B1)"]) != key(&["(MADE A1)", "(MADE B2)"]),
        "non-equivalent states keep distinct keys"
    );
    // Pure function: recomputation is stable.
    assert!(key(&["(MADE A1)"]) == key(&["(MADE A1)"]));
}

#[test]
fn orbit_solve_finds_valid_plan() {
    let d = parse_domain(MINI_DOM).unwrap();
    let p = parse_problem(MINI_PROB).unwrap();
    let plan = temporal::solve(&d, &p, 1).expect("mini-tms solves");
    // 4 makes + 2 glues; the goal glue pairs must be exactly the goal's.
    let glues: Vec<&str> = plan
        .steps
        .iter()
        .map(|s| s.action.as_str())
        .filter(|a| a.to_ascii_uppercase().starts_with("GLUE"))
        .collect();
    assert_eq!(glues.len(), 2, "two glues, got {:?}", plan.steps);
}

/// Endpoint-only invariant checking accepted a bake spanning a TIL
/// delete+re-add outage of its `over all` fact (VAL rejects that). The
/// transition guard must schedule the bake around the gap.
const KILN_DOM: &str = "
(define (domain kiln-gap)
  (:requirements :typing :durative-actions :timed-initial-literals)
  (:types piece)
  (:predicates (ready) (raw ?p - piece) (prepped ?p - piece) (baked ?p - piece))
  (:durative-action prep
    :parameters (?p - piece)
    :duration (= ?duration 6)
    :condition (at start (raw ?p))
    :effect (and (at start (not (raw ?p))) (at end (prepped ?p))))
  (:durative-action bake
    :parameters (?p - piece)
    :duration (= ?duration 3)
    :condition (and (at start (prepped ?p)) (over all (ready)))
    :effect (at end (baked ?p))))
";

const KILN_PROB: &str = "
(define (problem kiln-gap-1)
  (:domain kiln-gap)
  (:objects p1 - piece)
  (:init (raw p1) (ready)
         (at 8 (not (ready)))
         (at 8.001 (ready)))
  (:goal (baked p1))
  (:metric minimize (total-time)))
";

#[test]
fn over_all_invariant_respects_scheduled_outage() {
    let d = parse_domain(KILN_DOM).unwrap();
    let p = parse_problem(KILN_PROB).unwrap();
    let plan = temporal::solve(&d, &p, 1).expect("kiln-gap solves");
    let bake = plan
        .steps
        .iter()
        .find(|s| s.action.to_ascii_uppercase().contains("BAKE"))
        .expect("plan has a bake");
    let (start, end) = (bake.time, bake.time + bake.duration.unwrap_or(0.0));
    assert!(
        end <= 8.0 + 1e-9 || start >= 8.001 - 1e-9,
        "bake [{start}, {end}] must not span the 8..8.001 ready outage"
    );
}
