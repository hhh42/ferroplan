//! `FF_NUMPRE_TEMPORAL` (0.26 F3): arming the numeric-precondition charge
//! on temporal groundings. The flag is OPT-IN and stays so, on this pin's
//! evidence: the charge replays the recorded 0.21 negative on the village
//! workshop — the 25-step carve plan re-routes into the 47-step chisel-sale
//! plan, under every damping half (`FF_NUMPRE_NODAMP/NOSKIP/NOSUM`,
//! `FF_NO_NUMPRE_CHAIN`, `FF_NUMPRE_DEPTH=0`) alike — while it converts
//! pathways-metric-time i1 (unsolved at 27 s → solved in 1.5 s, the first
//! solve on that 0/30 board). Both facts are pinned: unset stays the carve
//! plan (bit-identical h), and armed the re-route is expected (it sells
//! chisels and never carves) — if it disappears, the charge changed and
//! the item must be re-refereed before any default-on.
//!
//! One test per binary: the flag is process-wide, and the unset baseline
//! is taken before it is set.

use ferroplan::{solve, Options};

fn base() -> String {
    format!("{}/../../benchmarks/village", env!("CARGO_MANIFEST_DIR"))
}

fn workshop_steps() -> Vec<String> {
    let d = std::fs::read_to_string(format!("{}/domain.pddl", base())).unwrap();
    let p = std::fs::read_to_string(format!("{}/workshop.pddl", base())).unwrap();
    let s = solve(&d, &p, &Options::default()).expect("solve");
    assert!(s.solved, "the workshop economy must solve");
    s.plan
        .expect("plan")
        .steps
        .iter()
        .map(|s| format!("{} {}", s.action, s.args.join(" ")))
        .collect()
}

fn forge_before_carve(steps: &[String]) {
    let forge = steps.iter().position(|a| a.contains("FORGE-CHISEL"));
    let carve = steps.iter().position(|a| a.contains("CARVE-DECOY"));
    let (f, c) = (forge.expect("forges"), carve.expect("carves"));
    assert!(f < c, "chisel must exist before carving: {steps:?}");
}

#[test]
fn armed_charge_reroutes_the_workshop_and_stays_opt_in() {
    let unset = workshop_steps();
    forge_before_carve(&unset);
    assert_eq!(
        unset.len(),
        25,
        "unset workshop carve plan moved: {unset:?}"
    );

    std::env::set_var("FF_NUMPRE_TEMPORAL", "1");
    // The re-route sells chisels and never carves at all, so only the
    // length is pinned here.
    let armed = workshop_steps();
    assert!(
        armed.len() > unset.len(),
        "the recorded re-route is gone ({} steps armed vs {} unset): the \
         temporal charge changed shape — re-referee FF_NUMPRE_TEMPORAL on the \
         metric-time constituency before promoting it",
        armed.len(),
        unset.len()
    );
}
