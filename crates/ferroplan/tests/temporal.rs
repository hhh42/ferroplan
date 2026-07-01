//! Temporal (PDDL2.1 durative-action) parsing + snap-action compilation
//! (EPIC-Temporal T1/T2).

use ferroplan::ground::{ground, Outcome};
use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::temporal;
use ferroplan::types::{Expr, Formula, TimeSpec};

const DOM: &str = "
(define (domain temporal-test)
  (:requirements :strips :typing :durative-actions :numeric-fluents)
  (:types location)
  (:predicates (at ?l - location) (connected ?a ?b - location) (free))
  (:functions (dist ?a ?b - location))
  (:durative-action move
    :parameters (?from ?to - location)
    :duration (= ?duration (dist ?from ?to))
    :condition (and (at start (at ?from))
                    (at start (connected ?from ?to))
                    (over all (free)))
    :effect (and (at start (not (at ?from)))
                 (at end (at ?to)))))
";

#[test]
fn parses_durative_action() {
    let d = parse_domain(DOM).expect("durative domain should parse");
    assert_eq!(d.durative_actions.len(), 1, "one durative action");
    let a = &d.durative_actions[0];
    assert_eq!(a.name, "MOVE");
    assert_eq!(a.params.len(), 2);

    // duration = (dist ?from ?to) — a fixed `=` collapses min == max to the fluent.
    assert!(
        matches!(a.duration.chosen(), Some(Expr::Fluent(f, _)) if f == "DIST"),
        "duration is the dist fluent, got {:?}",
        a.duration
    );
    assert!(
        a.duration.min.is_some() && a.duration.max.is_some(),
        "a fixed `=` duration bounds both sides"
    );

    // conditions: 2 at-start + 1 over-all
    let starts = a
        .conditions
        .iter()
        .filter(|(t, _)| *t == TimeSpec::Start)
        .count();
    let alls = a
        .conditions
        .iter()
        .filter(|(t, _)| *t == TimeSpec::All)
        .count();
    assert_eq!(starts, 2, "two at-start conditions");
    assert_eq!(alls, 1, "one over-all invariant");

    // effects: at-start + at-end
    assert_eq!(a.effects.len(), 2);
    assert!(a.effects.iter().any(|(t, _)| *t == TimeSpec::Start));
    assert!(a.effects.iter().any(|(t, _)| *t == TimeSpec::End));
}

#[test]
fn fixed_numeric_duration_and_single_clauses() {
    let dom = "(define (domain d) (:requirements :durative-actions)
        (:predicates (p) (q))
        (:durative-action a :parameters ()
            :duration (= ?duration 5)
            :condition (at start (p))
            :effect (at end (q))))";
    let d = parse_domain(dom).expect("parse");
    let a = &d.durative_actions[0];
    assert!(matches!(a.duration.chosen(), Some(Expr::Num(n)) if (n - 5.0).abs() < 1e-9));
    assert_eq!(a.conditions.len(), 1);
    assert_eq!(a.effects.len(), 1);
    assert_eq!(a.conditions[0].0, TimeSpec::Start);
    assert_eq!(a.effects[0].0, TimeSpec::End);
}

#[test]
fn classic_action_domains_still_parse() {
    // adding the durative machinery must not break non-temporal domains
    let dom = "(define (domain d) (:requirements :strips)
        (:predicates (p) (q))
        (:action a :parameters () :precondition (p) :effect (q)))";
    let d = parse_domain(dom).expect("parse");
    assert_eq!(d.actions.len(), 1);
    assert_eq!(d.durative_actions.len(), 0);
}

const DUR_DOM: &str = "
(define (domain t)
  (:requirements :strips :durative-actions :numeric-fluents)
  (:predicates (at) (goal) (light))
  (:durative-action act
    :parameters ()
    :duration (= ?duration 3)
    :condition (and (at start (at)) (over all (light)))
    :effect (and (at start (not (at))) (at end (goal)))))";
const DUR_PROB: &str = "(define (problem p) (:domain t) (:init (at) (light)) (:goal (goal)))";

#[test]
fn compiles_durative_to_snaps_and_grounds() {
    let dom = parse_domain(DUR_DOM).expect("domain");
    let prob = parse_problem(DUR_PROB).expect("problem");
    let c = temporal::compile(&dom, &prob);

    assert_eq!(c.snaps.len(), 1);
    let s = &c.snaps[0];
    assert_eq!(s.start_action, "ACT-START");
    assert_eq!(s.end_action, "ACT-END");
    assert!(matches!(s.duration.chosen(), Some(Expr::Num(n)) if (n - 3.0).abs() < 1e-9));
    // the over-all invariant is captured (the (light) atom, not True)
    assert!(matches!(&s.invariant, Formula::Atom(p, _) if p == "LIGHT"));
    assert!(
        c.domain.durative_actions.is_empty(),
        "durative compiled away"
    );

    let names: Vec<&str> = c.domain.actions.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"ACT-START") && names.contains(&"ACT-END"));

    // and the compiled classical domain grounds, with both snaps reachable
    match ground(&c.domain, &c.problem, 1) {
        Outcome::Task(t) => {
            let ops: Vec<&str> = t.op_display.iter().map(|s| s.as_str()).collect();
            assert!(
                ops.iter().any(|o| o.starts_with("ACT-START")),
                "start reachable"
            );
            assert!(
                ops.iter().any(|o| o.starts_with("ACT-END")),
                "end reachable"
            );
        }
        _ => panic!("compiled temporal domain should ground to a task"),
    }
}

#[test]
fn t3_decision_epoch_solves_simple_durative() {
    let dom = parse_domain(DUR_DOM).expect("domain");
    let prob = parse_problem(DUR_PROB).expect("problem");
    let plan = temporal::solve(&dom, &prob, 1).expect("temporal plan");
    // one durative step: ACT at time 0 with duration 3; makespan 3
    assert_eq!(plan.steps.len(), 1, "one durative action, end implied");
    assert_eq!(plan.steps[0].action, "ACT");
    assert!((plan.steps[0].time - 0.0).abs() < 1e-9);
    assert_eq!(plan.steps[0].duration, Some(3.0));
    assert!(
        (plan.makespan - 3.0).abs() < 1e-9,
        "makespan {}",
        plan.makespan
    );
}

#[test]
fn t3_required_concurrency_match_fuse() {
    // classic required-concurrency: mend-fuse must run *while* the match is lit;
    // the match (duration 5) provides (light) over its interval, mend (duration 2)
    // needs (light) over all. Sequential ordering can't work; concurrency must.
    let dom = "
    (define (domain mf)
      (:requirements :strips :durative-actions :numeric-fluents)
      (:predicates (light) (mended) (unused))
      (:durative-action light-match
        :parameters ()
        :duration (= ?duration 5)
        :condition (at start (unused))
        :effect (and (at start (and (light) (not (unused)))) (at end (not (light)))))
      (:durative-action mend-fuse
        :parameters ()
        :duration (= ?duration 2)
        :condition (over all (light))
        :effect (at end (mended))))";
    let prob = "(define (problem p) (:domain mf) (:init (unused)) (:goal (mended)))";
    let d = parse_domain(dom).expect("domain");
    let p = parse_problem(prob).expect("problem");
    let plan = temporal::solve(&d, &p, 1).expect("must find a concurrent plan");
    // both durative actions appear; mend-fuse runs within light-match's interval
    assert!(plan.steps.iter().any(|s| s.action == "LIGHT-MATCH"));
    assert!(plan.steps.iter().any(|s| s.action == "MEND-FUSE"));
    temporal::validate(&d, &p, &plan).expect("required-concurrency plan must validate");
}

#[test]
fn t4_public_api_routes_durative_to_temporal() {
    use ferroplan::{solve, Mode, Options};
    let plan = solve(DUR_DOM, DUR_PROB, &Options::default()).expect("solve");
    let p = plan.plan.expect("plan");
    assert_eq!(plan.mode, Mode::Temporal);
    assert_eq!(p.makespan, Some(3.0));
    assert_eq!(p.steps.len(), 1);
    assert_eq!(p.steps[0].action, "ACT");
    assert_eq!(p.steps[0].time, Some(0.0));
    assert_eq!(p.steps[0].duration, Some(3.0));
}

#[test]
fn t3_parameter_dependent_duration() {
    // duration = (dist ?from ?to) — a per-grounding value read from the init.
    // Two hops with different distances prove the duration is evaluated per
    // grounded action, not as a single constant.
    let dom = "
    (define (domain fly)
      (:requirements :typing :durative-actions :numeric-fluents)
      (:types loc)
      (:predicates (at ?l - loc))
      (:functions (dist ?a ?b - loc))
      (:durative-action fly
        :parameters (?from ?to - loc)
        :duration (= ?duration (dist ?from ?to))
        :condition (at start (at ?from))
        :effect (and (at start (not (at ?from))) (at end (at ?to)))))";
    let prob = "
    (define (problem p) (:domain fly)
      (:objects a b c - loc)
      (:init (at a) (= (dist a b) 7) (= (dist b c) 4))
      (:goal (at c)))";
    let d = parse_domain(dom).expect("domain");
    let p = parse_problem(prob).expect("problem");
    let plan = temporal::solve(&d, &p, 1).expect("temporal plan with param durations");
    let fly_ab = plan
        .steps
        .iter()
        .find(|s| s.action == "FLY A B")
        .expect("fly a b");
    let fly_bc = plan
        .steps
        .iter()
        .find(|s| s.action == "FLY B C")
        .expect("fly b c");
    assert_eq!(fly_ab.duration, Some(7.0), "dist a b = 7");
    assert_eq!(fly_bc.duration, Some(4.0), "dist b c = 4");
    // a then b then c, sequential: 7 + 4 = 11, plus one ε gap (fly b->c starts
    // just after fly a->b's at-end effect lands, for PDDL2.1 separation).
    assert!(
        plan.makespan >= 11.0 && plan.makespan < 11.0 + 0.01,
        "makespan {}",
        plan.makespan
    );
    temporal::validate(&d, &p, &plan).expect("param-duration plan must validate");
}

#[test]
fn validate_accepts_and_rejects_plans() {
    let d = parse_domain(DUR_DOM).expect("domain");
    let p = parse_problem(DUR_PROB).expect("problem");
    let plan = temporal::solve(&d, &p, 1).expect("plan");

    // positive: the solver's own plan executes and reaches the goal
    temporal::validate(&d, &p, &plan).expect("solved plan must validate");

    // negative: a tampered duration is caught by the domain cross-check
    let mut bad_dur = plan.clone();
    let s = bad_dur
        .steps
        .iter_mut()
        .find(|s| s.duration.is_some())
        .expect("a durative step");
    s.duration = Some(s.duration.unwrap() + 1.0);
    assert!(
        temporal::validate(&d, &p, &bad_dur).is_err(),
        "a wrong duration must be rejected"
    );

    // negative: the empty plan cannot achieve a non-trivial goal
    let empty = temporal::TimedPlan {
        steps: vec![],
        makespan: 0.0,
    };
    assert!(
        temporal::validate(&d, &p, &empty).is_err(),
        "empty plan must not validate against a real goal"
    );
}

// A renewable resource (a worker/crew pool): consumed at-start, released at-end,
// guarded by an at-start `>=` precondition. This is the durative resource-
// allocation pattern (workers, tools, machines, bandwidth). The decision-epoch
// search must hold the resource over each action's interval, so a tight pool
// forces serialization and a larger pool allows overlap.
const RESOURCE_DOM: &str = "
(define (domain crew)
  (:requirements :typing :durative-actions :numeric-fluents)
  (:types task)
  (:predicates (done ?t - task))
  (:functions (avail))
  (:durative-action do
    :parameters (?t - task)
    :duration (= ?duration 5)
    :condition (at start (>= (avail) 1))
    :effect (and (at start (decrease (avail) 1))
                 (at end (increase (avail) 1))
                 (at end (done ?t)))))
";

fn crew_makespan(capacity: u32) -> f64 {
    let prob = format!(
        "(define (problem c) (:domain crew) (:objects t1 t2 - task)
           (:init (= (avail) {capacity})) (:goal (and (done t1) (done t2))))"
    );
    let d = parse_domain(RESOURCE_DOM).expect("resource domain parses");
    let p = parse_problem(&prob).expect("resource problem parses");
    temporal::solve(&d, &p, 1)
        .expect("a resource-respecting plan exists")
        .makespan
}

#[test]
fn renewable_resource_limits_concurrency() {
    // Pool of 1: the two unit-cost (dur 5) tasks cannot overlap -> serialized ~10.
    let cap1 = crew_makespan(1);
    assert!(
        cap1 > 9.9 && cap1 < 10.5,
        "cap=1 must serialize (~10), got {cap1}"
    );
    // Pool of 2: they run concurrently -> ~5.
    let cap2 = crew_makespan(2);
    assert!(
        cap2 > 4.9 && cap2 < 5.5,
        "cap=2 allows overlap (~5), got {cap2}"
    );
    // The pool actually constrains the schedule.
    assert!(
        cap1 > cap2 + 4.0,
        "a larger resource pool must shorten the makespan ({cap1} vs {cap2})"
    );
}

// ---------------------------------------------------------------------------
// Duration inequalities: `(and (>= ?duration L) (<= ?duration U))` and friends.
// ---------------------------------------------------------------------------

const INEQ_DOM: &str = "
(define (domain ineq)
  (:requirements :durative-actions)
  (:predicates (done))
  (:durative-action work
    :parameters ()
    :duration (and (>= ?duration 2) (<= ?duration 5))
    :condition ()
    :effect (at end (done))))
";
const INEQ_PROB: &str = "(define (problem w) (:domain ineq) (:init) (:goal (done)))";

#[test]
fn duration_inequality_parses_both_bounds() {
    let d = parse_domain(INEQ_DOM).expect("inequality domain parses");
    let a = &d.durative_actions[0];
    assert!(
        matches!(&a.duration.min, Some(Expr::Num(n)) if (*n - 2.0).abs() < 1e-9),
        "min bound 2, got {:?}",
        a.duration.min
    );
    assert!(
        matches!(&a.duration.max, Some(Expr::Num(n)) if (*n - 5.0).abs() < 1e-9),
        "max bound 5, got {:?}",
        a.duration.max
    );
}

#[test]
fn duration_inequality_solves_shortest_feasible() {
    let d = parse_domain(INEQ_DOM).expect("parses");
    let p = parse_problem(INEQ_PROB).expect("parses");
    let plan = temporal::solve(&d, &p, 1).expect("a plan exists");
    temporal::validate(&d, &p, &plan).expect("plan validates");
    // The search commits to the lower bound (shortest feasible) -> makespan 2.
    assert!(
        (plan.makespan - 2.0).abs() < 1e-6,
        "shortest-feasible duration is the lower bound 2, got makespan {}",
        plan.makespan
    );
}

#[test]
fn validator_accepts_any_duration_in_range_and_rejects_outside() {
    let d = parse_domain(INEQ_DOM).expect("parses");
    let p = parse_problem(INEQ_PROB).expect("parses");
    // Base the plan on a real solve, then re-time just the duration — so the action
    // name/format matches exactly what the validator reconstructs snap names from.
    let base = temporal::solve(&d, &p, 1).expect("solves");
    let step = |dur: f64| {
        let mut pl = base.clone();
        pl.steps[0].duration = Some(dur);
        pl.makespan = dur;
        pl
    };
    // anywhere inside [2, 5] is legal
    temporal::validate(&d, &p, &step(2.0)).expect("min bound valid");
    temporal::validate(&d, &p, &step(3.5)).expect("interior duration valid");
    temporal::validate(&d, &p, &step(5.0)).expect("max bound valid");
    // outside the band is not
    assert!(
        temporal::validate(&d, &p, &step(1.0)).is_err(),
        "below the minimum must be rejected"
    );
    assert!(
        temporal::validate(&d, &p, &step(6.0)).is_err(),
        "above the maximum must be rejected"
    );
}

#[test]
fn single_sided_lower_bound_parses_and_solves() {
    let dom = "
(define (domain lb)
  (:requirements :durative-actions)
  (:predicates (done))
  (:durative-action work :parameters ()
    :duration (>= ?duration 3)
    :condition () :effect (at end (done))))
";
    let prob = "(define (problem w) (:domain lb) (:init) (:goal (done)))";
    let d = parse_domain(dom).expect("parses");
    let p = parse_problem(prob).expect("parses");
    let a = &d.durative_actions[0];
    assert!(
        a.duration.min.is_some() && a.duration.max.is_none(),
        "lower-only"
    );
    let plan = temporal::solve(&d, &p, 1).expect("solves");
    temporal::validate(&d, &p, &plan).expect("validates");
    assert!((plan.makespan - 3.0).abs() < 1e-6, "uses the lower bound 3");
}

// ---------------------------------------------------------------------------
// Timed initial literals: `(at <time> <literal>)` in :init.
// ---------------------------------------------------------------------------

// A gate opens at t=5 (a positive TIL). `pass` (dur 2) needs `(open)` at start, so
// no plan can finish before t=7. The only achiever of `(open)` is the TIL — without
// TIL support the goal would be a relaxed dead end.
const TIL_DOM: &str = "
(define (domain gate)
  (:requirements :durative-actions)
  (:predicates (open) (through))
  (:durative-action pass
    :parameters ()
    :duration (= ?duration 2)
    :condition (at start (open))
    :effect (at end (through))))
";
const TIL_PROB: &str = "(define (problem g) (:domain gate)
  (:init (at 5 (open)))
  (:goal (through)))";

#[test]
fn timed_initial_literal_parses() {
    let p = parse_problem(TIL_PROB).expect("problem with a TIL parses");
    assert_eq!(p.til.len(), 1, "one timed initial literal");
    let t = &p.til[0];
    assert!((t.time - 5.0).abs() < 1e-9 && t.add && t.pred == "OPEN");
    // the ordinary `(at ?x ?y)` predicate form must NOT be read as a TIL
    let p2 = parse_problem("(define (problem q) (:domain d) (:init (at a0 hub)) (:goal (done)))")
        .expect("parses");
    assert!(p2.til.is_empty(), "`(at a0 hub)` is a predicate, not a TIL");
    assert_eq!(p2.init_atoms.len(), 1);
}

#[test]
fn timed_initial_literal_gates_the_action() {
    let d = parse_domain(TIL_DOM).expect("parses");
    let p = parse_problem(TIL_PROB).expect("parses");
    let plan = temporal::solve(&d, &p, 1).expect("a TIL-enabled plan exists");
    temporal::validate(&d, &p, &plan).expect("plan validates with the TIL replayed");
    // `pass` can't start before the gate opens at 5, so it ends no earlier than 7.
    assert!(
        plan.makespan >= 7.0 - 1e-6,
        "the action is gated behind the t=5 TIL, makespan {} should be >= 7",
        plan.makespan
    );
}

#[test]
fn negative_timed_initial_literal_closes_a_window() {
    // `(door)` is open from the start but a TIL shuts it at t=3. `pass` (dur 2) needs
    // the door over-all, so it must start at 0 and finish by 2 — before the door shuts.
    let dom = "
(define (domain win)
  (:requirements :durative-actions)
  (:predicates (door) (through))
  (:durative-action pass :parameters ()
    :duration (= ?duration 2)
    :condition (over all (door))
    :effect (at end (through))))
";
    let prob = "(define (problem w) (:domain win)
      (:init (door) (at 3 (not (door))))
      (:goal (through)))";
    let d = parse_domain(dom).expect("parses");
    let p = parse_problem(prob).expect("parses");
    assert_eq!(p.til.len(), 1);
    assert!(!p.til[0].add, "a `(not ...)` TIL is a retraction");
    let plan = temporal::solve(&d, &p, 1).expect("a plan within the window exists");
    temporal::validate(&d, &p, &plan).expect("validates");
}

// ---------------------------------------------------------------------------
// Goal-relevance pruning (default-on, v0.2.2) + static unproducibility.
// ---------------------------------------------------------------------------

/// The rpg-world bread-line setup: a single fully-featured hub whose dozens of
/// unbounded accumulator actions (forage-food, gather-herbs, …) drowned the
/// unpruned complete search. Read from examples/ like the espc test reads
/// benchmarks/.
fn bread_line_srcs() -> (String, String) {
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/rpg-world");
    (
        std::fs::read_to_string(format!("{base}/domain.pddl")).unwrap(),
        std::fs::read_to_string(format!("{base}/hard/bread-line.pddl")).unwrap(),
    )
}

#[test]
fn relevance_pruning_solves_deep_chain_on_featureful_hub() {
    // `flour >= 2` is a 5-step chain (till -> plant -> irrigate -> harvest -> mill),
    // but before pruning graduated to the default tier the search exhausted its
    // node budget (~45 s in release) in the irrelevant-accumulator swamp and FAILED.
    // With default options it must now solve fast; the pass structure keeps an
    // unmasked complete backstop, so this asserts pure gain.
    let (d_src, p_src) = bread_line_srcs();
    let p_src = p_src.replace("(>= (bread) 2)", "(>= (flour) 2)");
    let d = parse_domain(&d_src).expect("rpg-world parses");
    let p = parse_problem(&p_src).expect("flour problem parses");
    let plan = temporal::solve(&d, &p, 1).expect("flour >= 2 must solve under defaults");
    temporal::validate(&d, &p, &plan).expect("flour plan validates");
    assert!(
        plan.steps.len() <= 8,
        "the chain is ~5 steps, got {}",
        plan.steps.len()
    );
}

#[test]
fn statically_unproducible_goal_fails_fast() {
    // A numeric goal whose fluent nothing can raise (`gold` only ever decreases)
    // must be rejected by the static no-producer check immediately, not by
    // exhausting every search pass' node budget. (This was rpg-world bread-line's
    // real story pre-v0.2.2: `bake-bread` yielded meals, so `(bread) >= 2` was
    // unproducible and the search burned ~45 s in release proving the inevitable.)
    let dom = "
(define (domain nogold)
  (:requirements :durative-actions :numeric-fluents)
  (:predicates (idle))
  (:functions (gold) (grain))
  (:durative-action spend :parameters ()
    :duration (= ?duration 1)
    :condition (at start (>= (gold) 1))
    :effect (at start (decrease (gold) 1)))
  (:durative-action farm :parameters ()
    :duration (= ?duration 1)
    :condition ()
    :effect (at end (increase (grain) 1))))
";
    let prob = "(define (problem g) (:domain nogold)
      (:init (= (gold) 1) (= (grain) 0))
      (:goal (and (>= (gold) 5))))";
    let d = parse_domain(dom).expect("parses");
    let p = parse_problem(prob).expect("parses");
    assert!(
        temporal::solve(&d, &p, 1).is_none(),
        "no action raises `gold` — must be unsolvable"
    );
}

#[test]
fn bread_line_solves_after_economy_fix() {
    // With `bake-bread` producing bread (the v0.2.2 domain fix), bread-line is the
    // deep DAG its name promises — farm -> grind -> bake — and must solve under
    // default options thanks to default-on relevance pruning.
    let (d_src, p_src) = bread_line_srcs();
    let d = parse_domain(&d_src).expect("rpg-world parses");
    let p = parse_problem(&p_src).expect("bread-line parses");
    let plan = temporal::solve(&d, &p, 1).expect("bread >= 2 must solve under defaults");
    temporal::validate(&d, &p, &plan).expect("bread plan validates");
}

#[test]
fn statically_unproducible_fact_goal_fails_fast() {
    // Same check for the predicate side: a goal fact with no adder anywhere.
    let dom = "
(define (domain nofact)
  (:requirements :durative-actions)
  (:predicates (a) (b))
  (:durative-action doa :parameters ()
    :duration (= ?duration 1)
    :condition ()
    :effect (at end (a))))
";
    let prob = "(define (problem n) (:domain nofact) (:init) (:goal (and (a) (b))))";
    let d = parse_domain(dom).expect("parses");
    let p = parse_problem(prob).expect("parses");
    assert!(
        temporal::solve(&d, &p, 1).is_none(),
        "goal fact `b` has no adder — must be unsolvable"
    );
}

#[test]
fn validate_plan_compiles_derived_axioms() {
    // The rpg-world domain has a `:derived (reachable ...)` axiom. Every solve path
    // compiles axioms away before grounding, but `plan::validate_plan` (the CLI
    // `--validate` entry) used the raw problem — grounding then missed the derived
    // init facts and rejected VALID plans ("problem grounds to unsolvable" /
    // "unknown action"). Solve corridor-12 and round-trip its IPC text through the
    // string-level validator.
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/rpg-world");
    let d_src = std::fs::read_to_string(format!("{base}/domain.pddl")).unwrap();
    let p_src = std::fs::read_to_string(format!("{base}/hard/corridor-12.pddl")).unwrap();
    let d = parse_domain(&d_src).expect("parses");
    let p = parse_problem(&p_src).expect("parses");
    let (dc, pc) = ferroplan::derived::compile(&d, &p).expect("axioms compile");
    let plan = temporal::solve(&dc, &pc, 1).expect("corridor-12 solves");
    match ferroplan::plan::validate_plan(&d_src, &p_src, &plan.to_ipc()) {
        Ok(ferroplan::plan::Validity::Valid) => {}
        other => panic!("valid corridor-12 plan must validate, got {other:?}"),
    }
}

#[test]
#[ignore = "escalation-ladder integration (~20 s release): rung-0 failure must burn its \
            node budget before the Full-tier rung solves. CI runs it via --ignored."]
fn escalation_ladder_rescues_predicate_build() {
    // cabin/crew-solo's goal (roof-on, door-hung, windows-glazed) needs the Full
    // tier's predicate-goal demand seeding — its own header documents
    // `FF_TDEMAND=1`. Under plain defaults the rung-0 search fails; the ladder's
    // Full rung must rescue it (measured makespan 109) without any flag.
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/cabin");
    let d_src = std::fs::read_to_string(format!("{base}/crew.pddl")).unwrap();
    let p_src = std::fs::read_to_string(format!("{base}/crew-solo.pddl")).unwrap();
    let d = parse_domain(&d_src).expect("parses");
    let p = parse_problem(&p_src).expect("parses");
    let plan = temporal::solve(&d, &p, 1).expect("the escalation ladder must rescue crew-solo");
    temporal::validate(&d, &p, &plan).expect("escalated plan validates");
    assert!(
        (plan.makespan - 109.0).abs() < 2.0,
        "expected the documented ~109 makespan, got {}",
        plan.makespan
    );
}
