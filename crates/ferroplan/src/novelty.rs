//! Novelty rung (0.17 roadmap Phase 3): greedy best-first ordered by
//! **state novelty** first, heuristic second — the BFWS recipe (Lipovetzky
//! & Geffner; the engine idea inside the IPC 2018 agile winner and both
//! IPC 2023 classical winners) in its width-1 form.
//!
//! Novelty here: a successor is NOVEL iff it makes some fact true that no
//! previously kept state in the same PARTITION CELL has made true. Cells
//! are the ⟨unachieved-goal count⟩ — the classic BFWS partition, kept
//! deliberately COARSE: a finer cell (an early draft partitioned by
//! parent-h too) makes nearly every state novel, and the order
//! degenerates back to plain h-greed (measured on the catalog-consume
//! fixture: the same 895-step wandering plan, byte-identical length).
//! Novel states are expanded before non-novel ones regardless of h;
//! within a novelty class the order is ⟨goal count, parent h⟩.
//!
//! Why a third rung: EHC and the LAMA rung both die where the relaxed
//! plan's gradient is wrong or exhausted — the modern corpora
//! (IPC 2018/2023) are full of exactly those domains, and every winner
//! there carries a novelty component (docs/landscape-2026.md). Novelty-
//! first exploration does not ask h for permission to visit a
//! structurally new state. This rung runs BOUNDED after the LAMA rung
//! gives up and before the complete weighted fallback — and the corpus
//! referee made it OPT-IN (`FF_NOVELTY=1`): "can only add coverage"
//! is true per-instance but not per-BUDGET — the rung's wall-time tax
//! ahead of the complete fallback cost 51 budget-edge instances against
//! 7 gained across the classical boards (full arithmetic in the 0.17
//! Phase 3 record). The gains are real where h truly dies (+3 on
//! 2018-sat, +3 on prop-2006) and stay reachable via the flag.
//! (`FF_NOVELTY_ONLY=1` is the probe hatch; `--search bfs` never
//! enters either.)
//!
//! Determinism: identical contract and structure to the LAMA rung —
//! fixed pop batches from dual (preferred/normal) heaps, order-preserving
//! parallel h evaluation, serial insertion; plans are identical at any
//! thread count.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::hash::FxHashMap;
use crate::heuristic::{relaxed_helpful, Scratch};
use crate::packed::{PackedTask, State};
use crate::par;

const PREF_BATCH: usize = 192;
const NORM_BATCH: usize = 64;
/// Key layout: novel flag dominates, then unachieved goals, then parent h.
const W_NOVEL: i64 = 1 << 40;
const W_GOALS: i64 = 1 << 20;

type Cand = (usize, usize, State, u64, i32, bool);

struct Node {
    state: State,
    father: usize,
    op: usize,
}

/// Per-cell per-relevant-fluent (min, max) quantized envelopes (FF_NUMNOV).
type NumEnvelopes = FxHashMap<(u16, u16), Vec<(i64, i64)>>;

/// Per-cell seen-fact tables. A cell's table is lazily allocated on first
/// touch; `novel_and_mark` reports whether `bits` carries any fact the
/// cell has never seen and ORs the state in either way.
///
/// `FF_NUMNOV=1` (0.21 Phase 3 probe rider b, the field's winning
/// direction — Panino's partitioned numeric novelty): the bit tables are
/// structurally blind on fluent-only progress (sailing has ONE predicate,
/// so every successor is bit-identical). Opt-in, a cell additionally
/// keeps a per-relevant-fluent seen-ENVELOPE over the quantized values
/// (packed.rs's 1e-6 state-key quantizer): a state whose fluent leaves
/// the envelope is novel, first touch of a cell is novel. OFF ⇒
/// `num_cells` is None and `qvals` stays empty — zero cost.
struct Seen {
    words: usize,
    cells: FxHashMap<(u16, u16), Vec<u64>>,
    num_cells: Option<NumEnvelopes>,
}

impl Seen {
    fn new(words: usize, numnov: bool) -> Self {
        Seen {
            words,
            cells: FxHashMap::default(),
            num_cells: numnov.then(FxHashMap::default),
        }
    }
    fn novel_and_mark(&mut self, cell: (u16, u16), bits: &[u64], qvals: &[i64]) -> bool {
        let t = self
            .cells
            .entry(cell)
            .or_insert_with(|| vec![0u64; self.words]);
        let mut novel = false;
        for (w, &b) in t.iter_mut().zip(bits) {
            if b & !*w != 0 {
                novel = true;
            }
            *w |= b;
        }
        if let Some(nc) = &mut self.num_cells {
            if !qvals.is_empty() {
                match nc.entry(cell) {
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert(qvals.iter().map(|&v| (v, v)).collect());
                        novel = true;
                    }
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        for (env, &v) in e.get_mut().iter_mut().zip(qvals) {
                            if v < env.0 {
                                env.0 = v;
                                novel = true;
                            }
                            if v > env.1 {
                                env.1 = v;
                                novel = true;
                            }
                        }
                    }
                }
            }
        }
        novel
    }
}

/// The FF_NUMNOV gate: opt-in AND numeric-task-gated — a task with no
/// relevant fluents keeps the bit-only tables even under the flag.
fn numnov_on(task: &PackedTask) -> bool {
    !task.rel_fluents.is_empty() && std::env::var("FF_NUMNOV").is_ok()
}

/// Quantized relevant-fluent vector for the envelope; empty when off (the
/// zero-cost contract).
fn qvals_of(task: &PackedTask, s: &State, on: bool) -> Vec<i64> {
    if !on {
        return Vec::new();
    }
    task.rel_fluents
        .iter()
        .map(|&i| PackedTask::quantized(s, i as usize))
        .collect()
}

fn unachieved(task: &PackedTask, s: &State, goal_pos: &[u32]) -> u16 {
    let mut n = 0u16;
    for &g in goal_pos {
        if !crate::bitset::test(&s.bits, g as usize) {
            n += 1;
        }
    }
    let _ = task;
    n
}

/// Bounded novelty-first greedy search toward the task goal. Returns the
/// plan ops and states evaluated, or None (dead end, cap, or node cap).
///
/// `slice` (0.22 Phase 5A a2): the rung's armed wall deadline — (start
/// clock, seconds) — checked at the batch boundary, where the per-pop
/// `relaxed_helpful` calls spend the wall. The ladder passes
/// `FF_NOV_WALL_FRAC` (default 0.30 — the last bounded rung can afford
/// the most) of the REMAINING wall; the `FF_NOVELTY_ONLY` probe passes
/// `None` (the probe prices the rung with the whole wall). `None` ⇒
/// unchecked ⇒ byte-identical.
pub fn search(
    task: &PackedTask,
    threads: usize,
    max_eval: usize,
    forbidden: &[bool],
    slice: Option<(crate::clock::Clock, f64)>,
) -> Option<(Vec<usize>, usize)> {
    let init = task.initial();
    search_subgoal(
        task,
        &init,
        &task.goal_pos,
        &task.goal_num,
        threads,
        max_eval,
        forbidden,
        slice,
    )
}

/// [`search`] generalized over a start state and subgoal (the partition
/// cascade's form; novelty tables are fresh per call by construction).
#[allow(clippy::too_many_arguments)]
pub fn search_subgoal(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[crate::types::NumPre],
    threads: usize,
    max_eval: usize,
    forbidden: &[bool],
    slice: Option<(crate::clock::Clock, f64)>,
) -> Option<(Vec<usize>, usize)> {
    let node_cap = crate::search::node_cap_for(task);
    let words = start.bits.len();
    let dbg = std::env::var("FF_RES_DEBUG").is_ok();
    if dbg {
        eprintln!("[novelty] enter: cap {max_eval}, {} ops", task.n_ops);
    }

    let init = start.clone();
    if task.goal_met_with(&init, goal_pos, goal_num) {
        return Some((Vec::new(), 0));
    }
    let mut nodes = vec![Node {
        state: init.clone(),
        father: usize::MAX,
        op: usize::MAX,
    }];
    let numnov = numnov_on(task);
    let mut seen = Seen::new(words, numnov);
    // The root seeds its cell (parent-h slot 0: no parent evaluation yet).
    let g0 = unachieved(task, &init, goal_pos);
    seen.novel_and_mark((g0, 0), &init.bits, &qvals_of(task, &init, numnov));

    let mut pref_heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    let mut norm_heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    norm_heap.push(Reverse((0, 0)));
    // Hash -> node-index dedup (0.20 Phase 4): exact equality against the
    // arena state, no second bitset copy per entry (see search_from).
    let mut visited: FxHashMap<u64, Vec<u32>> = FxHashMap::default();
    visited.insert(task.state_key_hash(&init, None), vec![0]);
    let mut expanded = vec![false; 1];
    let mut evaluated = 0usize;

    loop {
        let mut popped: Vec<usize> = Vec::with_capacity(PREF_BATCH + NORM_BATCH);
        for _ in 0..PREF_BATCH {
            match pref_heap.pop() {
                Some(Reverse((_, ni))) if !expanded[ni] => {
                    expanded[ni] = true;
                    popped.push(ni);
                }
                Some(_) => continue,
                None => break,
            }
        }
        for _ in 0..NORM_BATCH {
            match norm_heap.pop() {
                Some(Reverse((_, ni))) if !expanded[ni] => {
                    expanded[ni] = true;
                    popped.push(ni);
                }
                Some(_) => continue,
                None => break,
            }
        }
        if popped.is_empty() {
            if std::env::var("FF_RES_DEBUG").is_ok() {
                eprintln!(
                    "[novelty] open lists exhausted at {evaluated} evals, {} nodes",
                    nodes.len()
                );
            }
            return None;
        }

        for &ni in &popped {
            if task.goal_met_with(&nodes[ni].state, goal_pos, goal_num) {
                return Some((reconstruct(&nodes, ni), evaluated));
            }
        }

        // PARALLEL: FF h + helpful set per popped node.
        let hs: Vec<Option<(i32, Vec<u32>)>> = par::par_map_with(
            &popped,
            threads,
            || Scratch::new(task),
            |sc, &ni| {
                let s = &nodes[ni].state;
                relaxed_helpful(task, sc, &s.bits, &s.fv, &s.fdef, goal_pos, goal_num)
            },
        );
        evaluated += popped.len();
        if evaluated > max_eval || nodes.len() > node_cap {
            if std::env::var("FF_RES_DEBUG").is_ok() {
                eprintln!(
                    "[novelty] capped: {evaluated} evals (max {max_eval}), {} nodes (cap {node_cap})",
                    nodes.len()
                );
            }
            return None;
        }
        // The wall slice (0.22 Phase 5A a2) + the hard checkpoint (Phase 2
        // lever 1), both at the batch boundary where the h evaluations
        // spend the wall (same shape as the LAMA rung's).
        if let Some((t0, s)) = &slice {
            if t0.elapsed_secs() > *s {
                if std::env::var("FF_WALL_DEBUG").is_ok() {
                    eprintln!(
                        "wall: novelty slice exhausted ({evaluated} evals in {:.2}s), handing down the ladder",
                        t0.elapsed_secs()
                    );
                }
                return None;
            }
        }
        if crate::search::wall_hard_expired() {
            return None;
        }

        // PARALLEL: expand live nodes.
        let chunks: Vec<Vec<Cand>> = {
            let live: Vec<(usize, i32, &Vec<u32>)> = popped
                .iter()
                .zip(hs.iter())
                .filter_map(|(&ni, h)| h.as_ref().map(|(h, help)| (ni, *h, help)))
                .collect();
            par::par_map(&live, threads, |&(ni, ph, helpful)| {
                let st = &nodes[ni].state;
                let mut v = Vec::new();
                for oi in 0..task.n_ops {
                    if forbidden.get(oi).copied().unwrap_or(false) {
                        continue;
                    }
                    if task.op_applicable(oi, st) {
                        let ns = task.apply(oi, st);
                        let k = task.state_key_hash(&ns, None);
                        let pref = helpful.contains(&(oi as u32));
                        v.push((ni, oi, ns, k, ph, pref));
                    }
                }
                v
            })
        };

        // SERIAL: dedup + novelty + insert (deterministic — novelty tables
        // are updated in the same fixed order the candidates arrive).
        for chunk in chunks {
            for (pi, oi, s, k, ph, pref) in chunk {
                let bucket = visited.entry(k).or_default();
                if bucket
                    .iter()
                    .any(|&idx| task.state_key_eq(&nodes[idx as usize].state, &s, None))
                {
                    continue;
                }
                bucket.push(nodes.len() as u32);
                {
                    let g = unachieved(task, &s, goal_pos);
                    let cell = (g, 0);
                    let novel = seen.novel_and_mark(cell, &s.bits, &qvals_of(task, &s, numnov));
                    let key = if novel { 0 } else { W_NOVEL } + g as i64 * W_GOALS + ph as i64;
                    let idx = nodes.len();
                    nodes.push(Node {
                        state: s,
                        father: pi,
                        op: oi,
                    });
                    expanded.push(false);
                    norm_heap.push(Reverse((key, idx)));
                    if pref {
                        pref_heap.push(Reverse((key, idx)));
                    }
                }
            }
        }
    }
}

fn reconstruct(nodes: &[Node], mut ni: usize) -> Vec<usize> {
    let mut ops = Vec::new();
    while nodes[ni].father != usize::MAX {
        ops.push(nodes[ni].op);
        ni = nodes[ni].father;
    }
    ops.reverse();
    ops
}

/// The LIGHT novelty rung (0.20 Phase 3): IW(1)-style novelty-first with
/// GOAL-COUNT guidance and ZERO heuristic evaluations. The 0.20 scoping
/// probe found the h-guided rung above solves visit-all-2014 i1 but pays
/// 35 s of wall — all of it in per-pop `relaxed_helpful` calls the
/// width-1 structure never needed (BFWS dispatches visit-all in
/// milliseconds on exactly this recipe). This rung is that recipe: key =
/// ⟨novel, unachieved-goals, insertion order⟩, single heap, no h, no
/// preferred ops — a pop costs successor generation and a bitset OR, so
/// its wall footprint stays small by construction. Bounded like every
/// rung (eval cap + node cap); no dead-end pruning (nothing computes ∞
/// here) — the cap is the exit on hopeless tasks.
///
/// Determinism: single serial loop, fixed key layout, insertion-order
/// tie-break — identical plans at any thread count (threads unused).
pub fn search_light(
    task: &PackedTask,
    max_eval: usize,
    forbidden: &[bool],
) -> Option<(Vec<usize>, usize)> {
    // The ladder tax (0.21 Phase 5, lever 1): under an ARMED wall budget
    // the unconditional pop cap gains a wall-denominated bound —
    // `FF_NOVLIGHT_WALL_FRAC` (default 0.10) of the REMAINING wall at
    // rung entry, checked every 4096 pops (pops/sec spans orders of
    // magnitude across tasks, so a deadline, not a pre-converted pop
    // count). The receipted wins need plan-length pops — visit-all-2014:
    // 899/3135/3248, two orders under BOTH caps — so every 0.20 win fits
    // the slice by construction; what it cuts is the tens of seconds a
    // big task's 300k pops spend ahead of the rung that would have
    // solved (the 0.21 backfill's −34 receipt). No armed budget ⇒
    // `None` ⇒ byte-identical.
    let slice = crate::search::wall_remaining_secs().map(|rem| {
        (
            crate::clock::Clock::now(),
            crate::search::wall_frac_env("FF_NOVLIGHT_WALL_FRAC", 0.10) * rem,
        )
    });
    let node_cap = crate::search::node_cap_for(task);
    let init = task.initial();
    let goal_pos = &task.goal_pos;
    let goal_num = &task.goal_num;
    if task.goal_met_with(&init, goal_pos, goal_num) {
        return Some((Vec::new(), 0));
    }
    let words = init.bits.len();
    let mut nodes = vec![Node {
        state: init.clone(),
        father: usize::MAX,
        op: usize::MAX,
    }];
    let numnov = numnov_on(task);
    let mut seen = Seen::new(words, numnov);
    let g0 = unachieved(task, &init, goal_pos);
    seen.novel_and_mark((g0, 0), &init.bits, &qvals_of(task, &init, numnov));
    let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    heap.push(Reverse((0, 0)));
    // Hash -> node-index dedup (0.20 Phase 4): exact equality against the
    // arena state, no second bitset copy per entry (see search_from).
    let mut visited: FxHashMap<u64, Vec<u32>> = FxHashMap::default();
    visited.insert(task.state_key_hash(&init, None), vec![0]);
    let mut evaluated = 0usize;

    while let Some(Reverse((_, ni))) = heap.pop() {
        if task.goal_met_with(&nodes[ni].state, goal_pos, goal_num) {
            return Some((reconstruct(&nodes, ni), evaluated));
        }
        evaluated += 1;
        if evaluated > max_eval || nodes.len() > node_cap {
            if std::env::var("FF_RES_DEBUG").is_ok() {
                eprintln!(
                    "[novelty-light] capped: {evaluated} evals (max {max_eval}), {} nodes",
                    nodes.len()
                );
            }
            return None;
        }
        if evaluated & 0xFFF == 0 {
            if let Some((t0, s)) = &slice {
                if t0.elapsed_secs() > *s {
                    if std::env::var("FF_WALL_DEBUG").is_ok() {
                        eprintln!(
                            "wall: novelty-light slice exhausted ({evaluated} pops in {:.2}s)",
                            t0.elapsed_secs()
                        );
                    }
                    return None;
                }
            }
        }
        for oi in 0..task.n_ops {
            if forbidden.get(oi).copied().unwrap_or(false) {
                continue;
            }
            if !task.op_applicable(oi, &nodes[ni].state) {
                continue;
            }
            let ns = task.apply(oi, &nodes[ni].state);
            let k = task.state_key_hash(&ns, None);
            let bucket = visited.entry(k).or_default();
            if bucket
                .iter()
                .any(|&idx| task.state_key_eq(&nodes[idx as usize].state, &ns, None))
            {
                continue;
            }
            bucket.push(nodes.len() as u32);
            let g = unachieved(task, &ns, goal_pos);
            let novel = seen.novel_and_mark((g, 0), &ns.bits, &qvals_of(task, &ns, numnov));
            let key = if novel { 0 } else { W_NOVEL } + g as i64 * W_GOALS;
            let idx = nodes.len();
            nodes.push(Node {
                state: ns,
                father: ni,
                op: oi,
            });
            heap.push(Reverse((key, idx)));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    //! FF_NUMNOV smoke (0.21 Phase 3 probe rider b): the numeric envelope
    //! distinguishes two states differing ONLY in a fluent; the bit-only
    //! tables cannot.

    use super::*;

    fn numeric_task() -> PackedTask {
        let dom = "(define (domain nn)
          (:requirements :typing :numeric-fluents)
          (:predicates (flag))
          (:functions (x))
          (:action bump :parameters ()
            :precondition (<= (x) 100)
            :effect (increase (x) 1))
          (:action finish :parameters ()
            :precondition (>= (x) 3)
            :effect (flag)))";
        let prb = "(define (problem nn1) (:domain nn)
          (:init (= (x) 0)) (:goal (flag)))";
        let d = crate::parser::parse_domain(dom).unwrap();
        let p = crate::parser::parse_problem(prb).unwrap();
        crate::ground::ground_task(&d, &p, 1).unwrap()
    }

    #[test]
    fn envelope_distinguishes_fluent_only_states() {
        let task = numeric_task();
        let s0 = task.initial();
        let bump = (0..task.n_ops)
            .find(|&oi| task.op_display[oi].starts_with("BUMP"))
            .unwrap();
        let s1 = task.apply(bump, &s0);
        let cell = (1u16, 0u16);

        // Bit tables only: s1 is bit-identical to s0 — never novel.
        let mut plain = Seen::new(s0.bits.len(), false);
        plain.novel_and_mark(cell, &s0.bits, &qvals_of(&task, &s0, false));
        assert!(!plain.novel_and_mark(cell, &s1.bits, &qvals_of(&task, &s1, false)));

        // Envelope on: x moved 0 -> 1, outside the seen envelope — novel.
        let mut env = Seen::new(s0.bits.len(), true);
        env.novel_and_mark(cell, &s0.bits, &qvals_of(&task, &s0, true));
        assert!(env.novel_and_mark(cell, &s1.bits, &qvals_of(&task, &s1, true)));
        // And re-seeing the same value is NOT novel (an envelope, not a set).
        assert!(!env.novel_and_mark(cell, &s1.bits, &qvals_of(&task, &s1, true)));
    }
}
