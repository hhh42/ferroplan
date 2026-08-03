//! The static-fluent fold + fluent-space compaction (0.21 Phase 6,
//! docs/roadmap-0.21.md): defined-static fluent reads fold to `NExpr::Num`
//! in effect values, and the fluent tables compact to written+retained
//! fluents behind `FF_NO_FLUENT_FOLD` / `FF_NO_FLUENT_COMPACT`.
//!
//! One sequential test on purpose: both hatches are process-global env
//! knobs (the tests/espc.rs convention), so every scenario — the shrink
//! pin, the byte-identity A/B over the representative spread, and the
//! static-table duration checks — runs inside a single #[test] body.

use ferroplan::ground::{ground, Outcome};
use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::temporal;
use ferroplan::{solve, Options};

fn fixture(name: &str) -> String {
    let p = format!(
        "{}/../../benchmarks/bench/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p}: {e}"))
}

/// tpp-metric's shape in miniature: a price TABLE (goods x places) read
/// only by `increase` effect values (defined-static, irrelevant — folds
/// and compacts away), `request` read by the goal (defined-static but
/// RELEVANT — retained so visited keys and orbits never move), `bought`/
/// `spent` written.
const MINITPP_DOM: &str = "
(define (domain minitpp)
  (:requirements :strips :typing :numeric-fluents)
  (:types place goods)
  (:predicates (at ?p - place) (link ?a ?b - place))
  (:functions (price ?g - goods ?p - place) (bought ?g - goods)
              (request ?g - goods) (spent))
  (:action drive
    :parameters (?a ?b - place)
    :precondition (and (at ?a) (link ?a ?b))
    :effect (and (not (at ?a)) (at ?b)))
  (:action buy
    :parameters (?g - goods ?p - place)
    :precondition (and (at ?p) (< (bought ?g) (request ?g)))
    :effect (and (increase (bought ?g) 1)
                 (increase (spent) (price ?g ?p)))))";

const MINITPP_PRB: &str = "
(define (problem minitpp-1) (:domain minitpp)
  (:objects p1 p2 p3 - place g1 g2 - goods)
  (:init (at p1) (link p1 p2) (link p2 p3) (link p2 p1) (link p3 p2)
         (= (price g1 p1) 4) (= (price g1 p2) 2) (= (price g1 p3) 7)
         (= (price g2 p1) 5) (= (price g2 p2) 9) (= (price g2 p3) 1)
         (= (bought g1) 0) (= (bought g2) 0)
         (= (request g1) 2) (= (request g2) 1)
         (= (spent) 0))
  (:goal (and (>= (bought g1) (request g1)) (>= (bought g2) (request g2)))))";

/// `:action-costs` (the data-network shape: cost tables read only by
/// `increase (total-cost) ...`).
const COSTS_DOM: &str = "
(define (domain costfold)
  (:requirements :strips :typing :action-costs)
  (:types loc)
  (:predicates (at ?l - loc) (road ?a ?b - loc) (visited ?l - loc))
  (:functions (total-cost) (toll ?a ?b - loc))
  (:action go
    :parameters (?a ?b - loc)
    :precondition (and (at ?a) (road ?a ?b))
    :effect (and (not (at ?a)) (at ?b) (visited ?b)
                 (increase (total-cost) (toll ?a ?b)))))";

const COSTS_PRB: &str = "
(define (problem costfold-1) (:domain costfold)
  (:objects a b c d - loc)
  (:init (at a) (road a b) (road b c) (road a c) (road c d)
         (= (toll a b) 1) (= (toll b c) 1) (= (toll a c) 5) (= (toll c d) 2)
         (= (total-cost) 0))
  (:goal (visited d))
  (:metric minimize (total-cost)))";

/// ADL conditional effects whose numeric parts read a static table.
const ADL_DOM: &str = "
(define (domain adlfold)
  (:requirements :adl :typing :numeric-fluents)
  (:types box)
  (:predicates (open ?b - box) (heavy ?b - box) (moved ?b - box))
  (:functions (weight ?b - box) (carried))
  (:action move
    :parameters (?b - box)
    :precondition (not (moved ?b))
    :effect (and (moved ?b)
                 (when (heavy ?b) (increase (carried) (weight ?b))))))";

const ADL_PRB: &str = "
(define (problem adlfold-1) (:domain adlfold)
  (:objects b1 b2 b3 - box)
  (:init (heavy b1) (heavy b3)
         (= (weight b1) 10) (= (weight b2) 1) (= (weight b3) 3)
         (= (carried) 0))
  (:goal (and (moved b1) (moved b2) (moved b3))))";

/// State-dependent duration (the model-train shape) — `work`'s duration
/// reads `(speed)`, which `boost` raises; from tests/temporal.rs.
const DUREXPR_DOM: &str = "
(define (domain durexpr)
  (:requirements :strips :durative-actions :numeric-fluents)
  (:predicates (ready) (boosted) (done))
  (:functions (speed) (progress))
  (:durative-action boost
    :parameters ()
    :duration (= ?duration 1)
    :condition (at start (ready))
    :effect (and (at start (not (ready)))
                 (at start (increase (speed) 2))
                 (at end (boosted))))
  (:durative-action work
    :parameters ()
    :duration (= ?duration (speed))
    :condition (at start (boosted))
    :effect (and (at start (increase (progress) ?duration))
                 (at end (done)))))";

const DUREXPR_PRB: &str = "
(define (problem durexpr-1) (:domain durexpr)
  (:init (ready) (= (speed) 1) (= (progress) 0))
  (:goal (and (done) (>= (progress) 3))))";

struct Run {
    plan: Vec<String>,
    evaluated: usize,
}

fn run_classical(dom: &str, prb: &str) -> Run {
    let sol = solve(dom, prb, &Options::default()).expect("solve");
    assert!(sol.solved, "spread fixture must solve");
    let plan = sol
        .plan
        .as_ref()
        .unwrap()
        .steps
        .iter()
        .map(|s| format!("{} {}", s.action, s.args.join(" ")))
        .collect();
    Run {
        plan,
        evaluated: sol.statistics.evaluated_states,
    }
}

fn run_temporal(dom: &str, prb: &str) -> Vec<(u64, String, Option<u64>)> {
    let d = parse_domain(dom).expect("domain parses");
    let p = parse_problem(prb).expect("problem parses");
    let plan = temporal::solve(&d, &p, 1).expect("temporal fixture must solve");
    plan.steps
        .iter()
        .map(|s| {
            (
                s.time.to_bits(),
                s.action.clone(),
                s.duration.map(f64::to_bits),
            )
        })
        .collect()
}

fn n_fluents(dom: &str, prb: &str) -> usize {
    let d = parse_domain(dom).expect("domain parses");
    let p = parse_problem(prb).expect("problem parses");
    match ground(&d, &p, 1) {
        Outcome::Task(t) => {
            assert_eq!(t.fv0.len(), t.fdef0.len());
            assert_eq!(t.fv0.len(), t.fluent_names.len());
            t.fv0.len()
        }
        _ => panic!("minitpp must ground to a task"),
    }
}

#[test]
fn fold_and_compaction_shrink_tables_and_stay_byte_identical() {
    let hatches = ["FF_NO_FLUENT_FOLD", "FF_NO_FLUENT_COMPACT"];
    let set_all = |on: bool| {
        for h in hatches {
            if on {
                std::env::set_var(h, "1");
            } else {
                std::env::remove_var(h);
            }
        }
    };
    set_all(false);

    // --- the shrink pin (RED before the fold lands) -----------------------
    // minitpp fluents: 6 price + 2 bought + 2 request + 1 spent = 11 raw.
    // The price table is defined-static and irrelevant -> folded + dropped;
    // request is goal-read (relevant) -> retained; bought/spent written.
    let packed = n_fluents(MINITPP_DOM, MINITPP_PRB);
    set_all(true);
    let raw = n_fluents(MINITPP_DOM, MINITPP_PRB);
    set_all(false);
    assert_eq!(raw, 11, "hatched grounding keeps the raw fluent space");
    assert!(
        packed < raw,
        "fold+compaction must shrink the fluent table (raw {raw}, packed {packed})"
    );
    assert_eq!(
        packed, 5,
        "written (bought x2, spent) + relevant statics (request x2) survive"
    );

    // --- the 0.20 Phase 4 bar: hatches OFF vs ON, byte-identical ----------
    // Representative spread: STRIPS (visitgrid), costs (data-network shape),
    // ADL conditional numerics, numeric with a static table (minitpp),
    // temporal with a PURE static duration table (kiln-pack, the
    // elevator-2011 shape), temporal with a state-dependent duration
    // (durexpr). Plans AND eval counts must match exactly.
    let visit_dom = fixture("visitgrid-domain.pddl");
    let visit_prb = fixture("visitgrid-7x7.pddl");
    let kiln_dom = fixture("kiln-pack-domain.pddl");
    let kiln_prb = fixture("kiln-pack-2.pddl");

    let classical: [(&str, &str, &str); 4] = [
        ("visitgrid", &visit_dom, &visit_prb),
        ("costfold", COSTS_DOM, COSTS_PRB),
        ("adlfold", ADL_DOM, ADL_PRB),
        ("minitpp", MINITPP_DOM, MINITPP_PRB),
    ];
    for (name, dom, prb) in classical {
        let on = run_classical(dom, prb);
        set_all(true);
        let off = run_classical(dom, prb);
        set_all(false);
        assert_eq!(on.plan, off.plan, "{name}: plan differs with fold on/off");
        assert_eq!(
            on.evaluated, off.evaluated,
            "{name}: eval count differs with fold on/off"
        );
    }

    let temporal_fixtures: [(&str, &str, &str); 2] = [
        ("kiln-pack-2", &kiln_dom, &kiln_prb),
        ("durexpr", DUREXPR_DOM, DUREXPR_PRB),
    ];
    for (name, dom, prb) in temporal_fixtures {
        let on = run_temporal(dom, prb);
        set_all(true);
        let off = run_temporal(dom, prb);
        set_all(false);
        assert_eq!(
            on, off,
            "{name}: temporal plan (times/durations) differs with fold on/off"
        );
    }

    // kiln-pack durations come from a PURE static table (bake-time per
    // job): with the fold ON they must resolve from the task-side static
    // table to the exact init values.
    let kiln = run_temporal(&kiln_dom, &kiln_prb);
    assert!(
        kiln.iter()
            .any(|(_, a, d)| a.starts_with("BAKE1") && d.is_some()),
        "kiln plan must schedule a bake with a table-resolved duration"
    );

    // Each hatch alone must also hold the identity (fold without
    // compaction, compaction without fold).
    for h in hatches {
        let on = run_classical(MINITPP_DOM, MINITPP_PRB);
        std::env::set_var(h, "1");
        let half = run_classical(MINITPP_DOM, MINITPP_PRB);
        std::env::remove_var(h);
        assert_eq!(on.plan, half.plan, "{h} alone: plan differs");
        assert_eq!(on.evaluated, half.evaluated, "{h} alone: evals differ");
    }
}
