//! The goal-isomorphism probe arm (0.23 Phase 4 probe 1,
//! docs/roadmap-0.23.md): goal-blind units + designation tables +
//! witness permutation behind `FF_ORBIT_ISO`. The LOAD-BEARING fixture
//! is the round trip — a plan emitted under a goal remap must validate
//! against the ORIGINAL problem — with the RED half proved by replaying
//! the UN-remapped op sequence (skipping the inverse witness) and
//! watching it miss the goal.
//!
//! Every test but `flag_gates_iso_and_temporal_round_trip` avoids env
//! vars: detection goes through `orbits::detect_iso` (unconditionally
//! armed) and the searches take the map by hand, so the file stays safe
//! under the parallel harness; the one env-touching test keeps ALL its
//! phases (flag off, flag on, t1 vs t8) inside a single #[test].

use ferroplan::ground::{ground, ground_stratified, Outcome};
use ferroplan::orbits;
use ferroplan::packed::PackedTask;
use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::types::{Domain, Problem};

/// The goal-paired CLASSICAL mini shape: four init-identical blocks,
/// two designated by DIFFERENT unary goals. Strict detection can only
/// orbit the two goal-free blocks (b1's and b2's SOLO1 keys differ);
/// the iso arm orbits all four and carries (RED B1), (BLUE B2) as
/// designations.
const PAINT_DOM: &str = "
(define (domain iso-paint)
  (:requirements :strips :typing)
  (:types block)
  (:predicates (clean ?b - block) (red ?b - block) (blue ?b - block))
  (:action paint-red
    :parameters (?b - block)
    :precondition (clean ?b)
    :effect (and (not (clean ?b)) (red ?b)))
  (:action paint-blue
    :parameters (?b - block)
    :precondition (clean ?b)
    :effect (and (not (clean ?b)) (blue ?b))))
";

const PAINT_PRB: &str = "
(define (problem iso-paint-1)
  (:domain iso-paint)
  (:objects b1 b2 b3 b4 - block)
  (:init (clean b1) (clean b2) (clean b3) (clean b4))
  (:goal (and (red b1) (blue b2))))
";

fn classical_task(dom: &str, prb: &str) -> (Domain, Problem, PackedTask) {
    let d = parse_domain(dom).unwrap();
    let p = parse_problem(prb).unwrap();
    let task = match ground(&d, &p, 1) {
        Outcome::Task(t) => t,
        _ => panic!("grounding failed"),
    };
    (d, p, task)
}

fn op(task: &PackedTask, disp: &str) -> usize {
    (0..task.n_ops)
        .find(|&oi| task.op_display[oi] == disp)
        .unwrap_or_else(|| panic!("no op {disp}"))
}

fn replay(task: &PackedTask, ops: &[usize]) -> ferroplan::packed::State {
    let mut s = task.initial();
    for &oi in ops {
        assert!(
            task.op_applicable(oi, &s),
            "inapplicable {}",
            task.op_display[oi]
        );
        s = task.apply(oi, &s);
    }
    s
}

/// Detection pins: goal-paired objects join ONE goal-blind orbit, the
/// designations are carried, and the strict entries stay strict.
#[test]
fn iso_detection_forms_goal_blind_orbit_with_designations() {
    let (d, p, task) = classical_task(PAINT_DOM, PAINT_PRB);
    let om = orbits::detect_iso(&d, &p, &task).expect("iso arm detects");
    assert!(om.iso_active(), "designations present");
    assert_eq!(om.orbits.len(), 1, "one goal-blind orbit");
    assert_eq!(om.orbits[0].facts.len(), 4, "all four blocks join it");
    assert_eq!(
        om.iso_untouched_goal().unwrap().len(),
        0,
        "both goal atoms are designations"
    );
    // The strict classical entry NEVER designates — its maps flow to
    // consumers without the relaxed goal test.
    let strict = orbits::detect_classical(&d, &p, &task).expect("strict detects b3/b4");
    assert!(!strict.iso_active());
    assert_eq!(
        strict.orbits[0].facts.len(),
        2,
        "strict: goal-free pair only"
    );
}

/// The child-snack control: a whole-diagonal goal (every member serves
/// the same-shaped atom) is σ-INVARIANT already — the iso arm must stay
/// INERT (no designations, no h-weakening, byte-identical consumers),
/// because relaxing it buys zero collapse and costs guidance.
#[test]
fn iso_inert_on_whole_diagonal_goals() {
    const UNIFORM_PRB: &str = "
    (define (problem iso-paint-uniform)
      (:domain iso-paint)
      (:objects b1 b2 b3 b4 - block)
      (:init (clean b1) (clean b2) (clean b3) (clean b4))
      (:goal (and (red b1) (red b2) (red b3) (red b4))))
    ";
    let (d, p, task) = classical_task(PAINT_DOM, UNIFORM_PRB);
    let om = orbits::detect_iso(&d, &p, &task).expect("orbit still detects");
    assert!(!om.iso_active(), "uniform goals mint NO designations");
    assert_eq!(om.orbits[0].facts.len(), 4);
}

/// The witness: a state serving the goal UP TO relabeling yields a
/// non-identity σ; a state serving nobody yields none.
#[test]
fn iso_witness_found_exactly_when_a_relabeling_serves_the_goal() {
    let (d, p, task) = classical_task(PAINT_DOM, PAINT_PRB);
    let om = orbits::detect_iso(&d, &p, &task).unwrap();
    // Swapped service: b2 is red, b1 is blue.
    let swapped = replay(
        &task,
        &[op(&task, "PAINT-RED B2"), op(&task, "PAINT-BLUE B1")],
    );
    assert!(!task.goal_met(&swapped), "concretely NOT a goal state");
    let sigma = om
        .iso_goal_witness(&task, &swapped, &task.goal_pos, &task.goal_num)
        .expect("σ-image serves the goal");
    assert_ne!(
        sigma[0],
        (0..4).collect::<Vec<u16>>(),
        "witness is non-identity"
    );
    // Nobody served: no witness.
    let cold = task.initial();
    assert!(om
        .iso_goal_witness(&task, &cold, &task.goal_pos, &task.goal_num)
        .is_none());
    // Half served under relabeling only: still no witness (exact match).
    let half = replay(&task, &[op(&task, "PAINT-RED B3")]);
    assert!(om
        .iso_goal_witness(&task, &half, &task.goal_pos, &task.goal_num)
        .is_none());
}

/// THE ROUND TRIP, classical, RED then GREEN: the concrete op sequence
/// reaching the σ-image state does NOT solve the original problem
/// (RED — this is exactly what emitting without the inverse witness
/// would produce), while the witness-remapped sequence DOES (GREEN).
#[test]
fn iso_round_trip_red_without_remap_green_with() {
    let (d, p, task) = classical_task(PAINT_DOM, PAINT_PRB);
    let om = orbits::detect_iso(&d, &p, &task).unwrap();
    let raw = vec![op(&task, "PAINT-RED B2"), op(&task, "PAINT-BLUE B1")];
    let end = replay(&task, &raw);
    let sigma = om
        .iso_goal_witness(&task, &end, &task.goal_pos, &task.goal_num)
        .unwrap();
    // RED: skip the witness — the plan serves the PERMUTED goal, not ours.
    assert!(
        !task.goal_met(&replay(&task, &raw)),
        "un-remapped emission must fail the original goal"
    );
    // GREEN: apply it — the σ-image plan serves the ORIGINAL goal.
    let remapped: Vec<usize> = raw.iter().map(|&o| om.iso_remap_op(&sigma, o)).collect();
    assert!(
        task.goal_met(&replay(&task, &remapped)),
        "remapped emission must solve the original problem"
    );
    assert_eq!(
        task.op_display[remapped[0]], "PAINT-RED B1",
        "the red service relabels onto the designated block"
    );
    assert_eq!(task.op_display[remapped[1]], "PAINT-BLUE B2");
}

/// Variant whose designated blue block sits BEHIND the canonical
/// representative: with all four blocks one class, the dedup keeps the
/// first-generated (red b1, blue b2) state as the class rep, so the
/// goal (red b1, blue b3) is only reachable through the WITNESS branch
/// — the optimal pop deterministically exercises the remap.
const PAINT_PRB_B3: &str = "
(define (problem iso-paint-2)
  (:domain iso-paint)
  (:objects b1 b2 b3 b4 - block)
  (:init (clean b1) (clean b2) (clean b3) (clean b4))
  (:goal (and (red b1) (blue b3))))
";

/// Optimal A* under the iso map: certificate equals the plain run's,
/// the emitted ops solve the ORIGINAL problem on replay THROUGH the
/// witness branch (the emitted blue service names the designated b3,
/// which only the remap can produce), and the ladder is deterministic
/// run-to-run (optimal is serial, so t1 == t8 holds by construction;
/// determinism is pinned by running twice).
#[test]
fn iso_optimal_certifies_same_cost_and_replays_to_the_original_goal() {
    let (d, p, task) = classical_task(PAINT_DOM, PAINT_PRB_B3);
    let om = orbits::detect_iso(&d, &p, &task).unwrap();
    let cap = 1_000_000;
    let plain = ferroplan::optimal::solve(&task, None, cap, None);
    let iso = ferroplan::optimal::solve(&task, None, cap, Some(&om));
    assert!(plain.proven && iso.proven, "both certify");
    assert_eq!(plain.cost, iso.cost, "same certified cost");
    let ops = iso.ops.as_ref().unwrap();
    let end = replay(&task, ops);
    assert!(task.goal_met(&end), "iso plan solves the ORIGINAL problem");
    assert!(
        ops.iter().any(|&o| task.op_display[o] == "PAINT-BLUE B3"),
        "the blue service must land on the DESIGNATED block: {:?}",
        ops.iter().map(|&o| &task.op_display[o]).collect::<Vec<_>>()
    );
    let again = ferroplan::optimal::solve(&task, None, cap, Some(&om));
    assert_eq!(again.ops, iso.ops, "deterministic");
    assert_eq!(again.evaluated, iso.evaluated);
}

/// TEMPORAL mini shape for the iso arm: three init-identical pieces,
/// two designated by DIFFERENT finish goals; strict detection has no
/// orbit at all (two SOLO1 singletons + one lone SOLO), so any collapse
/// here belongs to the iso arm alone.
const BAKE_DOM: &str = "
(define (domain iso-bake)
  (:requirements :strips :typing :durative-actions)
  (:types piece)
  (:predicates (raw ?p - piece) (made ?p - piece)
               (fancy ?p - piece) (plain ?p - piece) (free))
  (:durative-action make
    :parameters (?p - piece)
    :duration (= ?duration 2)
    :condition (and (at start (raw ?p)) (over all (free)))
    :effect (and (at start (not (raw ?p))) (at end (made ?p))))
  (:durative-action finish-fancy
    :parameters (?p - piece)
    :duration (= ?duration 1)
    :condition (at start (made ?p))
    :effect (at end (fancy ?p)))
  (:durative-action finish-plain
    :parameters (?p - piece)
    :duration (= ?duration 1)
    :condition (at start (made ?p))
    :effect (at end (plain ?p))))
";

const BAKE_PRB: &str = "
(define (problem iso-bake-1)
  (:domain iso-bake)
  (:objects a b c - piece)
  (:init (raw a) (raw b) (raw c) (free))
  (:goal (and (fancy a) (plain b))))
";

/// The temporal designation tables: detection on the snap-compiled task
/// carries both designations, the witness fires on a σ-image goal
/// state, and the op remap lands on the snap ops the reconstruct hook
/// reads — the exact tables the temporal emission threads through.
#[test]
fn iso_temporal_witness_and_snap_op_remap() {
    let d = parse_domain(BAKE_DOM).unwrap();
    let p = parse_problem(BAKE_PRB).unwrap();
    let c = ferroplan::temporal::compile(&d, &p);
    let task = match ground_stratified(&c.domain, &c.problem, 1) {
        Outcome::Task(t) => t,
        _ => panic!("grounding failed"),
    };
    assert!(
        orbits::detect(&c.domain, &c.problem, &task).is_none(),
        "strict detection has NO orbit here (iso-only constituency)"
    );
    let om = orbits::detect_iso(&c.domain, &c.problem, &task).expect("iso detects");
    assert!(om.iso_active());
    assert_eq!(om.orbits.len(), 1);
    assert_eq!(om.orbits[0].facts.len(), 3, "a, b, c all one orbit");
    // σ-image goal state: b got the fancy finish, a the plain one.
    let mut s = task.initial();
    for name in ["(FANCY B)", "(PLAIN A)", "(MADE A)", "(MADE B)"] {
        let f = task
            .fact_id(name)
            .unwrap_or_else(|| panic!("no fact {name}"));
        ferroplan::bitset::set(&mut s.bits, f);
    }
    assert!(!task.goal_met(&s));
    let sigma = om
        .iso_goal_witness(&task, &s, &task.goal_pos, &task.goal_num)
        .expect("witness under swap");
    // The remap sends B's fancy snap ops onto A's — what reconstruct
    // renders — and leaves the orbit-free ops alone.
    let fancy_b = op(&task, "FINISH-FANCY-START B");
    assert_eq!(
        task.op_display[om.iso_remap_op(&sigma, fancy_b)],
        "FINISH-FANCY-START A"
    );
    let plain_a = op(&task, "FINISH-PLAIN-START A");
    assert_eq!(
        task.op_display[om.iso_remap_op(&sigma, plain_a)],
        "FINISH-PLAIN-START B"
    );
}

/// The env-gated integration round trip, all phases in ONE test so the
/// process-global flag never races a neighbor: flag off = plain solve
/// (the 0.22 path); flag on = the solve still validates against the
/// ORIGINAL problem (temporal round trip, the in-tree validator as
/// referee) and t1 == t8 byte-for-byte.
#[test]
fn flag_gates_iso_and_temporal_round_trip() {
    let d = parse_domain(BAKE_DOM).unwrap();
    let p = parse_problem(BAKE_PRB).unwrap();
    // Phase 1 — flag off: the plain path solves and validates.
    std::env::remove_var("FF_ORBIT_ISO");
    let off = ferroplan::temporal::solve(&d, &p, 1).expect("flag-off solves");
    ferroplan::temporal::validate(&d, &p, &off).expect("flag-off plan valid");
    // Phase 2 — flag on: the iso-armed solve must emit a plan that
    // validates against the ORIGINAL problem (the witness remap is
    // invisible in the contract, load-bearing in the emission).
    std::env::set_var("FF_ORBIT_ISO", "1");
    let on1 = ferroplan::temporal::solve(&d, &p, 1).expect("iso-armed solves");
    let r = ferroplan::temporal::validate(&d, &p, &on1);
    let on8 = ferroplan::temporal::solve(&d, &p, 8);
    std::env::remove_var("FF_ORBIT_ISO");
    r.expect("iso-armed plan validates against the ORIGINAL problem");
    // Phase 3 — determinism across thread counts (t1 == t8).
    let on8 = on8.expect("t8 solves");
    assert_eq!(on1.makespan, on8.makespan);
    assert_eq!(on1.steps.len(), on8.steps.len());
    for (x, y) in on1.steps.iter().zip(on8.steps.iter()) {
        assert_eq!(x.time, y.time);
        assert_eq!(x.action, y.action);
        assert_eq!(x.duration, y.duration);
    }
}

/// Construction RED, temporal: a schedule whose finish services are
/// swapped against the designations is exactly what the emission would
/// produce WITHOUT the inverse witness — the validator rejects it, and
/// the σ-remapped schedule passes. (The executable form of the fixture
/// note; the classical test above drives the same inverse through the
/// real remap tables.)
#[test]
fn iso_temporal_unremapped_schedule_is_red_remapped_green() {
    let d = parse_domain(BAKE_DOM).unwrap();
    let p = parse_problem(BAKE_PRB).unwrap();
    let step = |time: f64, action: &str, duration: f64| ferroplan::temporal::TimedStep {
        time,
        action: action.into(),
        duration: Some(duration),
    };
    // RED: b takes the fancy finish, a the plain one — the σ-image of a
    // valid plan, serving the PERMUTED goal.
    let red = ferroplan::temporal::TimedPlan {
        steps: vec![
            step(0.0, "MAKE A", 2.0),
            step(0.0, "MAKE B", 2.0),
            step(2.001, "FINISH-FANCY B", 1.0),
            step(2.001, "FINISH-PLAIN A", 1.0),
        ],
        makespan: 3.001,
    };
    assert!(
        ferroplan::temporal::validate(&d, &p, &red).is_err(),
        "un-remapped schedule must fail the original goal"
    );
    // GREEN: the same schedule through the witness (a<->b on the finishes).
    let green = ferroplan::temporal::TimedPlan {
        steps: vec![
            step(0.0, "MAKE A", 2.0),
            step(0.0, "MAKE B", 2.0),
            step(2.001, "FINISH-FANCY A", 1.0),
            step(2.001, "FINISH-PLAIN B", 1.0),
        ],
        makespan: 3.001,
    };
    ferroplan::temporal::validate(&d, &p, &green).expect("remapped schedule validates");
}
