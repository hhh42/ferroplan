//! SEARCH-side temporal wall discipline (0.24 Phase 6): the 0.23 Phase 6
//! re-referee walled the temporal GROUNDING and then found the leak's
//! second half — sokoban-t grounds in seconds (post-MCV) and the
//! decision-epoch search runs past a 60 s wall, because `temporal_search`
//! was eval-budget-denominated only (deterministic node/eval caps, zero
//! clock reads). The 0.22 Phase 2 idiom lands here too: a clock checkpoint
//! at the pop cadence with the teardown/report reserve (the temporal node
//! arena can be GBs — a verdict that misses the runner's wire is a zombie
//! with extra steps), `FF_NO_RUNG_WALLCAP=1` the hatch, unarmed wall
//! byte-identical.
//!
//! The fixture task grounds in milliseconds and searches forever: a RING
//! of bits where setting a bit knocks its predecessor off at the end
//! snap. All-on is the goal; the delete relaxation cannot see the knock
//! (h stays finite and friendly), while concretely the LAST end around
//! the ring always turns one bit off — every interleaving fails, the
//! frontier never exhausts at fixture scale, and only an honest clock can
//! end the pass. One sequential test, child processes per scenario (the
//! tests/tground_wall.rs convention: the wall clock is a process-global
//! OnceLock).

use std::process::Command;
use std::time::Instant;

/// A ring of `n` bits. set(?b) claims the bit at start (deletes `off`) and
/// at end turns it on while knocking its ring-predecessor off; unset(?b)
/// widens the space. Grounding is linear in `n` (the WRAPS static binds
/// ?p), so the task is search-bound by construction. `goal_bits = n` asks
/// for the full ring — unreachable for every n ≥ 2 (the LAST end around
/// the ring always knocks one bit off); `goal_bits < n` leaves slack for
/// the knocks and is reachable.
fn ring(n: usize, goal_bits: usize) -> (String, String) {
    let objs: String = (0..n).map(|i| format!(" b{i}")).collect();
    let wraps: String = (0..n)
        .map(|i| format!(" (WRAPS b{} b{})", (i + n - 1) % n, i))
        .collect();
    let offs: String = (0..n).map(|i| format!(" (off b{i})")).collect();
    let goal: String = (0..goal_bits).map(|i| format!(" (on b{i})")).collect();
    (
        "(define (domain tsearchring)
          (:requirements :typing :durative-actions)
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
            "(define (problem tsr) (:domain tsearchring) (:objects{objs} - bit) \
             (:init{wraps}{offs}) (:goal (and{goal})))"
        ),
    )
}

fn run_child(scenario: &str) -> (String, String, f64) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "temporal_search_pays_the_wall", "--nocapture"])
        .env("TSEARCH_WALL_CHILD", scenario)
        .env("FF_WALL_DEBUG", "1");
    match scenario {
        "swall" => {
            // THE PIN: with grounding done in milliseconds, a 1 s armed
            // wall must end the SEARCH honestly — narrated checkpoint,
            // prompt exit — instead of grinding the pass ladder's node
            // caps to their deterministic ends (RED today: the sokoban-t
            // shape, search past the wall with the clock never read).
            cmd.env("FF_TIME_LIMIT", "1");
        }
        "swall-hatched" => {
            // The permanent RED record: hatched off, the ladder ignores
            // the expired wall and grinds to its node caps (bounded here
            // so the record costs seconds, not the historical minutes).
            // 0.26 F3's ladder dedup skips the quartet's verbatim
            // re-runs, which halves this grind (the ring's masks keep
            // everything) and pulled the leg to the 0.5 s floor; the RED
            // shape this leg records is the FULL quartet, so it hatches
            // the dedup too (tests/ladder_dedup.rs pins the dedup itself).
            cmd.env("FF_TIME_LIMIT", "1")
                .env("FF_NO_RUNG_WALLCAP", "1")
                .env("FF_NO_LADDER_DEDUP", "1")
                .env("FF_TEMPORAL_NODE_CAP", "20000");
        }
        "scontrol" => {
            // A solvable ring task under a LONG wall solves exactly as
            // before — the checkpoint only ever refuses work that cannot
            // finish; found plans are never discarded.
            let wall = if cfg!(debug_assertions) { "300" } else { "60" };
            cmd.env("FF_TIME_LIMIT", wall);
        }
        other => panic!("unknown scenario {other}"),
    }
    let t0 = Instant::now();
    let out = cmd.output().unwrap();
    let secs = t0.elapsed().as_secs_f64();
    assert!(out.status.success(), "child {scenario} failed");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        secs,
    )
}

#[test]
fn temporal_search_pays_the_wall() {
    if let Ok(scenario) = std::env::var("TSEARCH_WALL_CHILD") {
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        match scenario.as_str() {
            "swall" | "swall-hatched" => {
                // Big enough that no pass exhausts the frontier before
                // the wall (the ring's reachable space is exponential in
                // n); the task is unsolvable by the knock-off invariant,
                // so any "solved" here is an engine bug, not a fixture
                // accident.
                let n = if cfg!(debug_assertions) { 10 } else { 14 };
                let (dom, prb) = ring(n, n);
                let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
                println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
            }
            "scontrol" => {
                // The PARTIAL goal is reachable — on b0 ∧ on b1 with b2
                // left free: set b1 (its end knocks b0, still off), then
                // set b0 (its end knocks b2, irrelevant). The full-ring
                // goal is the unsolvable arm; this leg pins that the
                // checkpoint never refuses work that finishes in budget.
                let (dom, prb) = ring(3, 2);
                let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
                println!("CHILD-scontrol-SOLVED:{}", sol.solved);
            }
            other => panic!("unknown scenario {other}"),
        }
        return;
    }

    // THE PIN: grounding takes milliseconds here, so a 1 s armed wall must
    // produce an honest SEARCH exit — the checkpoint narration and a
    // prompt return, never a node-cap grind past the budget.
    let (stdout, stderr, secs) = run_child("swall");
    assert!(
        stdout.contains("CHILD-swall-SOLVED:false"),
        "the ring task is unsolvable; the walled exit must say so honestly:\n{stdout}"
    );
    assert!(
        stderr.contains("wall: temporal search checkpoint expired"),
        "search-side trip narration missing:\n{stderr}"
    );
    if !cfg!(debug_assertions) {
        assert!(
            secs < 8.0,
            "the temporal search wall stop must be prompt: {secs:.1} s"
        );
    }

    // The hatched leg keeps the RED shape on the record: with checkpoints
    // off the ladder grinds its (bounded-for-the-suite) node caps well
    // past the 1 s wall and never narrates a search checkpoint.
    let (stdout, stderr, secs) = run_child("swall-hatched");
    assert!(
        stdout.contains("CHILD-swall-hatched-SOLVED:false"),
        "{stdout}"
    );
    assert!(
        !stderr.contains("wall: temporal search checkpoint expired"),
        "hatch must disarm the search checkpoint:\n{stderr}"
    );
    if !cfg!(debug_assertions) {
        // 0.24: 1.0 s read 0.9 s consistently on the M5 Air (faster than
        // whatever box this margin was set on) — not a flake, the node
        // cap's grind is just quicker here. 0.5 s still cleanly separates
        // "still grinding" from "exited near-instantly" (which would mean
        // the checkpoint fired anyway, or the fixture solved trivially).
        assert!(
            secs > 0.5,
            "the hatched ladder was expected to grind well past instant (the RED shape): {secs:.1} s"
        );
    }

    // Negative control: a solvable ring under a long wall still solves.
    let (stdout, _, _) = run_child("scontrol");
    assert!(stdout.contains("CHILD-scontrol-SOLVED:true"), "{stdout}");
}
