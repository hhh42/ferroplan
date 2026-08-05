//! The h-surgery probe (0.21 Phase 8, docs/roadmap-0.21.md): under
//! `FF_H_ENDGATE=1` the temporal think arms a start→end pair table on the
//! task, and relaxed-plan extraction discounts a selected START whose
//! paired END is selected in the SAME generation — the pair prices as one
//! unit, paid while the END is owed. Selection itself is untouched
//! (helpful sets must not move: emptying start selection would replay the
//! 0.11 FF_LAX_HELPFUL negative), and the classical path never builds the
//! table, so the flag is provably dormant there — pinned anyway.

use ferroplan::ground::{ground, Outcome};
use ferroplan::heuristic::{relaxed, relaxed_helpful, Scratch};
use ferroplan::packed::PackedTask;
use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::{solve, Options};

/// Two independent snaps standing in for one durative interval at the
/// heuristic's level of sight: START adds the at-start effect, END the
/// at-end payload. The pair table is set BY HAND — this pins extraction
/// mechanics, not the temporal plumbing that populates the table.
const SNAP_DOM: &str = "
(define (domain endgate)
  (:predicates (ga) (gb))
  (:action snap-start :parameters () :effect (ga))
  (:action snap-end   :parameters () :effect (gb)))";

fn snap_task(goal: &str) -> PackedTask {
    let prb = format!("(define (problem endgate-1) (:domain endgate) (:goal {goal}))");
    let d = parse_domain(SNAP_DOM).unwrap();
    let p = parse_problem(&prb).unwrap();
    match ground(&d, &p, 1) {
        Outcome::Task(t) => t,
        _ => panic!("endgate fixture must ground"),
    }
}

fn pair_table(task: &PackedTask) -> Vec<u32> {
    let op = |needle: &str| {
        (0..task.n_ops)
            .find(|&oi| task.op_display[oi].starts_with(needle))
            .unwrap_or_else(|| panic!("op {needle} must ground"))
    };
    let mut pairs = vec![u32::MAX; task.n_ops];
    pairs[op("SNAP-START")] = op("SNAP-END") as u32;
    pairs
}

fn h_of(task: &PackedTask) -> i32 {
    let init = task.initial();
    let mut sc = Scratch::new(task);
    relaxed(task, &mut sc, &init.bits, &init.fv, &init.fdef).expect("reachable")
}

/// Both snaps selected in one generation ⇒ the pair prices as ONE unit;
/// and the helpful set does not move an inch under the discount.
#[test]
fn pair_selected_together_prices_as_one() {
    let mut task = snap_task("(and (ga) (gb))");
    assert_eq!(h_of(&task), 2, "no table: start and end each charge 1");

    let baseline = {
        let init = task.initial();
        let mut sc = Scratch::new(&task);
        relaxed_helpful(
            &task,
            &mut sc,
            &init.bits,
            &init.fv,
            &init.fdef,
            &task.goal_pos,
            &task.goal_num,
        )
        .expect("reachable")
    };
    task.pair_end = Some(pair_table(&task));
    let gated = {
        let init = task.initial();
        let mut sc = Scratch::new(&task);
        relaxed_helpful(
            &task,
            &mut sc,
            &init.bits,
            &init.fv,
            &init.fdef,
            &task.goal_pos,
            &task.goal_num,
        )
        .expect("reachable")
    };
    assert_eq!(
        gated.0, 1,
        "end-gate: the start/end pair prices as one unit"
    );
    assert_eq!(
        gated.1, baseline.1,
        "the discount must never move the helpful set (the 0.11 lesson)"
    );
}

/// A START selected only for its at-start effects — END unselected this
/// generation — keeps its FULL charge: 1, never 0. The discount requires
/// both snaps selected in-gen.
#[test]
fn at_start_only_selection_still_charges_one() {
    let mut task = snap_task("(ga)");
    task.pair_end = Some(pair_table(&task));
    assert_eq!(h_of(&task), 1, "at-start-only START must charge 1, not 0");
}

/// Classical byte-identity: the classical grounding never carries a pair
/// table, so FF_H_ENDGATE set vs unset must produce the identical plan and
/// eval count on a seq-sat bench fixture. Provable from `pair_end: None`;
/// pinned anyway (the roadmap's standing rule for opt-in hatches).
#[test]
fn classical_flag_is_dormant_byte_identical() {
    let base = format!("{}/../../benchmarks/bench", env!("CARGO_MANIFEST_DIR"));
    let dom = std::fs::read_to_string(format!("{base}/visitgrid-domain.pddl")).unwrap();
    let prb = std::fs::read_to_string(format!("{base}/visitgrid-7x7.pddl")).unwrap();
    let run = || {
        let s = solve(&dom, &prb, &Options::default()).expect("solve");
        assert!(s.solved, "visitgrid-7x7 must solve");
        let plan: Vec<String> = s
            .plan
            .as_ref()
            .unwrap()
            .steps
            .iter()
            .map(|st| format!("{} {}", st.action, st.args.join(" ")))
            .collect();
        (plan, s.statistics.evaluated_states)
    };
    let off = run();
    std::env::set_var("FF_H_ENDGATE", "1");
    let on = run();
    std::env::remove_var("FF_H_ENDGATE");
    assert_eq!(off.0, on.0, "classical plan must not move under the flag");
    assert_eq!(
        off.1, on.1,
        "classical eval count must not move under the flag"
    );
}

/// Read (a) driver for the pre-registered probe: the village pair contract
/// at a 200k-eval think (examples/village.rs pins 1M with the witness that
/// 200k dies today). Run with the flag OFF and ON, record solve/die:
///
///   cargo test --release -p ferroplan --test endgate -- --ignored --nocapture
///   FF_H_ENDGATE=1 cargo test --release -p ferroplan --test endgate -- --ignored --nocapture
///
/// Reports only — the probe verdict lives in the roadmap, not in an assert.
#[test]
#[ignore = "measurement driver: run by hand for the Phase 8 probe reads"]
fn village_pair_contract_at_200k() {
    use ferroplan::session::Session;
    const THINK_EVALS: usize = 200_000;
    const THINK_MB: usize = 512;

    let base = format!("{}/../../benchmarks/village", env!("CARGO_MANIFEST_DIR"));
    let dom = std::fs::read_to_string(format!("{base}/domain.pddl")).unwrap();
    let prb = std::fs::read_to_string(format!("{base}/pair.pddl")).unwrap();
    let world = Session::new(&dom, &prb, &Options::default()).expect("session");

    let mut bob = world.fork();
    bob.restrict_ops(|d| d.contains(" BOB"));
    let mut mara = world.fork();
    mara.restrict_ops(|d| d.contains(" MARA"));

    let flag = std::env::var("FF_H_ENDGATE").is_ok();
    let hire = |name: &str, s: &mut Session, contract: &str| {
        s.set_goal(contract).expect("contract parses");
        let think = s.replan_budgeted(THINK_EVALS, Some(THINK_MB));
        println!(
            "[endgate probe] FF_H_ENDGATE={} {} `{}`: {} (evals {}, steps {:?})",
            flag as u8,
            name,
            contract,
            if think.solved { "SOLVED" } else { "DIED" },
            think.statistics.evaluated_states,
            think.plan.as_ref().map(|p| p.steps.len()),
        );
    };
    hire("bob", &mut bob, "(>= (stock stool) 1)");
    hire("mara", &mut mara, "(>= (stock decoy) 1)");
    hire("bob(re)", &mut bob, "(>= (stock plank) 3)");
}
