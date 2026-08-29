//! The temporal ladder's verbatim re-runs (0.26 F3, the trucks/storage-time
//! decode in `benchmarks/metrics/fieldgaps-A-trucks.md`): when the
//! relevance masks keep every op and the demand is empty, the four-pass
//! ladder (helpful → full+tight → full+sound → full+unmasked) is one
//! search run three more times, and the Full-tier escalation rung then
//! re-runs the identical quartet — ~70 % of storage-time i15's 60 s wall
//! spent re-deriving stats it already had. The dedup skips a pass whose
//! inputs equal an earlier pass's and the Full rung when its demand
//! would equal the numeric tier's; `FF_NO_LADDER_DEDUP=1` restores the
//! quartet, and that leg keeps the RED shape on the record.
//!
//! The fixture is tsearch_wall's ring with the full-ring (unsolvable)
//! goal: pure STRIPS-temporal, no fluents, so both masks are all-true
//! and every demand is empty — every pass is a duplicate by construction,
//! and every pass exhausts its tiny space, so the counts are exact.
//! Child processes per scenario (FF_* are process-global).

use std::process::Command;

fn ring(n: usize, goal_bits: usize) -> (String, String) {
    let objs: String = (0..n).map(|i| format!(" b{i}")).collect();
    let wraps: String = (0..n)
        .map(|i| format!(" (WRAPS b{} b{})", (i + n - 1) % n, i))
        .collect();
    let offs: String = (0..n).map(|i| format!(" (off b{i})")).collect();
    let goal: String = (0..goal_bits).map(|i| format!(" (on b{i})")).collect();
    (
        "(define (domain dedupring)
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
            "(define (problem dr) (:domain dedupring) (:objects{objs} - bit) \
             (:init{wraps}{offs}) (:goal (and{goal})))"
        ),
    )
}

fn run_child(scenario: &str) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "ladder_skips_its_verbatim_re_runs",
        "--nocapture",
    ])
    .env("LADDER_DEDUP_CHILD", scenario)
    .env("FF_RES_DEBUG", "1")
    .env("FF_WALL_DEBUG", "1")
    // The unsolvable ring's complete passes grind to their node cap; the
    // count is what is pinned, so the cap is bounded for the suite (the
    // tsearch_wall.rs convention) — seconds instead of minutes in debug.
    .env("FF_TEMPORAL_NODE_CAP", "20000");
    if scenario == "hatched" {
        cmd.env("FF_NO_LADDER_DEDUP", "1");
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child {scenario} failed");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn passes(stderr: &str) -> usize {
    stderr.matches("[tsearch] pass start").count()
}

#[test]
fn ladder_skips_its_verbatim_re_runs() {
    if let Ok(scenario) = std::env::var("LADDER_DEDUP_CHILD") {
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        // Full ring: unsolvable, so the whole ladder runs to the decomposer.
        let (dom, prb) = ring(4, 4);
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
        // Partial ring: solvable, the dedup must never lose a plan.
        let (dom, prb) = ring(3, 2);
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-{scenario}-PARTIAL:{}", sol.solved);
        return;
    }

    let (out, err) = run_child("dedup");
    assert!(out.contains("CHILD-dedup-SOLVED:false"), "{out}");
    assert!(out.contains("CHILD-dedup-PARTIAL:true"), "{out}");
    assert!(
        err.contains("[TREL] ladder dedup: tight pass skipped (≡ sound), unmasked pass skipped (sound keeps all)"),
        "pass dedup narration missing:\n{err}"
    );
    assert!(
        err.contains("wall: ladder Full tier skipped (demand identical to the numeric tier)"),
        "Full-rung skip narration missing:\n{err}"
    );
    let deduped = passes(&err);

    // The hatched leg: the full quartet twice over, no skip narration.
    let (out, err) = run_child("hatched");
    assert!(out.contains("CHILD-hatched-SOLVED:false"), "{out}");
    assert!(out.contains("CHILD-hatched-PARTIAL:true"), "{out}");
    assert!(!err.contains("ladder dedup"), "{err}");
    assert!(!err.contains("Full tier skipped"), "{err}");
    let quartet = passes(&err);
    // The monolithic ladder is 8 passes hatched (the quartet twice) and 2
    // deduped (helpful + one complete pass); the decomposer rung's contract
    // searches after it are the same on both legs, so the saving is exactly
    // six passes.
    assert_eq!(
        quartet - deduped,
        6,
        "dedup ran {deduped} passes against the quartet's {quartet}"
    );
}
