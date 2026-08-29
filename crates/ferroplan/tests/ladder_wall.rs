//! The ladder tax (0.21 Phase 5, docs/roadmap-0.21.md): under an armed
//! `FF_TIME_LIMIT`, every bounded rung's budget is denominated in the
//! currency the board charges — WALL. EHC's op-scaled eval budget gains
//! a deadline at `FF_EHC_WALL_FRAC` (default 0.25) of the remaining
//! wall (`FF_NO_EHC_WALLCAP=1` restores op-scaled-only), and
//! novelty-light's unconditional 300k-pop cap gains one at
//! `FF_NOVLIGHT_WALL_FRAC` (default 0.10), checked every 4096 pops.
//! With no armed wall the ladder is byte-identical.
//!
//! 0.22 Phase 2 + 5A (docs/roadmap-0.22.md) finish the lesson: LAMA
//! gains a slice at `FF_LAMA_WALL_FRAC` (0.25), the h-guided novelty
//! rung one at `FF_NOV_WALL_FRAC` (0.30 then; 0.50 since 0.26 F3, the
//! transport decode), affordability is re-read at
//! every rung entry, the best-first fallback checks the clock at its
//! batch boundary, and GROUNDING's binding enumeration checks it too
//! (trip ⇒ an honest no-verdict failure, never a partial task).
//! `FF_NO_RUNG_WALLCAP=1` hatches all of it, joining FF_NO_EHC_WALLCAP
//! — the `no-rung-wallcap` and `ground-wall-hatched` legs keep the RED
//! overrun shapes on the record permanently.
//!
//! One sequential test on purpose: the wall clock is a process-global
//! OnceLock and FF_* are process-global env knobs, so each scenario runs
//! in a CHILD process (the tests/refill.rs convention).
//!
//! The openstacks-shaped fixture (`laddertax`): goals (ga) and (gb) each
//! consume the (one) token, and the token refills only after an L-step
//! chain walk. EHC's first bfs_improve walks BOTH goal branches
//! single-file (~2L expansions) and a mid-chain hFF eval builds ~(L−i)
//! RPG layers over ~L ops — an L³ wall cost (calibrated solo on the idle
//! box: L=1700 → 4.2 s, L=1900 → 6.1 s; ~2.5–3× that when `nice -n 15`
//! lands the child on Efficiency cores) under an op-scaled budget that
//! never trips (~4k evals ≪ the 30k floor) — while novelty-light
//! dispatches the same task in ~L cheap pops (calibrated: 1904 pops,
//! 0.02 s — below the 4096-pop wall-check cadence, so the light leg here
//! is scheduling-noise-proof). L must stay ≤ ~1900: by 2000 the first
//! lookahead crosses BFS_CAP and EHC bails into LAMA on its own, which
//! this battery would catch loudly (the fell-back note appears in the
//! hatched leg). Today EHC-direct eats the whole armed wall and solves
//! at ~2× the budget; the wall-capped ladder hands down at 0.25× and
//! the light rung finishes inside it.

use std::process::Command;
use std::time::Instant;

/// The tetris shape, distilled (0.22 Phase 5A a1): a DECEPTIVE LAWN the
/// relaxed heuristic loves and a NOVEL CORRIDOR it prices at h≈m. The
/// fake goal route (`winop`, behind the opt_wall.rs mutex trick: mk-a
/// and mk-b both consume the one `fr` token) keeps every lawn state at
/// h ≤ 3+k while `enter` DELETES `fr`, so the real corridor opens at
/// h = m+1 — LAMA grinds the 2^k lawn to its 400k-eval cap without ever
/// popping the corridor, while the novelty rung walks it structurally
/// (every c_i is a never-seen fact) in ~m rounds. `junk` zero-effect-
/// variety actions (all add the same `jf`, preconditioned on the mutex
/// pair so they stay relaxed-reachable but never applicable) price each
/// heuristic evaluation like a medium task without growing the fact
/// space.
fn deceptlawn(k: usize, m: usize, junk: usize) -> (String, String) {
    let mut preds = String::from(" (win) (fr) (mua) (mub) (jf)");
    for i in 0..k {
        preds.push_str(&format!(" (on{i}) (off{i})"));
    }
    for i in 0..=m {
        preds.push_str(&format!(" (c{i})"));
    }
    let mut acts = String::new();
    for i in 0..k {
        acts.push_str(&format!(
            "(:action set{i} :parameters () :precondition (off{i}) :effect (and (on{i}) (not (off{i}))))\n\
             (:action unset{i} :parameters () :precondition (on{i}) :effect (and (off{i}) (not (on{i}))))\n"
        ));
    }
    let allon: String = (0..k).map(|i| format!(" (on{i})")).collect();
    acts.push_str(&format!(
        "(:action winop :parameters () :precondition (and (mua) (mub){allon}) :effect (win))\n\
         (:action mk-a :parameters () :precondition (fr) :effect (and (mua) (not (fr))))\n\
         (:action mk-b :parameters () :precondition (fr) :effect (and (mub) (not (fr))))\n\
         (:action enter :parameters () :precondition (fr) :effect (and (c0) (not (fr))))\n\
         (:action realwin :parameters () :precondition (c{m}) :effect (win))\n"
    ));
    for i in 0..m {
        acts.push_str(&format!(
            "(:action walk{i} :parameters () :precondition (c{i}) :effect (c{})\n)",
            i + 1
        ));
    }
    for j in 0..junk {
        acts.push_str(&format!(
            "(:action junk{j} :parameters () :precondition (and (mua) (mub)) :effect (jf))\n"
        ));
    }
    let offs: String = (0..k).map(|i| format!(" (off{i})")).collect();
    (
        format!("(define (domain deceptlawn) (:predicates{preds}) {acts})"),
        format!("(define (problem dl) (:domain deceptlawn) (:init{offs} (fr)) (:goal (win)))"),
    )
}

/// The 2048 grounding shape, distilled (0.22 Phase 2 lever 1): one
/// 5-parameter action whose only static literal names the FIRST and
/// LAST parameters, so the binding enumeration walks the full n^4
/// prefix product and checks n^5 candidates at the leaf — with zero
/// survivors (the `trip` predicate has no init atoms). The task itself
/// is solved by one trivial action AFTER grounding, so the hatched leg
/// proves the overrun shape: grounding grinds far past the armed wall
/// and then "solves".
fn bindstorm(n: usize) -> (String, String) {
    let objs: String = (0..n).map(|i| format!(" o{i}")).collect();
    (
        "(define (domain bindstorm)
          (:requirements :typing)
          (:types obj)
          (:predicates (trip ?a - obj ?e - obj) (start) (gw))
          (:action heavy :parameters (?a - obj ?b - obj ?c - obj ?d - obj ?e - obj)
            :precondition (trip ?a ?e) :effect (gw))
          (:action easy :parameters () :precondition (start) :effect (gw)))"
            .into(),
        format!(
            "(define (problem bs) (:domain bindstorm) (:objects{objs} - obj) \
             (:init (start)) (:goal (gw)))"
        ),
    )
}

/// The openstacks shape, distilled (see module docs). Zero-ary
/// predicates/actions ground 1:1, so `n_ops ≈ L+3` and grounding stays
/// in the noise.
fn laddertax(l: usize) -> (String, String) {
    let mut preds = String::from(" (ga) (gb) (one)");
    for i in 0..=l {
        preds.push_str(&format!(" (p{i})"));
    }
    let mut acts = String::from(
        "(:action make-ga :parameters () :precondition (one) :effect (and (ga) (not (one))))\n\
         (:action make-gb :parameters () :precondition (one) :effect (and (gb) (not (one))))\n",
    );
    acts.push_str(&format!(
        "(:action refill :parameters () :precondition (p{l}) :effect (and (one) (p0) (not (p{l}))))\n"
    ));
    for i in 0..l {
        acts.push_str(&format!(
            "(:action step{i} :parameters () :precondition (p{i}) :effect (and (p{}) (not (p{i}))))\n",
            i + 1
        ));
    }
    (
        format!("(define (domain laddertax) (:predicates{preds}) {acts})"),
        "(define (problem lt) (:domain laddertax) (:init (one) (p0)) (:goal (and (ga) (gb))))"
            .into(),
    )
}

/// A 13-bit toggle lawn whose only goal fact sits behind an all-bits-on
/// op: goal-count is flat and novelty runs out after ~2K states, so the
/// light rung pays >4096 pops (calibrated: 24548) before it wins —
/// enough to cross the wall-slice check cadence. The `junk` ops are
/// relaxed-reachable but really locked (mua/mub both need the one (fr)
/// token — the tests/opt_wall.rs mutex trick), so they survive grounding
/// and price every pop's op scan without ever being applicable.
fn lightlawn(k: usize, junk: usize) -> (String, String) {
    let mut preds = String::from(" (win) (fr) (mua) (mub)");
    for i in 0..k {
        preds.push_str(&format!(" (on{i}) (off{i})"));
    }
    for j in 0..junk {
        preds.push_str(&format!(" (jf{j})"));
    }
    let mut acts = String::new();
    for i in 0..k {
        acts.push_str(&format!(
            "(:action set{i} :parameters () :precondition (off{i}) :effect (and (on{i}) (not (off{i}))))\n\
             (:action unset{i} :parameters () :precondition (on{i}) :effect (and (off{i}) (not (on{i}))))\n"
        ));
    }
    let allon: String = (0..k).map(|i| format!(" (on{i})")).collect();
    acts.push_str(&format!(
        "(:action winop :parameters () :precondition (and{allon}) :effect (win))\n\
         (:action mk-a :parameters () :precondition (fr) :effect (and (mua) (not (fr))))\n\
         (:action mk-b :parameters () :precondition (fr) :effect (and (mub) (not (fr))))\n"
    ));
    for j in 0..junk {
        acts.push_str(&format!(
            "(:action junk{j} :parameters () :precondition (and (mua) (mub)) :effect (jf{j}))\n"
        ));
    }
    let offs: String = (0..k).map(|i| format!(" (off{i})")).collect();
    (
        format!("(define (domain lightlawn) (:predicates{preds}) {acts})"),
        format!("(define (problem ll) (:domain lightlawn) (:init{offs} (fr)) (:goal (win)))"),
    )
}

fn run_child(scenario: &str) -> (String, String, f64) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "ladder_rungs_pay_the_wall", "--nocapture"])
        .env("LADDER_WALL_CHILD", scenario)
        .env("FF_WALL_DEBUG", "1");
    match scenario {
        "capped" => {
            cmd.env("FF_TIME_LIMIT", "3");
        }
        "no-ehc-wallcap" => {
            cmd.env("FF_TIME_LIMIT", "3").env("FF_NO_EHC_WALLCAP", "1");
        }
        "light-slice" => {
            cmd.env("FF_TIME_LIMIT", "5")
                .env("FF_NOVLIGHT_ONLY", "1")
                .env("FF_NOVLIGHT_WALL_FRAC", "0.001");
        }
        "lama-slice" => {
            // The tetris shape (0.22 Phase 5A a1): light is stood down so
            // the scenario pins the LAMA rung itself — the light rung's
            // own slice legs sit above. EHC bails on its own here (the
            // mutex pair dead-ends its lookahead in milliseconds).
            cmd.env("FF_TIME_LIMIT", "5").env("FF_NO_NOVLIGHT", "1");
        }
        "no-rung-wallcap" => {
            // The permanent RED record (the FF_NO_EHC_WALLCAP pattern):
            // unsliced LAMA grinds its 400k-eval budget straight through
            // the armed wall before the ladder moves on.
            cmd.env("FF_TIME_LIMIT", "5")
                .env("FF_NO_NOVLIGHT", "1")
                .env("FF_NO_RUNG_WALLCAP", "1");
        }
        "ground-wall" => {
            // 0.22 Phase 2 lever 1: grounding must exit HONESTLY at a
            // tiny armed wall — no partial task, no zombie enumeration.
            // Phase 7's MCV join order collapses bindstorm's late-static
            // shape outright (its own battery pins that win), so this
            // leg hatches MCV off: the wall check is the backstop for
            // above-threshold shapes MCV cannot collapse.
            cmd.env("FF_TIME_LIMIT", "1").env("FF_NO_MCV_JOIN", "1");
        }
        "ground-wall-hatched" => {
            // The permanent RED record: unchecked enumeration grinds far
            // past the wall, then "solves" — the 2048 receipt shape.
            cmd.env("FF_TIME_LIMIT", "1")
                .env("FF_NO_RUNG_WALLCAP", "1")
                .env("FF_NO_MCV_JOIN", "1");
        }
        "light-whole-slice" => {
            // A LONG armed wall: the default 0.10 slice must hold the
            // light rung's full calibrated dispatch (~25k pops) with
            // contention headroom — the receipted-wins-survive shape.
            //
            // The wall is PROFILE-SCALED because the slice is denominated in
            // wall time while the dispatch is denominated in pops, and an
            // unoptimised build walks that dispatch ~20x slower. At 40 s the
            // slice is 4 s, and a debug run was measured doing 20,480 pops in
            // 4.40 s — i.e. it fails by 10%, sometimes. That is the worst kind
            // of test: green on a fast box, red on a slow or busy one, and
            // green again on a re-run. Scale the wall with the profile so the
            // ASSERTION (a default slice holds a full dispatch) is what varies
            // with the engine, not with how the test binary was compiled.
            let wall = if cfg!(debug_assertions) { "400" } else { "40" };
            cmd.env("FF_TIME_LIMIT", wall).env("FF_NOVLIGHT_ONLY", "1");
        }
        "no-budget" => {}
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
fn ladder_rungs_pay_the_wall() {
    if let Ok(scenario) = std::env::var("LADDER_WALL_CHILD") {
        let (dom, prb) = match scenario.as_str() {
            "capped" | "no-ehc-wallcap" => laddertax(1900),
            "no-budget" => laddertax(120),
            "light-slice" | "light-whole-slice" => lightlawn(13, 6000),
            // Profile-scaled like light-whole-slice's wall: the RED shape
            // is denominated in evals/binding nodes while the wall is in
            // seconds, and a debug build walks both ~10-20x slower.
            "lama-slice" | "no-rung-wallcap" => {
                if cfg!(debug_assertions) {
                    deceptlawn(14, 40, 200)
                } else {
                    deceptlawn(19, 60, 2000)
                }
            }
            // Unlike lama-slice/no-rung-wallcap this one is NOT
            // profile-scaled down for debug: n=14 (measured 0.22 Phase 9
            // pre-flight) grounds and solves in well under the 1 s wall
            // even with MCV off — n^5 candidate bindings for `heavy` is
            // just not that much work either build profile, so a smaller
            // debug fixture only recreated the false-pass, not a faster
            // one. n=30 reliably overruns on both profiles (checkpoint
            // fires 1.0-1.7 s in, every run).
            "ground-wall" | "ground-wall-hatched" => bindstorm(30),
            other => panic!("unknown scenario {other}"),
        };
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
        println!("CHILD-NOTE:{}", sol.notes.join(" | "));
        return;
    }

    // THE PIN (lever 2 + the ladder floor): armed 3 s wall on the
    // openstacks shape. EHC's slice expires at 0.25 × remaining, the
    // ladder reaches novelty-light with real wall left, and the light
    // rung dispatches — inside the wall. Today: EHC-direct grinds ~6 s
    // solo (~2× the declared budget) before anything else runs. The
    // wall-denominated slices keep this leg's timing scheduling-proof:
    // EHC hands down at ~0.75 s of WALL however slow the cores are.
    let (stdout, stderr, secs) = run_child("capped");
    assert!(stdout.contains("CHILD-capped-SOLVED:true"), "{stdout}");
    assert!(
        stderr.contains("wall: EHC slice exhausted"),
        "EHC must hand down at its wall slice:\n{stderr}"
    );
    assert!(
        stderr.contains("wall: solved by novelty-light"),
        "the light rung must get the dispatch:\n{stderr}"
    );
    if !cfg!(debug_assertions) {
        assert!(secs < 3.0, "blew the 3 s wall: {secs:.1} s");
    }

    // The hatch restores op-scaled-only: EHC-direct solves, but only by
    // blowing through the armed wall — the recorded negative shape this
    // phase repairs (a runner would have killed it unsolved).
    let (stdout, stderr, secs) = run_child("no-ehc-wallcap");
    assert!(
        stdout.contains("CHILD-no-ehc-wallcap-SOLVED:true"),
        "{stdout}"
    );
    assert!(
        !stdout.contains("EHC found no improving state"),
        "hatched run must solve EHC-direct: {stdout}"
    );
    assert!(
        !stderr.contains("wall: EHC slice exhausted"),
        "hatch must disarm the slice:\n{stderr}"
    );
    if !cfg!(debug_assertions) {
        assert!(
            secs > 3.0,
            "op-scaled EHC was expected to blow the 3 s wall (the RED shape): {secs:.1} s"
        );
    }

    // Lever 1: a deliberately tiny slice trips the light rung's wall
    // check at the 4096-pop cadence (the lawn needs ~25k pops), and the
    // ladder still finishes: EHC picks the counter task up in
    // milliseconds after the trip (so the fell-back note stays absent).
    let (stdout, stderr, _) = run_child("light-slice");
    assert!(stdout.contains("CHILD-light-slice-SOLVED:true"), "{stdout}");
    assert!(
        stderr.contains("wall: novelty-light slice exhausted"),
        "light must trip its wall slice:\n{stderr}"
    );
    assert!(
        !stdout.contains("EHC found no improving state"),
        "post-trip solve should be EHC-direct: {stdout}"
    );

    // The default slice keeps every receipted win: the same ~25k-pop
    // dispatch fits 0.10 of a 40 s wall with headroom, so the light
    // probe itself solves (its fell-back note is the witness) and the
    // slice never fires.
    let (stdout, stderr, _) = run_child("light-whole-slice");
    assert!(
        stdout.contains("CHILD-light-whole-slice-SOLVED:true"),
        "{stdout}"
    );
    assert!(
        !stderr.contains("novelty-light slice exhausted"),
        "default slice must hold the full dispatch:\n{stderr}"
    );
    assert!(
        stdout.contains("EHC found no improving state"),
        "the light probe itself must solve it: {stdout}"
    );

    // 0.22 Phase 5A a1 — THE PIN (the tetris shape): armed 5 s wall on
    // the deceptive lawn. LAMA's slice expires at 0.25 x remaining, the
    // ladder reaches the h-guided novelty rung with real wall left, and
    // novelty walks the corridor structurally — inside the wall. RED
    // today: LAMA grinds its 400k-eval budget past the whole wall
    // before novelty ever runs (the tetris i4 board loss to the
    // letter).
    let (stdout, stderr, secs) = run_child("lama-slice");
    assert!(stdout.contains("CHILD-lama-slice-SOLVED:true"), "{stdout}");
    assert!(
        stderr.contains("wall: LAMA slice exhausted"),
        "LAMA must hand down at its wall slice:\n{stderr}"
    );
    assert!(
        stderr.contains("wall: solved by novelty"),
        "the novelty rung must get the dispatch:\n{stderr}"
    );
    if !cfg!(debug_assertions) {
        assert!(secs < 5.0, "blew the 5 s wall: {secs:.1} s");
    }

    // The hatch keeps the RED shape on the record permanently (the
    // FF_NO_EHC_WALLCAP pattern): unsliced LAMA eats its whole eval
    // budget through the armed wall; the solve still lands (novelty,
    // eventually) but only by blowing the budget a runner would have
    // killed.
    let (stdout, stderr, secs) = run_child("no-rung-wallcap");
    assert!(
        stdout.contains("CHILD-no-rung-wallcap-SOLVED:true"),
        "{stdout}"
    );
    assert!(
        !stderr.contains("wall: LAMA slice exhausted"),
        "hatch must disarm the slice:\n{stderr}"
    );
    if !cfg!(debug_assertions) {
        assert!(
            secs > 5.0,
            "unsliced LAMA was expected to blow the 5 s wall (the RED shape): {secs:.1} s"
        );
    }

    // 0.22 Phase 2 lever 1 — grounding checks the clock: a 1 s armed
    // wall on the binding storm must end HONESTLY — solved:false, the
    // no-verdict note, no partial task — instead of enumerating n^5
    // candidates to the end (RED today: the 2048 shape, 67-74 s of a
    // 60 s budget spent pre-search).
    let (stdout, stderr, secs) = run_child("ground-wall");
    assert!(
        stdout.contains("CHILD-ground-wall-SOLVED:false"),
        "{stdout}"
    );
    assert!(
        stdout.contains("grounding stopped at the declared budget")
            && stdout.contains("wall budget exhausted during binding enumeration"),
        "the honest no-verdict note must name the mechanism: {stdout}"
    );
    assert!(
        stderr.contains("wall: grounding checkpoint expired"),
        "grounding trip narration missing:\n{stderr}"
    );
    if !cfg!(debug_assertions) {
        assert!(
            secs < 3.0,
            "grounding must end within a beat of the 1 s wall: {secs:.1} s"
        );
    }

    // The hatched leg keeps the zombie shape on the record: unchecked
    // enumeration grinds past the wall, then solves via the trivial op.
    let (stdout, stderr, secs) = run_child("ground-wall-hatched");
    assert!(
        stdout.contains("CHILD-ground-wall-hatched-SOLVED:true"),
        "{stdout}"
    );
    assert!(
        !stderr.contains("wall: grounding checkpoint expired"),
        "hatch must disarm the grounding check:\n{stderr}"
    );
    if !cfg!(debug_assertions) {
        assert!(
            secs > 1.0,
            "unchecked grounding was expected to blow the 1 s wall (the RED shape): {secs:.1} s"
        );
    }

    // No declared budget: no slice arms anywhere — byte-identical
    // ladder (the 0.20 Phase 1 pattern; every other test in the suite
    // runs wall-less and is the real identity referee).
    let (stdout, stderr, _) = run_child("no-budget");
    assert!(stdout.contains("CHILD-no-budget-SOLVED:true"), "{stdout}");
    assert!(
        !stderr.contains("slice exhausted"),
        "nothing may arm without a budget:\n{stderr}"
    );
}
