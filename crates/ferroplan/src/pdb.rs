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
//! The pattern: the first selector is THE GOAL PATTERN -- the variables
//! that carry a goal fact, smallest domains first, as many as fit under
//! the state cap (`FF_OPT_PDB=1`). The second (`FF_OPT_PDB=ipdb`) is a
//! COLLECTION: iPDB's hill-climbing (Haslum et al. 2007) from one
//! singleton pattern per goal variable, each step growing one pattern by
//! one causal-graph predecessor, the step taken the one that raises h on
//! the most sampled states; the patterns are then combined by SATURATED
//! COST PARTITIONING (Seipp & Helmert JAIR 2018): in an order, each
//! database is built under the costs the ones before it left, takes only
//! the cost it needs -- `sat(o) = max over its abstract o-transitions
//! s -> t of h(s) - h(t)` -- and the rest passes on, so the SUM stays
//! admissible. Several orders, kept only when one raises the max on a
//! sample (the diversification of Seipp, Keller & Helmert JAIR 2020), are
//! max()ed. A single database is a collection of one order of one.
//!
//! The installed collection travels to the A* through a thread-local
//! (the `LADDER_BASE` idiom): the optimal ladder builds several engines
//! over one task, and the one solve that built the tables is the one they
//! all read. `install` returns a guard that clears it.

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
    /// Per op, the cost this database needs of it under the costs it was
    /// built with: `max(0, max over abstract o-transitions s -> t, both
    /// finite, of h(s) - h(t))` -- what saturated cost partitioning takes
    /// and passes the rest on.
    pub saturated: Vec<f64>,
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
    /// The concrete op it projects (the edge label SCP saturates).
    op: u32,
    pre: Vec<(u32, u32)>,
    dels: Vec<(u32, Eff)>,
    adds: Vec<(u32, Eff)>,
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
        for oi in 0..task.n_ops.min(costs.len()) {
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
            // Through `cond_effs`, never `cond` alone: the shared monitor
            // block of a constraint task is part of the op too, and a
            // projection that missed it would miss transitions.
            let touching: Vec<&crate::packed::CondEff> = task
                .cond_effs(oi)
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
                op: oi as u32,
                pre,
                dels,
                adds,
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
        // The edge's op, its cost looked up in `costs` (exact f64: SCP's
        // leftover costs are fractions, and an f32 edge could round h a
        // hair above the true distance).
        let mut plab = vec![0u32; n_edges];
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
                        plab[slot] = op.op;
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
                let nd = d + costs[plab[e] as usize];
                if nd < dist[v] {
                    dist[v] = nd;
                    heap.push(Reverse((Cost(nd), v as u32)));
                }
            }
        }
        // Saturated costs: every abstract edge s -o-> t with both ends
        // finite asks h(s) - h(t) of o. Self-loops ask 0 (the clamp), and
        // an edge into or out of a dead state asks nothing (-inf).
        let mut saturated = vec![0.0f64; task.n_ops];
        for t in 0..n {
            if dist[t].is_infinite() {
                continue;
            }
            for e in off[t] as usize..off[t + 1] as usize {
                let s = pred[e] as usize;
                if dist[s].is_finite() {
                    let o = plab[e] as usize;
                    let need = dist[s] - dist[t];
                    if need > saturated[o] {
                        saturated[o] = need;
                    }
                }
            }
        }
        Ok(Pdb {
            vars,
            pattern,
            strides,
            dist,
            saturated,
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

/// What the A* reads: a max over ORDERS, each order a sum over databases
/// built under saturated cost partitioning. One database is one order of
/// one, and its `h` is that database's.
pub struct Collection {
    pub orders: Vec<Vec<Arc<Pdb>>>,
}

impl Collection {
    pub fn single(p: Arc<Pdb>) -> Collection {
        Collection {
            orders: vec![vec![p]],
        }
    }

    /// `max over orders of sum over the order's databases`; `∞` as soon
    /// as any database of any order says the state is dead (each order is
    /// admissible alone, so one ∞ is a proof).
    pub fn h(&self, bits: &[u64]) -> f64 {
        let mut best = 0.0f64;
        for order in &self.orders {
            let mut sum = 0.0;
            for p in order {
                let h = p.h(bits);
                if h.is_infinite() {
                    return f64::INFINITY;
                }
                sum += h;
            }
            if sum > best {
                best = sum;
            }
        }
        best
    }

    /// The distinct databases (an order shares no table with another --
    /// each was built under its own leftover costs).
    pub fn n_pdbs(&self) -> usize {
        self.orders.iter().map(|o| o.len()).sum()
    }

    pub fn n_states(&self) -> usize {
        self.orders
            .iter()
            .flat_map(|o| o.iter())
            .map(|p| p.n_states)
            .sum()
    }
}

/// Build one order under saturated cost partitioning: each pattern in
/// turn under the costs left by the ones before it. A pattern refused by
/// the limits is skipped (named in the second return), never fatal: the
/// sum over what was built is admissible regardless.
pub fn scp_order(
    task: &PackedTask,
    vars: &Arc<Variables>,
    patterns: &[Vec<u32>],
    costs: &[f64],
    limits: &Limits,
) -> (Vec<Arc<Pdb>>, Vec<String>) {
    let mut left: Vec<f64> = costs.to_vec();
    let mut built = Vec::new();
    let mut refused = Vec::new();
    for pat in patterns {
        match Pdb::build(task, vars.clone(), pat.clone(), &left, limits) {
            Ok(p) => {
                for (c, s) in left.iter_mut().zip(&p.saturated) {
                    // `max(0, ..)`: an f64 subtraction a hair below zero is
                    // a cost the next database may not spend.
                    *c = (*c - *s).max(0.0);
                }
                built.push(Arc::new(p));
            }
            Err(why) => refused.push(why),
        }
    }
    (built, refused)
}

/// The settings of an iPDB build.
#[derive(Clone, Debug)]
pub struct IpdbConfig {
    /// Largest abstract space one pattern may have.
    pub pattern_cap: usize,
    /// Largest sum of abstract spaces over the collection.
    pub collection_cap: usize,
    /// Sampled states the candidates are scored on.
    pub samples: usize,
    /// A step is taken only if it raises h on at least this many samples.
    pub min_improvement: usize,
    /// Orders tried for the diversification (the first is the
    /// hill-climb's own, the second its reverse, the rest shuffles).
    pub orders: usize,
    pub seed: u64,
}

impl Default for IpdbConfig {
    fn default() -> Self {
        IpdbConfig {
            pattern_cap: 1 << 16,
            collection_cap: 1 << 22,
            samples: 200,
            min_improvement: 2,
            orders: 4,
            seed: 0x5EED_0031,
        }
    }
}

/// The receipt of an iPDB build, for the notes.
#[derive(Clone, Debug, Default)]
pub struct IpdbReceipt {
    pub steps: usize,
    pub candidates_built: usize,
    pub candidates_refused: usize,
    pub patterns: Vec<Vec<u32>>,
    pub orders_tried: usize,
    pub orders_kept: usize,
    pub h_init: f64,
    pub secs: f64,
    /// Why the climb stopped.
    pub stop: String,
}

/// splitmix64: deterministic, seedable, dependency-free.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// The causal graph's predecessor sets over the variables: `v` precedes
/// `w` when some op has `v` in its precondition (or its conditions) and
/// an effect on `w` -- the variables that can change what `w` may do.
fn causal_preds(task: &PackedTask, vars: &Variables) -> Vec<Vec<u32>> {
    let mut preds: Vec<Vec<u32>> = vec![Vec::new(); vars.len()];
    let var = |f: &u32| vars.fact_var[*f as usize];
    for oi in 0..task.n_ops {
        let mut pre: Vec<u32> = task.pre_pos.slice(oi).iter().map(var).collect();
        let mut eff: Vec<u32> = task
            .add
            .slice(oi)
            .iter()
            .chain(task.del.slice(oi))
            .map(var)
            .collect();
        for ce in task.cond_effs(oi) {
            pre.extend(ce.cond_pos.iter().map(var));
            eff.extend(ce.add.iter().chain(ce.del.iter()).map(var));
        }
        pre.sort_unstable();
        pre.dedup();
        eff.sort_unstable();
        eff.dedup();
        for &w in &eff {
            for &v in pre.iter().chain(eff.iter()) {
                if v != w {
                    preds[w as usize].push(v);
                }
            }
        }
    }
    for p in preds.iter_mut() {
        p.sort_unstable();
        p.dedup();
    }
    preds
}

fn space(vars: &Variables, pat: &[u32]) -> u128 {
    pat.iter().map(|&v| vars.size(v as usize) as u128).product()
}

/// Random walks from the initial state, Haslum et al.'s sampler: a walk
/// length drawn around twice the initial estimate in steps, the walk
/// stopped early at a dead end.
fn sample_states(task: &PackedTask, h0_steps: usize, n: usize, rng: &mut Rng) -> Vec<Vec<u64>> {
    let mut out = Vec::with_capacity(n);
    let mut ops = Vec::new();
    let max_len = (2 * h0_steps.max(1)).min(200);
    for _ in 0..n {
        let mut s = task.initial();
        let len = rng.below(max_len + 1);
        for _ in 0..len {
            ops.clear();
            task.applicable_ops(&s, &mut ops);
            if ops.is_empty() {
                break;
            }
            let o = ops[rng.below(ops.len())] as usize;
            s = task.apply(o, &s);
        }
        out.push(s.bits);
    }
    out
}

/// The iPDB collection under SCP: hill-climb the patterns (each scored
/// alone, under the full costs, on the sampled states -- the step raises
/// one pattern's own h, which is what an SCP sum then divides), then build
/// the diversified orders. `Err` only when not even the initial
/// singletons build; past that, every bound hit stops the climb and the
/// collection built so far is the answer.
pub fn ipdb(
    task: &PackedTask,
    vars: Arc<Variables>,
    costs: &[f64],
    cfg: &IpdbConfig,
    limits: &Limits,
) -> Result<(Collection, IpdbReceipt), String> {
    let t0 = crate::clock::Clock::now();
    let mut rc = IpdbReceipt::default();
    let past_deadline = || {
        limits
            .deadline
            .as_ref()
            .is_some_and(|(t, s)| t.elapsed_secs() > *s)
    };
    let one = Limits {
        max_states: cfg.pattern_cap.min(limits.max_states),
        max_edges: limits.max_edges,
        deadline: limits.deadline,
    };

    // The start: one singleton per goal variable.
    let mut gv: Vec<u32> = task
        .goal_pos
        .iter()
        .map(|&f| vars.fact_var[f as usize])
        .collect();
    gv.sort_unstable();
    gv.dedup();
    let mut patterns: Vec<Vec<u32>> = Vec::new();
    let mut tables: Vec<Arc<Pdb>> = Vec::new();
    for &v in &gv {
        match Pdb::build(task, vars.clone(), vec![v], costs, &one) {
            Ok(p) => {
                patterns.push(vec![v]);
                tables.push(Arc::new(p));
            }
            Err(_) => rc.candidates_refused += 1,
        }
    }
    if tables.is_empty() {
        return Err("no goal variable's singleton database built".into());
    }

    let mut rng = Rng(cfg.seed);
    let s0 = task.initial();
    let h0 = tables.iter().map(|p| p.h(&s0.bits)).fold(0.0, f64::max);
    let mean_cost = {
        let pos: Vec<f64> = costs.iter().copied().filter(|c| *c > 0.0).collect();
        if pos.is_empty() {
            1.0
        } else {
            pos.iter().sum::<f64>() / pos.len() as f64
        }
    };
    let h0_steps = if h0.is_finite() {
        (h0 / mean_cost).ceil() as usize
    } else {
        10
    };
    let samples = sample_states(task, h0_steps, cfg.samples, &mut rng);
    let preds = causal_preds(task, &vars);

    // Per pattern, its best extension P ∪ {v} (gain on the samples, the
    // table), computed once and kept until P itself changes: one step
    // changes one pattern, so the others' candidates are scored once, not
    // once a step. Only the best per pattern is kept (the memory of the
    // climb is one candidate table a pattern).
    type Best = Option<(usize, Vec<u32>, Arc<Pdb>)>;
    let mut cache: std::collections::HashMap<Vec<u32>, Best> = std::collections::HashMap::new();
    rc.stop = "no candidate improves".into();
    'climb: loop {
        let total: u128 = patterns.iter().map(|p| space(&vars, p)).sum();
        for (pi, pat) in patterns.iter().enumerate() {
            if cache.contains_key(pat) {
                continue;
            }
            let mut cand_vars: Vec<u32> = pat
                .iter()
                .flat_map(|&w| preds[w as usize].iter().copied())
                .filter(|v| !pat.contains(v))
                .collect();
            cand_vars.sort_unstable();
            cand_vars.dedup();
            let mut best: Best = None;
            for v in cand_vars {
                if past_deadline() {
                    rc.stop = "deadline".into();
                    break 'climb;
                }
                let mut np = pat.clone();
                np.push(v);
                np.sort_unstable();
                if space(&vars, &np) > cfg.pattern_cap as u128 {
                    continue;
                }
                let cand = match Pdb::build(task, vars.clone(), np.clone(), costs, &one) {
                    Ok(p) => {
                        rc.candidates_built += 1;
                        p
                    }
                    Err(_) => {
                        rc.candidates_refused += 1;
                        continue;
                    }
                };
                let old = &tables[pi];
                let gain = samples
                    .iter()
                    .filter(|b| cand.h(b) > old.h(b) + 1e-9)
                    .count();
                if best.as_ref().map_or(true, |b| gain > b.0) {
                    best = Some((gain, np, Arc::new(cand)));
                }
            }
            cache.insert(pat.clone(), best);
        }
        // The step: the best gain over every pattern's best extension that
        // still fits the collection's cap.
        let mut step: Option<(usize, usize)> = None; // (gain, pi)
        for (pi, pat) in patterns.iter().enumerate() {
            if let Some(Some((gain, np, _))) = cache.get(pat) {
                let fits =
                    total - space(&vars, pat) + space(&vars, np) <= cfg.collection_cap as u128;
                if fits && *gain >= cfg.min_improvement && step.map_or(true, |(g, _)| *gain > g) {
                    step = Some((*gain, pi));
                }
            }
        }
        let Some((_, pi)) = step else {
            break;
        };
        let old = patterns[pi].clone();
        let Some(Some((_, np, table))) = cache.remove(&old) else {
            break;
        };
        patterns[pi] = np;
        tables[pi] = table;
        rc.steps += 1;
    }
    // Two patterns may have grown into one: keep the first.
    {
        let mut seen = std::collections::HashSet::new();
        patterns.retain(|p| seen.insert(p.clone()));
    }

    // The diversified orders: the climb's own, its reverse, shuffles;
    // an order is kept when it raises the running max on a sample.
    let mut orders: Vec<Vec<Vec<u32>>> = vec![patterns.clone()];
    if cfg.orders >= 2 && patterns.len() >= 2 {
        let mut r = patterns.clone();
        r.reverse();
        orders.push(r);
    }
    while orders.len() < cfg.orders && patterns.len() >= 3 {
        let mut sh = patterns.clone();
        for i in (1..sh.len()).rev() {
            sh.swap(i, rng.below(i + 1));
        }
        orders.push(sh);
    }
    let mut kept: Vec<Vec<Arc<Pdb>>> = Vec::new();
    let mut running = vec![0.0f64; samples.len()];
    for order in &orders {
        if past_deadline() && !kept.is_empty() {
            break;
        }
        rc.orders_tried += 1;
        let (built, _refused) = scp_order(task, &vars, order, costs, &one);
        if built.is_empty() {
            continue;
        }
        let vals: Vec<f64> = samples
            .iter()
            .map(|b| built.iter().map(|p| p.h(b)).fold(0.0, |a, h| a + h))
            .collect();
        let raises = kept.is_empty() || vals.iter().zip(&running).any(|(v, r)| *v > *r + 1e-9);
        if raises {
            for (r, v) in running.iter_mut().zip(&vals) {
                *r = r.max(*v);
            }
            kept.push(built);
        }
    }
    if kept.is_empty() {
        // Not one order built under the leftover costs: the climbed
        // tables, max()ed (each under the full costs, each admissible).
        kept = tables.into_iter().map(|t| vec![t]).collect();
    }
    rc.orders_kept = kept.len();
    rc.patterns = patterns;
    let coll = Collection { orders: kept };
    rc.h_init = coll.h(&s0.bits);
    rc.secs = t0.elapsed_secs();
    Ok((coll, rc))
}

thread_local! {
    static INSTALLED: RefCell<Option<Arc<Collection>>> = const { RefCell::new(None) };
}

/// Clears the installed collection when dropped.
pub struct Guard(());

impl Drop for Guard {
    fn drop(&mut self) {
        INSTALLED.with(|p| *p.borrow_mut() = None);
    }
}

/// Install one database as the collection the optimal A* on this thread
/// reads (`None` clears it); the guard clears it again when it drops.
pub fn install(pdb: Option<Arc<Pdb>>) -> Guard {
    install_collection(pdb.map(|p| Arc::new(Collection::single(p))))
}

/// Install a collection (`None` clears it).
pub fn install_collection(c: Option<Arc<Collection>>) -> Guard {
    INSTALLED.with(|p| *p.borrow_mut() = c);
    Guard(())
}

/// The collection installed on this thread, if any.
pub fn current() -> Option<Arc<Collection>> {
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
