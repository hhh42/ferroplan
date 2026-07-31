//! The refill loop (0.20 Phase 1, docs/roadmap-0.20.md): a CAPPED
//! best-first fallback with >10% of an armed wall remaining re-enters
//! greedier (w_h ×4, max_eval ×4, memory bound untouched) instead of
//! returning unsolved with budget on the table.
//!
//! One sequential test on purpose: the wall clock is a process-global
//! OnceLock and FF_* are process-global env knobs, so each scenario runs
//! in a CHILD process (the tests/node_cap.rs convention) — the parent
//! sets the env, spawns itself, and asserts on the child's narration.
//! The fixture task is a 7×7 visit-all grid (benchmarks/bench/visitgrid)
//! that needs ~40k insertions under best-first at any weight; a forced
//! 3000-node cap makes EVERY round cap, which pins the loop's shape:
//! exactly REFILL_MAX_ROUNDS re-entries, then an honest unsolved return.

use std::process::Command;

fn fixture(name: &str) -> String {
    let p = format!(
        "{}/../../benchmarks/bench/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p}: {e}"))
}

fn run_child(scenario: &str) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "refill_loop_spends_the_wall", "--nocapture"])
        .env("REFILL_CHILD", scenario)
        .env("FF_SEARCH_NODE_CAP", "3000")
        .env("FF_WALL_DEBUG", "1");
    match scenario {
        "refill" => {
            cmd.env("FF_TIME_LIMIT", "600");
        }
        "no-refill" => {
            cmd.env("FF_TIME_LIMIT", "600").env("FF_NO_REFILL", "1");
        }
        "no-budget" => {}
        other => panic!("unknown scenario {other}"),
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child {scenario} failed");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn refill_loop_spends_the_wall() {
    if let Ok(scenario) = std::env::var("REFILL_CHILD") {
        // Child: one best-first solve under the parent's env. BestFirst
        // skips the bounded rungs, so the capped fallback — the refill
        // loop's home — is the whole ladder here.
        let dom = fixture("visitgrid-domain.pddl");
        let prb = fixture("visitgrid-7x7.pddl");
        let opts = ferroplan::Options {
            search: ferroplan::Search::BestFirst,
            ..Default::default()
        };
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
        return;
    }

    // Armed wall + capped rounds: exactly REFILL_MAX_ROUNDS (6)
    // re-entries fire, each narrated, and the return is still honest
    // (the forced node cap holds every round, so unsolved).
    let (stdout, stderr) = run_child("refill");
    let rounds = stderr.matches("wall: refill round").count();
    assert_eq!(rounds, 6, "expected 6 refill rounds, stderr:\n{stderr}");
    assert!(stdout.contains("CHILD-refill-SOLVED:false"), "{stdout}");

    // The hatch disarms the loop entirely.
    let (stdout, stderr) = run_child("no-refill");
    assert_eq!(stderr.matches("wall: refill round").count(), 0, "{stderr}");
    assert!(stdout.contains("CHILD-no-refill-SOLVED:false"), "{stdout}");

    // No declared budget: single round, byte-identical to the 0.19
    // ladder (wall_remaining_frac is None, the loop never arms).
    let (stdout, stderr) = run_child("no-budget");
    assert_eq!(stderr.matches("wall: refill round").count(), 0, "{stderr}");
    assert!(stdout.contains("CHILD-no-budget-SOLVED:false"), "{stdout}");
}
