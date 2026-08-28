//! Fallback enrichment (0.26 F1, docs/field-gaps-execution-0.26.md): the
//! complete weighted best-first fallback — the rung that does most of the
//! solving on the ipc5-prop and 2018 tails — carries the LAMA recipe by
//! default: preferred-operator alternation over a dual open list plus the
//! landmark-count term. `FF_NO_ENRICH=1` restores the bare single-queue
//! fallback bit for bit; `FF_CLM` keeps its exact opt-in semantics under
//! the hatch, which is what lets the referee separate the two halves.
//!
//! One sequential test on purpose: FF_* are process-global env knobs, so
//! each scenario runs in a CHILD process (the tests/refill.rs convention).
//!
//! The fixture is the plateau the preferred queue exists for: a chain of L
//! steps to the goal, and at every state K independent "junk" toggles that
//! add facts nothing needs. Under deferred evaluation every successor of a
//! popped node carries its PARENT's h, so the chain step and the K junk
//! toggles tie on the key and the single queue walks the 2^K junk lattice
//! level by level — ~2^K evaluations per chain step. The chain step is the
//! one applicable op in the relaxed plan, so it is HELPFUL, and the
//! preferred heap pops it next round; the enriched fallback reaches the
//! goal in ~L rounds. The eval cap below sits between the two.

use std::process::Command;

fn plateau(l: usize, k: usize) -> (String, String) {
    let mut preds = String::new();
    for i in 0..=l {
        preds.push_str(&format!(" (c{i})"));
    }
    for j in 0..k {
        preds.push_str(&format!(" (j{j})"));
    }
    let mut acts = String::new();
    // Junk first, so its ops sort before the chain step at equal keys — the
    // tie-break the single queue loses.
    for j in 0..k {
        acts.push_str(&format!(
            "(:action junk{j} :parameters () :precondition (not (j{j})) :effect (j{j}))\n"
        ));
    }
    for i in 0..l {
        acts.push_str(&format!(
            "(:action step{i} :parameters () :precondition (c{i}) :effect (and (c{} ) (not (c{i}))))\n",
            i + 1
        ));
    }
    let dom = format!(
        "(define (domain plateau) (:requirements :strips :negative-preconditions)\n(:predicates{preds})\n{acts})"
    );
    let prb = format!("(define (problem p) (:domain plateau) (:init (c0)) (:goal (c{l})))");
    (dom, prb)
}

fn run_child(scenario: &str) -> String {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "the_enriched_fallback_crosses_the_plateau",
        "--nocapture",
    ])
    .env("ENRICH_CHILD", scenario)
    .env_remove("FF_NO_ENRICH")
    .env_remove("FF_CLM")
    .env_remove("FF_TIME_LIMIT");
    match scenario {
        "armed-t1" | "armed-t8" => {}
        "hatched" => {
            cmd.env("FF_NO_ENRICH", "1");
        }
        "hatched-clm" => {
            cmd.env("FF_NO_ENRICH", "1").env("FF_CLM", "3");
        }
        other => panic!("unknown scenario {other}"),
    }
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "child {scenario} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn line(stdout: &str, key: &str) -> String {
    stdout
        .lines()
        .find(|l| l.starts_with(key))
        .unwrap_or_else(|| panic!("no {key} line in:\n{stdout}"))
        .to_string()
}

#[test]
fn the_enriched_fallback_crosses_the_plateau() {
    if let Ok(scenario) = std::env::var("ENRICH_CHILD") {
        // Child: BestFirst skips the bounded rungs, so the fallback is the
        // whole ladder; the explicit eval cap is the budget the single queue
        // cannot cross and the preferred queue does not need.
        let k: usize = std::env::var("ENRICH_K")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(14);
        let cap: usize = std::env::var("ENRICH_CAP")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4_000);
        let (dom, prb) = plateau(30, k);
        let opts = ferroplan::Options {
            search: ferroplan::Search::BestFirst,
            threads: if scenario == "armed-t8" { 8 } else { 1 },
            max_evaluated: Some(cap),
            ..Default::default()
        };
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        let plan: Vec<String> = sol
            .plan
            .as_ref()
            .map(|p| p.steps.iter().map(|s| s.action.clone()).collect())
            .unwrap_or_default();
        println!("SOLVED:{}", sol.solved);
        println!("EVALS:{}", sol.statistics.evaluated_states);
        println!("PLAN:{}", plan.join(" "));
        return;
    }

    // RED pin: the bare single queue drowns in the junk lattice under the
    // cap; the enriched fallback walks the chain.
    let hatched = run_child("hatched");
    assert_eq!(line(&hatched, "SOLVED:"), "SOLVED:false", "{hatched}");
    let armed = run_child("armed-t1");
    assert_eq!(line(&armed, "SOLVED:"), "SOLVED:true", "{armed}");
    assert_eq!(
        line(&armed, "PLAN:").to_lowercase().matches("step").count(),
        30,
        "the chain, and nothing else: {armed}"
    );

    // Determinism: fixed batch shares, order-preserving parallel h, serial
    // insertion — the plan is identical at any thread count.
    let armed8 = run_child("armed-t8");
    assert_eq!(line(&armed8, "PLAN:"), line(&armed, "PLAN:"));
    assert_eq!(line(&armed8, "EVALS:"), line(&armed, "EVALS:"));

    // The decomposition arm: FF_CLM under the hatch is the 0.11 opt-in path,
    // exactly as it was. MEASURED on this fixture (K = 14..20, cap 200k):
    // the term alone changes nothing -- 7,034 evals, identical to the bare
    // queue at every K -- because the deferred h already orders the chain
    // one level per round in both, and the preferred queue's whole gain is
    // batch composition (~65 evaluations per round instead of 256). So
    // this pins that the arm RUNS and lands where the bare queue lands;
    // whether the term earns anything is the board A/B's question, on
    // boards where h plateaus and it can.
    let clm = run_child("hatched-clm");
    assert_eq!(line(&clm, "SOLVED:"), "SOLVED:false", "{clm}");
    assert_eq!(line(&clm, "EVALS:"), line(&hatched, "EVALS:"), "{clm}");
    eprintln!(
        "enrich pin: hatched {} / clm {} (both capped) / armed {} evals",
        line(&hatched, "EVALS:"),
        line(&clm, "EVALS:"),
        line(&armed, "EVALS:")
    );
}
