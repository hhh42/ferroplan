//! A BANKED ROW MUST CROSS THE WIRE (0.28 Lane S). The complex-preference
//! tiers bank a plan with every preference dropped, then chase quality with
//! every preference hardened. Until 0.28 the chase was charged to the same
//! wall as the plan it was trying to improve: it ran to the deadline, the
//! ladder under it opened further rungs past the deadline, and the caller
//! then grounded the original pair AGAIN to score the result. The board's
//! runner kills at the wall, so the row read "unsolved" with a valid plan
//! in memory -- pathways-preferences-complex i7 returned its banked,
//! VAL-valid plan at 21.94 s against a 20 s wall, and 24 of that domain's
//! 30 rows were lost the same way.
//!
//! The fixture is tests/tsearch_wall.rs's ring with the unreachable
//! full-ring goal moved into a PREFERENCE. The hard goal is empty, so the
//! banked plan is the empty plan; the hardened chase can never succeed
//! (the last end around the ring always knocks a bit off) and never
//! exhausts at fixture scale. Only the wall ends it -- and the pin is that
//! it ends INSIDE the wall, with the banked plan reported AND scored.
//!
//! The BALLAST is what makes the old shape RED rather than merely tight:
//! a three-parameter action behind a fact nothing can ever add. The solve
//! side prunes it, but every grounding still has to enumerate its K^3
//! bindings, so grounding costs real time -- which is the corpus shape
//! (pathways-complex i20 grounds for ~12 s). v0.27.1 on this fixture:
//! returned at 3.99 s against a 3 s wall, the overrun being the scorer's
//! grounding, paid after the chase had already spent the wall.
//!
//! Child processes per scenario (the tests/tground_wall.rs convention: the
//! wall clock is a process-global OnceLock).

use std::process::Command;

const WALL_SECS: f64 = 6.0;

/// `n` ring bits; the preference asks for the first `pref_bits` of them on.
/// `junk` objects feed the ballast action (`junk`^3 bindings per grounding).
fn ring_pref(n: usize, pref_bits: usize, junk: usize) -> (String, String) {
    let objs: String = (0..n).map(|i| format!(" b{i}")).collect();
    let junks: String = (0..junk).map(|i| format!(" j{i}")).collect();
    let wraps: String = (0..n)
        .map(|i| format!(" (WRAPS b{} b{})", (i + n - 1) % n, i))
        .collect();
    let offs: String = (0..n).map(|i| format!(" (off b{i})")).collect();
    let want: String = (0..pref_bits).map(|i| format!(" (on b{i})")).collect();
    (
        "(define (domain prefchasering)
          (:requirements :typing :durative-actions :preferences)
          (:types bit junk)
          (:predicates (WRAPS ?p - bit ?b - bit) (on ?b - bit) (off ?b - bit)
                       (never) (armed) (heavy ?x ?y ?z - junk))
          (:durative-action set
            :parameters (?b - bit ?p - bit)
            :duration (= ?duration 1)
            :condition (and (at start (off ?b)) (at start (WRAPS ?p ?b)))
            :effect (and (at start (not (off ?b)))
                         (at end (on ?b))
                         (at end (not (on ?p)))
                         (at end (off ?p))))
          (:durative-action unset
            :parameters (?b - bit)
            :duration (= ?duration 1)
            :condition (at start (on ?b))
            :effect (and (at start (not (on ?b)))
                         (at end (off ?b))))
          (:durative-action arm
            :parameters ()
            :duration (= ?duration 1)
            :condition (at start (never))
            :effect (at end (armed)))
          (:durative-action ballast
            :parameters (?x ?y ?z - junk)
            :duration (= ?duration 1)
            :condition (at start (armed))
            :effect (at end (heavy ?x ?y ?z))))"
            .into(),
        format!(
            "(define (problem pcr) (:domain prefchasering) \
             (:objects{objs} - bit{junks} - junk) \
             (:init{wraps}{offs}) \
             (:goal (and (preference allon (and{want})))) \
             (:metric minimize (is-violated allon)))"
        ),
    )
}

fn run_child(scenario: &str) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "a_banked_plan_is_reported_inside_the_wall",
        "--nocapture",
    ])
    .env("PREF_CHASE_WALL_CHILD", scenario)
    .env("FF_WALL_DEBUG", "1");
    match scenario {
        "chase" => {
            cmd.env("FF_TIME_LIMIT", WALL_SECS.to_string());
        }
        "control" => {
            let wall = if cfg!(debug_assertions) { "300" } else { "60" };
            cmd.env("FF_TIME_LIMIT", wall);
        }
        other => panic!("unknown scenario {other}"),
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child {scenario} failed");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn field<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix(key))
        .unwrap_or_else(|| panic!("child printed no {key}:\n{stdout}"))
}

#[test]
fn a_banked_plan_is_reported_inside_the_wall() {
    if let Ok(scenario) = std::env::var("PREF_CHASE_WALL_CHILD") {
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let (dom, prb) = match scenario.as_str() {
            // Unreachable preference, frontier that never exhausts.
            "chase" => {
                let (n, junk) = if cfg!(debug_assertions) {
                    (10, 16)
                } else {
                    (14, 60)
                };
                ring_pref(n, n, junk)
            }
            // Reachable preference (tests/tsearch_wall.rs's scontrol goal):
            // the chase must still WIN when it can.
            "control" => ring_pref(3, 2, 2),
            other => panic!("unknown scenario {other}"),
        };
        // The wall arms at solve entry, so THIS interval is what the wall
        // is compared against -- not the parent's spawn-to-exit time.
        let t0 = std::time::Instant::now();
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        let secs = t0.elapsed().as_secs_f64();
        println!("CHILD-SOLVED:{}", sol.solved);
        println!("CHILD-SECS:{secs:.3}");
        let plan = sol.plan.as_ref();
        println!("CHILD-LEN:{}", plan.map_or(usize::MAX, |p| p.length));
        println!("CHILD-METRIC:{:?}", plan.and_then(|p| p.metric));
        return;
    }

    // THE PIN: the chase cannot succeed and cannot exhaust, so the wall is
    // the only exit -- and the banked (empty) plan must come back solved,
    // scored, and BEFORE the wall the runner kills at.
    let (stdout, stderr) = run_child("chase");
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert_eq!(
        field(&stdout, "CHILD-LEN:"),
        "0",
        "the banked plan of an all-soft goal is the empty plan:\n{stdout}"
    );
    if !cfg!(debug_assertions) {
        let secs: f64 = field(&stdout, "CHILD-SECS:").parse().unwrap();
        assert!(
            secs < WALL_SECS,
            "the banked plan must be reported INSIDE the {WALL_SECS} s wall, not at {secs:.3} s"
        );
        assert!(
            secs > WALL_SECS * 0.5,
            "the chase is supposed to USE the wall it is given ({secs:.3} s of {WALL_SECS})"
        );
        assert_eq!(
            field(&stdout, "CHILD-METRIC:"),
            "Some(1.0)",
            "the banked plan violates the one preference, and the report says so:\n{stdout}"
        );
    }
    assert!(
        stderr.contains("wall: preference chase found nothing; returning the banked plan"),
        "the chase must have RUN and been ended by the wall:\n{stderr}"
    );

    // Negative control: a preference the chase CAN satisfy is still won --
    // the reserve only ever takes wall the chase could not have finished in.
    let (stdout, stderr) = run_child("control");
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert_eq!(
        field(&stdout, "CHILD-METRIC:"),
        "Some(0.0)",
        "the chase plan satisfies the preference:\n{stdout}"
    );
    assert_ne!(field(&stdout, "CHILD-LEN:"), "0", "{stdout}");
}
