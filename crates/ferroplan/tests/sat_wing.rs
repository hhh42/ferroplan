//! The SAT wing (0.24 Phases 2+3): the ∃-step bounded-layer encoder, the
//! horizon ramp, `Mode::Sat`, and the temporal face (pairing clauses +
//! STN-taught CEGAR). Fixtures first — the required-concurrency micro-task
//! below was run RED against the pre-wing engine (auto AND temporal modes
//! both fail it; the decision-epoch search can only start actions at
//! epochs derived from other happenings' end times, and this task needs a
//! start strictly inside an open interval no epoch chain reaches), and
//! goes GREEN only via SAT+STN.

use ferroplan::{solve, Mode, Options};

/// The load-bearing required-concurrency micro-task (the match-cellar
/// SHAPE, sharpened until decision epochs provably miss):
///
/// - `shine` (dur 20) provides `(light)` only DURING its run — the
///   detector's fire-kiln/match-cellar envelope shape;
/// - `mend` (dur 9) needs `(light)` over-all — solvable by decision
///   epochs alone (start both at t≈0);
/// - `door` (dur 2) provides `(open)` only during its run;
/// - `deliver` (dur 6) needs `(open)` AT END — so `door` must start in
///   the open interval `(s_deliver+4, s_deliver+6)`. Every action is
///   single-use, so the reachable decision epochs are exactly the chain
///   sums of {2, 9, 20}: {0, 2, 9, 11, 20, 22, 29, 31}, whose pairwise
///   differences ({2, 7, 9, 11, 13, 18, 20, 22, ...}) never land in
///   (4, 6). The decision-epoch search — every tier of its ladder — is
///   structurally unable to schedule it; a layered causal encoding with
///   STN scheduling does it without noticing.
const RC_DOMAIN: &str = "
(define (domain sat-rc)
  (:requirements :strips :durative-actions)
  (:predicates (light) (open) (fresh-shine) (fresh-mend) (fresh-deliver)
               (fresh-door) (mended) (delivered))
  (:durative-action shine
    :parameters ()
    :duration (= ?duration 20)
    :condition (at start (fresh-shine))
    :effect (and (at start (not (fresh-shine)))
                 (at start (light))
                 (at end (not (light)))))
  (:durative-action mend
    :parameters ()
    :duration (= ?duration 9)
    :condition (and (at start (fresh-mend)) (over all (light)))
    :effect (and (at start (not (fresh-mend))) (at end (mended))))
  (:durative-action deliver
    :parameters ()
    :duration (= ?duration 6)
    :condition (and (at start (fresh-deliver)) (at end (open)))
    :effect (and (at start (not (fresh-deliver))) (at end (delivered))))
  (:durative-action door
    :parameters ()
    :duration (= ?duration 2)
    :condition (at start (fresh-door))
    :effect (and (at start (not (fresh-door)))
                 (at start (open))
                 (at end (not (open))))))
";

const RC_PROBLEM: &str = "
(define (problem sat-rc-1) (:domain sat-rc)
  (:init (fresh-shine) (fresh-mend) (fresh-deliver) (fresh-door))
  (:goal (and (mended) (delivered))))
";

/// The load-bearing fixture, both halves in ONE test fn (the RED half
/// sets `FF_NO_SAT`, and integration tests share a process — the var must
/// not leak into a concurrently running auto-mode solve).
///
/// RED half, pinned FOREVER: with the SAT rung disarmed (`FF_NO_SAT`,
/// the byte-identity restore), every existing mode fails the
/// required-concurrency micro-task. Run against the PRE-wing engine
/// (commit `f661a8d`) this passed identically — no rung existed to
/// disarm; that run is the recorded RED.
///
/// GREEN half: with the rung armed, plain `auto` solves it — the
/// detector promotes SAT early on the temporal ladder, the ∃-step
/// encoding + STN scheduling find the door slot no decision epoch
/// reaches, and the plan validates against the ORIGINAL problem. The
/// wing adds EXPRESSIVENESS, not speed.
#[test]
fn required_concurrency_red_without_sat_green_via_auto() {
    std::env::set_var("FF_NO_SAT", "1");
    for mode in [Mode::Auto, Mode::Temporal] {
        let opts = Options {
            mode,
            ..Default::default()
        };
        let sol = solve(RC_DOMAIN, RC_PROBLEM, &opts).expect("solve runs");
        assert!(
            !sol.solved,
            "{mode:?} mode must NOT solve the required-concurrency micro-task \
             (decision epochs cannot reach a start inside (s+4, s+6)) — if this \
             fires, the fixture no longer proves the wing adds expressiveness"
        );
    }
    std::env::remove_var("FF_NO_SAT");

    let sol = solve(RC_DOMAIN, RC_PROBLEM, &Options::default()).expect("solve runs");
    assert!(
        sol.solved,
        "auto must solve the micro-task once the SAT rung is armed (promoted \
         early by the required-concurrency detector); notes: {:?}",
        sol.notes
    );
    let plan = sol.plan.expect("plan present");
    assert!(plan.makespan.is_some(), "temporal plan carries a makespan");
    // Independent oracle: replay through the plan validator (which folds
    // the ORIGINAL PDDL semantics, not the wing's encoding).
    let ipc = render_ipc(&plan);
    match ferroplan::plan::validate_plan(RC_DOMAIN, RC_PROBLEM, &ipc) {
        Ok(ferroplan::plan::Validity::Valid) => {}
        other => panic!("SAT plan failed the internal oracle: {other:?}\nplan:\n{ipc}"),
    }
}

/// GREEN, explicit `Mode::Sat` (which `FF_NO_SAT` — a ROUTER restore —
/// never touches): solves the micro-task, validates, and the decoded
/// schedule passes the ε-cap pin: every dispatch instant sits on the ε
/// grid and no two steps coincide (the emission machinery's separation
/// survived the SAT route).
#[test]
fn required_concurrency_green_via_mode_sat_with_eps_pin() {
    let opts = Options {
        mode: Mode::Sat,
        ..Default::default()
    };
    let sol = solve(RC_DOMAIN, RC_PROBLEM, &opts).expect("solve runs");
    assert!(
        sol.solved,
        "Mode::Sat solves the micro-task: {:?}",
        sol.notes
    );
    assert_eq!(sol.mode, Mode::Sat);
    assert!(
        sol.notes.iter().any(|n| n.contains("validated")),
        "the temporal face notes its oracle pass: {:?}",
        sol.notes
    );
    let plan = sol.plan.expect("plan");
    // ε-cap pin: pairwise-distinct dispatch times, ε-grid aligned.
    let mut times: Vec<f64> = plan.steps.iter().filter_map(|s| s.time).collect();
    assert_eq!(times.len(), plan.steps.len(), "every step carries a time");
    times.sort_by(f64::total_cmp);
    for w in times.windows(2) {
        assert!(
            w[1] - w[0] >= 0.0005,
            "two dispatches closer than ε/2: {times:?}"
        );
    }
    for t in &times {
        let slots = t / 0.001;
        assert!(
            (slots - slots.round()).abs() < 1e-6,
            "dispatch off the ε grid: {t}"
        );
    }
    let ipc = render_ipc(&plan);
    match ferroplan::plan::validate_plan(RC_DOMAIN, RC_PROBLEM, &ipc) {
        Ok(ferroplan::plan::Validity::Valid) => {}
        other => panic!("SAT plan failed the internal oracle: {other:?}\nplan:\n{ipc}"),
    }
    // VAL, when the referee is on this machine (the benchmark rig sets
    // FERROPLAN_VAL; CI without it skips this half, the oracle above ran).
    val_check(RC_DOMAIN, RC_PROBLEM, &ipc, true);
}

/// The required-concurrency detector: fires on the fire-kiln/match-cellar
/// envelope shape, stays quiet on a plain temporal domain (the 486-row
/// protection is structural — no promotion, and the exhaustion rung only
/// arms under a wall with time remaining).
#[test]
fn detector_fires_on_envelope_shape_only() {
    let d = ferroplan::parser::parse_domain(RC_DOMAIN).unwrap();
    let p = ferroplan::parser::parse_problem(RC_PROBLEM).unwrap();
    assert!(ferroplan::sat::requires_concurrency(&d, &p));

    let plain_dom = "
    (define (domain plain-t)
      (:requirements :strips :durative-actions)
      (:predicates (a) (b))
      (:durative-action go
        :parameters ()
        :duration (= ?duration 3)
        :condition (at start (a))
        :effect (at end (b))))";
    let plain_prb = "(define (problem p) (:domain plain-t) (:init (a)) (:goal (b)))";
    let d = ferroplan::parser::parse_domain(plain_dom).unwrap();
    let p = ferroplan::parser::parse_problem(plain_prb).unwrap();
    assert!(!ferroplan::sat::requires_concurrency(&d, &p));
}

/// The bounded-bet seam (the 0.24 cut regression: promoted SAT ground
/// match-cellar's h32 conflict budget with ZERO STN refutations — the
/// thrash bail never fired — and ate the whole wall, so the ladder that
/// solves those rows in 0.02 s was refused at pass entry). The promoted
/// router entry now hands the wing its own wall slice; a spent slice
/// must decline before the first horizon with the honest note, and the
/// `None` slice must stay byte-identical to [`ferroplan::sat::solve_temporal`].
#[test]
fn spent_promo_slice_declines_before_the_first_horizon() {
    let d = ferroplan::parser::parse_domain(RC_DOMAIN).unwrap();
    let p = ferroplan::parser::parse_problem(RC_PROBLEM).unwrap();
    let cfg = ferroplan::sat::SatCfg::default();
    let t0 = std::time::Instant::now();
    let o = ferroplan::sat::solve_temporal_within(&d, &p, 1, &cfg, Some(0.0));
    assert!(o.plan.is_none(), "a spent slice must not produce a plan");
    assert!(
        o.notes
            .iter()
            .any(|n| n.contains("promoted wall slice expired")),
        "the honest slice note is the receipt: {:?}",
        o.notes
    );
    assert!(
        t0.elapsed().as_secs() < 10,
        "a spent slice returns without solving"
    );
    assert!(
        !o.proven_at_every_horizon,
        "a slice expiry is a budget stop, never a proof"
    );
    // No slice = the plain entry, and the wing still solves the task.
    let o = ferroplan::sat::solve_temporal_within(&d, &p, 1, &cfg, None);
    assert!(o.plan.is_some(), "{:?}", o.notes);
}

// ---------------------------------------------------------------------------
// The classical face (Phase 2): round trip + ramp honesty.
// ---------------------------------------------------------------------------

/// Micro-STRIPS chain: two moves, so horizon 1 is genuinely UNSAT and the
/// ramp's escape is exercised on every solve.
const MICRO_DOMAIN: &str = "
(define (domain sat-micro)
  (:requirements :strips :typing)
  (:types loc)
  (:predicates (at ?l - loc) (adj ?a ?b - loc))
  (:action move
    :parameters (?a ?b - loc)
    :precondition (and (at ?a) (adj ?a ?b))
    :effect (and (not (at ?a)) (at ?b))))
";
const MICRO_PROBLEM: &str = "
(define (problem sat-micro-1) (:domain sat-micro)
  (:objects l1 l2 l3 - loc)
  (:init (at l1) (adj l1 l2) (adj l2 l3))
  (:goal (at l3)))
";

/// THE round-trip fixture (RED first: recorded — before the wing existed,
/// `Mode::Sat` did not compile/exist): a SAT-decoded classical plan
/// validates against the ORIGINAL problem via the internal oracle, and
/// via VAL when the referee is installed. Plan soundness is structurally
/// free on this route — that is the beauty of compilation.
#[test]
fn classical_round_trip_oracle_and_val() {
    let opts = Options {
        mode: Mode::Sat,
        ..Default::default()
    };
    let sol = solve(MICRO_DOMAIN, MICRO_PROBLEM, &opts).expect("solve runs");
    assert!(sol.solved, "SAT solves the micro chain: {:?}", sol.notes);
    assert_eq!(sol.mode, Mode::Sat);
    assert!(
        sol.notes.iter().any(|n| n.contains("replay-verified")),
        "the decode carries its serialization proof: {:?}",
        sol.notes
    );
    let plan = sol.plan.expect("plan");
    assert_eq!(plan.length, 2, "two moves");
    // The oracle reads the classic `step N:` format; VAL reads parens.
    let oracle_src: String = plan
        .steps
        .iter()
        .map(|s| format!("step {}: {} {}\n", s.index, s.action, s.args.join(" ")))
        .collect();
    match ferroplan::plan::validate_plan(MICRO_DOMAIN, MICRO_PROBLEM, &oracle_src) {
        Ok(ferroplan::plan::Validity::Valid) => {}
        other => panic!("SAT plan failed the internal oracle: {other:?}\nplan:\n{oracle_src}"),
    }
    let val_src: String = plan
        .steps
        .iter()
        .map(|s| format!("({} {})\n", s.action, s.args.join(" ")))
        .collect();
    val_check(MICRO_DOMAIN, MICRO_PROBLEM, &val_src, false);
}

/// The UNSAT-at-horizon-1 pin: the ramp escape names itself LOUD — the
/// notes carry the trail (`h1 UNSAT`) instead of silently widening.
#[test]
fn unsat_at_horizon_one_ramps_loud() {
    let opts = Options {
        mode: Mode::Sat,
        ..Default::default()
    };
    let sol = solve(MICRO_DOMAIN, MICRO_PROBLEM, &opts).expect("solve runs");
    assert!(sol.solved);
    assert!(
        sol.notes
            .iter()
            .any(|n| n.contains("SAT ramp:") && n.contains("h1 UNSAT")),
        "the ramp escape must name itself: {:?}",
        sol.notes
    );
}

/// Encoder self-pricing: over a budget-derived cap the encoder DECLINES
/// with a named note — never a hang, and never the word "unsolvable".
/// Driven through the library cfg (no env, no races).
#[test]
fn encoder_declines_over_cap_honestly() {
    let d = ferroplan::parser::parse_domain(MICRO_DOMAIN).unwrap();
    let p = ferroplan::parser::parse_problem(MICRO_PROBLEM).unwrap();
    let task = match ferroplan::ground::ground(&d, &p, 1) {
        ferroplan::ground::Outcome::Task(t) => t,
        _ => panic!("grounding did not yield a task"),
    };
    let cfg = ferroplan::sat::SatCfg {
        cap_lits: 10, // absurdly small: everything declines
        ..Default::default()
    };
    let o = ferroplan::sat::solve_classical(&task, &[], &cfg);
    assert!(o.plan.is_none());
    assert!(
        o.notes.iter().any(|n| n.contains("declined")),
        "decline names itself: {:?}",
        o.notes
    );
    assert!(
        o.notes
            .iter()
            .all(|n| !n.to_lowercase().contains("unsolvable")),
        "a cap is never a proof: {:?}",
        o.notes
    );
}

/// Bounded-horizon honesty: a ramp that proves UNSAT at every horizon it
/// reaches (max_horizon 1 on a 2-step task) reports a bounded-horizon
/// verdict — "no plan within horizon H", explicitly NOT unsolvability.
#[test]
fn no_plan_within_horizon_wording() {
    let d = ferroplan::parser::parse_domain(MICRO_DOMAIN).unwrap();
    let p = ferroplan::parser::parse_problem(MICRO_PROBLEM).unwrap();
    let task = match ferroplan::ground::ground(&d, &p, 1) {
        ferroplan::ground::Outcome::Task(t) => t,
        _ => panic!("grounding did not yield a task"),
    };
    let cfg = ferroplan::sat::SatCfg {
        max_horizon: 1,
        ..Default::default()
    };
    let o = ferroplan::sat::solve_classical(&task, &[], &cfg);
    assert!(o.plan.is_none());
    assert!(o.proven_at_every_horizon, "h1 UNSAT is a real proof");
    assert!(
        o.notes
            .iter()
            .any(|n| n.contains("no plan within horizon 1")),
        "bounded-horizon wording: {:?}",
        o.notes
    );
    assert!(
        o.notes
            .iter()
            .all(|n| !n.to_lowercase().contains("unsolvable")),
        "bounded-horizon is never unsolvability: {:?}",
        o.notes
    );
}

/// The temporal face declines TIL tasks with a named note (absolute times
/// have no place in a duration-free CNF) — and a decline is never
/// "unsolvable".
#[test]
fn temporal_face_declines_tils_honestly() {
    let prb_til = "
    (define (problem sat-rc-til) (:domain sat-rc)
      (:init (fresh-shine) (fresh-mend) (fresh-deliver) (fresh-door)
             (at 5 (light)))
      (:goal (and (mended) (delivered))))";
    let opts = Options {
        mode: Mode::Sat,
        ..Default::default()
    };
    let sol = solve(RC_DOMAIN, prb_til, &opts).expect("solve runs");
    assert!(!sol.solved);
    assert!(
        sol.notes
            .iter()
            .any(|n| n.contains("declined") && n.contains("timed initial literals")),
        "TIL decline names itself: {:?}",
        sol.notes
    );
    assert!(
        sol.notes
            .iter()
            .all(|n| !n.to_lowercase().contains("unsolvable")),
        "a decline is never a proof: {:?}",
        sol.notes
    );
}

/// The TMS twin-op shape (found by the final-stage batch, RED first: the
/// pairing decode hit an END whose START pointer named the other twin,
/// and the release path mislabeled it a budget trip): an object declared
/// under TWO subtypes (`kiln0 - kiln8` AND `kiln0 - kiln20` in TMS-2011)
/// makes supertype enumeration ground every op twice — true twins,
/// identical display and content. The wing must drop the shadow twin and
/// pair cleanly.
#[test]
fn dual_typed_twin_ops_pair_cleanly() {
    let dom = "
    (define (domain sat-twin)
      (:requirements :strips :typing :durative-actions)
      (:types ta tb - t)
      (:predicates (done ?x - t) (fresh))
      (:durative-action work
        :parameters (?x - t)
        :duration (= ?duration 2)
        :condition (at start (fresh))
        :effect (and (at start (not (fresh))) (at end (done ?x)))))";
    let prb = "
    (define (problem sat-twin-1) (:domain sat-twin)
      (:objects o1 - ta o1 - tb)
      (:init (fresh))
      (:goal (done o1)))";
    let opts = Options {
        mode: Mode::Sat,
        ..Default::default()
    };
    let sol = solve(dom, prb, &opts).expect("solve runs");
    assert!(
        sol.solved,
        "twin ground ops must pair cleanly: {:?}",
        sol.notes
    );
    let plan = sol.plan.expect("plan");
    let ipc = render_ipc(&plan);
    match ferroplan::plan::validate_plan(dom, prb, &ipc) {
        Ok(ferroplan::plan::Validity::Valid) => {}
        other => panic!("twin plan failed the oracle: {other:?}\nplan:\n{ipc}"),
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Render an API plan in the IPC temporal format the validator parses.
fn render_ipc(plan: &ferroplan::Plan) -> String {
    let mut s = String::new();
    for st in &plan.steps {
        let args = if st.args.is_empty() {
            String::new()
        } else {
            format!(" {}", st.args.join(" ").to_lowercase())
        };
        s.push_str(&format!(
            "{:.3}: ({}{}) [{:.3}]\n",
            st.time.unwrap_or(0.0),
            st.action.to_lowercase(),
            args,
            st.duration.unwrap_or(0.001),
        ));
    }
    s
}

/// Run VAL when `FERROPLAN_VAL` points at the referee; skip silently
/// otherwise (the internal oracle already ran).
fn val_check(domain: &str, problem: &str, plan: &str, temporal: bool) {
    let Ok(val) = std::env::var("FERROPLAN_VAL") else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("ffsatwing-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dp = dir.join("d.pddl");
    let pp = dir.join("p.pddl");
    let lp = dir.join("plan.txt");
    std::fs::write(&dp, domain).unwrap();
    std::fs::write(&pp, problem).unwrap();
    std::fs::write(&lp, plan).unwrap();
    let mut cmd = std::process::Command::new(val);
    if temporal {
        cmd.arg("-t").arg("0.001");
    }
    let outp = cmd.arg(&dp).arg(&pp).arg(&lp).output().expect("VAL runs");
    let stdout = String::from_utf8_lossy(&outp.stdout);
    assert!(
        stdout.contains("Plan valid"),
        "VAL rejected the SAT plan:\n{stdout}\n{}",
        String::from_utf8_lossy(&outp.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
