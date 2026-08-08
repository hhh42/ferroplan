//! The optimal ladder spends the whole wall (0.21 Phase 4,
//! docs/roadmap-0.21.md): under an armed `FF_TIME_LIMIT` the h^max
//! sprint is TIME-boxed and a ROOT GATE decides whether LM-cut earns
//! the remainder at all; with no armed wall the ladder is bit-identical
//! to the 0.20 node-split.
//!
//! 0.22 Phase 3 grows the battery three ways (docs/roadmap-0.22.md):
//! the gate becomes MARGIN-shaped (`FF_OPT_GATE_MARGIN`, default 1.4 —
//! thin ratios b-flip to h^max; margin 1.0 restores the 0.21 binary
//! gate), the sprint slice is margin-SCALED (ratio ≥ 2 reads
//! `FF_OPT_SPRINT_FRAC_HI`, default 0.1 — narrated as "0.1-class"), and
//! the sprint's A* state SURVIVES the handover: LM-cut runs a bounded
//! probe (`FF_OPT_LMCUT_PROBE_FRAC`) and on its failure h^max RESUMES
//! its own open list with the leftover wall (`FF_OPT_NO_RESUME=1`
//! restores the throw-away handover).
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

/// The ratio dial: an `n`-step serial chain (h^max root = n) plus `m`
/// independent flips (LM-cut root = n + m ⇒ ratio (n+m)/n), with the
/// gatecheck junk chain (`junk` facts) inflating every heuristic eval so
/// wall slices are actually FELT. n=3/m=1 is the city-car shape (ratio
/// 1.33 — thin margin, must b-flip); n=6/m=6 is the lost-h^max-cert
/// shape (ratio 2.0 — inside the true-c band, only resume saves it).
fn chain_plus_flips(n: usize, m: usize, junk: usize) -> (String, String) {
    let mut preds = String::new();
    let mut acts = String::new();
    let mut goal = String::new();
    for i in 0..=n {
        preds.push_str(&format!(" (p{i})"));
    }
    for i in 1..=n {
        acts.push_str(&format!(
            "(:action s{i} :parameters () :precondition (p{}) :effect (p{i}))\n",
            i - 1
        ));
    }
    goal.push_str(&format!(" (p{n})"));
    for i in 0..m {
        preds.push_str(&format!(" (on-{i})"));
        acts.push_str(&format!(
            "(:action flip-{i} :parameters () :precondition (and) :effect (on-{i}))\n"
        ));
        goal.push_str(&format!(" (on-{i})"));
    }
    let mut init = "(p0)".to_string();
    if junk > 0 {
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
        init.push_str(" (free)");
    }
    (
        format!("(define (domain ratio) (:predicates{preds}) {acts})"),
        format!("(define (problem r) (:domain ratio) (:init {init}) (:goal (and{goal})))"),
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
        "default" | "gate-b" | "margin-b" => {}
        "no-sprint" => {
            cmd.env("FF_NO_HMAX_SPRINT", "1");
        }
        "no-lmcut" => {
            cmd.env("FF_NO_LMCUT", "1");
        }
        "no-rootgate" => {
            cmd.env("FF_OPT_NO_ROOTGATE", "1");
        }
        "margin-1" => {
            cmd.env("FF_OPT_GATE_MARGIN", "1.0");
        }
        "resume" => {
            // 30 s, not 10 (0.22 Phase 9 pre-flight): the fractions are
            // proportional to the wall, so the forced-tiny-slice shape
            // is unchanged (sprint 0.15 s, probe ~30 ms of the ~29.85 s
            // remaining) — only the RESUMED search's absolute margin
            // grows. Isolated, the resume completes deterministically
            // in well under 1 s of real work (2523 expansions, cost 13,
            // every time) — but this test always runs immediately after
            // the `--include-ignored` release pass in the real
            // pre-flight sequence, and this machine's own background
            // noise is on the record elsewhere (cut22-sweeps.sh: "a
            // browser, Spotlight, a Docker VM running CI, Time
            // Machine"). At 10 s that was enough to flip
            // CHILD-resume-SOLVED intermittently — the worst kind of
            // failure (RELEASING.md), caught twice by this exact
            // pre-flight and never once in 20+ isolated reruns.
            cmd.env("FF_TIME_LIMIT", "30")
                .env("FF_OPT_SPRINT_FRAC_HI", "0.005")
                .env("FF_OPT_LMCUT_PROBE_FRAC", "0.001");
        }
        "no-resume" => {
            cmd.env("FF_TIME_LIMIT", "30")
                .env("FF_OPT_SPRINT_FRAC_HI", "0.005")
                .env("FF_OPT_NO_RESUME", "1");
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
            "margin-b" | "margin-1" => chain_plus_flips(3, 1, 0),
            "resume" | "no-resume" => chain_plus_flips(6, 7, 3000),
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

    // Default ladder, armed 5 s wall: the root gate sees LM-cut 18 vs
    // h^max 1 (ratio 18 ≥ 2 ⇒ the 0.1-class sprint slice), the sprint
    // trips fast, and LM-cut certifies inside its probe — inside the
    // wall (0.21's node-split sprint flooded ~14 s past it).
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
    assert!(
        stderr.contains("0.1-class"),
        "ratio 18 must pick the margin-scaled sprint slice:\n{stderr}"
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
    // budget holds the wall (not the 0.4 sprint slice — the lower bound
    // discriminates against 2.0 s), then returns the honest inconclusive
    // instead of flooding ~14 s past the limit. Since 0.22 Phase 2 the
    // deadline carries a TEARDOWN RESERVE (stored-bytes / 4e8 s, the
    // measured arena-drop rate), so the trip lands a beat BEFORE the
    // wall and the process still EXITS by it — the upper bound is the
    // honesty assertion, the lower one the slice discrimination.
    let (stdout, _, secs) = run_child("no-lmcut");
    assert!(stdout.contains("CHILD-no-lmcut-SOLVED:false"), "{stdout}");
    assert!(stdout.contains("inconclusive"), "{stdout}");
    assert!(
        (2.4..15.0).contains(&secs),
        "h^max must hold the wall past any sprint slice, then stop: {secs:.1} s"
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

    // The margin b-flip (0.22 Phase 3 lever 1): LM-cut 4 vs h^max 3 —
    // strictly greater, so the 0.21 binary gate went c-branch — but the
    // ratio 1.33 sits in the city-car band (its six lost v0.19 proofs
    // gated at 1.09–1.36 while every probed TRUE-c domain reads
    // 2.2–6.0), so the margin gate hands h^max the whole wall.
    let (stdout, stderr, _) = run_child("margin-b");
    assert!(stdout.contains("CHILD-margin-b-SOLVED:true"), "{stdout}");
    assert!(stdout.contains("h^max"), "{stdout}");
    assert!(
        stderr.contains("h^max holds the wall"),
        "ratio 1.33 < margin 1.4 must b-flip:\n{stderr}"
    );

    // FF_OPT_GATE_MARGIN=1.0 restores the 0.21 binary gate exactly: the
    // same 1.33-ratio fixture goes c-branch again (and, below 2, keeps
    // the plain 0.4 sprint slice — no "0.1-class" narration).
    let (stdout, stderr, _) = run_child("margin-1");
    assert!(stdout.contains("CHILD-margin-1-SOLVED:true"), "{stdout}");
    assert!(
        stderr.contains("LM-cut earns the remainder"),
        "margin 1.0 must restore lc > hm:\n{stderr}"
    );
    assert!(
        !stderr.contains("0.1-class"),
        "ratio 1.33 < 2 must keep the plain sprint slice:\n{stderr}"
    );

    // Sprint-resume (0.22 Phase 3 lever 3): ratio 13/6 ≈ 2.17 — INSIDE
    // the true-c band, the shape of the ten h^max certificates the 0.21
    // slice killed (root ratios 2.59–3.5). The sprint trips at its
    // (tiny, knob-forced) slice holding 3000-junk-inflated state, the
    // LM-cut probe trips at its (tiny, knob-forced) bound, and h^max
    // RESUMES its own open list and certifies cost 13 with the leftover
    // wall — the seconds the 0.21 ladder threw away at handover.
    let (stdout, stderr, _) = run_child("resume");
    assert!(stdout.contains("CHILD-resume-SOLVED:true"), "{stdout}");
    assert!(
        stderr.contains("h^max resumes its open list"),
        "the probe handover must narrate:\n{stderr}"
    );
    assert!(
        stdout.contains("cost 13") && stdout.contains("h^max"),
        "the RESUMED sprint must carry the certificate: {stdout}"
    );

    // FF_OPT_NO_RESUME restores the throw-away handover: no resume
    // narration on the same fixture (LM-cut keeps the whole remainder;
    // whether it finishes is a timing question the hatch pin does not
    // ride on).
    let (_, stderr, _) = run_child("no-resume");
    assert!(
        !stderr.contains("resumes its open list"),
        "the hatch must silence the resume machinery:\n{stderr}"
    );
}
