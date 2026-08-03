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
