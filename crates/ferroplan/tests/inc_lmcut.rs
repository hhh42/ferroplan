//! Incremental LM-cut's child battery (0.23 Phase 5,
//! docs/roadmap-0.23.md): the ladder's LM-cut probe persists BOTH ways
//! — per-eval (cut lists inherited parent→child, so each evaluation
//! pays only its top-up label passes) and per-round (a failed bounded
//! probe RESUMES at round 2 with the raised node budget once h^max's
//! resumed slice node-caps with wall standing, instead of forfeiting
//! round 1's work — symmetric to what sprint-resume did for h^max at
//! 0.22 Phase 3).
//!
//! `FF_*` are process-global env knobs, so each scenario runs in a
//! CHILD process (the tests/refill.rs convention), and the pins ride
//! the FF_WALL_DEBUG narration counters:
//!   "inc-lmcut probe resumes round 2 (N label passes banked, ...)"
//!   "opt cert diagnostics: L label passes, R rpg evals, E evaluated"
//!
//! The fixture is the opt_wall.rs gatecheck shape re-tuned: 18
//! independent flips (h^max root 1, LM-cut root 18 — ratio 18 picks the
//! 0.1-class sprint) plus a 1500-fact junk chain that inflates every
//! label pass, under a 300-node cap (`FF_SEARCH_NODE_CAP`) that makes
//! every trip a DETERMINISTIC node-cap: the probe (cap/2 = 150 stored)
//! parks mid-flight, the resumed h^max (300, refilled 600) caps far
//! short of its 2^18 space with the wall still standing — the round-2
//! entry condition by construction, no timing races (the first draft
//! starved the probe with a 30 ms clock slice and incremental
//! evaluation promptly certified INSIDE it — recorded, re-shaped).

use std::process::Command;

fn gatecheck(n: usize, junk: usize) -> (String, String) {
    let mut preds = String::new();
    let mut acts = String::new();
    let mut goal = String::new();
    for i in 0..n {
        preds.push_str(&format!(" (on-{i})"));
        acts.push_str(&format!(
            "(:action flip-{i} :parameters () :precondition (and) :effect (on-{i}))\n"
        ));
        goal.push_str(&format!(" (on-{i})"));
    }
    preds.push_str(" (free) (mua) (mub)");
    acts.push_str(
        "(:action mk-a :parameters () :precondition (free) :effect (and (mua) (not (free))))\n\
         (:action mk-b :parameters () :precondition (free) :effect (and (mub) (not (free))))\n\
         (:action chain-0 :parameters () :precondition (and (mua) (mub)) :effect (jf-0))\n",
    );
    for j in 0..junk {
        preds.push_str(&format!(" (jf-{j})"));
        if j > 0 {
            acts.push_str(&format!(
                "(:action chain-{j} :parameters () :precondition (jf-{}) :effect (jf-{j}))\n",
                j - 1
            ));
        }
    }
    (
        format!("(define (domain gatecheck) (:predicates{preds}) {acts})"),
        format!("(define (problem g) (:domain gatecheck) (:init (free)) (:goal (and{goal})))"),
    )
}

fn run_child(scenario: &str) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "inc_lmcut_probe_resumes_round_two",
        "--nocapture",
    ])
    .env("INC_LMCUT_CHILD", scenario)
    .env("FF_TIME_LIMIT", "30")
    .env("FF_WALL_DEBUG", "1");
    match scenario {
        // The round-1/round-2 shape: sprint (cap 75) and probe (cap
        // 150) node-cap deterministically, h^max resume + refill cap at
        // 300/600 stored, round 2 (cap 600 ≥ the proof's ~210 stored)
        // certifies on the banked state.
        "inc" => {
            cmd.env("FF_SEARCH_NODE_CAP", "300");
        }
        // The hatch restores the 0.22 one-shot probe: no round 2, and
        // the probe's work is forfeited — the near-miss RED shape.
        "no-inc" => {
            cmd.env("FF_SEARCH_NODE_CAP", "300")
                .env("FF_NO_INC_LMCUT", "1");
        }
        // From-zero comparator: the pure LM-cut rung (no inc — it only
        // arms on the probe engine), room to certify in one pass.
        "scratch" => {
            cmd.env("FF_SEARCH_NODE_CAP", "1200")
                .env("FF_NO_HMAX_SPRINT", "1");
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

/// First integer after `pat` in `hay`.
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
fn inc_lmcut_probe_resumes_round_two() {
    if let Ok(scenario) = std::env::var("INC_LMCUT_CHILD") {
        let (dom, prb) = gatecheck(18, 1500);
        let opts = ferroplan::Options {
            mode: ferroplan::Mode::Optimal,
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
        println!(
            "CHILD-LENGTH:{}",
            sol.plan.as_ref().map(|p| p.length).unwrap_or(0)
        );
        println!("CHILD-NOTE:{}", sol.notes.join(" | "));
        return;
    }

    // GREEN — the probe fails round 1, h^max's resumed slice (and its
    // in-place refill) node-cap, and the RESUMED round 2 certifies on
    // the probe's banked state.
    let (stdout, stderr) = run_child("inc");
    assert!(
        stdout.contains("CHILD-inc-SOLVED:true"),
        "{stdout}\n{stderr}"
    );
    assert!(stdout.contains("CHILD-LENGTH:18"), "{stdout}");
    assert!(
        stdout.contains("LM-cut"),
        "round 2 must carry the certificate: {stdout}"
    );
    assert!(
        stderr.contains("h^max resumes its open list"),
        "round 1 must fail into the resume first:\n{stderr}"
    );
    assert!(
        stderr.contains("inc-lmcut probe resumes round 2"),
        "the round-2 narration is the fixture's subject:\n{stderr}"
    );
    let banked = int_after(&stderr, "round 2 (");
    assert!(
        banked > 0,
        "round 1's label passes must carry over: {banked}"
    );
    let labels_inc = int_after(&stderr, "cert diagnostics:");
    let evals_inc = int_after(&stderr, "rpg evals,");
    assert!(
        labels_inc > banked,
        "round 2 did work of its own: {labels_inc} vs banked {banked}"
    );
    let round2 = labels_inc - banked;

    // From-zero comparator: same certificate, label passes paid in full
    // (~one Dijkstra per landmark round per eval vs the incremental
    // top-up) — the resumed round 2 must be cheaper than ANY from-zero
    // certification, and the per-eval label rate must drop hard (the
    // 2–6× round economics, pinned at ≥3× on this synthetic).
    let (stdout_s, stderr_s) = run_child("scratch");
    assert!(stdout_s.contains("CHILD-scratch-SOLVED:true"), "{stdout_s}");
    assert!(stdout_s.contains("CHILD-LENGTH:18"), "{stdout_s}");
    let labels_scratch = int_after(&stderr_s, "cert diagnostics:");
    let evals_scratch = int_after(&stderr_s, "rpg evals,");
    assert!(
        round2 < labels_scratch,
        "the resumed round 2 ({round2}) must recompute fewer labels than \
         from-zero certification ({labels_scratch})"
    );
    let rate_inc = labels_inc as f64 / evals_inc as f64;
    let rate_scratch = labels_scratch as f64 / evals_scratch as f64;
    assert!(
        rate_inc * 3.0 <= rate_scratch,
        "per-eval label passes must drop ≥3×: inc {rate_inc:.2} vs scratch {rate_scratch:.2}"
    );

    // RED via hatch — FF_NO_INC_LMCUT restores the 0.22 one-shot probe:
    // no round 2, the probe's work forfeited, the near-miss stays a
    // miss on this budget.
    let (stdout_n, stderr_n) = run_child("no-inc");
    assert!(
        !stderr_n.contains("resumes round 2"),
        "the hatch must silence the round-2 machinery:\n{stderr_n}"
    );
    assert!(
        stdout_n.contains("CHILD-no-inc-SOLVED:false"),
        "without resumption this budget is a 0.22-shaped near-miss: {stdout_n}"
    );
}
