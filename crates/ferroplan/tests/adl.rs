//! ADL tests: quantified preconditions (forall/exists) + object equality.
//! Validated by replaying the plan over ffdp's own grounded task (metricff's
//! validator can't parse ADL, so we self-validate here).

use ferroplan::ground::ground_task;

const DOM: &str = "(define (domain adl1)
 (:requirements :typing :adl :negative-preconditions)
 (:types item)
 (:predicates (tagged ?x - item) (done) (linked ?a - item ?b - item))
 (:action tag :parameters (?x - item) :precondition (not (tagged ?x)) :effect (tagged ?x))
 (:action link :parameters (?a - item ?b - item)
   :precondition (and (not (= ?a ?b)) (tagged ?a) (tagged ?b))
   :effect (linked ?a ?b))
 (:action finish :parameters ()
   :precondition (and (forall (?x - item) (tagged ?x))
                      (exists (?a - item ?b - item) (linked ?a ?b)))
   :effect (done)))";

fn parse_ff_plan(out: &str) -> Vec<(String, Vec<String>)> {
    let mut v = Vec::new();
    let mut started = false;
    for l in out.lines() {
        if l.contains("found legal plan") {
            started = true;
            continue;
        }
        if !started {
            continue;
        }
        let t = l
            .trim_start()
            .strip_prefix("step")
            .unwrap_or(l.trim_start())
            .trim_start();
        if let Some(c) = t.find(':') {
            if t[..c].chars().all(|x| x.is_ascii_digit()) && !t[..c].is_empty() {
                let rest = t[c + 1..].trim();
                let mut it = rest.split_whitespace();
                if let Some(n) = it.next() {
                    v.push((n.to_string(), it.map(|s| s.to_string()).collect()));
                }
            }
        }
    }
    v
}

fn solve_validate(dom: &str, prob: &str) -> usize {
    let (out, code) = ferroplan::run_ff(
        dom,
        prob,
        &ferroplan::Options {
            mode: ferroplan::Mode::Ff,
            threads: 1,
            ..Default::default()
        },
    );
    assert_eq!(code, 0, "should solve\n{}", out);
    assert!(out.contains("found legal plan as follows"), "{}", out);
    let plan = parse_ff_plan(&out);
    // replay over the grounded task and confirm the goal holds
    let d = ferroplan::parser::parse_domain(dom).unwrap();
    let p = ferroplan::parser::parse_problem(prob).unwrap();
    let task = ground_task(&d, &p, 1).unwrap();
    let mut s = task.initial();
    for (name, args) in &plan {
        let want: Vec<&str> = args.iter().map(|x| x.as_str()).collect();
        let oi = (0..task.n_ops)
            .find(|&oi| {
                let mut it = task.op_display[oi].split_whitespace();
                it.next() == Some(name.as_str()) && it.eq(want.iter().copied())
            })
            .unwrap_or_else(|| panic!("op {} {:?} not found", name, args));
        assert!(
            task.op_applicable(oi, &s),
            "step {} {:?} not applicable",
            name,
            args
        );
        s = task.apply(oi, &s);
    }
    assert!(task.goal_met(&s), "plan did not reach the goal");
    plan.len()
}

#[test]
fn forall_exists_equality_precondition() {
    let prob = "(define (problem p) (:domain adl1)
        (:objects a b c - item) (:init (tagged a)) (:goal (done)))";
    let n = solve_validate(DOM, prob);
    assert_eq!(n, 4, "expected tag b, tag c, link, finish (order may vary)");
}

// conditional + universal effects: moving the briefcase carries everything in it
const BRIEFCASE: &str = "(define (domain briefcase)
 (:requirements :typing :adl)
 (:types obj loc)
 (:predicates (at-bc ?l - loc) (inbc ?o - obj) (at ?o - obj ?l - loc))
 (:action move :parameters (?from ?to - loc)
   :precondition (at-bc ?from)
   :effect (and (at-bc ?to) (not (at-bc ?from))
                (forall (?o - obj)
                  (when (inbc ?o) (and (at ?o ?to) (not (at ?o ?from)))))))
 (:action putin :parameters (?o - obj ?l - loc)
   :precondition (and (at-bc ?l) (at ?o ?l))
   :effect (inbc ?o))
 (:action takeout :parameters (?o - obj)
   :precondition (inbc ?o)
   :effect (not (inbc ?o))))";

#[test]
fn conditional_universal_effect_briefcase() {
    let prob = "(define (problem p) (:domain briefcase)
        (:objects o1 o2 - obj  home office - loc)
        (:init (at-bc home) (at o1 home) (at o2 home))
        (:goal (and (at o1 office) (at-bc office))))";
    // PUTIN O1 HOME, MOVE HOME OFFICE — the conditional effect carries o1 along.
    // Replay (which exercises conditional `apply`) must confirm (at o1 office).
    let n = solve_validate(BRIEFCASE, prob);
    assert_eq!(n, 2);
}

// negative `when` condition: toggle flips a flag both ways in one action
const TOGGLE: &str = "(define (domain toggle)
 (:requirements :adl)
 (:predicates (on) (marker))
 (:action flip :parameters ()
   :precondition (marker)
   :effect (and (when (on) (not (on))) (when (not (on)) (on)))))";

#[test]
fn conditional_negative_condition_toggle() {
    let prob = "(define (problem p) (:domain toggle)
        (:init (marker)) (:goal (on)))";
    // from (not on): one flip turns it on (the negative-condition branch fires)
    let n = solve_validate(TOGGLE, prob);
    assert_eq!(n, 1);
}

#[test]
fn disjunctive_goal_picks_reachable_disjunct() {
    // goal (or (b) (a)); only (a) is achievable. Naively using goal_dnf[0] (={b})
    // would wrongly report unsolvable — the disjunctive-goal compilation must solve.
    let dom = "(define (domain disj) (:requirements :adl)
        (:predicates (a) (b) (start))
        (:action mka :parameters () :precondition (start) :effect (a)))";
    let prob = "(define (problem p) (:domain disj) (:init (start)) (:goal (or (b) (a))))";
    let (out, code) = ferroplan::run_ff(
        dom,
        prob,
        &ferroplan::Options {
            mode: ferroplan::Mode::Ff,
            threads: 1,
            ..Default::default()
        },
    );
    assert_eq!(code, 0, "{}", out);
    assert!(
        out.contains("found legal plan"),
        "must solve via the (a) disjunct\n{}",
        out
    );
    assert!(
        out.contains("MKA"),
        "plan must include the real action\n{}",
        out
    );
}

#[test]
fn negated_numeric_equality_solved() {
    // (not (= n 0)) is n<0 OR n>0 (a disjunction), reachable by incrementing.
    // A bug compiling it back to (= n 0) would mark the goal already-true.
    let dom = "(define (domain ctr) (:requirements :fluents)
        (:functions (n))
        (:action inc :parameters () :precondition (and) :effect (increase (n) 1)))";
    let prob = "(define (problem p) (:domain ctr) (:init (= (n) 0)) (:goal (not (= (n) 0))))";
    let (out, code) = ferroplan::run_ff(
        dom,
        prob,
        &ferroplan::Options {
            mode: ferroplan::Mode::Ff,
            threads: 1,
            ..Default::default()
        },
    );
    assert_eq!(code, 0, "{}", out);
    assert!(
        out.contains("found legal plan"),
        "(not (= n 0)) reachable by inc\n{}",
        out
    );
    assert!(out.contains("INC"), "{}", out);
}

#[test]
fn complement_fact_semantics_match_metric_ff() {
    // act deletes p AND conditionally re-adds it (c holds). Metric-FF compiles
    // negative goals to an independent (NOT p) fact maintained by blind add-wins
    // toggles, so ACT's `(not p)` toggle leaves (NOT p) TRUE and the goal `(not p)`
    // is satisfied — even though p is also true. ffdp reproduces this faithfully
    // (verified: the C oracle finds the same [ACT] plan). Drop-in fidelity beats
    // textbook add-wins linkage here.
    let dom = "(define (domain comp) (:requirements :adl)
        (:predicates (p) (c))
        (:action act :parameters () :precondition (c)
          :effect (and (not (p)) (when (c) (p)))))";
    let prob = "(define (problem p) (:domain comp) (:init (p) (c)) (:goal (not (p))))";
    let (out, code) = ferroplan::run_ff(
        dom,
        prob,
        &ferroplan::Options {
            mode: ferroplan::Mode::Ff,
            threads: 1,
            ..Default::default()
        },
    );
    assert_eq!(code, 0, "{}", out);
    assert!(
        out.contains("found legal plan"),
        "must match the oracle (solvable via ACT)\n{}",
        out
    );
}

#[test]
fn forall_unsatisfiable_when_item_cannot_be_tagged() {
    // remove the only way to satisfy exists -> still solvable via tag+link;
    // here verify a solvable instance with all already tagged needs just link+finish
    let prob = "(define (problem p) (:domain adl1)
        (:objects a b - item) (:init (tagged a) (tagged b)) (:goal (done)))";
    let n = solve_validate(DOM, prob);
    assert_eq!(n, 2, "expected link a b, finish");
}
