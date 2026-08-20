//! The promoted rung is a BOUNDED bet (the 0.24 cut regression,
//! docs/roadmap-0.24.md): the required-concurrency detector promotes SAT
//! ahead of the ladder, and on match-cellar's cut instances the h32
//! conflict budget ground in pure SAT conflicts — no STN refutations, so
//! the pre-registered thrash bail never fired — until the WHOLE wall was
//! spent and the ladder (0.02 s on those rows) was refused at pass
//! entry: 0/20 where the canaries had promised 40/40. The fix: the
//! promoted entry gets `FF_SAT_PROMO_WALL_FRAC` (default 0.5) of the
//! remaining wall as its own slice; expiry is an honest hand-back, and
//! `FF_NO_RUNG_WALLCAP=1` hatches the slice with the other checkpoints.
//!
//! One sequential test on purpose: the wall clock is a process-global
//! OnceLock and FF_* are process-global env knobs, so each scenario runs
//! in a CHILD process (the tests/refill.rs convention).

use std::process::Command;

/// The required-concurrency micro-task (the sat_wing.rs fixture): GREEN
/// only via SAT+STN — the ladder is structurally unable to schedule it,
/// which is what makes it the probe for "did the slice stop the wing".
const RC_DOMAIN: &str = "
(define (domain sat-rc)
  (:requirements :strips :durative-actions)
  (:predicates (light) (open) (fresh-shine) (fresh-mend) (fresh-deliver)
               (fresh-door) (mended) (delivered))
  (:durative-action shine
    :parameters ()
    :duration (= ?duration 20)
    :condition (at start (fresh-shine))
    :effect (and (at start (not (fresh-shine)))
                 (at start (light))
                 (at end (not (light)))))
  (:durative-action mend
    :parameters ()
    :duration (= ?duration 9)
    :condition (and (at start (fresh-mend)) (over all (light)))
    :effect (and (at start (not (fresh-mend))) (at end (mended))))
  (:durative-action deliver
    :parameters ()
    :duration (= ?duration 6)
    :condition (and (at start (fresh-deliver)) (at end (open)))
    :effect (and (at start (not (fresh-deliver))) (at end (delivered))))
  (:durative-action door
    :parameters ()
    :duration (= ?duration 2)
    :condition (at start (fresh-door))
    :effect (and (at start (not (fresh-door)))
                 (at start (open))
                 (at end (not (open))))))
";
const RC_PROBLEM: &str = "
(define (problem sat-rc-1) (:domain sat-rc)
  (:init (fresh-shine) (fresh-mend) (fresh-deliver) (fresh-door))
  (:goal (and (mended) (delivered))))
";

/// The match-cellar shape in miniature: the same envelope (mend's
/// over-all needs `light`, which exists only DURING shine — the detector
/// fires and promotes) but decision epochs solve it (start both at t≈0).
/// The row the regression lost: promotion fails, the ladder must still
/// get wall to solve it.
const ENVELOPE_PROBLEM: &str = "
(define (problem sat-rc-env) (:domain sat-rc)
  (:init (fresh-shine) (fresh-mend))
  (:goal (mended)))
";

fn run_child(scenario: &str) -> String {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "promoted_sat_is_a_bounded_bet", "--nocapture"])
        .env("SAT_PROMO_CHILD", scenario)
        .env("FF_TIME_LIMIT", "600");
    match scenario {
        // Default frac: the slice is 300 s, the wing solves in ms — the
        // promoted wins are untouched by the slice's existence.
        "win" => {}
        // A slice too small to reach the first horizon: the bet is
        // BOUNDED — the wing must come back empty-handed instead of
        // spending the wall (RED before the fix: no slice existed, the
        // wing solved regardless of the knob).
        "slice" => {
            cmd.env("FF_SAT_PROMO_WALL_FRAC", "0.000000001");
        }
        // The checkpoint hatch disarms the slice with the rest of the
        // 0.22 clock checkpoints — the pre-slice shape stays pinnable.
        "hatched" => {
            cmd.env("FF_SAT_PROMO_WALL_FRAC", "0.000000001")
                .env("FF_NO_RUNG_WALLCAP", "1");
        }
        // The regression's own story: promotion gets nothing done (spent
        // slice) on a LADDER-SOLVABLE envelope row — the ladder must
        // inherit the wall and solve, not be refused at pass entry.
        "ladder-inherits" => {
            cmd.env("FF_SAT_PROMO_WALL_FRAC", "0.000000001");
        }
        other => panic!("unknown scenario {other}"),
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child {scenario} failed");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn promoted_sat_is_a_bounded_bet() {
    if let Ok(scenario) = std::env::var("SAT_PROMO_CHILD") {
        let problem = match scenario.as_str() {
            "ladder-inherits" => ENVELOPE_PROBLEM,
            _ => RC_PROBLEM,
        };
        let sol = ferroplan::solve(RC_DOMAIN, problem, &ferroplan::Options::default()).unwrap();
        println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
        return;
    }

    // A real slice (default frac, 600 s wall) never costs a promoted win.
    let out = run_child("win");
    assert!(out.contains("CHILD-win-SOLVED:true"), "{out}");

    // A spent slice bounds the bet: the SAT-only task goes honestly
    // unsolved — the wing was stopped by ITS OWN clock, not the wall's.
    let out = run_child("slice");
    assert!(out.contains("CHILD-slice-SOLVED:false"), "{out}");

    // The hatch restores the pre-slice shape (the wing runs unbounded
    // and solves).
    let out = run_child("hatched");
    assert!(out.contains("CHILD-hatched-SOLVED:true"), "{out}");

    // And the row the regression lost: spent slice on a ladder-solvable
    // envelope row still solves — the ladder inherited the wall.
    let out = run_child("ladder-inherits");
    assert!(out.contains("CHILD-ladder-inherits-SOLVED:true"), "{out}");
}
