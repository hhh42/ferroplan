//! The rep-folded numeric labels' RED/GREEN pair (0.23 Phase 5 rung 2,
//! docs/roadmap-0.23.md), on the gap-60 pump chain
//! (benchmarks/bench/numopt-p04.pddl): the 0.22 Phase 4 arm bought the
//! collapse but paid the interval RPG at EVERY node; the fold precomputes
//! the audited admissible repetition bound once per solve and the
//! per-eval tax drops to a margin read. On p04 the fold's bound is
//! EXACTLY the layer bound (gap/1 with a fire-free mover), so the search
//! is identical node for node — same certificate, same expansions, zero
//! per-eval RPG builds. `FF_OPT_NO_NUMFOLD=1` restores the 0.22 per-eval
//! RPG bit for bit.
//!
//! `FF_*` are process-global env knobs, so each scenario runs in a CHILD
//! process (the tests/refill.rs convention); the tax meter rides the
//! FF_WALL_DEBUG cert diagnostics ("L label passes, R rpg evals, E
//! evaluated") and the arming narration ("numfold armed ..." /
//! "numfold declined ...").

use std::process::Command;

fn run_child(scenario: &str) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "num_fold_drops_the_per_eval_rpg_tax",
        "--nocapture",
    ])
    .env("NUMFOLD_CHILD", scenario)
    .env("FF_WALL_DEBUG", "1");
    match scenario {
        "fold" => {}
        "no-fold" => {
            cmd.env("FF_OPT_NO_NUMFOLD", "1");
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

fn int_after(hay: &str, pat: &str) -> u64 {
    let tail = hay
        .split(pat)
        .nth(1)
        .unwrap_or_else(|| panic!("`{pat}` not found in:\n{hay}"));
    let digits: String = tail
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("no integer after `{pat}` in:\n{hay}"))
}

#[test]
fn num_fold_drops_the_per_eval_rpg_tax() {
    if let Ok(scenario) = std::env::var("NUMFOLD_CHILD") {
        let dom = include_str!("../../../benchmarks/bench/numopt-pump-domain.pddl");
        let prb = include_str!("../../../benchmarks/bench/numopt-p04.pddl");
        let opts = ferroplan::Options {
            mode: ferroplan::Mode::Optimal,
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(dom, prb, &opts).unwrap();
        println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
        println!(
            "CHILD-LENGTH:{}",
            sol.plan.as_ref().map(|p| p.length).unwrap_or(0)
        );
        println!("CHILD-NOTE:{}", sol.notes.join(" | "));
        return;
    }

    // GREEN — the fold arms, the certificate is byte-for-byte the 0.22
    // one (cost 60, "+numRPG" prover), and the RPG tax meter reads ZERO.
    let (stdout, stderr) = run_child("fold");
    assert!(stdout.contains("CHILD-fold-SOLVED:true"), "{stdout}");
    assert!(stdout.contains("CHILD-LENGTH:60"), "{stdout}");
    assert!(
        stdout.contains("h^max+numRPG"),
        "the prover keeps its numeric component name: {stdout}"
    );
    assert!(
        stderr.contains("numfold armed"),
        "arming must narrate:\n{stderr}"
    );
    let rpg_fold = int_after(&stderr, "label passes,");
    assert_eq!(rpg_fold, 0, "the fold pays no per-eval RPG:\n{stderr}");
    let evals_fold = int_after(&stderr, "rpg evals,");

    // RED via hatch — FF_OPT_NO_NUMFOLD restores the per-eval RPG: same
    // certificate, same search (the fold's bound is exact here), one
    // RPG build per evaluation.
    let (stdout_n, stderr_n) = run_child("no-fold");
    assert!(stdout_n.contains("CHILD-no-fold-SOLVED:true"), "{stdout_n}");
    assert!(stdout_n.contains("CHILD-LENGTH:60"), "{stdout_n}");
    assert!(
        stdout_n.contains("h^max+numRPG"),
        "the hatch must not strip the arm itself: {stdout_n}"
    );
    assert!(
        stderr_n.contains("numfold declined"),
        "the hatch narrates as a decline:\n{stderr_n}"
    );
    let rpg_unfold = int_after(&stderr_n, "label passes,");
    let evals_unfold = int_after(&stderr_n, "rpg evals,");
    assert_eq!(
        evals_fold, evals_unfold,
        "p04's fold bound is exact — the search must be identical"
    );
    assert!(
        rpg_unfold >= evals_unfold / 2,
        "unfolded, the RPG is paid per eval: {rpg_unfold} of {evals_unfold}"
    );
}
