//! Relaxed-plan lookahead in the complete fallback (0.26 F2,
//! docs/field-gaps-execution-0.26.md): at a popped node the relaxed plan is
//! EXECUTED greedily on the concrete state and the deep state joins the open
//! list as one successor with a multi-op edge. OPT-IN (`FF_LOOKAHEAD=1`);
//! flag-off is byte-identical.
//!
//! One sequential test on purpose (FF_* are process-global): each scenario
//! runs in a CHILD process, the tests/refill.rs convention.
//!
//! The fixture is a serial chain: L steps, each enabled by the last. The
//! relaxed plan IS the plan, so a single lookahead from the root executes
//! the whole chain and the search returns after ONE evaluation, where the
//! plain fallback evaluates once per step (deferred h, one round a level).

use std::process::Command;

fn chain(l: usize) -> (String, String) {
    let mut preds = String::new();
    for i in 0..=l {
        preds.push_str(&format!(" (c{i})"));
    }
    let mut acts = String::new();
    for i in 0..l {
        acts.push_str(&format!(
            "(:action step{i} :parameters () :precondition (c{i}) :effect (and (c{}) (not (c{i}))))\n",
            i + 1
        ));
    }
    let dom =
        format!("(define (domain chain) (:requirements :strips)\n(:predicates{preds})\n{acts})");
    let prb = format!("(define (problem p) (:domain chain) (:init (c0)) (:goal (c{l})))");
    (dom, prb)
}

fn run_child(scenario: &str) -> String {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "lookahead_walks_the_chain_in_one_evaluation",
        "--nocapture",
    ])
    .env("LOOKAHEAD_CHILD", scenario)
    .env_remove("FF_LOOKAHEAD")
    .env_remove("FF_TIME_LIMIT");
    match scenario {
        "on" | "on-t8" => {
            cmd.env("FF_LOOKAHEAD", "1");
        }
        "off" => {}
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

fn evals(stdout: &str) -> usize {
    line(stdout, "EVALS:")[6..].parse().unwrap()
}

#[test]
fn lookahead_walks_the_chain_in_one_evaluation() {
    const L: usize = 40;
    if let Ok(scenario) = std::env::var("LOOKAHEAD_CHILD") {
        let (dom, prb) = chain(L);
        let opts = ferroplan::Options {
            search: ferroplan::Search::BestFirst,
            threads: if scenario == "on-t8" { 8 } else { 1 },
            ..Default::default()
        };
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        let plan: Vec<String> = sol
            .plan
            .as_ref()
            .map(|p| p.steps.iter().map(|s| s.action.to_lowercase()).collect())
            .unwrap_or_default();
        println!("SOLVED:{}", sol.solved);
        println!("EVALS:{}", sol.statistics.evaluated_states);
        println!("PLAN:{}", plan.join(" "));
        return;
    }

    // Flag-off: the plain fallback, one evaluation per chain step.
    let off = run_child("off");
    assert_eq!(line(&off, "SOLVED:"), "SOLVED:true", "{off}");
    let want: Vec<String> = (0..L).map(|i| format!("step{i}")).collect();
    assert_eq!(line(&off, "PLAN:"), format!("PLAN:{}", want.join(" ")));
    assert!(
        evals(&off) >= L,
        "deferred evaluation, one round a level: {off}"
    );

    // Flag-on: the root's lookahead executes the whole relaxed plan — the
    // deep node meets the goal, and the plan is the spliced edge, valid by
    // construction (every op passed op_applicable on the concrete state).
    let on = run_child("on");
    assert_eq!(line(&on, "SOLVED:"), "SOLVED:true", "{on}");
    assert_eq!(
        line(&on, "PLAN:"),
        line(&off, "PLAN:"),
        "the same plan, in one jump"
    );
    assert!(
        evals(&on) <= 2,
        "one root evaluation (the goal-met terminal costs none): {on}"
    );

    // Determinism: order-preserving parallel evaluation, serial insertion.
    let on8 = run_child("on-t8");
    assert_eq!(line(&on8, "PLAN:"), line(&on, "PLAN:"));
    assert_eq!(evals(&on8), evals(&on));
}

/// The read-out the lookahead consumes: the last extraction's selected ops
/// in RPG-layer order, deterministic.
#[test]
fn the_relaxed_plan_reads_out_in_layer_order() {
    let (dom, prb) = chain(6);
    let d = ferroplan::parser::parse_domain(&dom).unwrap();
    let p = ferroplan::parser::parse_problem(&prb).unwrap();
    let task = ferroplan::ground::ground_task(&d, &p, 1).unwrap();
    let init = task.initial();
    let mut sc = ferroplan::heuristic::Scratch::new(&task);
    let h = ferroplan::heuristic::relaxed_to(
        &task,
        &mut sc,
        &init.bits,
        &init.fv,
        &init.fdef,
        &task.goal_pos,
        &task.goal_num,
    )
    .expect("reachable");
    assert_eq!(h, 6);
    let ops = ferroplan::heuristic::extraction_plan_ops(&sc);
    assert_eq!(ops.len(), 6, "{ops:?}");
    let names: Vec<String> = ops
        .iter()
        .map(|&oi| task.op_display[oi as usize].to_lowercase())
        .collect();
    for (i, n) in names.iter().enumerate() {
        assert!(
            n.contains(&format!("step{i}")),
            "layer order is chain order: {names:?}"
        );
    }
}
