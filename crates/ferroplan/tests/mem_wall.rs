//! MEMORY IS A WALL TOO (0.28 Lane M, `src/mem.rs`): bounded work over a
//! plan already in hand stops at the declared memory budget, and the plan is
//! what comes back -- instead of the runner's RSS watchdog killing the
//! process, and the plan with it.
//!
//! The board receipt is the first read of the 0.28 lanes through the crucible
//! (6 GB a run): fifteen rows the hand-rolled sit had solved under 8 GB came
//! back `mem-cap`, every one with its plan already found -- ten
//! `pipesworld-metric-time` rows the compression rung banks in seconds, killed
//! in the snap grounding of the quality chase that follows.
//!
//! The fixture is tests/pref_chase_wall.rs's ring: an all-soft goal (the
//! banked plan is the empty plan) and a hardened chase that can never succeed
//! and never exhausts. The node cap is lifted out of the way so the chase's
//! arena grows for as long as it is allowed to; the wall is LONG, so only
//! memory can end it early. Child processes (the budget is read from the
//! process environment).

use std::process::Command;

const BUDGET_GB: &str = "0.25";
const BUDGET_BYTES: u64 = (1u64 << 30) / 4;
const WALL_SECS: f64 = 8.0;

fn ring_pref(n: usize) -> (String, String) {
    let objs: String = (0..n).map(|i| format!(" b{i}")).collect();
    let wraps: String = (0..n)
        .map(|i| format!(" (WRAPS b{} b{})", (i + n - 1) % n, i))
        .collect();
    let offs: String = (0..n).map(|i| format!(" (off b{i})")).collect();
    let want: String = (0..n).map(|i| format!(" (on b{i})")).collect();
    (
        "(define (domain memring)
          (:requirements :typing :durative-actions :preferences)
          (:types bit)
          (:predicates (WRAPS ?p - bit ?b - bit) (on ?b - bit) (off ?b - bit))
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
                         (at end (off ?b)))))"
            .into(),
        format!(
            "(define (problem mr) (:domain memring) (:objects{objs} - bit) \
             (:init{wraps}{offs}) \
             (:goal (and (preference allon (and{want})))) \
             (:metric minimize (is-violated allon)))"
        ),
    )
}

fn run_child(hatched: bool) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "a_banked_plan_survives_the_memory_budget",
        "--nocapture",
    ])
    .env("MEM_WALL_CHILD", "1")
    .env("FF_WALL_DEBUG", "1")
    .env("FF_TIME_LIMIT", WALL_SECS.to_string())
    .env("FF_MEM_BUDGET_GB", BUDGET_GB)
    // Out of the way: the MODELLED caps must not end the chase first.
    .env("FF_TEMPORAL_NODE_CAP", "100000000")
    .env("FF_NO_TCOMPRESS", "1");
    if hatched {
        cmd.env("FF_NO_MEM_WALL", "1");
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child failed (hatched={hatched})");
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
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn a_banked_plan_survives_the_memory_budget() {
    if std::env::var("MEM_WALL_CHILD").is_ok() {
        let n = if cfg!(debug_assertions) { 12 } else { 16 };
        let (dom, prb) = ring_pref(n);
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let t0 = std::time::Instant::now();
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-SECS:{:.3}", t0.elapsed().as_secs_f64());
        println!("CHILD-SOLVED:{}", sol.solved);
        println!("CHILD-LEN:{}", sol.plan.map_or(usize::MAX, |p| p.length));
        println!(
            "CHILD-PEAK:{}",
            ferroplan::mem::peak_resident_bytes().expect("readable here")
        );
        return;
    }
    if cfg!(debug_assertions) {
        // The arena grows an order slower unoptimised; the release run is the pin.
        return;
    }

    // THE PIN: the chase is cut by MEMORY, long before the wall, the banked
    // plan is what comes back, and the process never reached its budget --
    // which is the line a runner's watchdog kills at.
    let (stdout, stderr) = run_child(false);
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert_eq!(field(&stdout, "CHILD-LEN:"), "0", "the banked (empty) plan");
    assert!(
        stderr.contains("wall: temporal search MEMORY checkpoint"),
        "the chase must have been ended by the memory wall:\n{stderr}"
    );
    let peak: u64 = field(&stdout, "CHILD-PEAK:").parse().unwrap();
    assert!(
        peak < BUDGET_BYTES,
        "peak resident {peak} must stay under the declared {BUDGET_BYTES}"
    );
    // A TRIP IS STICKY for the scope it happened in. The arena it freed goes
    // back to the system, so the chase's next tier reads a resident set under
    // the line again -- and, until the mark, grounded and searched its way
    // back up to it (elevator-strips i30 through the crucible: tripped at 5.6
    // GB, 6.4 three tiers later, killed by the watchdog at 6 with its plan in
    // hand). After the first trip, whatever the scope opens stops at its FIRST
    // look: a grounding at the door, a search on its first pop.
    let trips: Vec<&str> = stderr
        .lines()
        .filter(|l| l.contains("MEMORY checkpoint"))
        .collect();
    assert!(
        trips.len() >= 2,
        "the chase has more than one tier, and each must be seen refusing:\n{stderr}"
    );
    for later in &trips[1..] {
        assert!(
            later.contains("at entry") || later.contains("(nodes 1, evaluated 0)"),
            "after a trip the scope must do no more work, but: {later}\n{stderr}"
        );
    }
    let secs: f64 = field(&stdout, "CHILD-SECS:").parse().unwrap();
    assert!(
        secs < WALL_SECS * 0.5,
        "memory, not the wall, ended it: {secs:.1} s of {WALL_SECS}"
    );

    // The permanent RED record (`FF_NO_MEM_WALL=1`, the 0.27 shape): the
    // chase grows until the WALL, straight through the budget -- measured at
    // 8.2 GB after 39 s on this 16-bit ring, against 209 MB and half a second
    // with the wall. Under a runner that is a SIGKILL and an unsolved row.
    let (stdout, stderr) = run_child(true);
    assert!(
        !stderr.contains("MEMORY checkpoint"),
        "the hatch must disarm the memory wall:\n{stderr}"
    );
    let peak: u64 = field(&stdout, "CHILD-PEAK:").parse().unwrap();
    assert!(
        peak > BUDGET_BYTES,
        "unwalled, the chase was expected to outgrow its budget (the RED shape): {peak}"
    );
}
