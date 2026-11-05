//! Red-black relaxed plans (0.31 Lane D, docs/roadmap-0.31.md;
//! `redblack.rs`, `vars.rs`).
//!
//! Three pins. (1) THE PLAN THE RELAXATION GETS WRONG BY CONSTRUCTION: a
//! shuttle -- one truck, two places, a package to bring home -- where
//! h^FF reads 3 (drive out, load, unload: the truck never left home in
//! the relaxation) and the true cost is 4; with the truck's location
//! black the repair inserts the drive back and red-black reads exactly 4.
//! An empty black set is h^FF to the unit. (2) THE PAINTING: only
//! RSE-invertible multi-valued variables are black, and the black causal
//! graph is acyclic -- on a task whose only op moves two tokens at once
//! (a 2-cycle in the causal graph, both tokens invertible) exactly one
//! of them is painted. (3) THE RUNG and THE SEAM: `FF_RUNG_ONLY=rb` solves
//! the shuttle and its notes name the rung and the painting;
//! `FF_RB=1` under LAMA solves it too, with the painting noted.

use std::collections::{HashMap, VecDeque};
use std::process::Command;
use std::sync::Arc;

use ferroplan::heuristic::{self, Scratch};
use ferroplan::packed::PackedTask;
use ferroplan::redblack::RedBlack;
use ferroplan::vars::Variables;

fn ground(dom: &str, prb: &str) -> (ferroplan::types::Domain, PackedTask) {
    let d = ferroplan::parser::parse_domain(dom).unwrap();
    let p = ferroplan::parser::parse_problem(prb).unwrap();
    let t = ferroplan::ground::ground_task(&d, &p, 1).unwrap();
    (d, t)
}

/// One truck at `l0`, a package at `l1`, the goal the package at `l0`.
fn shuttle() -> (String, String) {
    (
        "(define (domain shuttle)
          (:requirements :strips :typing)
          (:types truck place pkg)
          (:predicates (at ?t - truck ?l - place) (pat ?p - pkg ?l - place)
                       (in ?p - pkg ?t - truck) (road ?a - place ?b - place))
          (:action drive
            :parameters (?t - truck ?a - place ?b - place)
            :precondition (and (at ?t ?a) (road ?a ?b))
            :effect (and (at ?t ?b) (not (at ?t ?a))))
          (:action load
            :parameters (?p - pkg ?t - truck ?l - place)
            :precondition (and (pat ?p ?l) (at ?t ?l))
            :effect (and (in ?p ?t) (not (pat ?p ?l))))
          (:action unload
            :parameters (?p - pkg ?t - truck ?l - place)
            :precondition (and (in ?p ?t) (at ?t ?l))
            :effect (and (pat ?p ?l) (not (in ?p ?t)))))"
            .into(),
        "(define (problem sh) (:domain shuttle)
          (:objects t - truck l0 l1 - place p - pkg)
          (:init (at t l0) (pat p l1) (road l0 l1) (road l1 l0))
          (:goal (pat p l0)))"
            .into(),
    )
}

/// Two tokens on a three-place line, moved ONLY together: every op
/// changes both, so the causal graph has `x <-> y`, and both are
/// invertible (the joint move back undoes either half).
fn twins() -> (String, String) {
    (
        "(define (domain twins)
          (:requirements :strips)
          (:predicates (xat ?l) (yat ?l) (adj ?a ?b))
          (:action both
            :parameters (?a ?b ?c ?d)
            :precondition (and (xat ?a) (adj ?a ?b) (yat ?c) (adj ?c ?d))
            :effect (and (xat ?b) (not (xat ?a)) (yat ?d) (not (yat ?c)))))"
            .into(),
        "(define (problem tw) (:domain twins) (:objects l0 l1 l2)
          (:init (xat l0) (yat l2) (adj l0 l1) (adj l1 l0) (adj l1 l2) (adj l2 l1))
          (:goal (and (xat l2) (yat l0))))"
            .into(),
    )
}

/// The true unit-cost distance from the initial state to a goal state.
fn exact_cost(task: &PackedTask) -> usize {
    let init = task.initial();
    let mut seen: HashMap<ferroplan::packed::StateKey, usize> = HashMap::new();
    seen.insert(task.state_key(&init), 0);
    let mut q = VecDeque::from([(init, 0usize)]);
    let mut cands = Vec::new();
    while let Some((s, d)) = q.pop_front() {
        if task.goal_met(&s) {
            return d;
        }
        cands.clear();
        task.applicable_ops(&s, &mut cands);
        for &oi in &cands {
            let ns = task.apply(oi as usize, &s);
            let k = task.state_key(&ns);
            if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(k) {
                e.insert(d + 1);
                q.push_back((ns, d + 1));
            }
        }
    }
    panic!("the fixture is solvable");
}

#[test]
fn the_drive_back_is_counted() {
    let (dom, prb) = shuttle();
    let (d, t) = ground(&dom, &prb);
    let vars = Arc::new(Variables::from_task(&d, &t));
    let truck_fact = t.fact_id("(AT T L0)").expect("the truck's fact");
    let truck = vars.fact_var[truck_fact];
    assert!(
        vars.vars[truck as usize].len() >= 2,
        "the truck's location is one multi-valued variable: {:?}",
        vars.vars
    );

    let s0 = t.initial();
    let mut sc = Scratch::new(&t);
    let h_ff = heuristic::relaxed(&t, &mut sc, &s0.bits, &s0.fv, &s0.fdef).unwrap();
    assert_eq!(h_ff, 3, "the relaxation never drives back");
    assert_eq!(exact_cost(&t), 4);

    let rb = RedBlack::build(&t, vars.clone(), 8);
    assert!(
        rb.black.contains(&truck),
        "the truck is painted black: {}",
        rb.note(&t)
    );
    let (h_rb, helpful) = rb.h(&t, &mut sc, &s0, &t.goal_pos, &t.goal_num).unwrap();
    assert!(h_rb > h_ff, "red-black {h_rb} must see past h^FF {h_ff}");
    assert_eq!(h_rb, 4, "the repaired plan is the true plan here");
    assert!(!helpful.is_empty(), "the helpful set is h^FF's, untouched");

    // Nothing black: h^FF to the unit.
    let none = RedBlack::build(&t, vars, 0);
    assert!(none.black.is_empty());
    let (h0, _) = none.h(&t, &mut sc, &s0, &t.goal_pos, &t.goal_num).unwrap();
    assert_eq!(h0, h_ff);
}

#[test]
fn the_black_set_is_invertible_and_acyclic() {
    for (dom, prb) in [shuttle(), twins()] {
        let (d, t) = ground(&dom, &prb);
        let vars = Arc::new(Variables::from_task(&d, &t));
        let rb = RedBlack::build(&t, vars.clone(), 8);
        assert!(rb.black_acyclic(), "{}", rb.note(&t));
        for &v in &rb.black {
            assert!(rb.invertible[v as usize], "black but not invertible: {v}");
            assert!(vars.vars[v as usize].len() >= 2, "black but binary: {v}");
        }
    }

    // The twins: both tokens invertible, but painting both closes the
    // 2-cycle -- exactly one is black.
    let (dom, prb) = twins();
    let (d, t) = ground(&dom, &prb);
    let vars = Arc::new(Variables::from_task(&d, &t));
    let x = vars.fact_var[t.fact_id("(XAT L0)").expect("x's fact")];
    let y = vars.fact_var[t.fact_id("(YAT L2)").expect("y's fact")];
    assert_ne!(x, y);
    let rb = RedBlack::build(&t, vars, 8);
    assert!(
        rb.invertible[x as usize] && rb.invertible[y as usize],
        "{}",
        rb.note(&t)
    );
    assert_eq!(
        rb.black.iter().filter(|&&v| v == x || v == y).count(),
        1,
        "one twin black, the other red: {}",
        rb.note(&t)
    );
    // The repaired plan still reads a finite h on it.
    let s0 = t.initial();
    let mut sc = Scratch::new(&t);
    assert!(rb.h(&t, &mut sc, &s0, &t.goal_pos, &t.goal_num).is_some());
}

fn run_child(rung: &str, rb: bool) -> String {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "the_rung_and_the_seam", "--nocapture"])
        .env("REDBLACK_CHILD", "1")
        .env("FF_RUNG_ONLY", rung)
        .env("FF_TIME_LIMIT", "10");
    if rb {
        cmd.env("FF_RB", "1");
    } else {
        cmd.env_remove("FF_RB");
    }
    cmd.env_remove("FF_PARTITION_RUNG");
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child {rung} failed");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn field<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix(key))
        .unwrap_or_else(|| panic!("child printed no {key}:\n{stdout}"))
}

#[test]
fn the_rung_and_the_seam() {
    if std::env::var("REDBLACK_CHILD").is_ok() {
        let (dom, prb) = shuttle();
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-SOLVED:{}", sol.solved);
        println!(
            "CHILD-STEPS:{}",
            sol.plan.as_ref().map_or(0, |p| p.steps.len())
        );
        println!("CHILD-NOTES:{}", sol.notes.join(" | "));
        return;
    }

    // The rung alone: solves, names itself and the painting.
    let stdout = run_child("rb", false);
    assert_eq!(field(&stdout, "CHILD-SOLVED:"), "true", "{stdout}");
    assert_eq!(field(&stdout, "CHILD-STEPS:"), "4", "{stdout}");
    let notes = field(&stdout, "CHILD-NOTES:");
    assert!(notes.contains("rung: solved by red-black"), "{notes}");
    assert!(notes.contains("redblack: "), "{notes}");

    // The seam: LAMA under FF_RB=1 solves with the painting noted.
    let stdout = run_child("lama", true);
    assert_eq!(field(&stdout, "CHILD-SOLVED:"), "true", "{stdout}");
    let notes = field(&stdout, "CHILD-NOTES:");
    assert!(notes.contains("rung: solved by LAMA"), "{notes}");
    assert!(notes.contains("redblack: "), "{notes}");

    // Unset: no instance, no note.
    let stdout = run_child("lama", false);
    assert_eq!(field(&stdout, "CHILD-SOLVED:"), "true", "{stdout}");
    assert!(
        !field(&stdout, "CHILD-NOTES:").contains("redblack"),
        "{stdout}"
    );
}
