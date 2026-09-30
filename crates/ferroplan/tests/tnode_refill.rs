//! THE REFILL (0.29 Lane 3, `temporal::refill_ladder`): when the temporal
//! ladder's node cap trips with wall left on a task the compression rung
//! declines, the cap is doubled and the ladder runs again under the memory
//! wall, until a plan, the wall, or the 64x round.
//!
//! The board receipt: 37 cells of `ipc5-constraints` (tpp 23 of 30,
//! pipesworld 14 of 20) banked "temporal ladder exhausted its budgets with N s
//! of wall left" on cut28 -- the cap, a MODEL of what the memory affords,
//! tripped in seconds and the process exited with most of a minute unspent.
//!
//! The fixture: a chain of N durative steps under a hard `always`
//! constraint (so the compression rung declines it), with the node cap
//! forced tiny through `FF_TEMPORAL_NODE_CAP` in a child process. Without
//! the refill the ladder exhausts its cap and reports wall left; with it the
//! plan is found on a later round.

use std::process::Command;

fn chain(n: usize) -> (String, String) {
    let objs: String = (0..=n).map(|i| format!(" p{i}")).collect();
    let next: String = (0..n).map(|i| format!(" (NEXT p{i} p{})", i + 1)).collect();
    (
        "(define (domain chain)
          (:requirements :typing :durative-actions :constraints)
          (:types pos)
          (:predicates (NEXT ?a - pos ?b - pos) (at ?p - pos) (safe) (mark ?p - pos))
          (:durative-action step
            :parameters (?a - pos ?b - pos)
            :duration (= ?duration 1)
            :condition (and (at start (at ?a)) (at start (NEXT ?a ?b)) (over all (safe)))
            :effect (and (at start (not (at ?a))) (at end (at ?b)) (at end (mark ?b))))
          (:durative-action wobble
            :parameters (?a - pos)
            :duration (= ?duration 1)
            :condition (at start (at ?a))
            :effect (and (at end (mark ?a)))))"
            .into(),
        format!(
            "(define (problem c) (:domain chain) (:objects{objs} - pos) \
             (:init (at p0) (safe){next}) \
             (:constraints (always (safe))) \
             (:goal (at p{n})))"
        ),
    )
}

#[test]
fn the_ladder_refills_its_node_cap_while_wall_remains() {
    if std::env::var("REFILL_CHILD").is_ok() {
        let (d, p) = chain(12);
        let sol = ferroplan::solve(
            &d,
            &p,
            &ferroplan::Options {
                threads: 1,
                ..Default::default()
            },
        )
        .unwrap();
        println!("CHILD-SOLVED:{}", sol.solved);
        println!("CHILD-NOTES:{}", sol.notes.join(" | "));
        return;
    }
    let run = |hatch: bool| {
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.args([
            "--exact",
            "the_ladder_refills_its_node_cap_while_wall_remains",
            "--nocapture",
        ])
        .env("REFILL_CHILD", "1")
        .env("FF_TIME_LIMIT", "20")
        .env("FF_WALL_DEBUG", "1")
        // Six nodes: the twelve-step chain cannot be searched under it.
        .env("FF_TEMPORAL_NODE_CAP", "6")
        .env("FF_NO_ESCALATE", "1");
        if hatch {
            cmd.env("FF_NO_TNODE_REFILL", "1");
        }
        let out = cmd.output().unwrap();
        assert!(out.status.success());
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    };
    let (stdout, stderr) = run(true);
    assert!(
        stdout.contains("CHILD-SOLVED:false"),
        "the fixture must exhaust a six-node cap without the refill:\n{stdout}"
    );
    assert!(
        stdout.contains("exhausted its budgets with"),
        "and say the wall was left unspent:\n{stdout}\n{stderr}"
    );
    let (stdout, stderr) = run(false);
    assert!(
        stdout.contains("CHILD-SOLVED:true"),
        "the refill must find the plan:\n{stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("temporal node cap refilled x"),
        "and narrate the round:\n{stderr}"
    );
}
