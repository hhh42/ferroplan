//! The village domain (0.17 Phase 4): the abstract crafting-economy rules
//! — one gather / make / buy / sell, catalogs as init data — must carry
//! both ladder rungs end to end: the lone craftsman's chain and the
//! workshop's tool-forging economy loop.

use ferroplan::{solve, Options};

fn base() -> String {
    format!("{}/../../benchmarks/village", env!("CARGO_MANIFEST_DIR"))
}

fn run(problem: &str) -> ferroplan::Solution {
    let d = std::fs::read_to_string(format!("{}/domain.pddl", base())).unwrap();
    let p = std::fs::read_to_string(format!("{}/{problem}.pddl", base())).unwrap();
    solve(&d, &p, &Options::default()).expect("solve")
}

#[test]
fn craftsman_earns_his_coins() {
    let s = run("craftsman");
    assert!(s.solved, "the lone craftsman must solve");
    let plan = s.plan.expect("plan");
    // The chain is forced: pick up both tools, gather, saw, join, sell.
    // The action field is the bare abstract verb; recipes ride in args —
    // exactly the one-make-rule contract.
    let steps: Vec<String> = plan
        .steps
        .iter()
        .map(|s| format!("{} {}", s.action, s.args.join(" ")))
        .collect();
    let joined = steps.join(" | ");
    for needle in ["SAW-PLANKS", "JOIN-STOOL", "SELL"] {
        assert!(joined.contains(needle), "plan must {needle}: {joined}");
    }
}

#[test]
fn workshop_forges_the_chisel_before_carving() {
    let s = run("workshop");
    assert!(s.solved, "the workshop economy must solve");
    let plan = s.plan.expect("plan");
    let steps: Vec<String> = plan
        .steps
        .iter()
        .map(|s| format!("{} {}", s.action, s.args.join(" ")))
        .collect();
    let forge = steps.iter().position(|a| a.contains("FORGE-CHISEL"));
    let carve = steps.iter().position(|a| a.contains("CARVE-DECOY"));
    let (f, c) = (forge.expect("forges"), carve.expect("carves"));
    assert!(f < c, "chisel must exist before carving: {steps:?}");
}
