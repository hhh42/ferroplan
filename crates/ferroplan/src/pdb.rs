//! Pattern databases (0.31 Lane C.1, docs/roadmap-0.31.md).
//!
//! An admissible heuristic from an ABSTRACTION: project the task onto a
//! PATTERN -- a few of the multi-valued variables of [`crate::vars`] -- and
//! solve the projection exactly, once, for every abstract state; the
//! distance from a state's projection to the nearest abstract goal is a
//! lower bound on its real cost (Haslum, Botea, Helmert, Bonet & Koenig
//! 2007, `docs/papers/haslum-etal-2007-ipdb.pdf`; Seipp, Keller & Helmert
//! 2020 for what to do with a collection of them). Every optimal-track
//! winner since 2014 carries this family; the engine had one admissible
//! heuristic, LM-cut, and 42 % of its unsolved rows on the proof boards.
//!
//! The projection is SOUND against the full task: every concrete
//! transition has an image. Positive preconditions on pattern facts become
//! value tests; effects on pattern facts set or clear a variable (a
//! deleted fact clears its variable only if it was the value -- the
//! groups are at-most-one, so this is exact); numeric preconditions and
//! effects are dropped (a superset of transitions, never a subset);
//! conditional effects that touch the pattern are made NONDETERMINISTIC
//! -- the op gets a variant per subset of them that may fire, the firing
//! subset's conditions on pattern facts as extra value tests -- or, past
//! three such effects on one op, the database is refused rather than
//! built wrong. The abstract goal is the goal facts' values on the
//! pattern; the numeric goal is dropped.
//!
//! The build enumerates every abstract state (a mixed-radix index), lays
//! the reverse transition graph out as CSR in two passes, and runs one
//! backward Dijkstra from the goal states. Bounded three ways -- states,
//! edges, and a deadline -- and a bound hit is a refusal with its reason,
//! never a partial table. A refusal costs the wall it spent and nothing
//! else: the A* runs on LM-cut as before.
//!
//! The pattern: this cycle's first selector is THE GOAL PATTERN -- the
//! variables that carry a goal fact, smallest domains first, as many as
//! fit under the state cap. iPDB hill-climbing and the collection with
//! saturated cost partitioning are the roadmap's next steps, judged on
//! the proof count and the build-time receipt first.
//!
//! The installed database travels to the A* through a thread-local (the
//! `LADDER_BASE` idiom): the optimal ladder builds several engines over
//! one task, and the one solve that built the table is the one they all
//! read. `install` returns a guard that clears it.

use std::cell::RefCell;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::sync::Arc;

use crate::packed::PackedTask;
use crate::vars::Variables;

/// What a build may spend before it refuses.
#[derive(Clone, Debug)]
pub struct Limits {
    pub max_states: usize,
    pub max_edges: usize,
    /// (start, seconds): the build checks the clock every few thousand
    /// states and refuses past it.
    pub deadline: Option<(crate::clock::Clock, f64)>,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_states: 1 << 20,
            max_edges: 1 << 25,
            deadline: None,
        }
    }
}

pub struct Pdb {
    vars: Arc<Variables>,
    /// Variable ids in the pattern, ascending.
    pub pattern: Vec<u32>,
    strides: Vec<u64>,
    dist: Vec<f64>,
    pub n_states: usize,
    pub n_edges: usize,
    pub build_secs: f64,
}

/// A projected effect on one pattern position.
#[derive(Clone, Copy, Debug)]
enum Eff {
    /// The variable takes this value.
    Set(u32),
    /// The variable becomes `none` if it held this value.
    ClearIf(u32),
}

/// One projected operator (or one nondeterministic variant of one).
#[derive(Clone, Debug)]
struct POp {
    pre: Vec<(u32, u32)>,
    dels: Vec<(u32, Eff)>,
    adds: Vec<(u32, Eff)>,
    cost: f64,
}

#[derive(Clone, Copy, PartialEq)]
struct Cost(f64);
impl Eq for Cost {}
impl PartialOrd for Cost {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Cost {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}

impl Pdb {
    /// The goal pattern: the variables carrying a goal fact, smallest
    /// domain first, as many as keep the abstract space under
    /// `max_states`. Empty when the first goal variable alone is over
    /// the cap.
    pub fn goal_pattern(vars: &Variables, task: &PackedTask, max_states: usize) -> Vec<u32> {
        let mut gv: Vec<u32> = task
            .goal_pos
            .iter()
            .map(|&f| vars.fact_var[f as usize])
            .collect();
        gv.sort_unstable();
        gv.dedup();
        gv.sort_by_key(|&v| (vars.size(v as usize), v));
        let mut pattern = Vec::new();
        let mut product: u128 = 1;
        for v in gv {
            let next = product * vars.size(v as usize) as u128;
            if next > max_states as u128 {
                break;
            }
            product = next;
            pattern.push(v);
        }
        pattern.sort_unstable();
        pattern
    }

    /// Build the database for `pattern` over `vars`, with `costs` per op.
    pub fn build(
        task: &PackedTask,
        vars: Arc<Variables>,
        pattern: Vec<u32>,
        costs: &[f64],
        limits: &Limits,
    ) -> Result<Pdb, String> {
        let t0 = crate::clock::Clock::now();
        if pattern.is_empty() {
            return Err("empty pattern".into());
        }
        let mut pos = vec![u32::MAX; vars.len()];
        for (i, &v) in pattern.iter().enumerate() {
            pos[v as usize] = i as u32;
        }
        let sizes: Vec<u32> = pattern.iter().map(|&v| vars.size(v as usize)).collect();
        let mut strides = vec![1u64; pattern.len()];
        let mut n: u128 = 1;
        for i in 0..pattern.len() {
            strides[i] = n as u64;
            n *= sizes[i] as u128;
            if n > limits.max_states as u128 {
                return Err(format!(
                    "abstract space over the cap ({n} > {} states)",
                    limits.max_states
                ));
            }
        }
        let n = n as usize;

        // Project the operators.
        let mut pops: Vec<POp> = Vec::new();
        for (oi, &cost) in costs.iter().enumerate().take(task.n_ops) {
            let mut pre: Vec<(u32, u32)> = Vec::new();
            let mut contradictory = false;
            for &f in task.pre_pos.slice(oi) {
                let p = pos[vars.fact_var[f as usize] as usize];
                if p == u32::MAX {
                    continue;
                }
                let val = vars.fact_val[f as usize];
                match pre.iter().find(|(q, _)| *q == p) {
                    Some((_, v)) if *v != val => contradictory = true,
                    Some(_) => {}
                    None => pre.push((p, val)),
                }
            }
            if contradictory {
                continue; // never applicable: two values of one variable
            }
            let mut dels: Vec<(u32, Eff)> = Vec::new();
            let mut adds: Vec<(u32, Eff)> = Vec::new();
            for &f in task.del.slice(oi) {
                let p = pos[vars.fact_var[f as usize] as usize];
                if p != u32::MAX {
                    dels.push((p, Eff::ClearIf(vars.fact_val[f as usize])));
                }
            }
            for &f in task.add.slice(oi) {
                let p = pos[vars.fact_var[f as usize] as usize];
                if p != u32::MAX {
                    adds.push((p, Eff::Set(vars.fact_val[f as usize])));
                }
            }
            // Conditional effects on the pattern: nondeterministic variants.
            let touching: Vec<&crate::packed::CondEff> = task
                .cond
                .slice(oi)
                .iter()
                .filter(|ce| {
                    ce.add
                        .iter()
                        .chain(ce.del.iter())
                        .any(|&f| pos[vars.fact_var[f as usize] as usize] != u32::MAX)
                })
                .collect();
            if touching.len() > 3 {
                return Err(format!(
                    "op `{}` has {} conditional effects on the pattern",
                    task.op_display[oi],
                    touching.len()
                ));
            }
            let base = POp {
                pre,
                dels,
                adds,
                cost,
            };
            if touching.is_empty() {
                if !base.dels.is_empty() || !base.adds.is_empty() {
                    pops.push(base);
                }
                continue;
            }
            for mask in 0..(1u32 << touching.len()) {
                let mut v = base.clone();
                let mut ok = true;
                for (k, ce) in touching.iter().enumerate() {
                    if mask & (1 << k) == 0 {
                        continue;
                    }
                    // The firing branch requires the condition's pattern
                    // facts; the other branch requires nothing it can say.
                    for &f in &ce.cond_pos {
                        let p = pos[vars.fact_var[f as usize] as usize];
                        if p == u32::MAX {
                            continue;
                        }
                        let val = vars.fact_val[f as usize];
                        match v.pre.iter().find(|(q, _)| *q == p) {
                            Some((_, pv)) if *pv != val => ok = false,
                            Some(_) => {}
                            None => v.pre.push((p, val)),
                        }
                    }
                    for &f in &ce.del {
                        let p = pos[vars.fact_var[f as usize] as usize];
                        if p != u32::MAX {
                            v.dels.push((p, Eff::ClearIf(vars.fact_val[f as usize])));
                        }
                    }
                    for &f in &ce.add {
                        let p = pos[vars.fact_var[f as usize] as usize];
                        if p != u32::MAX {
                            v.adds.push((p, Eff::Set(vars.fact_val[f as usize])));
                        }
                    }
                }
                if ok && (!v.dels.is_empty() || !v.adds.is_empty()) {
                    pops.push(v);
                }
            }
        }

        // Abstract goal: the goal facts' values on the pattern.
        let mut goal: Vec<(u32, u32)> = Vec::new();
        for &f in &task.goal_pos {
            let p = pos[vars.fact_var[f as usize] as usize];
            if p == u32::MAX {
                continue;
            }
            let val = vars.fact_val[f as usize];
            match goal.iter().find(|(q, _)| *q == p) {
                Some((_, v)) if *v != val => {
                    return Err("the goal asks two values of one variable".into())
                }
                Some(_) => {}
                None => goal.push((p, val)),
            }
        }

        let k = pattern.len();
        let decode = |idx: usize, vals: &mut [u32]| {
            let mut r = idx as u64;
            for i in 0..k {
                vals[i] = (r % sizes[i] as u64) as u32;
                r /= sizes[i] as u64;
            }
        };
        let encode = |vals: &[u32]| -> usize {
            vals.iter()
                .zip(&strides)
                .map(|(&v, &s)| v as u64 * s)
                .sum::<u64>() as usize
        };
        let apply = |op: &POp, vals: &[u32], out: &mut Vec<u32>| -> bool {
            if op.pre.iter().any(|&(p, v)| vals[p as usize] != v) {
                return false;
            }
            out.clear();
            out.extend_from_slice(vals);
            for &(p, e) in &op.dels {
                if let Eff::ClearIf(v) = e {
                    if out[p as usize] == v {
                        out[p as usize] = sizes[p as usize] - 1;
                    }
                }
            }
            for &(p, e) in &op.adds {
                if let Eff::Set(v) = e {
                    out[p as usize] = v;
                }
            }
            true
        };
        let check_clock = |i: usize| -> Result<(), String> {
            if i & 0xFFF == 0 {
                if let Some((t, s)) = &limits.deadline {
                    if t.elapsed_secs() > *s {
                        return Err(format!(
                            "build past its {s:.1}s deadline at state {i} of {n}"
                        ));
                    }
                }
            }
            Ok(())
        };

        // Pass 1: in-degrees of the reverse graph (edges INTO succ).
        let mut vals = vec![0u32; k];
        let mut nvals = vec![0u32; k];
        let mut indeg = vec![0u32; n + 1];
        let mut n_edges = 0usize;
        for idx in 0..n {
            check_clock(idx)?;
            decode(idx, &mut vals);
            for op in &pops {
                if apply(op, &vals, &mut nvals) {
                    let t = encode(&nvals);
                    if t != idx {
                        indeg[t] += 1;
                        n_edges += 1;
                        if n_edges > limits.max_edges {
                            return Err(format!(
                                "abstract graph over the cap (> {} edges)",
                                limits.max_edges
                            ));
                        }
                    }
                }
            }
        }
        // Prefix sums -> CSR offsets.
        let mut off = vec![0u32; n + 1];
        for i in 0..n {
            off[i + 1] = off[i] + indeg[i];
        }
        let mut fill = off.clone();
        let mut pred = vec![0u32; n_edges];
        let mut pcost = vec![0f32; n_edges];
        // Pass 2: fill.
        for idx in 0..n {
            check_clock(idx)?;
            decode(idx, &mut vals);
            for op in &pops {
                if apply(op, &vals, &mut nvals) {
                    let t = encode(&nvals);
                    if t != idx {
                        let slot = fill[t] as usize;
                        pred[slot] = idx as u32;
                        pcost[slot] = op.cost as f32;
                        fill[t] += 1;
                    }
                }
            }
        }
        drop(indeg);
        drop(fill);

        // Backward Dijkstra from every abstract goal state.
        let mut dist = vec![f64::INFINITY; n];
        let mut heap: BinaryHeap<Reverse<(Cost, u32)>> = BinaryHeap::new();
        for (idx, d) in dist.iter_mut().enumerate() {
            decode(idx, &mut vals);
            if goal.iter().all(|&(p, v)| vals[p as usize] == v) {
                *d = 0.0;
                heap.push(Reverse((Cost(0.0), idx as u32)));
            }
        }
        while let Some(Reverse((Cost(d), u))) = heap.pop() {
            let u = u as usize;
            if d > dist[u] {
                continue;
            }
            for e in off[u] as usize..off[u + 1] as usize {
                let v = pred[e] as usize;
                let nd = d + pcost[e] as f64;
                if nd < dist[v] {
                    dist[v] = nd;
                    heap.push(Reverse((Cost(nd), v as u32)));
                }
            }
        }
        Ok(Pdb {
            vars,
            pattern,
            strides,
            dist,
            n_states: n,
            n_edges,
            build_secs: t0.elapsed_secs(),
        })
    }

    /// The abstract index of a concrete state.
    pub fn index(&self, bits: &[u64]) -> usize {
        self.pattern
            .iter()
            .zip(&self.strides)
            .map(|(&v, &s)| self.vars.value(v as usize, bits) as u64 * s)
            .sum::<u64>() as usize
    }

    /// The heuristic: the abstract distance to the goal, `∞` when the
    /// projection cannot reach it (a safe prune).
    pub fn h(&self, bits: &[u64]) -> f64 {
        self.dist[self.index(bits)]
    }
}

thread_local! {
    static INSTALLED: RefCell<Option<Arc<Pdb>>> = const { RefCell::new(None) };
}

/// Clears the installed database when dropped.
pub struct Guard(());

impl Drop for Guard {
    fn drop(&mut self) {
        INSTALLED.with(|p| *p.borrow_mut() = None);
    }
}

/// Install the database the optimal A* on this thread reads (`None`
/// clears it); the guard clears it again when it drops.
pub fn install(pdb: Option<Arc<Pdb>>) -> Guard {
    INSTALLED.with(|p| *p.borrow_mut() = pdb);
    Guard(())
}

/// The database installed on this thread, if any.
pub fn current() -> Option<Arc<Pdb>> {
    INSTALLED.with(|p| p.borrow().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(dom: &str, prb: &str) -> (crate::types::Domain, PackedTask) {
        let d = crate::parser::parse_domain(dom).unwrap();
        let p = crate::parser::parse_problem(prb).unwrap();
        let t = crate::ground::ground_task(&d, &p, 1).unwrap();
        (d, t)
    }

    /// A chain l0 -> l1 -> ... -> l4, each step deleting the one before:
    /// one mutex group, one variable, exact distances. The chain is
    /// `(at ?l)` over a static `next`, not five nullary predicates --
    /// the synthesis seeds no nullary predicate (`invariants.rs`), so a
    /// nullary chain is five binary variables and a goal pattern of one.
    #[test]
    fn a_chain_is_exact() {
        let (d, t) = task(
            "(define (domain chain) (:predicates (at ?l) (next ?a ?b))
               (:action s :parameters (?a ?b)
                 :precondition (and (at ?a) (next ?a ?b))
                 :effect (and (at ?b) (not (at ?a)))))",
            "(define (problem c) (:domain chain) (:objects l0 l1 l2 l3 l4)
               (:init (at l0) (next l0 l1) (next l1 l2) (next l2 l3) (next l3 l4))
               (:goal (at l4)))",
        );
        let vars = Arc::new(Variables::from_task(&d, &t));
        let pattern = Pdb::goal_pattern(&vars, &t, 1 << 16);
        assert_eq!(pattern.len(), 1, "the goal's variable");
        let costs = vec![1.0; t.n_ops];
        let pdb = Pdb::build(&t, vars.clone(), pattern, &costs, &Limits::default()).unwrap();
        let s0 = t.initial();
        assert_eq!(pdb.h(&s0.bits), 4.0);
        let s1 = t.apply(
            (0..t.n_ops)
                .find(|&o| t.op_display[o].starts_with("S L0"))
                .unwrap(),
            &s0,
        );
        assert_eq!(pdb.h(&s1.bits), 3.0);
        // `none` on the chain's variable: no fact true -- unreachable from
        // the goal backward, so infinite.
        let mut none = s0.clone();
        for w in none.bits.iter_mut() {
            *w = 0;
        }
        assert!(pdb.h(&none.bits).is_infinite());
    }
}
