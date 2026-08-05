//! The optimal ladder spends the whole wall (0.21 Phase 4,
//! docs/roadmap-0.21.md): under an armed `FF_TIME_LIMIT` the h^max
//! sprint is TIME-boxed and a ROOT GATE decides whether LM-cut earns
//! the remainder at all; with no armed wall the ladder is bit-identical
//! to the 0.20 node-split.
//!
//! One sequential test on purpose: the wall clock is a process-global
//! OnceLock and FF_* are process-global env knobs, so each scenario runs
//! in a CHILD process (the tests/refill.rs convention).
//!
//! The scanalyzer-shaped fixture: 18 independent switches (h^max sees 1
//! everywhere — a uniform flood of ~786k expansions, ~14 s solo) plus a
//! relaxed-reachable-but-really-unreachable 1000-fact junk chain that
//! prices every heuristic evaluation like a medium task, while LM-cut's
//! 18 disjoint landmarks prove in 18 expansions. Today's node-split
//! sprint runs the whole flood past the 5 s wall — the 500-timeout
//! shape; the gated time-boxed ladder certifies inside it.

use std::process::Command;
use std::time::Instant;

/// 18 independent unit goals (LM-cut root 18, h^max root 1) + the junk
/// chain. mua/mub are relaxed-reachable but really mutex (mk-a and mk-b
/// both consume free), so the chain inflates every Dijkstra without
/// touching the reachable state space.
fn gatecheck() -> (String, String) {
    const N: usize = 18;
    const M: usize = 1000;
    let mut preds = String::new();
    let mut acts = String::new();
    let mut goal = String::new();
    for i in 0..N {
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
    for j in 0..M {
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

/// A serial chain: h^max root == LM-cut root (no landmark structure
/// beyond the critical path) — the city-car/genome class the root gate
/// must hand the whole wall to h^max on.
fn chain() -> (String, String) {
    (
        "(define (domain chain3)
          (:predicates (p0) (p1) (p2) (p3))
          (:action s1 :parameters () :precondition (p0) :effect (p1))
          (:action s2 :parameters () :precondition (p1) :effect (p2))
          (:action s3 :parameters () :precondition (p2) :effect (p3)))"
            .into(),
        "(define (problem c) (:domain chain3) (:init (p0)) (:goal (p3)))".into(),
    )
}

fn run_child(scenario: &str) -> (String, String, f64) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "opt_ladder_spends_the_wall", "--nocapture"])
        .env("OPT_WALL_CHILD", scenario)
        .env("FF_TIME_LIMIT", "5")
        .env("FF_WALL_DEBUG", "1");
    match scenario {
        "default" | "gate-b" => {}
        "no-sprint" => {
            cmd.env("FF_NO_HMAX_SPRINT", "1");
        }
        "no-lmcut" => {
            cmd.env("FF_NO_LMCUT", "1");
        }
        "no-rootgate" => {
            cmd.env("FF_OPT_NO_ROOTGATE", "1");
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

fn note_expansions(stdout: &str) -> usize {
    let head = stdout
        .split(" expansions)")
        .next()
        .unwrap_or_else(|| panic!("no expansions in {stdout}"));
    let digits: String = head
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits
        .chars()
        .rev()
        .collect::<String>()
        .parse()
        .unwrap_or_else(|_| panic!("no expansion count in {stdout}"))
}

#[test]
fn opt_ladder_spends_the_wall() {
    if let Ok(scenario) = std::env::var("OPT_WALL_CHILD") {
        let (dom, prb) = match scenario.as_str() {
            "gate-b" | "no-rootgate" => chain(),
            _ => gatecheck(),
        };
        let opts = ferroplan::Options {
            mode: ferroplan::Mode::Optimal,
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
        println!("CHILD-NOTE:{}", sol.notes.join(" | "));
        return;
    }

    // Default ladder, armed 5 s wall: the root gate sees LM-cut 18 >
    // h^max 1, the time-boxed sprint trips at 0.4 x wall, and LM-cut
    // certifies with the remainder — inside the wall (today: the
    // node-split sprint floods ~14 s past it and h^max gets the credit).
    let (stdout, stderr, secs) = run_child("default");
    assert!(stdout.contains("CHILD-default-SOLVED:true"), "{stdout}");
    assert!(
        stdout.contains("LM-cut"),
        "the certificate must name LM-cut, not the starved sprint's h^max: {stdout}"
    );
    assert!(
        stderr.contains("LM-cut earns the remainder"),
        "gate verdict missing from stderr:\n{stderr}"
    );
    if !cfg!(debug_assertions) {
        assert!(secs < 5.0, "blew the 5 s wall: {secs:.1} s");
    }

    // FF_NO_HMAX_SPRINT keeps its pure-rung meaning: LM-cut only, no
    // sprint expansions folded in (the gate must not resurrect it).
    let (stdout, _, _) = run_child("no-sprint");
    assert!(stdout.contains("CHILD-no-sprint-SOLVED:true"), "{stdout}");
    assert!(stdout.contains("LM-cut"), "{stdout}");
    let exp = note_expansions(&stdout);
    assert!(
        exp < 1000,
        "sprint expansions leaked into pure LM-cut: {exp}"
    );

    // FF_NO_LMCUT keeps its pure-rung meaning: h^max with the FULL node
    // budget holds the FULL wall (not the sprint slice), then returns
    // the honest inconclusive instead of flooding ~14 s past the limit.
    let (stdout, _, secs) = run_child("no-lmcut");
    assert!(stdout.contains("CHILD-no-lmcut-SOLVED:false"), "{stdout}");
    assert!(stdout.contains("inconclusive"), "{stdout}");
    assert!(
        (4.5..15.0).contains(&secs),
        "h^max must hold the whole 5 s wall, then stop: {secs:.1} s"
    );

    // The gate's b-branch: LM-cut root == h^max root on a serial chain,
    // so h^max keeps the full budget and the wall — the 2014-opt class
    // the unconditional sprint split was starving.
    let (stdout, stderr, _) = run_child("gate-b");
    assert!(stdout.contains("CHILD-gate-b-SOLVED:true"), "{stdout}");
    assert!(stdout.contains("h^max"), "{stdout}");
    assert!(
        stderr.contains("h^max holds the wall"),
        "gate verdict missing from stderr:\n{stderr}"
    );

    // FF_OPT_NO_ROOTGATE restores the unconditional ladder: no gate
    // verdict, the sprint runs (and proves the tiny chain itself).
    let (stdout, stderr, _) = run_child("no-rootgate");
    assert!(stdout.contains("CHILD-no-rootgate-SOLVED:true"), "{stdout}");
    assert!(stdout.contains("h^max"), "{stdout}");
    assert!(
        !stderr.contains("opt root gate"),
        "hatch must silence the gate:\n{stderr}"
    );
}
