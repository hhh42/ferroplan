//! Mutex-group synthesis: simplified Helmert-style monotonicity invariants.
//!
//! This is the "SAS+ slice" — it produces **mutex groups** (sets of ground facts
//! of which at most one is ever true) WITHOUT switching the planner off its
//! propositional bitset state. The groups are exactly what a SAS+ representation
//! would give as multi-valued variables, and they are the load-bearing
//! dependency for SGPlan/ESPC subgoal partitioning (see `pddl3.rs`).
//!
//! Approach (single-predicate monotonicity invariants — the common case):
//!   - A candidate is `(predicate p, counted argument position c)`: for any fixing
//!     of the other positions, at most one ground atom `p(…)` (varying only at `c`)
//!     is true. The group for each fixed binding is those ground atoms.
//!   - A candidate is verified against the **lifted** action effects: every action
//!     must keep the per-group count from increasing — each add of `p` must be
//!     paired with a delete of `p` that agrees on the fixed positions, and no
//!     action may add two atoms with the same fixed binding. Conditional (`when`)
//!     and universal (`forall`) effects on `p` are treated conservatively (reject).
//!   - Finally each instantiated group must hold `at most one` true in the initial
//!     state. Anything uncertain is rejected, so emitted groups are always sound.

use crate::packed::PackedTask;
use crate::types::{Domain, Effect, Term};
use crate::{bitset, hash::FxHashMap};

/// Synthesize sound mutex groups (each a set of ground fact ids, ≥2 members, at
/// most one true at a time) for a grounded task.
pub fn synthesize(domain: &Domain, task: &PackedTask) -> Vec<Vec<u32>> {
    // fact id -> (predicate, args); None for compiled `(NOT …)` facts / arity 0.
    let keys: Vec<Option<(String, Vec<String>)>> =
        task.fact_names.iter().map(|s| parse_fact(s)).collect();

    // Index ground fact ids by (lowercased) predicate name.
    let mut by_pred: FxHashMap<String, Vec<u32>> = FxHashMap::default();
    for (id, k) in keys.iter().enumerate() {
        if let Some((p, _)) = k {
            by_pred
                .entry(p.to_ascii_lowercase())
                .or_default()
                .push(id as u32);
        }
    }

    let mut groups: Vec<Vec<u32>> = Vec::new();
    for (name, args) in &domain.predicates {
        let arity = args.len();
        if arity == 0 {
            continue;
        }
        let ids = match by_pred.get(&name.to_ascii_lowercase()) {
            Some(v) if v.len() >= 2 => v,
            _ => continue,
        };
        for counted in 0..arity {
            if !lifted_safe(domain, name, arity, counted) {
                continue;
            }
            if let Some(mut gs) = instantiate(ids, &keys, counted, arity, task) {
                groups.append(&mut gs);
            }
        }
    }

    // dedup identical groups
    for g in &mut groups {
        g.sort_unstable();
    }
    groups.sort();
    groups.dedup();
    groups
}

/// Parse a rendered fact name `"(pred a0 a1)"` into `(pred, [args])`.
/// Returns None for `"(NOT …)"` compiled facts and 0-arity facts.
fn parse_fact(s: &str) -> Option<(String, Vec<String>)> {
    let inner = s.strip_prefix('(')?.strip_suffix(')')?.trim();
    if inner.is_empty() {
        return None;
    }
    let mut toks = inner.split_whitespace();
    let pred = toks.next()?;
    if pred.eq_ignore_ascii_case("not") {
        return None;
    }
    let args: Vec<String> = toks.map(|t| t.to_string()).collect();
    if args.is_empty() {
        return None;
    }
    Some((pred.to_string(), args))
}

#[derive(Default)]
struct PredEffects {
    adds: Vec<Vec<Term>>,
    dels: Vec<Vec<Term>>,
    /// `p` appears under a `when`/`forall` — we can't verify the balance, reject.
    guarded: bool,
}

fn collect(e: &Effect, p: &str, out: &mut PredEffects, guarded: bool) {
    match e {
        Effect::And(es) => {
            for x in es {
                collect(x, p, out, guarded);
            }
        }
        Effect::Add(n, a) if n.eq_ignore_ascii_case(p) => {
            out.guarded |= guarded;
            out.adds.push(a.clone());
        }
        Effect::Del(n, a) if n.eq_ignore_ascii_case(p) => {
            out.guarded |= guarded;
            out.dels.push(a.clone());
        }
        Effect::When(_, inner) => collect(inner, p, out, true),
        Effect::Forall(_, inner) => collect(inner, p, out, true),
        _ => {}
    }
}

fn term_eq(a: &Term, b: &Term) -> bool {
    match (a, b) {
        (Term::Var(x), Term::Var(y)) => x.eq_ignore_ascii_case(y),
        (Term::Const(x), Term::Const(y)) => x.eq_ignore_ascii_case(y),
        _ => false,
    }
}

/// Two atoms of the same predicate agree on every position except `counted`.
fn fixed_match(a: &[Term], b: &[Term], counted: usize) -> bool {
    a.len() == b.len() && (0..a.len()).all(|i| i == counted || term_eq(&a[i], &b[i]))
}

/// Does every action preserve the `at most one` count for `(p, counted)`?
fn lifted_safe(domain: &Domain, p: &str, arity: usize, counted: usize) -> bool {
    for act in &domain.actions {
        let mut pe = PredEffects::default();
        collect(&act.effect, p, &mut pe, false);
        if pe.guarded {
            return false;
        }
        // arity sanity (skip malformed)
        if pe
            .adds
            .iter()
            .chain(pe.dels.iter())
            .any(|t| t.len() != arity)
        {
            return false;
        }
        // every add must be paired with a del on the same fixed binding
        for add in &pe.adds {
            if !pe.dels.iter().any(|del| fixed_match(add, del, counted)) {
                return false;
            }
        }
        // no two adds with the same fixed binding (could go 0 -> 2)
        for i in 0..pe.adds.len() {
            for j in (i + 1)..pe.adds.len() {
                if fixed_match(&pe.adds[i], &pe.adds[j], counted) {
                    return false;
                }
            }
        }
    }
    true
}

/// Partition ground atoms of a verified candidate by their fixed-arg tuple, keep
/// groups of ≥2 — but only if the whole candidate is `at most one` true in init.
fn instantiate(
    ids: &[u32],
    keys: &[Option<(String, Vec<String>)>],
    counted: usize,
    arity: usize,
    task: &PackedTask,
) -> Option<Vec<Vec<u32>>> {
    let mut by_fixed: FxHashMap<Vec<String>, Vec<u32>> = FxHashMap::default();
    for &id in ids {
        let (_, a) = keys[id as usize].as_ref()?;
        if a.len() != arity {
            return None;
        }
        let fixed: Vec<String> = a
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != counted)
            .map(|(_, s)| s.to_ascii_lowercase())
            .collect();
        by_fixed.entry(fixed).or_default().push(id);
    }
    let mut out = Vec::new();
    for (_, g) in by_fixed {
        // init-consistency: at most one of the group true initially
        let true_in_init = g
            .iter()
            .filter(|&&id| bitset::test(&task.init_bits, id as usize))
            .count();
        if true_in_init > 1 {
            return None; // candidate is not an `at most one` invariant
        }
        if g.len() >= 2 {
            out.push(g);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::{ground, Outcome};
    use crate::parser::{parse_domain, parse_problem};

    fn groups_of(dom: &str, prob: &str) -> (PackedTask, Vec<Vec<u32>>) {
        let d = parse_domain(dom).expect("domain");
        let p = parse_problem(prob).expect("problem");
        let task = match ground(&d, &p, 1) {
            Outcome::Task(t) => t,
            _ => panic!("expected a task"),
        };
        let g = synthesize(&d, &task);
        (task, g)
    }

    fn group_names(task: &PackedTask, g: &[u32]) -> Vec<String> {
        let mut v: Vec<String> = g
            .iter()
            .map(|&i| task.fact_names[i as usize].clone())
            .collect();
        v.sort();
        v
    }

    const LOGISTICS: &str = "(define (domain log)
      (:requirements :strips :typing)
      (:types truck location)
      (:predicates (at ?t - truck ?l - location) (road ?a ?b - location))
      (:action drive :parameters (?t - truck ?from ?to - location)
        :precondition (and (at ?t ?from) (road ?from ?to))
        :effect (and (not (at ?t ?from)) (at ?t ?to))))";
    const LOG_PROB: &str = "(define (problem p) (:domain log)
      (:objects t1 - truck a b c - location)
      (:init (at t1 a) (road a b) (road b c) (road a c))
      (:goal (at t1 c)))";

    #[test]
    fn truck_location_is_a_mutex_group() {
        let (task, groups) = groups_of(LOGISTICS, LOG_PROB);
        // the truck's `at` over {a,b,c} is a single at-most-one group of size 3
        let truck_grp = groups
            .iter()
            .find(|g| {
                group_names(&task, g)
                    .iter()
                    .all(|n| n.to_lowercase().starts_with("(at t1"))
            })
            .expect("truck `at` mutex group");
        assert_eq!(truck_grp.len(), 3);
    }

    #[test]
    fn static_road_is_not_a_group() {
        // `road` is static and has many edges from `a` in init -> not at-most-one,
        // so no road-based group should survive the init-consistency filter.
        let (task, groups) = groups_of(LOGISTICS, LOG_PROB);
        for g in &groups {
            assert!(
                !group_names(&task, g)[0].to_lowercase().starts_with("(road"),
                "road must not form a mutex group"
            );
        }
    }

    const GRIPPER: &str = "(define (domain gripper)
      (:requirements :strips :typing)
      (:types room ball)
      (:predicates (at-robby ?r - room) (ball-at ?b - ball ?r - room) (carry ?b - ball))
      (:action move :parameters (?from ?to - room)
        :precondition (at-robby ?from)
        :effect (and (not (at-robby ?from)) (at-robby ?to)))
      (:action pick :parameters (?b - ball ?r - room)
        :precondition (and (ball-at ?b ?r) (at-robby ?r))
        :effect (and (not (ball-at ?b ?r)) (carry ?b)))
      (:action drop :parameters (?b - ball ?r - room)
        :precondition (and (carry ?b) (at-robby ?r))
        :effect (and (not (carry ?b)) (ball-at ?b ?r))))";
    const GRIP_PROB: &str = "(define (problem p) (:domain gripper)
      (:objects ra rb - room b1 - ball)
      (:init (at-robby ra) (ball-at b1 ra))
      (:goal (at-robby rb)))";

    #[test]
    fn robby_room_is_a_mutex_group() {
        let (task, groups) = groups_of(GRIPPER, GRIP_PROB);
        // (at-robby ?) over {ra, rb} is an at-most-one group (arity-1, all fixed empty)
        let robby = groups
            .iter()
            .find(|g| {
                group_names(&task, g)
                    .iter()
                    .all(|n| n.to_lowercase().starts_with("(at-robby"))
            })
            .expect("at-robby mutex group");
        assert_eq!(robby.len(), 2);
    }
}
