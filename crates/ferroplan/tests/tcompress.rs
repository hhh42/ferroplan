//! THE COMPRESSION RUNG (0.28 Lane T, `tcompress.rs`): temporal tasks that
//! do not need concurrency are planned classically -- one instantaneous
//! action per durative action -- and the plan is put back on the clock.
//!
//! The rows it exists for are board-scale (the 2026-09-20 probe: 121
//! conversions across the IPC-5 temporal tracks, every one VAL-valid), and
//! one of them is locked here as an `#[ignore]`d heavy test: the smallest
//! `pathways-metric-time` instance, an 18-step plan the decision-epoch
//! ladder cannot find -- its zero-duration `choose`/`initialize` happenings
//! pile up at t = 0 and the search permutes them until its node caps
//! ("temporal ladder exhausted its budgets with 46 s of wall left", the
//! whole domain 0/30 on the 0.27 board).
//!
//! The default-run tests pin the three places the rung can be wrong: what
//! one compressed action means (a token held for the duration), where the
//! left-shift may and may not overlap, and what the rung refuses.

use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::tcompress::{self, Bet};
use ferroplan::temporal;

/// `run` takes its machine for the duration: `busy` is added at start,
/// checked at end, deleted at end -- a token the action holds for ITSELF.
/// The compressed precondition must not ask the world for `busy`
/// beforehand (it is never true between jobs), and the compressed effect
/// must net the token to its DELETE, or the machine never frees.
const SHOP: &str = "(define (domain shop)
 (:requirements :typing :durative-actions)
 (:types job machine)
 (:predicates (todo ?j - job) (done ?j - job) (fits ?j - job ?m - machine)
              (free ?m - machine) (busy ?m - machine))
 (:durative-action run
   :parameters (?j - job ?m - machine)
   :duration (= ?duration 3)
   :condition (and (at start (todo ?j)) (at start (free ?m))
                   (over all (fits ?j ?m)) (at end (busy ?m)))
   :effect (and (at start (not (todo ?j))) (at start (not (free ?m))) (at start (busy ?m))
                (at end (not (busy ?m))) (at end (free ?m)) (at end (done ?j)))))";

fn shop(jobs: &[(&str, &str)], machines: &[&str]) -> String {
    let objs: String = jobs.iter().map(|(j, _)| format!(" {j}")).collect();
    let ms: String = machines.iter().map(|m| format!(" {m}")).collect();
    let init: String = jobs
        .iter()
        .map(|(j, m)| format!(" (todo {j}) (fits {j} {m})"))
        .chain(machines.iter().map(|m| format!(" (free {m})")))
        .collect();
    let goal: String = jobs.iter().map(|(j, _)| format!(" (done {j})")).collect();
    format!(
        "(define (problem s) (:domain shop) (:objects{objs} - job{ms} - machine) \
         (:init{init}) (:goal (and{goal})))"
    )
}

fn rung(dom: &str, prob: &str) -> temporal::TimedPlan {
    let d = parse_domain(dom).unwrap();
    let p = parse_problem(prob).unwrap();
    assert_eq!(tcompress::declines(&d, &p), None);
    let plan = tcompress::solve(&d, &p, 1, Bet::Rest).expect("the rung solves it");
    // The rung validates at the PLAN's size (a domain specialised to the
    // ground steps). The full-task validator must agree with it.
    temporal::validate(&d, &p, &plan).expect("and the full validator agrees");
    plan
}

#[test]
fn a_held_token_nets_to_its_release() {
    // Two jobs, ONE machine: the second `run` is only applicable if the
    // first one's compression freed the machine and dropped `busy`.
    let plan = rung(SHOP, &shop(&[("j1", "m1"), ("j2", "m1")], &["m1"]));
    assert_eq!(plan.steps.len(), 2);
    // Serialised on the machine: the second starts after the first ends.
    let (a, b) = (&plan.steps[0], &plan.steps[1]);
    assert!(b.time >= a.time + a.duration.unwrap(), "{plan:?}");
    assert!((6.0..6.1).contains(&plan.makespan), "{}", plan.makespan);
}

#[test]
fn independent_work_overlaps_and_interference_does_not() {
    // Two jobs on two machines touch nothing in common: the left-shift
    // starts both at the first epoch, and the makespan is ONE job's.
    let plan = rung(SHOP, &shop(&[("j1", "m1"), ("j2", "m2")], &["m1", "m2"]));
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].time, plan.steps[1].time, "{plan:?}");
    assert!((3.0..3.1).contains(&plan.makespan), "{}", plan.makespan);

    // Three jobs, two machines: two overlap, the third waits for ITS machine.
    let plan = rung(
        SHOP,
        &shop(&[("j1", "m1"), ("j2", "m2"), ("j3", "m1")], &["m1", "m2"]),
    );
    assert!((6.0..6.1).contains(&plan.makespan), "{}", plan.makespan);
}

#[test]
fn zero_duration_steps_validate() {
    // The validator fired ENDs before STARTs inside one epoch, which put a
    // zero-duration step's end ahead of its own start and refused the plan
    // (0.28: `temporal::epoch_rank`). tests/zero_duration.rs pins that such
    // tasks SOLVE; this pins that their plans VALIDATE.
    let dom = "(define (domain z0)
      (:requirements :strips :durative-actions)
      (:predicates (a) (g) (h2))
      (:durative-action zap :parameters () :duration (= ?duration 0)
        :condition (at start (a)) :effect (at end (g)))
      (:durative-action chain :parameters () :duration (= ?duration 2)
        :condition (at start (g)) :effect (at end (h2))))";
    let prob = "(define (problem z1) (:domain z0) (:init (a)) (:goal (h2)))";
    let plan = rung(dom, prob);
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].duration, Some(0.0));
}

#[test]
fn the_rung_declines_what_it_cannot_promise() {
    let d = parse_domain(SHOP).unwrap();
    // A timed initial literal: the classical search has no clock to meet it.
    let til = shop(&[("j1", "m1")], &["m1"]).replace("(:init", "(:init (at 5 (not (free m1)))");
    let p = parse_problem(&til).unwrap();
    assert_eq!(tcompress::declines(&d, &p), Some("timed initial literals"));

    // A trajectory constraint: a compressed action has no inside for an
    // `always` to look at.
    let con = shop(&[("j1", "m1")], &["m1"])
        .replace("(:goal", "(:constraints (always (free m1))) (:goal");
    let p = parse_problem(&con).unwrap();
    assert_eq!(tcompress::declines(&d, &p), Some("trajectory constraints"));

    // Required concurrency (the fire-kiln shape): `bake` needs heat that
    // only exists WHILE `fire` runs. No sequential plan exists.
    let kiln = parse_domain(
        "(define (domain kiln)
          (:requirements :durative-actions)
          (:predicates (hot) (baked))
          (:durative-action fire :parameters () :duration (= ?duration 10)
            :condition (and) :effect (and (at start (hot)) (at end (not (hot)))))
          (:durative-action bake :parameters () :duration (= ?duration 3)
            :condition (over all (hot)) :effect (at end (baked))))",
    )
    .unwrap();
    let p = parse_problem("(define (problem k) (:domain kiln) (:init) (:goal (baked)))").unwrap();
    assert_eq!(tcompress::declines(&kiln, &p), Some("required concurrency"));
}

#[test]
#[ignore = "heavy IPC temporal solve; opt-in via --include-ignored"]
fn pathways_metric_time_p01_solves() {
    // 0.27: unsolved, deterministically ("temporal ladder exhausted its
    // budgets"), and with it all 30 rows of the domain. SGPlan5: 0.00 s.
    let base = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../benchmarks/ipc/temporal/pathways-metric-time"
    );
    let d = std::fs::read_to_string(format!("{base}/domain.pddl")).unwrap();
    let p = std::fs::read_to_string(format!("{base}/p01.pddl")).unwrap();
    let sol = ferroplan::solve(&d, &p, &ferroplan::Options::default()).unwrap();
    assert!(sol.solved, "{:?}", sol.notes);
    let plan = sol.plan.unwrap();
    assert!(plan.makespan.is_some());
    let timed = temporal::TimedPlan {
        steps: plan
            .steps
            .iter()
            .map(|s| temporal::TimedStep {
                time: s.time.expect("timed"),
                action: std::iter::once(s.action.clone())
                    .chain(s.args.iter().cloned())
                    .collect::<Vec<_>>()
                    .join(" "),
                duration: s.duration,
            })
            .collect(),
        makespan: plan.makespan.unwrap(),
    };
    temporal::validate(
        &parse_domain(&d).unwrap(),
        &parse_problem(&p).unwrap(),
        &timed,
    )
    .expect("the reported plan validates on the full task");
}
