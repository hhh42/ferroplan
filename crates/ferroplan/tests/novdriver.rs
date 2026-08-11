//! The partitioned h-free driver (0.22 Phase 5B, docs/roadmap-0.22.md —
//! THE CENTERPIECE). Four levers, smallest-first, each behind its own
//! hatch; these fixtures pin the separations that justify each lever and
//! keep the RED shapes on the record permanently (the ladder_wall.rs
//! convention: the hatched leg IS the recorded negative).
//!
//! - `combolock`: R-partition vs goal-count cells. Goal count is FLAT (one
//!   goal fact) across m stages that REUSE k toggles, so once every toggle
//!   atom has been seen the goal-count cell has no novelty left on the
//!   critical step and BFS-grinds the lawn; the R-partition's fresh
//!   per-stage cell (the c-chain accumulates into R) re-arms novelty at
//!   exactly that depth-1 step. (A first draft — parity patterns over one
//!   shared bank, `stagelock` — measured only 12.4k vs 16.4k pops: sibling
//!   insertions mark the whole bank inside one cell, so within-stage
//!   depth>1 beelines die. The recorded lesson: the partition pays where
//!   progress GROWS R and the per-cell work is shallow.)
//! - `pairlock`: width 2 vs width 1. The key state (p ∧ qn) carries ZERO
//!   new atoms — p seen at the root, qn seen on the chain — so width-1
//!   ranks it behind a u-toggle lawn of equally non-novel states;
//!   the (p, qn) R-PAIR is new, and the novel-2 rank pops it ahead.
//! - `childsnack_mini`: the negative control, EXCLUDED from the pot up
//!   front (symmetry's constituency, Phase 6): the driver must NOT solve
//!   it inside the rung's cap, so the centerpiece's referees stay
//!   undiluted.

use ferroplan::novelty::{self, DriverCfg};
use ferroplan::packed::PackedTask;
use std::process::Command;

fn ground(dom: &str, prb: &str) -> PackedTask {
    let d = ferroplan::parser::parse_domain(dom).unwrap();
    let p = ferroplan::parser::parse_problem(prb).unwrap();
    ferroplan::ground::ground_task(&d, &p, 1).unwrap()
}

/// Replay `ops` from the initial state; every op must be applicable in
/// sequence and the final state must satisfy the goal.
fn assert_valid(task: &PackedTask, ops: &[usize]) {
    let mut s = task.initial();
    for (i, &oi) in ops.iter().enumerate() {
        assert!(
            task.op_applicable(oi, &s),
            "step {i} ({}) inapplicable",
            task.op_display[oi]
        );
        s = task.apply(oi, &s);
    }
    assert!(
        task.goal_met_with(&s, &task.goal_pos, &task.goal_num),
        "goal unmet after replay"
    );
}

/// The combo lock: m stages; stage s consumes toggle (s mod k), so the
/// toggles are REUSED after the first k stages — no new atom is left on
/// the critical set-step, only a new (stage, toggle) combination. One
/// goal fact; the c-chain is R's milestone ladder.
fn combolock(k: usize, m: usize) -> (String, String) {
    let mut preds = String::from(" (gw)");
    for s in 0..=m {
        preds.push_str(&format!(" (c{s})"));
    }
    for i in 0..k {
        preds.push_str(&format!(" (on{i}) (off{i})"));
    }
    let mut acts = String::new();
    for i in 0..k {
        acts.push_str(&format!(
            "(:action set{i} :parameters () :precondition (off{i}) :effect (and (on{i}) (not (off{i}))))\n\
             (:action unset{i} :parameters () :precondition (on{i}) :effect (and (off{i}) (not (on{i}))))\n"
        ));
    }
    for s in 0..m {
        let j = s % k;
        acts.push_str(&format!(
            "(:action adv{s} :parameters () :precondition (and (c{s}) (on{j})) :effect (and (c{}) (not (c{s})) (not (on{j})) (off{j})))\n",
            s + 1
        ));
    }
    acts.push_str(&format!(
        "(:action win :parameters () :precondition (c{m}) :effect (gw))\n"
    ));
    let offs: String = (0..k).map(|i| format!(" (off{i})")).collect();
    (
        format!("(define (domain combolock) (:predicates{preds}) {acts})"),
        format!("(define (problem cl) (:domain combolock) (:init (c0){offs}) (:goal (gw)))"),
    )
}

/// The pair conjunction: (p) is true at the root and consumed by the only
/// op that opens the q-chain; re-establishing it AFTER the chain (fix
/// keeps qn) creates a state with no new atom but a new (p, qn) R-pair.
/// u free toggles supply the equally-non-novel lawn that buries the state
/// under width 1.
fn pairlock(u: usize, n: usize) -> (String, String) {
    let mut preds = String::from(" (gw) (p)");
    for j in 0..=n {
        preds.push_str(&format!(" (q{j})"));
    }
    for i in 0..u {
        preds.push_str(&format!(" (tf{i}) (tn{i})"));
    }
    let mut acts = String::from(
        "(:action brk :parameters () :precondition (p) :effect (and (q0) (not (p))))\n",
    );
    for j in 0..n {
        acts.push_str(&format!(
            "(:action step{j} :parameters () :precondition (q{j}) :effect (and (q{}) (not (q{j}))))\n",
            j + 1
        ));
    }
    acts.push_str(&format!(
        "(:action fix :parameters () :precondition (q{n}) :effect (p))\n\
         (:action win :parameters () :precondition (and (p) (q{n})) :effect (gw))\n"
    ));
    for i in 0..u {
        acts.push_str(&format!(
            "(:action flip{i} :parameters () :precondition (tf{i}) :effect (and (tn{i}) (not (tf{i}))))\n\
             (:action flop{i} :parameters () :precondition (tn{i}) :effect (and (tf{i}) (not (tn{i}))))\n"
        ));
    }
    let tfs: String = (0..u).map(|i| format!(" (tf{i})")).collect();
    (
        format!("(define (domain pairlock) (:predicates{preds}) {acts})"),
        format!("(define (problem pl) (:domain pairlock) (:init (p){tfs}) (:goal (gw)))"),
    )
}

/// The IPC child-snack shape, inline (kitchen → sandwich → tray → table),
/// with `nc` children (`na` of them gluten-allergic), exactly-sufficient
/// gluten-free supplies, and interchangeable everything — the factorial
/// core three engines deep at 0.21.
fn childsnack_mini(nc: usize, na: usize, trays: usize) -> (String, String) {
    let dom = "(define (domain child-snack)
      (:requirements :typing)
      (:types child bread-portion content-portion sandwich tray place)
      (:predicates (at_kitchen_bread ?b - bread-portion)
                   (at_kitchen_content ?c - content-portion)
                   (at_kitchen_sandwich ?s - sandwich)
                   (no_gluten_bread ?b - bread-portion)
                   (no_gluten_content ?c - content-portion)
                   (ontray ?s - sandwich ?t - tray)
                   (no_gluten_sandwich ?s - sandwich)
                   (allergic_gluten ?c - child)
                   (not_allergic_gluten ?c - child)
                   (served ?c - child)
                   (waiting ?c - child ?p - place)
                   (at ?t - tray ?p - place)
                   (notexist ?s - sandwich))
      (:action make_sandwich_no_gluten
        :parameters (?s - sandwich ?b - bread-portion ?c - content-portion)
        :precondition (and (at_kitchen_bread ?b) (at_kitchen_content ?c)
                           (no_gluten_bread ?b) (no_gluten_content ?c) (notexist ?s))
        :effect (and (not (at_kitchen_bread ?b)) (not (at_kitchen_content ?c))
                     (at_kitchen_sandwich ?s) (no_gluten_sandwich ?s) (not (notexist ?s))))
      (:action make_sandwich
        :parameters (?s - sandwich ?b - bread-portion ?c - content-portion)
        :precondition (and (at_kitchen_bread ?b) (at_kitchen_content ?c) (notexist ?s))
        :effect (and (not (at_kitchen_bread ?b)) (not (at_kitchen_content ?c))
                     (at_kitchen_sandwich ?s) (not (notexist ?s))))
      (:action put_on_tray
        :parameters (?s - sandwich ?t - tray)
        :precondition (and (at_kitchen_sandwich ?s) (at ?t kitchen))
        :effect (and (not (at_kitchen_sandwich ?s)) (ontray ?s ?t)))
      (:action serve_sandwich_no_gluten
        :parameters (?s - sandwich ?c - child ?t - tray ?p - place)
        :precondition (and (allergic_gluten ?c) (ontray ?s ?t) (waiting ?c ?p)
                           (no_gluten_sandwich ?s) (at ?t ?p))
        :effect (and (not (ontray ?s ?t)) (served ?c)))
      (:action serve_sandwich
        :parameters (?s - sandwich ?c - child ?t - tray ?p - place)
        :precondition (and (not_allergic_gluten ?c) (waiting ?c ?p) (ontray ?s ?t) (at ?t ?p))
        :effect (and (not (ontray ?s ?t)) (served ?c)))
      (:action move_tray
        :parameters (?t - tray ?p1 - place ?p2 - place)
        :precondition (at ?t ?p1)
        :effect (and (not (at ?t ?p1)) (at ?t ?p2))))";
    let mut objs = String::new();
    for i in 1..=nc {
        objs.push_str(&format!(" child{i}"));
    }
    objs.push_str(" - child");
    for i in 1..=nc {
        objs.push_str(&format!(" bread{i}"));
    }
    objs.push_str(" - bread-portion");
    for i in 1..=nc {
        objs.push_str(&format!(" content{i}"));
    }
    objs.push_str(" - content-portion");
    for i in 1..=trays {
        objs.push_str(&format!(" tray{i}"));
    }
    objs.push_str(" - tray");
    for i in 1..=nc {
        objs.push_str(&format!(" sandw{i}"));
    }
    objs.push_str(" - sandwich kitchen table1 table2 table3 - place");
    let mut init = String::new();
    for i in 1..=trays {
        init.push_str(&format!(" (at tray{i} kitchen)"));
    }
    for i in 1..=nc {
        init.push_str(&format!(
            " (at_kitchen_bread bread{i}) (at_kitchen_content content{i}) (notexist sandw{i})"
        ));
    }
    // Exactly-sufficient gluten-free supplies for the allergic block.
    for i in 1..=na {
        init.push_str(&format!(
            " (no_gluten_bread bread{i}) (no_gluten_content content{i})"
        ));
    }
    for i in 1..=nc {
        let table = 1 + (i % 3);
        if i <= na {
            init.push_str(&format!(" (allergic_gluten child{i})"));
        } else {
            init.push_str(&format!(" (not_allergic_gluten child{i})"));
        }
        init.push_str(&format!(" (waiting child{i} table{table})"));
    }
    let goals: String = (1..=nc).map(|i| format!(" (served child{i})")).collect();
    (
        dom.to_string(),
        format!(
            "(define (problem cs-mini) (:domain child-snack) (:objects{objs}) (:init{init}) (:goal (and{goals})))"
        ),
    )
}

// ---------------------------------------------------------------------------
// The RED records (permanent, the ladder_wall.rs convention): today's h-free
// rung — novelty-LIGHT, goal-count cells, width 1 — caps on both shapes.
// These legs pin the mechanism the driver's levers exist to fix.
// ---------------------------------------------------------------------------

#[test]
fn light_rung_caps_on_combolock_the_red_record() {
    let (dom, prb) = combolock(10, 20);
    let task = ground(&dom, &prb);
    assert!(
        novelty::search_light(&task, 3_000, &[]).is_none(),
        "goal-count width-1 was expected to cap on the combo lock (the RED shape; \
         calibrated: it needs 16,384 pops)"
    );
}

#[test]
fn light_rung_caps_on_pairlock_the_red_record() {
    let (dom, prb) = pairlock(40, 10);
    let task = ground(&dom, &prb);
    assert!(
        novelty::search_light(&task, 700, &[]).is_none(),
        "width-1 was expected to cap on the pair conjunction (the RED shape)"
    );
}

// ---------------------------------------------------------------------------
// The GREEN half: each lever measured alone, its hatched twin the RED.
// ---------------------------------------------------------------------------

/// Lever 1 (the R-partition): partitioning by achieved-R re-arms novelty
/// at every lock stage; goal-count cells (the `FF_NOV_PART=0` shape) stay
/// flat and BFS-grind the lawn to the cap. Width 2 is OFF in both legs so
/// the A/B prices lever 1 ALONE. Calibrated on this box: 86 pops
/// partitioned vs 16,384 goal-count — a 190× separation.
#[test]
fn r_partition_beats_goal_count() {
    let (dom, prb) = combolock(10, 20);
    let task = ground(&dom, &prb);
    let part_only = DriverCfg {
        partition: true,
        width2: false,
        ..DriverCfg::default()
    };
    let goal_count_only = DriverCfg {
        partition: false,
        width2: false,
        ..DriverCfg::default()
    };
    let (ops, evaluated) = novelty::search_driver(&task, 3_000, &[], None, &part_only)
        .expect("the partitioned driver should walk the combo lock");
    assert_valid(&task, &ops);
    assert!(
        evaluated < 500,
        "partitioned pops should sit two orders under the cap, got {evaluated}"
    );
    assert!(
        novelty::search_driver(&task, 3_000, &[], None, &goal_count_only).is_none(),
        "goal-count cells were expected to cap (the FF_NOV_PART=0 RED shape)"
    );
    // The default (all levers on) must keep the win.
    let (ops, _) = novelty::search_driver(&task, 3_000, &[], None, &DriverCfg::default())
        .expect("the default driver keeps the combo-lock win");
    assert_valid(&task, &ops);
}

/// Lever 2 (width 2): the (p, qn) state carries zero new atoms but a new
/// R-pair — the novel-2 rank pops it ahead of the non-novel lawn; width 1
/// (the `FF_NOV_W2=0` shape) buries it and caps. Partition is OFF in both
/// legs so the A/B prices lever 2 ALONE.
#[test]
fn width2_pair_conjunction() {
    let (dom, prb) = pairlock(40, 10);
    let task = ground(&dom, &prb);
    let w2_only = DriverCfg {
        partition: false,
        width2: true,
        ..DriverCfg::default()
    };
    let w1_only = DriverCfg {
        partition: false,
        width2: false,
        ..DriverCfg::default()
    };
    let (ops, evaluated) = novelty::search_driver(&task, 700, &[], None, &w2_only)
        .expect("the width-2 rank should pop the pair conjunction");
    assert_valid(&task, &ops);
    assert!(
        evaluated < 200,
        "novel-2 should jump the lawn, got {evaluated} pops"
    );
    assert!(
        novelty::search_driver(&task, 700, &[], None, &w1_only).is_none(),
        "width-1 was expected to cap (the FF_NOV_W2=0 RED shape)"
    );
    // The default (partition on too) must keep the win.
    let (ops, _) = novelty::search_driver(&task, 700, &[], None, &DriverCfg::default())
        .expect("the default driver keeps the pair-conjunction win");
    assert_valid(&task, &ops);
}

/// The R-extraction unit pin on sailing-band (the 0.21 charge's witness).
/// Sailing's shape is the INSTRUCTIVE edge: `crewed` is static (grounding
/// prunes it from preconditions), so the whole approach machinery is
/// numeric and R collapses to exactly the goal — the bit-partition has
/// nothing to hold there. (This edge is exactly the gap the
/// FF_NOV_NUMFEAT rider was pitched at; the rider left the tree at 0.23
/// null-armed — see DriverCfg's archaeology note — and the edge is
/// pinned here so the record of WHY stays executable.)
#[test]
fn r_extraction_pin_on_sailing_band() {
    let dom = include_str!("../../../benchmarks/bench/sailing-band-domain.pddl");
    let prb = include_str!("../../../benchmarks/bench/sailing-band-i1.pddl");
    let task = ground(dom, prb);
    let init = task.initial();
    let r = novelty::r_partition_facts(&task, &init, &task.goal_pos, &task.goal_num, 256);
    let mut goals = task.goal_pos.clone();
    goals.sort_unstable();
    assert_eq!(
        r, goals,
        "sailing-band's R is exactly the goal: statics pruned, progress numeric"
    );
}

/// The R-extraction pin on a propositional task: R = goal ∪ stamps,
/// lowest-fact_layer-first under the cap (`FF_NOV_R_CAP`'s mechanism) —
/// layer-0 stamps (init-true preconditions) survive a tight cap ahead of
/// deeper facts, and the capped set is a subset of the full one.
#[test]
fn r_extraction_cap_keeps_lowest_layers() {
    let (dom, prb) = combolock(6, 8);
    let task = ground(&dom, &prb);
    let init = task.initial();
    let r = novelty::r_partition_facts(&task, &init, &task.goal_pos, &task.goal_num, 256);
    for &g in &task.goal_pos {
        assert!(r.contains(&g), "R must contain every goal fact");
    }
    assert!(
        r.len() > task.goal_pos.len() + 8,
        "R must carry the c-chain and toggle stamps, got {} facts",
        r.len()
    );
    let r3 = novelty::r_partition_facts(&task, &init, &task.goal_pos, &task.goal_num, 3);
    assert_eq!(r3.len(), 3, "the cap must bite");
    for f in &r3 {
        assert!(
            ferroplan::bitset::test(&init.bits, *f as usize),
            "a tight cap keeps only layer-0 stamps (init-true facts), got fact {f}"
        );
        assert!(r.contains(f), "the capped R is a subset of the full R");
    }
}

/// The negative control, pinned so the centerpiece's referees stay
/// undiluted: the child-snack shape (factorial symmetry, Phase 6's pot)
/// stays UNSOLVED under the driver rung's own cap. If this ever turns
/// green the pot moves phases — it does not dilute this one.
#[test]
fn childsnack_shape_stays_unsolved_under_the_rung_cap() {
    let (dom, prb) = childsnack_mini(8, 4, 3);
    let task = ground(&dom, &prb);
    assert!(
        novelty::search_driver(&task, 400_000, &[], None, &DriverCfg::default()).is_none(),
        "the child-snack shape must stay symmetry's pot, not novelty's"
    );
}

/// Serial determinism smoke: the driver is one serial loop — two runs are
/// byte-identical (the ladder-level t1/t8 pin lives in the child battery).
#[test]
fn driver_is_deterministic_across_runs() {
    let (dom, prb) = combolock(8, 10);
    let task = ground(&dom, &prb);
    let a = novelty::search_driver(&task, 10_000, &[], None, &DriverCfg::default());
    let b = novelty::search_driver(&task, 10_000, &[], None, &DriverCfg::default());
    assert_eq!(
        a.as_ref().map(|(p, e)| (p.clone(), *e)),
        b.as_ref().map(|(p, e)| (p.clone(), *e))
    );
    assert!(a.is_some());
}

/// ARCHAEOLOGY (0.23 Phase 1): this test pinned the diversify probe's
/// tie-seed mechanics (`FF_REFILL_DIVERSIFY`, 0.22 Phase 5B). The probe
/// left the tree null-armed — data-network i12, its one motivating
/// receipt, solves in round 1 on the 0.22 binary (identical 473,488
/// evals both ways), so the seed never fired and no sweep ever armed
/// the opt-in flag. What SURVIVES is the law the seed-0 leg pinned and
/// the removal makes unconditional: the best-first key carries no
/// jitter term, so two identical runs are byte-identical.
#[test]
fn best_first_key_is_jitter_free_and_deterministic() {
    let (dom, prb) = combolock(5, 6);
    let task = ground(&dom, &prb);
    let cfg0 = ferroplan::search::SearchCfg::from_weights(1.0, 5.0, Some(200_000));
    let r1 = ferroplan::search::search(&task, 1, cfg0);
    let r2 = ferroplan::search::search(&task, 1, cfg0);
    let plan_of = |r: &ferroplan::search::PlanResult| match r {
        ferroplan::search::PlanResult::Plan { ops, .. } => ops.clone(),
        _ => panic!("combolock(5,6) should solve"),
    };
    assert_eq!(plan_of(&r1), plan_of(&r2), "two runs are byte-identical");
    assert_valid(&task, &plan_of(&r1));
}

// ---------------------------------------------------------------------------
// The ladder battery (child processes, the ladder_wall.rs convention:
// FF_* are process-global). Pins: the canary (novelty-light still runs
// BEFORE the new rung), the FF_NOV_OLD byte-identity, the no-wall
// byte-identity, and t1 ≡ t8 through the ladder slot.
// ---------------------------------------------------------------------------

/// The tetris shape, distilled small (ladder_wall.rs's deceptlawn): EHC
/// dead-ends on the mutex pair, LAMA is hatched off in the rung legs, and
/// the novelty slot gets the dispatch.
fn deceptmini(k: usize, m: usize) -> (String, String) {
    let mut preds = String::from(" (win) (fr) (mua) (mub)");
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
    let offs: String = (0..k).map(|i| format!(" (off{i})")).collect();
    (
        format!("(define (domain deceptmini) (:predicates{preds}) {acts})"),
        format!("(define (problem dm) (:domain deceptmini) (:init{offs} (fr)) (:goal (win)))"),
    )
}

fn run_child(scenario: &str, extra_env: &[(&str, &str)]) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "driver_ladder_pins", "--nocapture"])
        .env("NOVDRIVER_CHILD", scenario)
        .env("FF_WALL_DEBUG", "1");
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child {scenario} failed");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn child_plan(stdout: &str) -> Vec<usize> {
    let line = stdout
        .lines()
        .find(|l| l.starts_with("CHILD-PLAN:"))
        .expect("child plan line");
    line["CHILD-PLAN:".len()..]
        .split_whitespace()
        .map(|t| t.parse().unwrap())
        .collect()
}

fn child_evals(stdout: &str) -> usize {
    stdout
        .lines()
        .find(|l| l.starts_with("CHILD-EVALS:"))
        .expect("child evals line")["CHILD-EVALS:".len()..]
        .trim()
        .parse()
        .unwrap()
}

#[test]
fn driver_ladder_pins() {
    if let Ok(scenario) = std::env::var("NOVDRIVER_CHILD") {
        match scenario.as_str() {
            // The rung legs: EHC dies on the mutex, LAMA hatched off, the
            // novelty slot dispatches — old rung or driver per FF_NOV_OLD.
            "old-rung" | "driver-rung" => {
                let (dom, prb) = deceptmini(8, 12);
                let task = ground(&dom, &prb);
                let cfg = ferroplan::search::SearchCfg::from_weights(1.0, 5.0, None);
                let o = ferroplan::search::plan(&task, 1, cfg, true, None);
                let ops = o.ops.expect("rung leg must solve");
                let rendered: Vec<String> = ops.iter().map(|o| o.to_string()).collect();
                println!("CHILD-PLAN:{}", rendered.join(" "));
                println!("CHILD-EVALS:{}", o.evaluated);
            }
            // The no-wall byte-identity legs: without an armed wall the
            // novelty slot never arms — FF_NOV_OLD must be invisible.
            "no-wall" | "no-wall-oldhatch" => {
                let (dom, prb) = deceptmini(8, 12);
                let task = ground(&dom, &prb);
                let cfg = ferroplan::search::SearchCfg::from_weights(1.0, 5.0, None);
                let o = ferroplan::search::plan(&task, 1, cfg, true, None);
                let ops = o.ops.expect("no-wall leg must solve");
                let rendered: Vec<String> = ops.iter().map(|o| o.to_string()).collect();
                println!("CHILD-PLAN:{}", rendered.join(" "));
                println!("CHILD-EVALS:{}", o.evaluated);
            }
            // t1 ≡ t8 through the ladder slot (the driver is serial; the
            // pin guards any future parallelization of the rung).
            "threads" => {
                let (dom, prb) = deceptmini(8, 12);
                let task = ground(&dom, &prb);
                let cfg = ferroplan::search::SearchCfg::from_weights(1.0, 5.0, None);
                let o1 = ferroplan::search::plan(&task, 1, cfg, true, None);
                let o8 = ferroplan::search::plan(&task, 8, cfg, true, None);
                assert_eq!(o1.ops, o8.ops, "t1 and t8 plans must be identical");
                assert_eq!(o1.evaluated, o8.evaluated, "t1/t8 eval counts too");
                println!("CHILD-THREADS-OK:{}", o1.ops.is_some());
            }
            // The ladder-order canary: a visit-all mini under an armed
            // wall is still solved by novelty-LIGHT, before the new rung
            // (the 10x10 grid — the light rung's own separation fixture;
            // EHC walks the 7x7 on its own).
            "canary" => {
                let dom = include_str!("../../../benchmarks/bench/visitgrid-domain.pddl");
                let prb = include_str!("../../../benchmarks/bench/visitgrid-10x10.pddl");
                let opts = ferroplan::Options {
                    threads: 1,
                    ..Default::default()
                };
                let sol = ferroplan::solve(dom, prb, &opts).unwrap();
                println!("CHILD-CANARY-SOLVED:{}", sol.solved);
            }
            other => panic!("unknown scenario {other}"),
        }
        return;
    }

    // FF_NOV_OLD=1 reproduces the 0.21 rung EXACTLY: the ladder's plan and
    // eval count equal a direct call into the untouched rung code.
    let (dom, prb) = deceptmini(8, 12);
    let task = ground(&dom, &prb);
    let rung_env = [
        ("FF_NOVELTY", "1"),
        ("FF_NO_LAMA", "1"),
        ("FF_NOV_OLD", "1"),
    ];
    let (stdout, stderr) = run_child("old-rung", &rung_env);
    let (direct_ops, direct_evals) =
        novelty::search(&task, 1, 400_000, &[], None).expect("direct 0.21 rung solve");
    assert_eq!(
        child_plan(&stdout),
        direct_ops,
        "FF_NOV_OLD must reproduce the 0.21 rung's plan"
    );
    assert_eq!(
        child_evals(&stdout),
        direct_evals,
        "FF_NOV_OLD must reproduce the 0.21 rung's eval count"
    );
    assert!(
        stderr.contains("wall: solved by novelty") && !stderr.contains("novelty-driver"),
        "the hatch must dispatch the OLD rung:\n{stderr}"
    );

    // The driver at the same slot: plan and evals equal a direct call.
    let (stdout, stderr) = run_child("driver-rung", &[("FF_NOVELTY", "1"), ("FF_NO_LAMA", "1")]);
    let (drv_ops, drv_evals) =
        novelty::search_driver(&task, 400_000, &[], None, &DriverCfg::default())
            .expect("direct driver solve");
    assert_eq!(
        child_plan(&stdout),
        drv_ops,
        "ladder driver == direct driver"
    );
    assert_eq!(child_evals(&stdout), drv_evals);
    assert!(
        stderr.contains("wall: solved by novelty-driver"),
        "the driver must own the slot's narration:\n{stderr}"
    );

    // No armed wall ⇒ the slot never arms ⇒ FF_NOV_OLD is invisible and
    // nothing novelty-shaped runs: byte-identical plans, LAMA's solve.
    let (out_a, err_a) = run_child("no-wall", &[]);
    let (out_b, _) = run_child("no-wall-oldhatch", &[("FF_NOV_OLD", "1")]);
    assert_eq!(
        child_plan(&out_a),
        child_plan(&out_b),
        "without an armed wall the hatch must be byte-invisible"
    );
    assert_eq!(child_evals(&out_a), child_evals(&out_b));
    assert!(
        !err_a.contains("solved by novelty"),
        "no novelty rung may run without a wall or FF_NOVELTY:\n{err_a}"
    );

    // t1 ≡ t8 through the ladder slot.
    let (stdout, _) = run_child("threads", &[("FF_NOVELTY", "1"), ("FF_NO_LAMA", "1")]);
    assert!(stdout.contains("CHILD-THREADS-OK:true"), "{stdout}");

    // The canary: the visit-all mini still belongs to novelty-LIGHT,
    // ahead of the new rung — the ladder order is untouched. EHC is stood
    // down through its own 5A slice knob (it walks the mini grid solo
    // otherwise — boards are where it plateaus), so the pin reads pure:
    // light dispatches, the driver is never entered.
    let (stdout, stderr) = run_child(
        "canary",
        &[("FF_TIME_LIMIT", "30"), ("FF_EHC_WALL_FRAC", "0.0001")],
    );
    assert!(stdout.contains("CHILD-CANARY-SOLVED:true"), "{stdout}");
    assert!(
        stderr.contains("wall: solved by novelty-light"),
        "the visit-all mini must stay the light rung's win:\n{stderr}"
    );
    assert!(
        !stderr.contains("novelty-driver"),
        "the driver must never be entered ahead of the light rung:\n{stderr}"
    );
}
