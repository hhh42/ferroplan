//! THE PREFERENCE OPTIMIZER'S FIRST IMPROVING STEP (0.29 Lane 1,
//! `pddl3::polish`): from an incumbent that solves the hard goals, plan one
//! more preference at a time as a hard goal on top of everything the
//! incumbent already satisfies, price the candidate by replay over the
//! ORIGINAL pair, and keep it iff the metric fell.
//!
//! The board shape it exists for cannot be built at fixture scale (the
//! optimizer finds a five-cell corridor's optimum itself); what is pinned is
//! the mechanism -- the greedy order, the monotone acceptance, the exact
//! replay price, the refusals -- and, in a child process where the
//! optimizer's budget can be starved, that the route applies it.

use ferroplan::pddl3;
use ferroplan::search::SearchCfg;

const CORRIDOR: &str = "(define (domain corridor-plain)
 (:requirements :strips :typing :preferences)
 (:types cell)
 (:predicates (at ?c - cell) (adj ?a ?b - cell) (visited ?c - cell))
 (:action move :parameters (?a ?b - cell)
   :precondition (and (at ?a) (adj ?a ?b))
   :effect (and (not (at ?a)) (at ?b) (visited ?b))))";

/// c0 - c1 - c2 - c3, side cells s1 off c1 and s2 off c2. Hard goal: c3.
/// Soft: visit s1 (5), visit s2 (3), and be back at c0 at the end (2 --
/// unreachable together with the hard goal, so it stays violated).
const WALK: &str = "(define (problem walk) (:domain corridor-plain)
 (:objects c0 c1 c2 c3 s1 s2 - cell)
 (:init (at c0) (adj c0 c1) (adj c1 c0) (adj c1 c2) (adj c2 c1) (adj c2 c3) (adj c3 c2)
        (adj c1 s1) (adj s1 c1) (adj c2 s2) (adj s2 c2))
 (:goal (and (at c3)
             (preference side1 (visited s1))
             (preference side2 (visited s2))
             (preference home (at c0))))
 (:metric minimize (+ 1 (* 5 (is-violated side1)) (* 3 (is-violated side2)) (* 2 (is-violated home)))))";

fn steps(names: &[&str]) -> Vec<(String, Vec<String>)> {
    names
        .iter()
        .map(|n| {
            let mut it = n.split_whitespace();
            (
                it.next().unwrap().to_string(),
                it.map(str::to_string).collect(),
            )
        })
        .collect()
}

#[test]
fn the_polish_satisfies_the_heaviest_reachable_preferences_first_and_prices_by_replay() {
    let d = ferroplan::parser::parse_domain(CORRIDOR).unwrap();
    let p = ferroplan::parser::parse_problem(WALK).unwrap();
    let direct = steps(&["MOVE C0 C1", "MOVE C1 C2", "MOVE C2 C3"]);
    // The incumbent violates everything: 1 + 5 + 3 + 2 = 11.
    let out = pddl3::polish(&d, &p, &direct, 1, SearchCfg::default(), None).expect("improves");
    assert_eq!(out.gained, vec!["SIDE1".to_string(), "SIDE2".to_string()]);
    assert!(
        (out.metric - 3.0).abs() < 1e-9,
        "1 + home's 2, both sides won: {}",
        out.metric
    );
    let v = ferroplan::verify::verify_pair(&d, &p, &out.steps).unwrap();
    assert!(v.hard_goal_met);
    assert_eq!(v.goal_pref_sat, vec![true, true, false]);
    // Monotone: polishing the polished plan finds nothing more.
    assert!(pddl3::polish(&d, &p, &out.steps, 1, SearchCfg::default(), None).is_none());
}

#[test]
fn a_kept_preference_stays_kept_when_the_next_one_is_planned() {
    let d = ferroplan::parser::parse_domain(CORRIDOR).unwrap();
    let p = ferroplan::parser::parse_problem(WALK).unwrap();
    // An incumbent that already has side1: the polish plans side2 with
    // side1 as a hard conjunct, so the result holds both.
    let with_s1 = steps(&[
        "MOVE C0 C1",
        "MOVE C1 S1",
        "MOVE S1 C1",
        "MOVE C1 C2",
        "MOVE C2 C3",
    ]);
    let out = pddl3::polish(&d, &p, &with_s1, 1, SearchCfg::default(), None).expect("improves");
    assert_eq!(out.gained, vec!["SIDE2".to_string()]);
    let v = ferroplan::verify::verify_pair(&d, &p, &out.steps).unwrap();
    assert_eq!(v.goal_pref_sat, vec![true, true, false]);
}

/// What the polish refuses, and says so by returning `None`: a metric with a
/// term the replay cannot price; a precondition preference; an incumbent
/// that does not replay; the hatch.
#[test]
fn the_polish_refuses_what_it_cannot_price() {
    let d = ferroplan::parser::parse_domain(CORRIDOR).unwrap();
    let direct = steps(&["MOVE C0 C1", "MOVE C1 C2", "MOVE C2 C3"]);
    let costed = WALK.replace("(+ 1 ", "(+ (total-cost) ");
    let p = ferroplan::parser::parse_problem(&costed).unwrap();
    assert!(pddl3::polish(&d, &p, &direct, 1, SearchCfg::default(), None).is_none());
    let p = ferroplan::parser::parse_problem(WALK).unwrap();
    assert!(
        pddl3::polish(
            &d,
            &p,
            &steps(&["MOVE C0 C1"]),
            1,
            SearchCfg::default(),
            None
        )
        .is_none(),
        "an incumbent that does not reach the hard goal is not a floor"
    );
    let pref_pre = CORRIDOR.replace(
        ":precondition (and (at ?a) (adj ?a ?b))",
        ":precondition (and (at ?a) (adj ?a ?b) (preference lit (visited ?a)))",
    );
    let d2 = ferroplan::parser::parse_domain(&pref_pre).unwrap();
    assert!(pddl3::polish(&d2, &p, &direct, 1, SearchCfg::default(), None).is_none());
}

/// THE ROUTE APPLIES IT. In a child process the optimizer's own budget is
/// starved (`FF_PREF_EVAL_BUDGET=1`) so it returns the hard-goal seed at
/// metric 11; the polish takes the route's answer to 3. With the hatch, 11.
#[test]
fn the_route_polishes_what_the_optimizer_leaves() {
    if std::env::var("POLISH_CHILD").is_ok() {
        let sol = ferroplan::solve(
            CORRIDOR,
            WALK,
            &ferroplan::Options {
                threads: 1,
                ..Default::default()
            },
        )
        .unwrap();
        println!(
            "CHILD-METRIC:{:?}",
            sol.plan.as_ref().and_then(|p| p.metric)
        );
        println!("CHILD-NOTES:{}", sol.notes.join(" | "));
        return;
    }
    let run = |hatch: bool| {
        let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
        cmd.args([
            "--exact",
            "the_route_polishes_what_the_optimizer_leaves",
            "--nocapture",
        ])
        .env("POLISH_CHILD", "1")
        .env("FF_PREF_EVAL_BUDGET", "1")
        .env("FF_PREF_NO_ESCALATE", "1")
        .env("FF_PREF_NO_RESTARTS", "1")
        .env("FF_NO_ESPC", "1");
        if hatch {
            cmd.env("FF_NO_PREF_POLISH", "1");
        }
        let out = cmd.output().unwrap();
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let polished = run(false);
    let metric = |s: &str| -> f64 {
        s.lines()
            .find_map(|l| l.strip_prefix("CHILD-METRIC:Some("))
            .and_then(|v| v.trim_end_matches(')').parse().ok())
            .unwrap_or_else(|| panic!("no metric in\n{s}"))
    };
    assert!(
        polished.contains("preference polish"),
        "the route must have polished:\n{polished}"
    );
    assert!((metric(&polished) - 3.0).abs() < 1e-9, "{polished}");
    let hatched = run(true);
    assert!(!hatched.contains("preference polish"), "{hatched}");
    assert!(
        metric(&hatched) > 3.0 + 1e-9,
        "unpolished, the route's metric stays: {hatched}"
    );
}

/// ONE GROUNDING PER POLISH (0.29 Lane 1, the second step). The first build
/// grounded the task twice per attempt -- once to search the candidate, once
/// to price it -- and on pathways, where a grounding is 8 s, that is why the
/// polish never reached the domain the simple band is lost on. Now the
/// hard-goal task is grounded once and each candidate swaps its goal facts
/// in, and the verifier's context is prepared once and every pricing replays
/// on it: two groundings for the whole polish, however many candidates --
/// three here, all on the fast path. The counter is process-wide and the
/// runner is parallel, so the count is taken in a child running this test
/// alone.
#[test]
fn the_polish_grounds_twice_however_many_candidates() {
    const CHILD: &str = "FERROPLAN_POLISH_COUNT_CHILD";
    if std::env::var(CHILD).is_ok() {
        let d = ferroplan::parser::parse_domain(CORRIDOR).unwrap();
        let p = ferroplan::parser::parse_problem(WALK).unwrap();
        let direct = steps(&["MOVE C0 C1", "MOVE C1 C2", "MOVE C2 C3"]);
        let before = pddl3::POLISH_GROUNDINGS.load(std::sync::atomic::Ordering::Relaxed);
        let out = pddl3::polish(&d, &p, &direct, 1, SearchCfg::default(), None).expect("improves");
        let after = pddl3::POLISH_GROUNDINGS.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(out.gained.len(), 2);
        assert_eq!(
            after - before,
            2,
            "the hard-goal task and the verifier's context, once each; the three \
             candidates ride the goal-swap path"
        );
        return;
    }
    let exe = std::env::current_exe().expect("the test binary");
    let out = std::process::Command::new(exe)
        .args([
            "the_polish_grounds_twice_however_many_candidates",
            "--exact",
            "--test-threads=1",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .expect("the child runs");
    assert!(
        out.status.success(),
        "the child's count failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// With the route's seed task in hand -- a plain grounding of the pair --
/// the polish grounds NOTHING: it searches candidates on the seed's task and
/// prices on it too. Same child-process count as above.
#[test]
fn the_polish_with_the_seed_task_grounds_nothing() {
    const CHILD: &str = "FERROPLAN_POLISH_SEED_COUNT_CHILD";
    if std::env::var(CHILD).is_ok() {
        let d = ferroplan::parser::parse_domain(CORRIDOR).unwrap();
        let p = ferroplan::parser::parse_problem(WALK).unwrap();
        let (names, seed) = pddl3::hard_goal_plan_with_task(&d, &p, 1, SearchCfg::default());
        let seed = seed.expect("the hard goal grounds");
        assert!(pddl3::seed_is_plain(&seed.task));
        let direct = steps(
            &names
                .expect("a hard-goal plan")
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        );
        let before = pddl3::POLISH_GROUNDINGS.load(std::sync::atomic::Ordering::Relaxed);
        let out =
            pddl3::polish(&d, &p, &direct, 1, SearchCfg::default(), Some(&seed)).expect("improves");
        let after = pddl3::POLISH_GROUNDINGS.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(out.gained.len(), 2);
        assert_eq!(
            after - before,
            0,
            "the seed task serves both the search and the pricing"
        );
        return;
    }
    let exe = std::env::current_exe().expect("the test binary");
    let out = std::process::Command::new(exe)
        .args([
            "the_polish_with_the_seed_task_grounds_nothing",
            "--exact",
            "--test-threads=1",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .expect("the child runs");
    assert!(
        out.status.success(),
        "the child's count failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
