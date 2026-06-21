//! Renewable "counter" resource detection for resource-aware search guidance.
//!
//! Many planning problems encode a *renewable resource with a fixed capacity* —
//! openstacks' `stacks-avail`, but equally a crew pool, machine count, or power
//! budget — as a one-hot **count chain**: a mutex group whose members are levels
//! `0..=C`, with operators that *consume* (delete level `n`, add level `n-1`) and
//! *restore* (delete `n`, add `n+1`). Exactly one level holds at a time.
//!
//! The delete-relaxed RPG ([`crate::heuristic::relaxed_to`]) is **blind** to such
//! a resource: it drops the `(not (level n))` delete, so every level becomes
//! simultaneously reachable — "infinite capacity" — and the heuristic gives the
//! search no gradient toward staying within the pool. The fix (see
//! [`crate::search::SatGuidance`]) is a penalty on the **concrete** state, which
//! *can* see the live level. This module detects the chain and precomputes each
//! member's **occupancy** = how much of the resource is in use at that level
//! (distance from the full/initial end), ready for that penalty.
//!
//! Detection is domain-independent (it keys off the consume/restore *shape*, not
//! any predicate name) and conservative: a group only qualifies if its
//! level-transition operators form a single simple path covering every member and
//! the initial state sits at one end (the full-capacity level). Anything else is
//! ignored, so non-resource domains are unaffected.

use crate::hash::{FxHashMap, FxHashSet};
use crate::packed::PackedTask;

/// A detected renewable counter resource: each member fact id mapped to the
/// resource occupancy (units in use) when that member is the live level.
pub struct ResourceVar {
    /// `(member fact id, occupancy)`, occupancy 0 at the full/initial level.
    pub members: Vec<(u32, u32)>,
}

impl ResourceVar {
    /// Occupancy of this resource in the concrete state `bits` (0 if, defensively,
    /// no member is set — a one-hot counter always has exactly one).
    #[inline]
    pub fn occupancy(&self, bits: &[u64]) -> u32 {
        for &(f, occ) in &self.members {
            if crate::bitset::test(bits, f as usize) {
                return occ;
            }
        }
        0
    }
}

/// Detect renewable counter resources among the synthesized mutex `groups`.
///
/// `init` is the initial-state bitset (identifies the full-capacity level). Only
/// groups whose consume/restore operators form a single simple path over all
/// members, with the initial level at an endpoint, are returned.
pub fn detect_resources(task: &PackedTask, groups: &[Vec<u32>], init: &[u64]) -> Vec<ResourceVar> {
    let mut out = Vec::new();
    for g in groups {
        // Need a real counter (capacity >= 2, i.e. >= 3 levels) to be worth it.
        if g.len() < 3 {
            continue;
        }
        let gset: FxHashSet<u32> = g.iter().copied().collect();

        // Level-transition edges: an operator that deletes exactly one member and
        // adds exactly one *other* member moves the resource one level.
        let mut adj: FxHashMap<u32, FxHashSet<u32>> = FxHashMap::default();
        for &f in g {
            adj.entry(f).or_default();
        }
        for oi in 0..task.n_ops {
            let mut dels = task
                .del
                .slice(oi)
                .iter()
                .copied()
                .filter(|f| gset.contains(f));
            let mut adds = task
                .add
                .slice(oi)
                .iter()
                .copied()
                .filter(|f| gset.contains(f));
            let (d0, a0) = (dels.next(), adds.next());
            // exactly one deleted member + exactly one added member
            if let (Some(a), None, Some(b), None) = (d0, dels.next(), a0, adds.next()) {
                if a != b {
                    adj.get_mut(&a).unwrap().insert(b);
                    adj.get_mut(&b).unwrap().insert(a);
                }
            }
        }

        // The transitions must form a simple path over ALL members: every node has
        // degree 1 (the two endpoints) or 2 (interior).
        let mut endpoints = Vec::new();
        let mut shape_ok = true;
        for &f in g {
            match adj[&f].len() {
                1 => endpoints.push(f),
                2 => {}
                _ => {
                    shape_ok = false;
                    break;
                }
            }
        }
        if !shape_ok || endpoints.len() != 2 {
            continue;
        }

        // The initial level must be one endpoint (the full-capacity end), so that
        // occupancy = distance from init grows monotonically as the resource is
        // consumed.
        let start = match endpoints
            .iter()
            .copied()
            .find(|&f| crate::bitset::test(init, f as usize))
        {
            Some(f) => f,
            None => continue,
        };

        // Walk the path from the full end; ordinal = occupancy.
        let mut members = Vec::with_capacity(g.len());
        let mut prev: Option<u32> = None;
        let mut cur = start;
        let mut occ = 0u32;
        loop {
            members.push((cur, occ));
            let next = adj[&cur].iter().copied().find(|&n| Some(n) != prev);
            match next {
                Some(n) => {
                    prev = Some(cur);
                    cur = n;
                    occ += 1;
                }
                None => break,
            }
        }

        // Sanity: a clean path visits every member exactly once.
        if members.len() == g.len() {
            out.push(ResourceVar { members });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::{ground, Outcome};
    use crate::parser::{parse_domain, parse_problem};

    // A minimal renewable counter (the openstacks stacks-avail mechanic): `avail`
    // is a one-hot level; `consume` lowers it, `restore` raises it.
    const DOM: &str = "(define (domain ctr) (:requirements :typing)
      (:types count)
      (:predicates (avail ?s - count) (nxt ?lo ?hi - count))
      (:action consume :parameters (?a ?b - count)
        :precondition (and (avail ?a) (nxt ?b ?a))
        :effect (and (not (avail ?a)) (avail ?b)))
      (:action restore :parameters (?a ?b - count)
        :precondition (and (avail ?a) (nxt ?a ?b))
        :effect (and (not (avail ?a)) (avail ?b))))";
    const PROB: &str = "(define (problem ctr1) (:domain ctr)
      (:objects c0 c1 c2 c3 - count)
      (:init (avail c3) (nxt c0 c1) (nxt c1 c2) (nxt c2 c3))
      (:goal (avail c0)))";

    #[test]
    fn detects_counter_and_orders_occupancy_from_full() {
        let d = parse_domain(DOM).expect("domain");
        let p = parse_problem(PROB).expect("problem");
        let task = match ground(&d, &p, 1) {
            Outcome::Task(t) => t,
            _ => panic!("expected a task"),
        };
        let groups = crate::invariants::synthesize(&d, &task);
        let res = detect_resources(&task, &groups, &task.init_bits);

        assert_eq!(res.len(), 1, "exactly one counter resource detected");
        let r = &res[0];
        assert_eq!(r.members.len(), 4, "4 levels (capacity 3)");
        // The initial/full level (avail c3) is occupancy 0; occupancies are 0..=3.
        assert_eq!(r.occupancy(&task.init_bits), 0, "full level => 0 in use");
        let mut occs: Vec<u32> = r.members.iter().map(|&(_, o)| o).collect();
        occs.sort_unstable();
        assert_eq!(occs, vec![0, 1, 2, 3], "monotone occupancy along the chain");
    }
}
