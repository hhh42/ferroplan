//! FF_TDEMAND converging-resource demand guidance on the temporal path: a
//! multi-round numeric goal — each `top` consumes a `mid`, each `mid` consumes a
//! `raw` — where the delete-relaxed heuristic goes flat (it reuses each consumed
//! unit). With the demand term the search gets the full multi-round gradient and
//! produces a VALID timed plan.
//!
//! Self-contained domain (independent of the in-flux example corpus). One test in
//! its own binary so the process-global feature toggle can't race other suites.
//! Coverage gains on the real RPG corpus (+8 instances) are measured separately.
//!
//! As of v0.2 the demand term is **default ON** (graduated from the opt-in
//! `FF_TDEMAND`), so this solves with no flag set; the test also pins the
//! `FF_NO_TDEMAND` opt-out so the old bit-identical-off path stays recoverable.

use ferroplan::temporal::{solve, validate};

const DOM: &str = "(define (domain mr)
  (:requirements :durative-actions :numeric-fluents)
  (:predicates (ready))
  (:functions (raw) (mid) (top))
  (:durative-action gather :parameters ()
    :duration (= ?duration 1)
    :condition (at start (ready))
    :effect (at end (increase (raw) 1)))
  (:durative-action refine :parameters ()
    :duration (= ?duration 1)
    :condition (at start (>= (raw) 1))
    :effect (and (at start (decrease (raw) 1)) (at end (increase (mid) 1))))
  (:durative-action assemble :parameters ()
    :duration (= ?duration 1)
    :condition (at start (>= (mid) 1))
    :effect (and (at start (decrease (mid) 1)) (at end (increase (top) 1)))))";

const PROB: &str = "(define (problem mr3) (:domain mr)
  (:init (ready) (= (raw) 0) (= (mid) 0) (= (top) 0))
  (:goal (>= (top) 3)))";

#[test]
fn tdemand_default_on_solves_validates_and_opt_out_honored() {
    let dom = ferroplan::parser::parse_domain(DOM).expect("domain parses");
    let prob = ferroplan::parser::parse_problem(PROB).expect("problem parses");

    // Graduated to default ON in v0.2: solves with NO flag set.
    let plan = solve(&dom, &prob, 1).expect("default-on demand should solve the multi-round goal");
    // Independent validator: the plan executes legally and reaches top >= 3.
    validate(&dom, &prob, &plan).expect("the demand-guided plan must validate");
    // 3 rounds of gather -> refine -> assemble.
    assert!(
        plan.steps.len() >= 3,
        "expected a real multi-round plan, got {} steps",
        plan.steps.len()
    );

    // The opt-out is honored: FF_NO_TDEMAND flips the feature gate back off, and
    // removing it restores the default. (Tested on the getter, not via a fragile
    // "doesn't solve" assertion — the complete phase-2 pass can still solve slowly.)
    assert!(ferroplan::features::tdemand(), "demand defaults on");
    std::env::set_var("FF_NO_TDEMAND", "1");
    assert!(!ferroplan::features::tdemand(), "FF_NO_TDEMAND opts out");
    std::env::remove_var("FF_NO_TDEMAND");
    assert!(
        ferroplan::features::tdemand(),
        "removing FF_NO_TDEMAND restores default-on"
    );
}
