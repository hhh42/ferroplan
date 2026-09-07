//! THE PER-CALL BUDGET (0.28): `Options::wall_ms` and
//! `Options::should_continue`.
//!
//! WHY IT HAD TO EXIST. A long-lived host embedding ferroplan had neither
//! of the two budgets the engine already shipped:
//!
//! * `max_evaluated` bounds EVALUATED STATES, which cannot bound a binding
//!   enumeration or a goal-DNF expansion that has not produced a state yet.
//!   A call capped at ten million evaluations can still spend minutes
//!   upstream of its first one.
//! * `FF_TIME_LIMIT` is armed ONCE PER PROCESS (a `OnceLock`) and measured
//!   from the first solve. In a process that lives for hours it either
//!   bounds nothing or, past its total, refuses everything.
//!
//! With neither able to bound one call, `solve` was blocking and
//! uninterruptible: a host that abandoned the work -- a dropped future, an
//! entity that no longer needs a plan -- leaked that thread for the rest of
//! the process. The measurement that prompted this, from the host's own
//! instrumentation: 213 solves completed then 2 abandoned, after which the
//! pool was dead -- 0 completed, 4 abandoned.
//!
//! WHAT THE LEGS COST. Nothing here runs an unbounded control: the bomb
//! below is stopped by a SHORT budget in one leg and a LONGER one in the
//! next, and it is the difference between them that proves the budget is
//! what bound the call rather than a cheap finish. The suite must be safe
//! to run on a box that is also sweeping.
//!
//! The env-dependent legs run in a CHILD process (the tests/ground_wall.rs
//! convention) for one reason only: `FF_NO_GOAL_FACTOR` makes the bomb
//! expensive, and FF_* knobs are process-global. The BUDGET itself needs no
//! such dance -- that is the whole point of it -- so the leak, concurrency
//! and serde pins below are ordinary in-process tests.

use ferroplan::{solve, Options};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// K disjunctive goal items over disjoint fluent pairs: the goal DNF is
/// 4^K, hopeless for K=60 on any box, and the cost is paid in GROUNDING --
/// upstream of the first evaluated state. This is the shape `max_evaluated`
/// cannot bound. Borrowed from tests/ground_wall.rs, where the same bomb
/// pins the env-armed wall. Every pair reads UNEQUAL at init, so the task
/// is trivially solvable and a stop is purely the budget's doing.
fn orbomb(k: usize) -> (String, String) {
    let mut fns = String::new();
    let mut init = String::new();
    let mut goal = String::new();
    for i in 0..k {
        fns.push_str(&format!(" (x{i}) (y{i})"));
        init.push_str(&format!(" (= (x{i}) 0) (= (y{i}) 1)"));
        goal.push_str(&format!(
            " (or (not (= (x{i}) (y{i}))) (not (= (y{i}) (x{i}))))"
        ));
    }
    (
        format!(
            "(define (domain orbomb) (:requirements :numeric-fluents)
               (:functions{fns})
               (:action bump :parameters () :effect (increase (x0) 1)))"
        ),
        format!("(define (problem ob) (:domain orbomb) (:init{init}) (:goal (and{goal})))"),
    )
}

/// An honest classical task, sized so the pins about the budget's PLUMBING
/// (does it leak, is it per-thread) have room to be decisive: n=30 solves
/// unbudgeted in ~35 ms, so a 1 ms wall binds by a wide margin even on a
/// box that is also sweeping.
fn blocks(n: usize) -> (String, String) {
    let objs: Vec<String> = (0..n).map(|i| format!("b{i}")).collect();
    let init: String = objs
        .iter()
        .map(|b| format!(" (clear {b}) (ontable {b})"))
        .collect();
    let goal: String = (0..n - 1)
        .map(|i| format!(" (on b{i} b{})", i + 1))
        .collect();
    (
        "(define (domain bw) (:requirements :strips)
           (:predicates (on ?x ?y) (clear ?x) (ontable ?x) (holding ?x))
           (:action pick :parameters (?x) :precondition (and (clear ?x) (ontable ?x))
             :effect (and (holding ?x) (not (clear ?x)) (not (ontable ?x))))
           (:action stack :parameters (?x ?y) :precondition (and (holding ?x) (clear ?y))
             :effect (and (on ?x ?y) (clear ?x) (not (holding ?x)) (not (clear ?y)))))"
            .into(),
        format!(
            "(define (problem bw{n}) (:domain bw) (:objects {})
               (:init{init}) (:goal (and{goal})))",
            objs.join(" ")
        ),
    )
}

fn run_child(scenario: &str) -> (String, f64) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "a_declared_budget_bounds_one_call",
        "--nocapture",
    ])
    .env("CALL_BUDGET_CHILD", scenario)
    // The ONLY reason these legs need a child: it is what makes the
    // bomb expensive. 0.22 Phase 7's factored goal-check simply solves
    // it otherwise, and a budget that stops a cheap call proves nothing.
    .env("FF_NO_GOAL_FACTOR", "1")
    // No FF_TIME_LIMIT anywhere in this file, deliberately: every stop
    // below is the caller's own budget, armed in code.
    .env_remove("FF_TIME_LIMIT");
    let t0 = Instant::now();
    let out = cmd.output().unwrap();
    let secs = t0.elapsed().as_secs_f64();
    assert!(
        out.status.success(),
        "child {scenario} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (String::from_utf8_lossy(&out.stdout).into_owned(), secs)
}

/// THE HEADLINE. A budget declared in code bounds a call whose cost is all
/// upstream of search, and says so in a way the caller can act on.
#[test]
fn a_declared_budget_bounds_one_call() {
    if let Ok(scenario) = std::env::var("CALL_BUDGET_CHILD") {
        let (dom, prb) = orbomb(60);
        let mut opts = Options {
            threads: 1,
            // Armed, and generously: if THIS is what stopped the call the
            // legs below prove nothing. It cannot -- the bomb never reaches
            // a state to evaluate.
            max_evaluated: Some(10_000_000),
            ..Default::default()
        };
        match scenario.as_str() {
            "short" => opts.wall_ms = Some(400),
            "long" => opts.wall_ms = Some(3000),
            "withdraw" => {
                let flag = Arc::new(AtomicBool::new(true));
                let trigger = Arc::clone(&flag);
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    trigger.store(false, Ordering::Relaxed);
                });
                opts.should_continue = Some(flag);
            }
            // Already false when the call starts: the caller changed its
            // mind before we did any work at all.
            "withdrawn" => opts.should_continue = Some(Arc::new(AtomicBool::new(false))),
            other => panic!("unknown scenario {other}"),
        }
        let t0 = Instant::now();
        let sol = solve(&dom, &prb, &opts).expect("a budget stop is a verdict, not an error");
        println!("CHILD-SOLVED:{}", sol.solved);
        println!("CHILD-SECS:{:.3}", t0.elapsed().as_secs_f64());
        println!("CHILD-NOTE:{}", sol.notes.join(" | "));
        return;
    }

    // Leg 1: a 400 ms budget stops the 4^60 expansion promptly, with a note
    // that names the budget and never the word "unsolvable" -- running out
    // of time is not a proof (the 0.21 honesty rider).
    let (short, short_secs) = run_child("short");
    assert!(short.contains("CHILD-SOLVED:false"), "{short}");
    assert!(
        short.contains("declared budget"),
        "the note must name the budget:\n{short}"
    );
    assert!(
        !short.to_lowercase().contains("unsolvable"),
        "a budget stop is never a verdict:\n{short}"
    );
    if !cfg!(debug_assertions) {
        assert!(
            short_secs < 20.0,
            "the stop must be prompt (the child pays spawn and parse too): {short_secs:.1} s"
        );
    }

    // Leg 2, the control that gives leg 1 its meaning: the SAME call under a
    // longer budget runs longer. So it was the budget that ended it, not a
    // cheap finish -- and no unbounded run was needed to say so.
    let (long, _) = run_child("long");
    assert!(long.contains("CHILD-SOLVED:false"), "{long}");
    let secs_of = |out: &str| -> f64 {
        out.lines()
            .find_map(|l| l.strip_prefix("CHILD-SECS:"))
            .expect("child reported its own elapsed")
            .parse()
            .unwrap()
    };
    let (a, b) = (secs_of(&short), secs_of(&long));
    assert!(
        b > a * 1.5,
        "a longer budget must buy more work: 400ms leg {a:.2}s vs 3000ms leg {b:.2}s"
    );

    // Leg 3: `should_continue`, flipped from another thread mid-call. This
    // is what a host does when the work stops mattering.
    let (withdraw, _) = run_child("withdraw");
    assert!(withdraw.contains("CHILD-SOLVED:false"), "{withdraw}");
    assert!(
        withdraw.contains("should_continue"),
        "the note says WHICH budget stopped it, so the caller need not \
         infer it from a timing:\n{withdraw}"
    );

    // Leg 4: a flag that is already false costs a parse, not a plan.
    let (withdrawn, _) = run_child("withdrawn");
    assert!(withdrawn.contains("CHILD-SOLVED:false"), "{withdrawn}");
    assert!(secs_of(&withdrawn) < a, "the earliest possible stop");
}

/// THE LEAK GUARD, and the pin that matters most for a host: the budget is
/// armed by an RAII guard, so it cannot survive the call that armed it --
/// not past a return, not past a `?`, not past a panic. A stale deadline
/// leaking into the next call on the same thread would turn a host's tenth
/// solve into an instant unexplained failure, a worse bug than the one this
/// feature fixes.
///
/// The stopped legs use a flag that is already false, or a wall an order of
/// magnitude under the instance's own time. The first cut used a 1 ms wall
/// on a 0.7 ms instance and was a coin toss -- a call that finishes before
/// its first checkpoint finishes, which is correct behaviour and a useless
/// pin.
#[test]
fn a_spent_budget_does_not_leak_into_the_next_call_on_this_thread() {
    let (d, p) = blocks(30);
    let stopped = solve(
        &d,
        &p,
        &Options {
            should_continue: Some(Arc::new(AtomicBool::new(false))),
            ..Default::default()
        },
    )
    .expect("a verdict");
    assert!(
        !stopped.solved,
        "the withdrawn leg must stop: {:?}",
        stopped.notes
    );

    // Same thread, no budget: must behave as if the first call never ran.
    let after = solve(&d, &p, &Options::default()).expect("a verdict");
    assert!(
        after.solved,
        "the previous call's spent budget must not bound this one: {:?}",
        after.notes
    );

    // And once more with a wall, since that is the other thing that could
    // leak. blocks(30) solves in ~35 ms unbudgeted, so a 1 ms wall binds
    // with room to spare on a loaded box.
    let walled = solve(
        &d,
        &p,
        &Options {
            wall_ms: Some(1),
            ..Default::default()
        },
    )
    .expect("a verdict");
    assert!(!walled.solved, "the 1ms leg must stop: {:?}", walled.notes);
    assert!(
        solve(&d, &p, &Options::default())
            .expect("a verdict")
            .solved,
        "nor may a spent WALL leak into the next call"
    );
}

/// Concurrent solves keep their own budgets. This is why the budget lives
/// in a thread-local rather than a process-global: the host that asked for
/// it runs four solves at once, and one caller's withdrawal must never end
/// another caller's call.
#[test]
fn concurrent_calls_do_not_share_a_budget() {
    let (d, p) = blocks(30);
    let (d2, p2) = (d.clone(), p.clone());

    let bounded = std::thread::spawn(move || {
        solve(
            &d2,
            &p2,
            &Options {
                should_continue: Some(Arc::new(AtomicBool::new(false))),
                ..Default::default()
            },
        )
        .expect("a verdict")
        .solved
    });
    let free = solve(&d, &p, &Options::default()).expect("a verdict");

    assert!(!bounded.join().unwrap(), "one thread's call was bounded");
    assert!(
        free.solved,
        "the other thread's was never bounded: {:?}",
        free.notes
    );
}

/// An unarmed `Options` is the shape every prior release ran: no budget, no
/// note, same verdict.
#[test]
fn an_unarmed_options_is_unchanged() {
    let (d, p) = blocks(30);
    let sol = solve(&d, &p, &Options::default()).expect("a verdict");
    assert!(sol.solved);
    assert!(
        !sol.notes.iter().any(|n| n.contains("declared budget")),
        "no budget declared, so no budget note: {:?}",
        sol.notes
    );
}

/// The new fields are additive on the wire. `should_continue` is an
/// in-process handle with no JSON form, so it is `#[serde(skip)]`: a
/// round-tripped `Options` comes back with `None` there, and every older
/// JSON config still deserializes.
#[test]
fn options_still_round_trip_and_older_json_still_loads() {
    let opts = Options {
        wall_ms: Some(1234),
        should_continue: Some(Arc::new(AtomicBool::new(true))),
        ..Default::default()
    };
    let back: Options = serde_json::from_str(&serde_json::to_string(&opts).unwrap()).unwrap();
    assert_eq!(back.wall_ms, Some(1234));
    assert!(
        back.should_continue.is_none(),
        "a live handle does not survive JSON, and must not pretend to"
    );

    let old: Options = serde_json::from_str(r#"{"weight_h": 3.0}"#).unwrap();
    assert_eq!(old.wall_ms, None, "absent means unbounded, as before");
    assert!(old.should_continue.is_none());
}
