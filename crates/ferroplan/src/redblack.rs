//! Red-black relaxed plans (0.31 Lane D, docs/roadmap-0.31.md).
//!
//! h^FF relaxes EVERY variable: a fact once true stays true, so a truck
//! that drove away is still where it started, and a relaxed plan never
//! pays for the trip back. RED-BLACK planning (Katz, Hoffmann & Domshlak,
//! AAAI / ICAPS 2013; Domshlak, Hoffmann & Katz, AIJ 2015 "Red-black
//! planning: a new systematic approach to partial delete relaxation";
//! `docs/papers/katz-etal-2013-red-black-relaxed-plan.pdf`,
//! `who-said-relax-all.pdf`) keeps a few variables BLACK -- real
//! semantics, one value at a time -- and relaxes the rest (RED). Mercury,
//! IPC 2014 seq-sat runner-up, is this heuristic in LAMA's place.
//!
//! The machinery, over the multi-valued cover of [`crate::vars`]:
//!
//! 1. THE DOMAIN TRANSITION GRAPH of a variable `v`: an arc `d -> d'` per
//!    op whose effect sets `v = d'` with precondition `v = d` (or `v`
//!    unconstrained: an arc from every value), each labelled with the op's
//!    OUTSIDE condition (its preconditions on other variables) and OUTSIDE
//!    effect (its adds on other variables). A delete of `v`'s current fact
//!    with no add on `v` is an arc to `none` -- the cover's groups are
//!    at-most-one, and `vars.rs` carries that value.
//! 2. RSE-INVERTIBILITY (the AAAI paper's Definition 3): every arc
//!    `d -> d'` reachable from the initial value has an inverse arc
//!    `d' -> d` whose outside condition is contained in the forward arc's
//!    outside condition ∪ outside effect -- whatever the forward move
//!    needed or made true, the move back can use. Per variable,
//!    polynomial; bounded here by an arc cap and a check budget, a bound
//!    hit being a refusal (the variable stays red), never a guess.
//! 3. THE PAINTING: candidates are the invertible multi-valued variables
//!    no conditional effect touches (conditional effects stay red: their
//!    firing is a function of the whole state, and the DTG arc is not
//!    well-defined over a relaxed one -- the conservative side). Paint
//!    black greedily, most PREVAIL-USED first (how many ops need the
//!    variable's value without changing it -- the truck every load and
//!    unload reads, exactly the condition h^FF reads as free once true),
//!    then larger domains, dropping any candidate that would close a
//!    cycle in the BLACK CAUSAL GRAPH (`v -> v'` when some op has `v` in
//!    its precondition or effect and `v'` in its effect), up to
//!    `FF_RB_MAX_BLACK` (8). The paper's level- and conflict-ordered
//!    paintings are the next probes; this one costs one pass over the ops.
//! 4. THE HEURISTIC, `UNRELAX`: take h^FF's relaxed plan for the state in
//!    RPG layer order ([`crate::heuristic::selected_in_layer_order`]),
//!    walk it with the black variables at their REAL values and the red
//!    facts accumulating; before an op whose black precondition does not
//!    hold, insert the shortest path in that variable's DTG over arcs
//!    whose outside conditions hold now (red: accumulated; black: current
//!    value) -- and likewise for the black goal at the end. h is h^FF plus
//!    the repair ops inserted: exactly the repaired plan's length when
//!    h^FF is the relaxed plan's op count, and h^FF's own numeric
//!    repetition and consumption charges carried untouched when it is not
//!    (numeric conditions are red here, as everywhere in the relaxation).
//!
//! Theorem 2 of the AAAI paper: with RSE-invertible black variables and
//! an acyclic black causal graph the repair always exists. Two places this
//! code is weaker than the theorem's algorithm and says so: outside
//! conditions are judged against the red set as it stands when a path is
//! searched (the path's own red adds do not open later arcs of the same
//! path), and a black variable's path is searched with the others held at
//! their current values (repairs run sinks-first in the black causal
//! graph, so a dependent is moved before the variables it reads are). A
//! repair that finds no path returns h^FF for the state -- never less --
//! and is counted ([`FALLBACKS`], printed under `FF_WALL_DEBUG`).
//!
//! Costs: unit, as h^FF counts in LAMA and in the rung below. A cost-aware
//! repair (Dijkstra over op costs, the cost-h's `relaxed_costed` beside it)
//! is the obvious extension and waits for a board that asks.
//!
//! Opt-in: `FF_RB=1` puts this h in LAMA's evaluation (the helpful set
//! stays h^FF's), `FF_RUNG_ONLY=rb` runs [`search`], a greedy best-first
//! over it alone, as a rung. Unset, nothing here is built. The built
//! instance travels through a thread-local (the `pdb.rs` / `LADDER_BASE`
//! idiom): `api.rs` builds it where the domain is in hand, the rungs read
//! it on the solve's thread.

use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::bitset;
use crate::hash::FxHashMap;
use crate::heuristic::{relaxed_helpful, selected_in_layer_order, Scratch};
use crate::packed::{PackedTask, State};
use crate::types::{Domain, NumPre};
use crate::vars::Variables;

/// A transition's source when the op leaves the variable unconstrained.
const ANY: u32 = u32::MAX;
/// "No black index" / "no requirement".
const NO: u32 = u32::MAX;

/// The black set's default cap (`FF_RB_MAX_BLACK`).
pub const DEFAULT_MAX_BLACK: usize = 8;
/// A variable with more DTG arcs than this is not tested and stays red.
const MAX_ARCS: usize = 1 << 16;
/// Inverse-arc checks one variable's invertibility test may spend.
const MAX_INVERSE_CHECKS: usize = 1 << 22;

/// Repairs that found no DTG path (h fell back to h^FF), process-wide;
/// measurement only.
pub static FALLBACKS: AtomicU64 = AtomicU64::new(0);

/// One DTG arc: `from` (or [`ANY`]) -> `to`, by op `op`.
#[derive(Clone, Copy, Debug)]
struct Transition {
    from: u32,
    to: u32,
    op: u32,
}

/// A variable's domain transition graph.
#[derive(Clone, Debug, Default)]
struct Dtg {
    arcs: Vec<Transition>,
    /// value -> arcs leaving it (`from == value`).
    by_from: Vec<Vec<u32>>,
    /// arcs leaving every value (`from == ANY`).
    any: Vec<u32>,
}

pub struct RedBlack {
    vars: Arc<Variables>,
    /// The black variables, in REPAIR order: reverse topological in the
    /// black causal graph (sinks first), so moving a variable never
    /// unsettles one repaired before it.
    pub black: Vec<u32>,
    /// variable -> its index in `black`, or `u32::MAX` when red.
    black_pos: Vec<u32>,
    /// Per black index, its DTG.
    dtg: Vec<Dtg>,
    /// Per variable: passed the RSE-invertibility test (false for every
    /// variable never tested -- singletons, conditional-effect-touched,
    /// over the arc cap).
    pub invertible: Vec<bool>,
    /// The causal graph over the TESTED variables (`cg[v]` = the `v'`
    /// with an arc `v -> v'`, ascending); empty rows elsewhere.
    cg: Vec<Vec<u32>>,
    /// Multi-valued variables no conditional effect touches.
    pub n_candidates: usize,
    /// Of those, the RSE-invertible ones.
    pub n_invertible: usize,
    pub build_secs: f64,
}

impl RedBlack {
    /// The red-black instance for a task, from its mutex-group cover and
    /// the `FF_RB_MAX_BLACK` cap.
    pub fn from_task(domain: &Domain, task: &PackedTask) -> RedBlack {
        let vars = Arc::new(Variables::from_task(domain, task));
        RedBlack::build(task, vars, max_black_from_env())
    }

    /// Build over any cover: DTGs, the invertibility test, the painting.
    pub fn build(task: &PackedTask, vars: Arc<Variables>, max_black: usize) -> RedBlack {
        let t0 = crate::clock::Clock::now();
        let nv = vars.len();

        // Pass 1: variables a conditional effect touches never go black.
        let mut cond_touched = vec![false; nv];
        let mut mark_ce = |ce: &crate::packed::CondEff| {
            for &f in ce.add.iter().chain(ce.del.iter()) {
                cond_touched[vars.fact_var[f as usize] as usize] = true;
            }
        };
        for oi in 0..task.n_ops {
            for ce in task.cond.slice(oi) {
                mark_ce(ce);
            }
        }
        for ce in task.shared_cond.iter() {
            mark_ce(ce);
        }
        let tested: Vec<bool> = (0..nv)
            .map(|v| vars.vars[v].len() >= 2 && !cond_touched[v])
            .collect();
        let n_candidates = tested.iter().filter(|t| **t).count();

        // Pass 2: per tested variable, the ops that change it; the causal
        // graph among tested variables; the prevail counts.
        let mut ops_of: Vec<Vec<u32>> = vec![Vec::new(); nv];
        let mut cg: Vec<Vec<u32>> = vec![Vec::new(); nv];
        let mut prevail = vec![0usize; nv];
        let (mut pre_v, mut eff_v): (Vec<u32>, Vec<u32>) = (Vec::new(), Vec::new());
        for oi in 0..task.n_ops {
            pre_v.clear();
            eff_v.clear();
            for &f in task.pre_pos.slice(oi) {
                let v = vars.fact_var[f as usize];
                if tested[v as usize] {
                    pre_v.push(v);
                }
            }
            for &f in task.add.slice(oi).iter().chain(task.del.slice(oi).iter()) {
                let v = vars.fact_var[f as usize];
                if tested[v as usize] {
                    eff_v.push(v);
                }
            }
            pre_v.sort_unstable();
            pre_v.dedup();
            eff_v.sort_unstable();
            eff_v.dedup();
            for &e in &eff_v {
                ops_of[e as usize].push(oi as u32);
                for &p in pre_v.iter().chain(eff_v.iter()) {
                    if p != e {
                        cg[p as usize].push(e);
                    }
                }
            }
            for &p in &pre_v {
                if eff_v.binary_search(&p).is_err() {
                    prevail[p as usize] += 1;
                }
            }
        }
        for row in cg.iter_mut() {
            row.sort_unstable();
            row.dedup();
        }

        // The DTGs and the invertibility test.
        let mut dtgs: Vec<Option<Dtg>> = vec![None; nv];
        let mut invertible = vec![false; nv];
        for v in 0..nv {
            if !tested[v] {
                continue;
            }
            let Some(dtg) = build_dtg(task, &vars, v, &ops_of[v]) else {
                continue;
            };
            let init = vars.value(v, &task.init_bits);
            if is_invertible(task, &vars, v, &dtg, init) {
                invertible[v] = true;
                dtgs[v] = Some(dtg);
            }
        }
        drop(ops_of);
        let n_invertible = invertible.iter().filter(|i| **i).count();

        // The painting: most prevail-used first, then larger domains,
        // then variable id; a candidate closing a black cycle stays red.
        let mut order: Vec<u32> = (0..nv as u32).filter(|&v| invertible[v as usize]).collect();
        order.sort_by_key(|&v| {
            (
                Reverse(prevail[v as usize]),
                Reverse(vars.size(v as usize)),
                v,
            )
        });
        let mut is_black = vec![false; nv];
        let mut chosen: Vec<u32> = Vec::new();
        for c in order {
            if chosen.len() >= max_black {
                break;
            }
            is_black[c as usize] = true;
            if reaches(&cg, &is_black, c, c) {
                is_black[c as usize] = false;
            } else {
                chosen.push(c);
            }
        }

        // Repair order: reverse topological over the black subgraph (Kahn,
        // ties by variable id for determinism).
        let mut indeg: FxHashMap<u32, usize> = chosen.iter().map(|&v| (v, 0)).collect();
        for &v in &chosen {
            for &w in &cg[v as usize] {
                if is_black[w as usize] {
                    *indeg.get_mut(&w).unwrap() += 1;
                }
            }
        }
        let mut ready: Vec<u32> = chosen.iter().copied().filter(|v| indeg[v] == 0).collect();
        let mut topo: Vec<u32> = Vec::with_capacity(chosen.len());
        while let Some(pos) = ready
            .iter()
            .enumerate()
            .min_by_key(|(_, v)| **v)
            .map(|(i, _)| i)
        {
            let v = ready.swap_remove(pos);
            topo.push(v);
            for &w in &cg[v as usize] {
                if is_black[w as usize] {
                    let d = indeg.get_mut(&w).unwrap();
                    *d -= 1;
                    if *d == 0 {
                        ready.push(w);
                    }
                }
            }
        }
        debug_assert_eq!(
            topo.len(),
            chosen.len(),
            "the painting keeps the black graph acyclic"
        );
        topo.reverse();
        let black = topo;
        let mut black_pos = vec![NO; nv];
        let mut dtg: Vec<Dtg> = Vec::with_capacity(black.len());
        for (i, &v) in black.iter().enumerate() {
            black_pos[v as usize] = i as u32;
            dtg.push(dtgs[v as usize].take().unwrap_or_default());
        }
        // The causal graph is kept (over the tested variables only, as
        // built) for the acyclicity pin, `black_acyclic`.
        RedBlack {
            vars,
            black,
            black_pos,
            dtg,
            invertible,
            cg,
            n_candidates,
            n_invertible,
            build_secs: t0.elapsed_secs(),
        }
    }

    /// The variable cover this instance paints.
    pub fn vars(&self) -> &Variables {
        &self.vars
    }

    /// Is the black causal graph acyclic? True by construction; the
    /// fixtures pin it.
    pub fn black_acyclic(&self) -> bool {
        let mut is_black = vec![false; self.vars.len()];
        for &v in &self.black {
            is_black[v as usize] = true;
        }
        self.black
            .iter()
            .all(|&v| !reaches(&self.cg, &is_black, v, v))
    }

    /// The note line: `redblack: N black of M variables (names), ...`.
    pub fn note(&self, task: &PackedTask) -> String {
        let names: Vec<String> = self
            .black
            .iter()
            .map(|&v| {
                let facts = &self.vars.vars[v as usize];
                let first = facts
                    .first()
                    .and_then(|&f| task.fact_names.get(f as usize))
                    .map(String::as_str)
                    .unwrap_or("?");
                format!("{first}/{}", facts.len())
            })
            .collect();
        format!(
            "redblack: {} black of {} variables ({}); {} of {} multi-valued candidates RSE-invertible, built in {:.3}s",
            self.black.len(),
            self.vars.len(),
            names.join(", "),
            self.n_invertible,
            self.n_candidates,
            self.build_secs
        )
    }

    /// The red-black heuristic and h^FF's helpful set, the shape
    /// `relaxed_helpful` returns -- LAMA's evaluation swaps one for the
    /// other. `None` is h^FF's relaxed dead end (the red-black relaxation
    /// is no weaker a prune than the delete relaxation, so the same).
    pub fn h(
        &self,
        task: &PackedTask,
        sc: &mut Scratch,
        s: &State,
        goal_pos: &[u32],
        goal_num: &[NumPre],
    ) -> Option<(i32, Vec<u32>)> {
        let (h_ff, helpful) =
            relaxed_helpful(task, sc, &s.bits, &s.fv, &s.fdef, goal_pos, goal_num)?;
        if self.black.is_empty() || h_ff == 0 {
            return Some((h_ff, helpful));
        }
        let plan = selected_in_layer_order(task, sc);
        let h = match self.repair(task, &s.bits, &plan, goal_pos) {
            Some(extra) => h_ff.saturating_add(extra),
            None => {
                FALLBACKS.fetch_add(1, Ordering::Relaxed);
                h_ff
            }
        };
        Some((h, helpful))
    }

    /// `UNRELAX`: walk `plan` from `bits`, black variables real and red
    /// facts accumulating, inserting DTG paths for unmet black
    /// preconditions and black goals. The number of ops inserted, or
    /// `None` when some repair found no path.
    fn repair(
        &self,
        task: &PackedTask,
        bits: &[u64],
        plan: &[u32],
        goal_pos: &[u32],
    ) -> Option<i32> {
        let nb = self.black.len();
        let mut red: Vec<u64> = bits.to_vec();
        let mut val: Vec<u32> = self
            .black
            .iter()
            .map(|&v| self.vars.value(v as usize, bits))
            .collect();
        let mut need: Vec<u32> = vec![NO; nb];
        let mut extra = 0i32;
        for &oi in plan {
            let oi = oi as usize;
            self.settle(
                task,
                task.pre_pos.slice(oi),
                &mut need,
                &mut val,
                &mut red,
                &mut extra,
            )?;
            self.apply(task, oi, &mut val, &mut red);
        }
        self.settle(task, goal_pos, &mut need, &mut val, &mut red, &mut extra)?;
        Some(extra)
    }

    /// Bring every black variable `facts` constrain to the value they ask,
    /// in repair order. `need` is all-`NO` on entry and on exit.
    fn settle(
        &self,
        task: &PackedTask,
        facts: &[u32],
        need: &mut [u32],
        val: &mut [u32],
        red: &mut [u64],
        extra: &mut i32,
    ) -> Option<()> {
        let mut unmet = false;
        let mut touched = false;
        for &f in facts {
            let b = self.black_pos[self.vars.fact_var[f as usize] as usize];
            if b == NO {
                continue;
            }
            let want = self.vars.fact_val[f as usize];
            // Two values of one variable asked at once: the op is never
            // applicable; the first ask stands (the walk is an estimate).
            if need[b as usize] == NO {
                need[b as usize] = want;
                touched = true;
            }
            if val[b as usize] != need[b as usize] {
                unmet = true;
            }
        }
        let mut ok = true;
        if unmet {
            for b in 0..self.black.len() {
                if need[b] != NO && val[b] != need[b] {
                    match self.path(task, b, need[b], val, red) {
                        Some(n) => *extra = extra.saturating_add(n),
                        None => {
                            ok = false;
                            break;
                        }
                    }
                }
            }
            // A later repair moves only its own variable (no op changes
            // two black variables: that is a 2-cycle), so this holds.
            ok = ok && (0..self.black.len()).all(|b| need[b] == NO || val[b] == need[b]);
        }
        if touched {
            need.iter_mut().for_each(|n| *n = NO);
        }
        ok.then_some(())
    }

    /// Op `oi`'s effects on the walk: black variables take their new value
    /// (dels first, then adds -- `PackedTask::apply`'s order), red adds
    /// accumulate, conditional adds included (they are red by the
    /// painting, and the relaxation is optimistic about their firing).
    fn apply(&self, task: &PackedTask, oi: usize, val: &mut [u32], red: &mut [u64]) {
        for &f in task.del.slice(oi) {
            let v = self.vars.fact_var[f as usize];
            let b = self.black_pos[v as usize];
            if b != NO && val[b as usize] == self.vars.fact_val[f as usize] {
                val[b as usize] = self.vars.none(v as usize);
            }
        }
        for &f in task.add.slice(oi) {
            let b = self.black_pos[self.vars.fact_var[f as usize] as usize];
            if b != NO {
                val[b as usize] = self.vars.fact_val[f as usize];
            } else {
                bitset::set(red, f as usize);
            }
        }
        for ce in task.cond_effs(oi) {
            for &f in &ce.add {
                if self.black_pos[self.vars.fact_var[f as usize] as usize] == NO {
                    bitset::set(red, f as usize);
                }
            }
        }
    }

    /// Does op `oi`'s outside condition (w.r.t. variable `v`) hold now?
    fn outside_holds(
        &self,
        task: &PackedTask,
        v: u32,
        oi: usize,
        val: &[u32],
        red: &[u64],
    ) -> bool {
        task.pre_pos.slice(oi).iter().all(|&f| {
            let w = self.vars.fact_var[f as usize];
            if w == v {
                return true;
            }
            let b = self.black_pos[w as usize];
            if b != NO {
                val[b as usize] == self.vars.fact_val[f as usize]
            } else {
                bitset::test(red, f as usize)
            }
        })
    }

    /// The shortest DTG path (unit costs, BFS) moving black variable `b`
    /// to `target` over arcs whose outside conditions hold now; applied to
    /// the walk on success. The path's length, or `None`.
    fn path(
        &self,
        task: &PackedTask,
        b: usize,
        target: u32,
        val: &mut [u32],
        red: &mut [u64],
    ) -> Option<i32> {
        let v = self.black[b];
        let n = self.vars.size(v as usize) as usize;
        let dtg = &self.dtg[b];
        let start = val[b];
        if start == target {
            return Some(0);
        }
        // Per value: the arc that reached it and the value it came from.
        let mut via: Vec<u32> = vec![NO; n];
        let mut prev: Vec<u32> = vec![NO; n];
        let mut seen = vec![false; n];
        seen[start as usize] = true;
        let mut q: VecDeque<u32> = VecDeque::new();
        q.push_back(start);
        'bfs: while let Some(u) = q.pop_front() {
            for &ai in dtg.by_from[u as usize].iter().chain(dtg.any.iter()) {
                let t = dtg.arcs[ai as usize];
                if seen[t.to as usize] || !self.outside_holds(task, v, t.op as usize, val, red) {
                    continue;
                }
                seen[t.to as usize] = true;
                via[t.to as usize] = ai;
                prev[t.to as usize] = u;
                if t.to == target {
                    break 'bfs;
                }
                q.push_back(t.to);
            }
        }
        if !seen[target as usize] {
            return None;
        }
        let mut ops: Vec<u32> = Vec::new();
        let mut x = target;
        while x != start {
            ops.push(dtg.arcs[via[x as usize] as usize].op);
            x = prev[x as usize];
        }
        let len = ops.len() as i32;
        for &oi in ops.iter().rev() {
            // The path's ops change only `v` among the black variables
            // (acyclicity), so the walk's `apply` moves `v` along the path
            // and adds the red side effects.
            self.apply(task, oi as usize, val, red);
        }
        // Robust to a delete-then-add the arc did not model: the target is
        // where the BFS put the variable.
        val[b] = target;
        Some(len)
    }
}

/// `FF_RB_MAX_BLACK=<n>` (default [`DEFAULT_MAX_BLACK`]).
pub fn max_black_from_env() -> usize {
    std::env::var("FF_RB_MAX_BLACK")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_BLACK)
}

/// Is the red-black instance wanted on this solve? `FF_RB=1` (LAMA's h)
/// or the `rb` rung under the matrix hatch.
pub fn requested() -> bool {
    std::env::var("FF_RB").is_ok_and(|v| v.trim() == "1")
        || std::env::var("FF_RUNG_ONLY").is_ok_and(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "rb" | "redblack" | "red-black"
            )
        })
}

/// Op `oi`'s DTG arcs on variable `v` (see the module docs, item 1).
fn push_arcs(task: &PackedTask, vars: &Variables, v: usize, oi: usize, out: &mut Vec<Transition>) {
    let mut pre: Option<u32> = None;
    for &f in task.pre_pos.slice(oi) {
        if vars.fact_var[f as usize] as usize == v {
            let d = vars.fact_val[f as usize];
            match pre {
                Some(p) if p != d => return, // never applicable
                _ => pre = Some(d),
            }
        }
    }
    let add = task
        .add
        .slice(oi)
        .iter()
        .find(|&&f| vars.fact_var[f as usize] as usize == v)
        .map(|&f| vars.fact_val[f as usize]);
    let none = vars.none(v);
    if let Some(to) = add {
        let from = pre.unwrap_or(ANY);
        if from != to {
            out.push(Transition {
                from,
                to,
                op: oi as u32,
            });
        }
        return;
    }
    for &f in task.del.slice(oi) {
        if vars.fact_var[f as usize] as usize != v {
            continue;
        }
        let d = vars.fact_val[f as usize];
        if pre.map_or(true, |p| p == d) {
            out.push(Transition {
                from: d,
                to: none,
                op: oi as u32,
            });
        }
    }
}

/// The DTG of variable `v` from the ops that change it; `None` over the
/// arc cap.
fn build_dtg(task: &PackedTask, vars: &Variables, v: usize, ops: &[u32]) -> Option<Dtg> {
    let n = vars.size(v) as usize;
    let mut arcs: Vec<Transition> = Vec::new();
    for &oi in ops {
        push_arcs(task, vars, v, oi as usize, &mut arcs);
        if arcs.len() > MAX_ARCS {
            return None;
        }
    }
    let mut by_from: Vec<Vec<u32>> = vec![Vec::new(); n];
    let mut any: Vec<u32> = Vec::new();
    for (i, t) in arcs.iter().enumerate() {
        if t.from == ANY {
            any.push(i as u32);
        } else {
            by_from[t.from as usize].push(i as u32);
        }
    }
    Some(Dtg { arcs, by_from, any })
}

/// The facts of op `oi`'s preconditions on variables other than `v`.
fn outside_pre<'a>(
    task: &'a PackedTask,
    vars: &'a Variables,
    v: usize,
    oi: usize,
) -> impl Iterator<Item = u32> + 'a {
    task.pre_pos
        .slice(oi)
        .iter()
        .copied()
        .filter(move |&f| vars.fact_var[f as usize] as usize != v)
}

/// RSE-invertibility of `v` over the arcs reachable from `init` (module
/// docs, item 2). Over the check budget is `false`.
fn is_invertible(task: &PackedTask, vars: &Variables, v: usize, dtg: &Dtg, init: u32) -> bool {
    let n = vars.size(v) as usize;
    // Values reachable from the initial one, conditions ignored.
    let mut reach = vec![false; n];
    reach[init as usize] = true;
    let mut q: VecDeque<u32> = VecDeque::from([init]);
    while let Some(u) = q.pop_front() {
        for &ai in dtg.by_from[u as usize].iter().chain(dtg.any.iter()) {
            let to = dtg.arcs[ai as usize].to;
            if !reach[to as usize] {
                reach[to as usize] = true;
                q.push_back(to);
            }
        }
    }
    let mut by_to: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (i, t) in dtg.arcs.iter().enumerate() {
        by_to[t.to as usize].push(i as u32);
    }
    let mut budget = MAX_INVERSE_CHECKS;
    let mut allowed: Vec<u32> = Vec::new();
    for t in &dtg.arcs {
        if t.from != ANY && !reach[t.from as usize] {
            continue;
        }
        // What the forward move needed or made true outside `v`.
        allowed.clear();
        allowed.extend(outside_pre(task, vars, v, t.op as usize));
        allowed.extend(
            task.add
                .slice(t.op as usize)
                .iter()
                .copied()
                .filter(|&f| vars.fact_var[f as usize] as usize != v),
        );
        allowed.sort_unstable();
        allowed.dedup();
        let sources: Vec<u32> = if t.from == ANY {
            (0..n as u32)
                .filter(|&d| d != t.to && reach[d as usize])
                .collect()
        } else {
            vec![t.from]
        };
        for d in sources {
            let mut found = false;
            for &bi in &by_to[d as usize] {
                if budget == 0 {
                    return false;
                }
                budget -= 1;
                let back = dtg.arcs[bi as usize];
                if back.from != t.to && back.from != ANY {
                    continue;
                }
                if outside_pre(task, vars, v, back.op as usize)
                    .all(|f| allowed.binary_search(&f).is_ok())
                {
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }
    }
    true
}

/// Does `from` reach `goal` in the causal graph restricted to `on`
/// (over at least one arc)?
fn reaches(cg: &[Vec<u32>], on: &[bool], from: u32, goal: u32) -> bool {
    let mut seen = vec![false; cg.len()];
    let mut stack: Vec<u32> = cg[from as usize]
        .iter()
        .copied()
        .filter(|&w| on[w as usize])
        .collect();
    while let Some(u) = stack.pop() {
        if u == goal {
            return true;
        }
        if seen[u as usize] {
            continue;
        }
        seen[u as usize] = true;
        for &w in &cg[u as usize] {
            if on[w as usize] && !seen[w as usize] {
                stack.push(w);
            }
        }
    }
    false
}

/// THE `rb` RUNG (`FF_RUNG_ONLY=rb`): greedy best-first on the red-black
/// h alone -- eager evaluation, one open list keyed (h, generation
/// order), duplicates dropped by state hash, the goal tested at
/// generation. No preferred operators and no landmarks: the rung measures
/// the heuristic, not LAMA's recipe around it (`FF_RB=1` is that).
/// Bounded by `max_eval`, the node cap, the rung memory wall, `deadline`,
/// the hard wall and the caller's cancel flag; a bound hit is a
/// hand-down, never a proof. Returns the plan and the evaluations.
pub fn search(
    task: &PackedTask,
    rb: &RedBlack,
    max_eval: usize,
    forbidden: &[bool],
    deadline: Option<(crate::clock::Clock, f64)>,
) -> Option<(Vec<usize>, usize)> {
    struct Node {
        state: State,
        father: u32,
        op: u32,
    }
    let init = task.initial();
    if task.goal_met(&init) {
        return Some((Vec::new(), 0));
    }
    let (goal_pos, goal_num) = (&task.goal_pos, &task.goal_num);
    let mut sc = Scratch::new(task);
    let (h0, _) = rb.h(task, &mut sc, &init, goal_pos, goal_num)?;
    let mut evaluated = 1usize;
    let node_cap = crate::search::node_cap_for(task);
    let mem_wall = crate::mem::MemWall::arm_rung();
    let budget = crate::search::call_budget();
    let fallbacks0 = FALLBACKS.load(Ordering::Relaxed);
    let mut keys: FxHashMap<u128, u32> = FxHashMap::default();
    keys.insert(task.state_key_hash128(&init, None), 0);
    let mut nodes = vec![Node {
        state: init,
        father: u32::MAX,
        op: u32::MAX,
    }];
    let mut open: BinaryHeap<Reverse<(i32, u64, u32)>> = BinaryHeap::new();
    let mut seq = 0u64;
    open.push(Reverse((h0, seq, 0)));
    let mut cands: Vec<u32> = Vec::new();
    let reconstruct = |nodes: &[Node], mut ni: usize| -> Vec<usize> {
        let mut ops = Vec::new();
        while nodes[ni].father != u32::MAX {
            ops.push(nodes[ni].op as usize);
            ni = nodes[ni].father as usize;
        }
        ops.reverse();
        ops
    };
    let debug = std::env::var("FF_WALL_DEBUG").is_ok();
    let stop = |why: &str, evaluated: usize, held: usize| {
        if debug {
            eprintln!(
                "wall: red-black rung stopped ({why}) at {evaluated} evals, {held} nodes, {} repair fallbacks",
                FALLBACKS.load(Ordering::Relaxed) - fallbacks0
            );
        }
    };
    while let Some(Reverse((_, _, ni))) = open.pop() {
        let ni = ni as usize;
        cands.clear();
        task.applicable_ops(&nodes[ni].state, &mut cands);
        for &oi in &cands {
            let oi = oi as usize;
            if forbidden.get(oi).copied().unwrap_or(false) {
                continue;
            }
            let ns = task.apply(oi, &nodes[ni].state);
            let k = task.state_key_hash128(&ns, None);
            if keys.contains_key(&k) {
                continue;
            }
            let idx = nodes.len() as u32;
            keys.insert(k, idx);
            if task.goal_met(&ns) {
                nodes.push(Node {
                    state: ns,
                    father: ni as u32,
                    op: oi as u32,
                });
                if debug {
                    eprintln!(
                        "wall: red-black rung solved at {evaluated} evals, {} repair fallbacks",
                        FALLBACKS.load(Ordering::Relaxed) - fallbacks0
                    );
                }
                return Some((reconstruct(&nodes, idx as usize), evaluated));
            }
            let h = rb.h(task, &mut sc, &ns, goal_pos, goal_num);
            evaluated += 1;
            let Some((h, _)) = h else {
                continue; // a relaxed dead end: the key stays, the state is dropped
            };
            nodes.push(Node {
                state: ns,
                father: ni as u32,
                op: oi as u32,
            });
            seq += 1;
            open.push(Reverse((h, seq, idx)));
            if evaluated & 0xFF == 0 {
                if evaluated > max_eval || nodes.len() > node_cap {
                    stop("eval or node cap", evaluated, nodes.len());
                    return None;
                }
                if deadline
                    .as_ref()
                    .is_some_and(|(t, s)| t.elapsed_secs() > *s)
                    || crate::search::wall_hard_expired()
                    || budget.cancelled()
                {
                    stop("wall", evaluated, nodes.len());
                    return None;
                }
                if evaluated & 0xFFF == 0 && mem_wall.hit() {
                    stop("memory", evaluated, nodes.len());
                    return None;
                }
            }
        }
    }
    stop("open list exhausted", evaluated, nodes.len());
    None
}

/// The ladder's entry for [`crate::search::Rung::Rb`]: the instance
/// installed on this thread, or a narrated hand-down when none is.
pub(crate) fn run_rung(
    task: &PackedTask,
    max_eval: usize,
    forbidden: &[bool],
    deadline: Option<(crate::clock::Clock, f64)>,
) -> Option<(Vec<usize>, usize)> {
    let Some(rb) = current() else {
        eprintln!("red-black rung: no instance installed on this thread; handing down");
        return None;
    };
    search(task, &rb, max_eval, forbidden, deadline)
}

thread_local! {
    static INSTALLED: RefCell<Option<Arc<RedBlack>>> = const { RefCell::new(None) };
}

/// Clears the installed instance when dropped.
pub struct Guard(());

impl Drop for Guard {
    fn drop(&mut self) {
        INSTALLED.with(|p| *p.borrow_mut() = None);
    }
}

/// Install the instance the rungs on this thread read (`None` clears it);
/// the guard clears it again when it drops.
pub fn install(rb: Option<Arc<RedBlack>>) -> Guard {
    INSTALLED.with(|p| *p.borrow_mut() = rb);
    Guard(())
}

/// The instance installed on this thread, if any.
pub fn current() -> Option<Arc<RedBlack>> {
    INSTALLED.with(|p| p.borrow().clone())
}
