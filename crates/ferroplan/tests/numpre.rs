//! The numeric-precondition charge (0.21 Phase 3): a selected op's own
//! unsatisfied `pre_num` now charges through the numeric achievers, so a
//! propositional goal behind a numeric band (the sailing class) gets a
//! gradient where extraction saw a plateau. `FF_NO_NUMPRE=1` restores the
//! flat h. The trader fixture is the pinned NEGATIVE control: cyclic
//! resource flows stay out of reach (markettrader's recorded ceiling).

use std::sync::Mutex;

use ferroplan::Options;

/// Two tests mutate `FF_NO_NUMPRE`; every test takes this lock so the
/// default parallel runner cannot race the hatch (constraints.rs idiom).
static ENV_LOCK: Mutex<()> = Mutex::new(());

const SAIL_DOM: &str = include_str!("../../../benchmarks/bench/sailing-band-domain.pddl");
const SAIL_PRB: &str = include_str!("../../../benchmarks/bench/sailing-band-i1.pddl");
const TRADE_DOM: &str = include_str!("../../../benchmarks/bench/trader-cycle-domain.pddl");
const TRADE_PRB: &str = include_str!("../../../benchmarks/bench/trader-cycle-i1.pddl");

fn capped(n: usize) -> Options {
    Options {
        max_evaluated: Some(n),
        threads: 1,
        ..Default::default()
    }
}

/// End-to-end, mode AUTO, through the TEXT path — the exact route the
/// numeric boards take (run_planner -> partition -> monolithic ladder) —
/// plus the eval receipt: the flat h=1 engine crawls the whole 2D lattice
/// ball inside EHC's lookahead (3,019 evals measured on the 0.20 binary;
/// real sailing i1 burned 5M) while the charged gradient walks
/// 10 NE + 10 NW + save in a few dozen.
#[test]
fn sailing_band_mode_auto_solves_on_the_gradient() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (out, code) = ferroplan::run_planner(SAIL_DOM, SAIL_PRB, &capped(20_000), false);
    assert_eq!(code, 0, "clean exit:\n{out}");
    assert!(out.contains("found legal plan"), "must solve:\n{out}");

    let sol = ferroplan::solve(SAIL_DOM, SAIL_PRB, &capped(20_000)).unwrap();
    assert!(sol.solved, "library mode-auto solves too");
    assert!(
        sol.statistics.evaluated_states < 500,
        "the charge walks, never crawls: {} evals",
        sol.statistics.evaluated_states
    );
    assert_eq!(sol.plan.unwrap().length, 21, "10 NE + 10 NW + save");
}

/// The hatch: FF_NO_NUMPRE=1 restores the plateau — same fixture, back to
/// the 0.20 lattice crawl (3,019 evals; an order above the charged walk).
#[test]
fn ff_no_numpre_restores_the_plateau() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FF_NO_NUMPRE", "1");
    let sol = ferroplan::solve(SAIL_DOM, SAIL_PRB, &capped(20_000)).unwrap();
    std::env::remove_var("FF_NO_NUMPRE");
    assert!(
        sol.solved,
        "the hatch only flattens h, never breaks solving"
    );
    assert!(
        sol.statistics.evaluated_states > 2_000,
        "hatch must restore the flat-h crawl: {} evals",
        sol.statistics.evaluated_states
    );
}

/// The NEGATIVE control, end to end: the cyclic-profit fixture stays
/// unsolved at a cap the sailing class clears by two orders — grinding the
/// lap costs 7,305 evals (0.20 binary receipt) and the charge adds only
/// constant one-level terms to it, never a cycle gradient. Documents that
/// markettrader's class is re-attributed OUT of the winnable pot, not
/// quietly claimed.
#[test]
fn trader_cycle_stays_unsolved_at_a_small_cap() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let sol = ferroplan::solve(TRADE_DOM, TRADE_PRB, &capped(3_000)).unwrap();
    assert!(!sol.solved, "cycle-blindness stays (recorded ceiling)");
}

/// The honesty rider, end to end (both text paths): a CAPPED failure says
/// "cap reached", never "proven unsolvable" — that wording is reserved for
/// genuine open-list exhaustion (output.rs unit test pins the split).
#[test]
fn capped_text_says_cap_not_proof() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (out, code) = ferroplan::run_ff(TRADE_DOM, TRADE_PRB, &capped(3_000));
    assert_eq!(code, 0, "a capped run is still a clean exit:\n{out}");
    assert!(out.contains("search cap reached"), "honest cap:\n{out}");
    assert!(!out.contains("proven unsolvable"), "no false proof:\n{out}");
    let (out, _) = ferroplan::run_planner(TRADE_DOM, TRADE_PRB, &capped(3_000), false);
    assert!(out.contains("search cap reached"), "honest cap:\n{out}");
    assert!(!out.contains("proven unsolvable"), "no false proof:\n{out}");
}

// ---------------------------------------------------------------------------
// The a2 CHAINED charge (0.24 Phase 6) — the pathwaysmetric-i2 shape: a
// supply chain deeper than one level. a1 prices exactly finish's c-gap
// (h=5, flat across the whole 17-step approach) and the three drift
// dimensions blow the plateau ball past a small cap; the chained charge
// prices the full chain (h(init)=22, the optimum) and descends per step.
// Unit pins with the exact h arithmetic live in heuristic.rs.
// ---------------------------------------------------------------------------

const CHAIN_DOM: &str = include_str!("../../../benchmarks/bench/chained-band-domain.pddl");
const CHAIN_PRB: &str = include_str!("../../../benchmarks/bench/chained-band-i1.pddl");

/// GREEN: mode AUTO walks the chained gradient inside a cap two orders
/// under the plateau ball.
#[test]
fn chained_band_mode_auto_solves_on_the_chained_gradient() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (out, code) = ferroplan::run_planner(CHAIN_DOM, CHAIN_PRB, &capped(4_000), false);
    assert_eq!(code, 0, "clean exit:\n{out}");
    assert!(out.contains("found legal plan"), "must solve:\n{out}");

    let sol = ferroplan::solve(CHAIN_DOM, CHAIN_PRB, &capped(4_000)).unwrap();
    assert!(sol.solved, "library mode-auto solves too");
    assert!(
        sol.statistics.evaluated_states < 500,
        "the chained charge walks, never crawls: {} evals",
        sol.statistics.evaluated_states
    );
}

/// The RED twin (the permanent record of the hole a2 closes): one-level
/// pricing leaves the approach flat and the same cap must fail.
#[test]
fn chained_band_dies_one_level() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FF_NO_NUMPRE_CHAIN", "1");
    let sol = ferroplan::solve(CHAIN_DOM, CHAIN_PRB, &capped(4_000)).unwrap();
    std::env::remove_var("FF_NO_NUMPRE_CHAIN");
    assert!(
        !sol.solved,
        "the one-level charge was expected to strand the chain (the RED shape)"
    );
}

const WATER_DOM: &str = include_str!("../../../benchmarks/bench/watering-line-domain.pddl");
const WATER_PRB: &str = include_str!("../../../benchmarks/bench/watering-line-i1.pddl");

/// The charge's bill, end to end (0.22 Phase 1): the distilled
/// ext-plant-watering shape solves mode-auto under the DAMPED charge —
/// shared-achiever preconditions price at the SUM of their gaps, so every
/// arrival retires its term smoothly instead of re-pointing the charge at
/// a farther plant (the exact h values are pinned in heuristic.rs's
/// watering-mini unit tests; i7's solo receipt: 2.9M evals unsolved
/// first-wins → 860k solved damped, and the 0.21 near-wall solves
/// i5/i6/i8/i10/i16 all keep or better their times — the MAX damping
/// probed first traded them away and is the recorded negative).
#[test]
fn watering_line_mode_auto_solves_on_the_damped_charge() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (out, code) = ferroplan::run_planner(WATER_DOM, WATER_PRB, &capped(150_000), false);
    assert_eq!(code, 0, "clean exit:\n{out}");
    assert!(out.contains("found legal plan"), "must solve:\n{out}");

    let sol = ferroplan::solve(WATER_DOM, WATER_PRB, &capped(150_000)).unwrap();
    assert!(sol.solved, "library mode-auto solves too");
    assert!(
        sol.statistics.evaluated_states < 150_000,
        "the damped charge stays inside the cap: {} evals",
        sol.statistics.evaluated_states
    );
}

/// FF_NUMNOV plumbing smoke (probe rider b): the opt-in numeric-novelty
/// envelope must not break a numeric solve routed through the novelty rung
/// machinery. (The signature-level pin lives in novelty.rs's unit tests.)
#[test]
fn ff_numnov_smoke_still_solves() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FF_NUMNOV", "1");
    let sol = ferroplan::solve(SAIL_DOM, SAIL_PRB, &capped(20_000)).unwrap();
    std::env::remove_var("FF_NUMNOV");
    assert!(sol.solved, "numeric novelty is additive, never breaking");
}

// ---------------------------------------------------------------------------
// The do-not-give-back fixture (0.23 Phase 1, the damping docket) —
// fo-sailing i8 VERBATIM from IPC-2023 numeric (Scala & Ramirez's
// first-order sailing extension; 429-byte instance, vendored whole).
// The 0.22 SUM damping turned this row from a 55.7 s 0.21 timeout into a
// 0.01 s solve, and the 0.23 attribution probe split the halves: with the
// SUM half alone (FF_NUMPRE_NOSKIP=1) the solve is byte-identical at 287
// evals — the mover-skip half is inert here — while first-wins
// (FF_NUMPRE_NOSUM=1, and full NODAMP) grinds 3.5M/4.5M evals into the
// 60 s wall. Any future conditioning of the damping must keep BOTH tests
// green: this pair is the give-back detector for fo-sailing's +7.
// ---------------------------------------------------------------------------

const FOSAIL_DOM: &str = include_str!("../../../benchmarks/bench/fo-sailing-domain.pddl");
const FOSAIL_I8: &str = include_str!("../../../benchmarks/bench/fo-sailing-i8.pddl");

/// GREEN guard: the summed charge solves fo-sailing i8 inside a cap two
/// orders under the first-wins grind (287 evals measured solo; 10k cap
/// leaves headroom without admitting the plateau).
#[test]
fn fo_sailing_i8_fast_solve_must_survive() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let sol = ferroplan::solve(FOSAIL_DOM, FOSAIL_I8, &capped(10_000)).unwrap();
    assert!(sol.solved, "fo-sailing i8's 0.01 s solve must survive");
    assert!(
        sol.statistics.evaluated_states < 2_000,
        "the summed charge walks, never grinds: {} evals",
        sol.statistics.evaluated_states
    );
}

/// The RED twin that names the load-bearing half: first-wins pricing
/// (FF_NUMPRE_NOSUM=1) loses the gradient and the same cap must fail —
/// if this ever solves, the fixture above stopped guarding anything and
/// the damping docket needs re-reading.
#[test]
fn fo_sailing_i8_dies_first_wins() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FF_NUMPRE_NOSUM", "1");
    let sol = ferroplan::solve(FOSAIL_DOM, FOSAIL_I8, &capped(10_000)).unwrap();
    std::env::remove_var("FF_NUMPRE_NOSUM");
    assert!(
        !sol.solved,
        "first-wins was expected to grind past the cap (the RED shape)"
    );
}
