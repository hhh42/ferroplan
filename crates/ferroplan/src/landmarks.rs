//! Fact landmarks by first-achiever backchaining (0.9 roadmap Phase 3).
//!
//! A fact landmark is a fact every plan must make true at some point. This
//! module computes the classic delete-relaxation landmark set (LAMA's
//! backbone, à la Hoffmann/Porteous/Sebastia): build the relaxed planning
//! graph from the initial state to fixpoint, then backchain from the goal —
//! for a landmark `f` not true initially, every plan uses one of its FIRST
//! achievers (ops adding `f` whose own layer precedes `f`'s), so any fact
//! common to ALL first achievers' preconditions is itself a landmark.
//! Sound (never claims a non-landmark), incomplete (misses some), cheap:
//! O(landmarks × achiever preconditions) after one RPG build, and O(n_facts)
//! memory — no quadratic per-fact landmark table.
//!
//! Consumed by the LAMA-style rung ([`crate::lama`]) as a path-dependent
//! landmark-count heuristic: landmarks not yet accepted on the path measure
//! remaining necessary work, a signal the FF heuristic lacks exactly where
//! it plateaus (long goal-interaction chains: parking, floortile, barman).

use crate::heuristic::{reachability_layers, Scratch};
use crate::packed::PackedTask;

/// The goal's fact-landmark set (sorted, deduped fact ids). Includes the goal
/// facts themselves (trivial landmarks) minus any already true in `:init` —
/// counting those would reward standing still. Deterministic.
pub fn goal_landmarks(task: &PackedTask) -> Vec<u32> {
    // Relaxed reachability layers from init, goal-blind (to fixpoint).
    let mut sc = Scratch::new(task);
    let init = task.initial();
    let (fact_layer, op_layer) =
        reachability_layers(task, &mut sc, &init.bits, &init.fv, &init.fdef);

    let mut is_lm = vec![false; task.n_facts];
    let mut queue: Vec<u32> = Vec::new();
    for &g in &task.goal_pos {
        // Unreachable goals mean the task is unsolvable; landmark counting is
        // moot but must not crash — skip them.
        if fact_layer[g as usize] != u32::MAX && !is_lm[g as usize] {
            is_lm[g as usize] = true;
            queue.push(g);
        }
    }

    let mut head = 0;
    while head < queue.len() {
        let f = queue[head] as usize;
        head += 1;
        let fl = fact_layer[f];
        if fl == 0 {
            continue; // true in init: no achiever needed
        }
        // First achievers: ops adding f from a strictly earlier layer.
        let mut common: Option<Vec<u32>> = None;
        for &oi in task.add_by_fact.slice(f) {
            let oi = oi as usize;
            if op_layer[oi] >= fl {
                continue;
            }
            let pre: Vec<u32> = task.pre_pos.slice(oi).to_vec();
            common = Some(match common {
                None => pre,
                Some(prev) => prev.into_iter().filter(|p| pre.contains(p)).collect(),
            });
            if common.as_ref().is_some_and(|c| c.is_empty()) {
                break;
            }
        }
        for p in common.unwrap_or_default() {
            if !is_lm[p as usize] {
                is_lm[p as usize] = true;
                queue.push(p);
            }
        }
    }

    // Landmarks already true in init are pre-accepted — dropping them here
    // keeps the count meaning "necessary facts not yet made true".
    let mut out: Vec<u32> = (0..task.n_facts as u32)
        .filter(|&f| is_lm[f as usize] && !crate::bitset::test(&task.init_bits, f as usize))
        .collect();
    out.sort_unstable();
    out
}
