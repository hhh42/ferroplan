//! The classical symmetry consumer (0.22 Phase 6, docs/roadmap-0.22.md):
//! canonical visited keys on the satisficing dedup (L3), the
//! metric/cost-uniformity gate (L2), unary-goal SOLO units (L4), and the
//! goal-free passdown view for partition subsearches (L5). The optimal
//! (L1) assertions ride the separate optimal-hook commit.
//!
//! No test here touches env vars: detection and the searches are called
//! directly with the orbit handed in (or not), so the file is safe under
//! the parallel test harness.

use ferroplan::ground::{ground, Outcome};
use ferroplan::orbits;
use ferroplan::packed::PackedTask;
use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::search::{search_from, PlanResult, SearchCfg};
use ferroplan::types::{Domain, Problem};

/// Mini child-snack, classical: three children with UNARY served goals
/// (the L4 shape), four interchangeable sandwiches and three breads with
/// NO goal appearance (goal-free SOLO — the L5 constituency), one tray.
const SNACK_DOM: &str = "
(define (domain mini-snack)
  (:requirements :strips :typing)
  (:types child bread sandwich tray)
  (:predicates (at-kitchen-bread ?b - bread) (at-kitchen-sandwich ?s - sandwich)
               (notexist ?s - sandwich) (ontray ?s - sandwich ?t - tray)
               (served ?c - child) (waiting ?c - child))
  (:action make
    :parameters (?s - sandwich ?b - bread)
    :precondition (and (notexist ?s) (at-kitchen-bread ?b))
    :effect (and (not (notexist ?s)) (not (at-kitchen-bread ?b))
                 (at-kitchen-sandwich ?s)))
  (:action put
    :parameters (?s - sandwich ?t - tray)
    :precondition (at-kitchen-sandwich ?s)
    :effect (and (not (at-kitchen-sandwich ?s)) (ontray ?s ?t)))
  (:action serve
    :parameters (?s - sandwich ?t - tray ?c - child)
    :precondition (and (ontray ?s ?t) (waiting ?c))
    :effect (and (not (ontray ?s ?t)) (not (waiting ?c)) (served ?c))))
";

const SNACK_PRB: &str = "
(define (problem mini-snack-1)
  (:domain mini-snack)
  (:objects c1 c2 c3 - child b1 b2 b3 - bread s1 s2 s3 s4 - sandwich t1 - tray)
  (:init (waiting c1) (waiting c2) (waiting c3)
         (at-kitchen-bread b1) (at-kitchen-bread b2) (at-kitchen-bread b3)
         (notexist s1) (notexist s2) (notexist s3) (notexist s4))
  (:goal (and (served c1) (served c2) (served c3))))
";

fn task_of(dom: &str, prb: &str) -> (Domain, Problem, PackedTask) {
    let d = parse_domain(dom).unwrap();
    let p = parse_problem(prb).unwrap();
    let task = match ground(&d, &p, 1) {
        Outcome::Task(t) => t,
        _ => panic!("grounding failed"),
    };
    (d, p, task)
}

fn replay_solves(task: &PackedTask, ops: &[usize]) -> bool {
    let mut s = task.initial();
    for &oi in ops {
        if !task.op_applicable(oi, &s) {
            return false;
        }
        s = task.apply(oi, &s);
    }
    task.goal_met(&s)
}

/// L4: the unary-goal children form an orbit (goal-bound), and the
/// goal-free sandwiches/breads form theirs — detection fires on the
/// child-snack shape with nothing else needed.
#[test]
fn unary_goal_solo_units_detected_and_goal_bound() {
    let (d, p, task) = task_of(SNACK_DOM, SNACK_PRB);
    let om = orbits::detect_classical(&d, &p, &task).expect("mini-snack detects");
    let sizes: Vec<usize> = om.orbits.iter().map(|o| o.facts.len()).collect();
    assert!(sizes.contains(&3), "children (and breads) k=3: {sizes:?}");
    assert!(sizes.contains(&4), "sandwiches k=4: {sizes:?}");
    // Exactly one orbit is goal-bound (the children); sandwiches and
    // breads are the goal-free constituency the L5 view keeps.
    assert_eq!(
        om.goal_bound.iter().filter(|&&b| b).count(),
        1,
        "goal_bound {:?}",
        om.goal_bound
    );
    assert!(
        om.goal_free_view().is_some(),
        "goal-free orbits must survive the view"
    );
}

/// The canonical classical key merges π-related states only.
#[test]
fn canonical_skey_merges_permuted_states_only() {
    let (d, p, task) = task_of(SNACK_DOM, SNACK_PRB);
    let om = orbits::detect_classical(&d, &p, &task).unwrap();
    let after = |ops: &[&str]| {
        let mut s = task.initial();
        for name in ops {
            let oi = task
                .op_display
                .iter()
                .position(|o| o == name)
                .unwrap_or_else(|| panic!("no op {name}"));
            assert!(task.op_applicable(oi, &s), "{name} inapplicable");
            s = task.apply(oi, &s);
        }
        s
    };
    let key = |s: &ferroplan::packed::State| om.canonical_skey(&task, s, None);
    // Same state up to sandwich AND bread relabeling: one key.
    assert!(
        key(&after(&["MAKE S1 B1"])) == key(&after(&["MAKE S2 B2"])),
        "π-related states share a canonical key"
    );
    // Progress differs (one sandwich made vs two): distinct keys.
    assert!(
        key(&after(&["MAKE S1 B1"])) != key(&after(&["MAKE S1 B1", "MAKE S2 B2"])),
        "non-equivalent states keep distinct keys"
    );
    // Pure function: stable on recomputation.
    assert!(key(&after(&["MAKE S3 B2"])) == key(&after(&["MAKE S3 B2"])));
}

/// L3: the satisficing dedup with the orbit handed in pays fewer
/// evaluations than the plain run, finds a valid plan, and stays
/// thread-count independent (t1 ≡ t8).
#[test]
fn satisficing_canonical_dedup_shrinks_the_sweep() {
    let (d, p, task) = task_of(SNACK_DOM, SNACK_PRB);
    let om = orbits::detect_classical(&d, &p, &task).unwrap();
    let run = |orbit: Option<&orbits::OrbitMap>, threads: usize| match search_from(
        &task,
        &task.initial(),
        &task.goal_pos,
        &task.goal_num,
        None,
        f64::INFINITY,
        threads,
        SearchCfg::from_weights(1.0, 5.0, None),
        &[],
        None,
        None,
        orbit,
    ) {
        PlanResult::Plan { ops, evaluated, .. } => (ops, evaluated),
        PlanResult::Unsolvable { .. } => panic!("mini-snack solves"),
    };
    let (plain_ops, plain_evals) = run(None, 1);
    let (orbit_ops, orbit_evals) = run(Some(&om), 1);
    assert!(replay_solves(&task, &plain_ops));
    assert!(replay_solves(&task, &orbit_ops), "orbit plan must replay");
    assert!(
        orbit_evals * 2 <= plain_evals,
        "canonical dedup must shrink the sweep: {orbit_evals} vs {plain_evals}"
    );
    // Thread-count independence: same plan, same eval count at t8.
    let (orbit_ops8, orbit_evals8) = run(Some(&om), 8);
    assert_eq!(orbit_ops, orbit_ops8, "t1 == t8 plan");
    assert_eq!(orbit_evals, orbit_evals8, "t1 == t8 evals");
}

/// L5: the goal-free view freezes the children (their goals distinguish
/// them inside a subgoal split) while sandwiches keep merging.
#[test]
fn goal_free_view_freezes_goal_bound_orbits_only() {
    let (d, p, task) = task_of(SNACK_DOM, SNACK_PRB);
    let om = orbits::detect_classical(&d, &p, &task).unwrap();
    let view = om.goal_free_view().expect("sandwiches+breads survive");
    let serve = |c: &str, s: &ferroplan::packed::State| {
        let mut s = s.clone();
        let served = task.fact_id(&format!("(SERVED {c})")).unwrap();
        let waiting = task.fact_id(&format!("(WAITING {c})")).unwrap();
        ferroplan::bitset::set(&mut s.bits, served);
        ferroplan::bitset::clear(&mut s.bits, waiting);
        s
    };
    let init = task.initial();
    let (sc1, sc2) = (serve("C1", &init), serve("C2", &init));
    // Full map: swapping which child is served is π-related.
    assert!(om.canonical_skey(&task, &sc1, None) == om.canonical_skey(&task, &sc2, None));
    // View: children are frozen — a subgoal (served c1) must not see
    // (served c2) as the same state.
    assert!(view.canonical_skey(&task, &sc1, None) != view.canonical_skey(&task, &sc2, None));
    // Sandwiches still merge under the view.
    let make = |s: &str, b: &str| {
        let mut st = init.clone();
        let oi = task
            .op_display
            .iter()
            .position(|o| o == &format!("MAKE {s} {b}"))
            .unwrap();
        st = task.apply(oi, &st);
        st
    };
    assert!(
        view.canonical_skey(&task, &make("S1", "B1"), None)
            == view.canonical_skey(&task, &make("S2", "B3"), None)
    );
}

/// L5 end-to-end: the partition cascade with the orbit passed down
/// solves mini-snack with a valid plan, identically at 1 and 8 threads.
#[test]
fn partition_passdown_solves_with_identical_plans_across_threads() {
    let (d, p, task) = task_of(SNACK_DOM, SNACK_PRB);
    let om = orbits::detect_classical(&d, &p, &task).unwrap();
    let groups = ferroplan::invariants::synthesize(&d, &task);
    let cfg = SearchCfg::from_weights(1.0, 5.0, None);
    let run =
        |threads: usize| match ferroplan::resolve::solve(&task, threads, cfg, &groups, Some(&om)) {
            ferroplan::resolve::Solved::Plan(ops, _) => ops,
            ferroplan::resolve::Solved::Unsolvable { .. } => panic!("mini-snack solves"),
        };
    let (t1, t8) = (run(1), run(8));
    assert!(replay_solves(&task, &t1), "t1 plan must replay to the goal");
    assert_eq!(t1, t8, "t1 == t8 through the partition cascade");
}

/// The text path end-to-end (run_planner routes classical problems
/// through partition + the cost sweep with the orbit wired in).
#[test]
fn run_planner_solves_mini_snack_with_orbits_wired() {
    let (out, code) =
        ferroplan::run_planner(SNACK_DOM, SNACK_PRB, &ferroplan::Options::default(), false);
    assert_eq!(code, 0, "planner failed:\n{out}");
    assert!(out.contains("SERVE"), "plan should serve children:\n{out}");
}

// ---------------------------------------------------------------------
// L2 — the metric/cost-uniformity gate.

/// Two interchangeable gadgets with UNARY done-goals and a constant
/// serve cost — the gate certifies uniformity and detection fires.
const GADGET_DOM_CONST: &str = "
(define (domain gadgets)
  (:requirements :strips :typing :action-costs)
  (:types gadget)
  (:predicates (fresh ?g - gadget) (done ?g - gadget))
  (:functions (total-cost))
  (:action prep
    :parameters (?g - gadget)
    :precondition (fresh ?g)
    :effect (and (not (fresh ?g)) (done ?g) (increase (total-cost) 1))))
";

/// Same shape, but the cost reads a 0-ary fluent another op WRITES —
/// state-dependent, so no constant can be certified and the gate bails.
/// The 0-ary read is invisible to the init-profile pass (profiles catch
/// every per-member constant asymmetry earlier), so this is exactly the
/// reachable bail the gate exists for.
const GADGET_DOM_DYN: &str = "
(define (domain gadgets-dyn)
  (:requirements :strips :typing :action-costs :numeric-fluents)
  (:types gadget)
  (:predicates (fresh ?g - gadget) (done ?g - gadget))
  (:functions (total-cost) (surcharge))
  (:action prep
    :parameters (?g - gadget)
    :precondition (fresh ?g)
    :effect (and (not (fresh ?g)) (done ?g) (increase (total-cost) (surcharge))))
  (:action bump
    :parameters ()
    :precondition (and)
    :effect (increase (surcharge) 1)))
";

/// GADGET_DOM_DYN minus the bump: `surcharge` is a defined static, the
/// cost IS a certified constant, and detection must fire — the paired
/// control that pins the bail on the gate itself, not on the shape.
const GADGET_DOM_STATIC: &str = "
(define (domain gadgets-static)
  (:requirements :strips :typing :action-costs :numeric-fluents)
  (:types gadget)
  (:predicates (fresh ?g - gadget) (done ?g - gadget))
  (:functions (total-cost) (surcharge))
  (:action prep
    :parameters (?g - gadget)
    :precondition (fresh ?g)
    :effect (and (not (fresh ?g)) (done ?g) (increase (total-cost) (surcharge)))))
";

fn gadget_prb(domain: &str) -> String {
    format!(
        "(define (problem gadgets-1)
  (:domain {domain})
  (:objects g1 g2 - gadget)
  (:init (fresh g1) (fresh g2) (= (total-cost) 0){extra})
  (:goal (and (done g1) (done g2)))
  (:metric minimize (total-cost)))",
        domain = domain,
        extra = if domain == "gadgets" {
            ""
        } else {
            " (= (surcharge) 1)"
        }
    )
}

#[test]
fn cost_gate_admits_uniform_constants() {
    let (d, p, task) = task_of(GADGET_DOM_CONST, &gadget_prb("gadgets"));
    let om =
        orbits::detect_classical(&d, &p, &task).expect("uniform constant costs pass the L2 gate");
    assert_eq!(om.orbits.len(), 1);
    assert_eq!(om.orbits[0].facts.len(), 2);
    // The TEMPORAL entry keeps its 0.21 strictness: same problem, no
    // orbit — the cost carve-out is classical-only by construction.
    assert!(
        orbits::detect(&d, &p, &task).is_none(),
        "detect() must still bail on a non-total-time metric"
    );
}

#[test]
fn cost_gate_bails_on_state_dependent_cost() {
    let (d, p, task) = task_of(GADGET_DOM_DYN, &gadget_prb("gadgets-dyn"));
    assert!(
        orbits::detect_classical(&d, &p, &task).is_none(),
        "a written cost source is state-dependent: the gate must bail"
    );
    // The paired control: identical but for the writer — detection fires,
    // so the bail above is the GATE's verdict, not the shape's.
    let (d2, p2, task2) = task_of(GADGET_DOM_STATIC, &gadget_prb("gadgets-static"));
    assert!(
        orbits::detect_classical(&d2, &p2, &task2).is_some(),
        "static-read constant cost must pass"
    );
}

/// Constant-but-ASYMMETRIC per-member costs: the init-profile pass
/// already refuses to group the members (the fee values are in each
/// profile), so detection bails a layer above the gate — recorded
/// defense in depth: the gate re-proves cost symmetry on the GROUNDED
/// truth, so profile string-matching is not load-bearing for soundness.
/// The true (asymmetric) optimum survives end-to-end.
const FEE_DOM: &str = "
(define (domain fees)
  (:requirements :strips :typing :action-costs :numeric-fluents)
  (:types gadget)
  (:predicates (fresh ?g - gadget) (done ?g - gadget) (alldone))
  (:functions (total-cost) (fee ?g - gadget))
  (:action prep
    :parameters (?g - gadget)
    :precondition (fresh ?g)
    :effect (and (not (fresh ?g)) (done ?g) (increase (total-cost) (fee ?g))))
  (:action finish
    :parameters (?g - gadget)
    :precondition (done ?g)
    :effect (alldone)))
";

const FEE_PRB: &str = "
(define (problem fees-1)
  (:domain fees)
  (:objects g1 g2 - gadget)
  (:init (fresh g1) (fresh g2) (= (total-cost) 0)
         (= (fee g1) 1) (= (fee g2) 9))
  (:goal (alldone))
  (:metric minimize (total-cost)))
";

#[test]
fn cost_asymmetry_bails_and_true_optimum_kept() {
    let (d, p, task) = task_of(FEE_DOM, FEE_PRB);
    assert!(
        orbits::detect_classical(&d, &p, &task).is_none(),
        "asymmetric per-member costs must never form an orbit"
    );
    // End-to-end: optimal mode still certifies the TRUE optimum (prep
    // the cheap gadget, cost 1) — nothing was merged into nonsense.
    let opts = ferroplan::Options {
        mode: ferroplan::Mode::Optimal,
        ..Default::default()
    };
    let sol = ferroplan::solve(FEE_DOM, FEE_PRB, &opts).unwrap();
    assert!(sol.solved, "{:?}", sol.notes);
    assert_eq!(sol.plan.as_ref().unwrap().metric, Some(1.0));
}

/// The B&B key contract: the cost fluent is appended AFTER
/// canonicalization — π-related states at EQUAL cost merge, at
/// different cost stay distinct.
#[test]
fn bnb_key_appends_cost_after_canonicalization() {
    let (d, p, task) = task_of(GADGET_DOM_CONST, &gadget_prb("gadgets"));
    let om = orbits::detect_classical(&d, &p, &task).unwrap();
    let cf = task.fluent_id("(TOTAL-COST)").unwrap();
    let apply = |name: &str| {
        let oi = task.op_display.iter().position(|o| o == name).unwrap();
        task.apply(oi, &task.initial())
    };
    let (a, b) = (apply("PREP G1"), apply("PREP G2"));
    assert!(
        om.canonical_skey(&task, &a, Some(cf)) == om.canonical_skey(&task, &b, Some(cf)),
        "π-related, equal cost: one key"
    );
    let mut b_pricier = b.clone();
    b_pricier.fv[cf] += 1.0;
    assert!(
        om.canonical_skey(&task, &a, Some(cf)) != om.canonical_skey(&task, &b_pricier, Some(cf)),
        "equal orbit, different cost: distinct keys"
    );
}

// ---------------------------------------------------------------------
// L1 — optimal-mode canonical keys (the hook commit's fixture).

/// L1 on the mini child-snack: the same certified cost with and without
/// the orbit, the plan replays to the goal, and the orbit run pays no
/// more than a THIRD of the plain run's evaluations (the spec's bound).
/// optimal is serial, so t1 == t8 holds by construction; determinism is
/// pinned by running the orbit ladder twice.
#[test]
fn optimal_orbit_keys_certify_the_same_cost_at_a_third_the_evals() {
    let (d, p, task) = task_of(SNACK_DOM, SNACK_PRB);
    let om = orbits::detect_classical(&d, &p, &task).unwrap();
    let cap = 4_000_000;
    let plain = ferroplan::optimal::solve(&task, None, cap, None);
    let orbit = ferroplan::optimal::solve(&task, None, cap, Some(&om));
    assert!(plain.proven && orbit.proven, "both runs must certify");
    assert_eq!(plain.cost, orbit.cost, "certified cost must agree");
    assert!(replay_solves(&task, orbit.ops.as_ref().unwrap()));
    assert!(
        orbit.evaluated * 3 <= plain.evaluated,
        "orbit A* must collapse the frontier: {} vs {}",
        orbit.evaluated,
        plain.evaluated
    );
    let again = ferroplan::optimal::solve(&task, None, cap, Some(&om));
    assert_eq!(again.ops, orbit.ops, "orbit ladder is deterministic");
    assert_eq!(again.evaluated, orbit.evaluated);
}
