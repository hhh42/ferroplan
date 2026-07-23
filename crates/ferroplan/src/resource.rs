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

/// Resource-trip lower bound (0.14 ext Phase 11, the semantic-landmark
/// rung): each resource-linked goal (one whose achievers move a counter
/// level — transport's `drop` restores `capacity`) consumes one unit of a
/// shared pool per delivery cycle, so meeting `unmet` of them takes at
/// least `⌈unmet / pool⌉` rounds. Folded as a best-first ORDERING term
/// (`FF_RESLM=<w>`), never a pruning bound — the delete relaxation is
/// blind to the counter (levels accumulate), this reads the CONCRETE
/// state.
pub struct TripBound {
    /// Goal facts whose achievers touch a counter level.
    pub goals: Vec<u32>,
    /// Total pool capacity: Σ max occupancy over detected counters.
    pub pool: i64,
}

impl TripBound {
    /// `⌈unmet linked goals / pool⌉` in the concrete state `bits`.
    #[inline]
    pub fn trips(&self, bits: &[u64]) -> i64 {
        let unmet = self
            .goals
            .iter()
            .filter(|&&g| !crate::bitset::test(bits, g as usize))
            .count() as i64;
        (unmet + self.pool - 1) / self.pool
    }
}

/// Build the trip bound for a task, or `None` when no counter resource /
/// linked goal exists (the term is then a no-op by construction).
pub fn trip_bound(task: &PackedTask, groups: &[Vec<u32>], init: &[u64]) -> Option<TripBound> {
    let res = detect_resources(task, groups, init);
    if res.is_empty() {
        return None;
    }
    let members: FxHashSet<u32> = res
        .iter()
        .flat_map(|r| r.members.iter().map(|&(f, _)| f))
        .collect();
    let pool: i64 = res
        .iter()
        .map(|r| r.members.iter().map(|&(_, o)| o as i64).max().unwrap_or(0))
        .sum();
    if pool == 0 {
        return None;
    }
    let goals: Vec<u32> = task
        .goal_pos
        .iter()
        .copied()
        .filter(|&g| {
            task.add_by_fact.slice(g as usize).iter().any(|&oi| {
                task.add
                    .slice(oi as usize)
                    .iter()
                    .chain(task.del.slice(oi as usize).iter())
                    .any(|f| members.contains(f))
            })
        })
        .collect();
    (!goals.is_empty()).then_some(TripBound { goals, pool })
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

    // Transport-shaped micro fixture: one truck, capacity 2 (chain c0-c1-c2),
    // three package goals — the trip bound must read ⌈3/2⌉ = 2 at init.
    const TDOM: &str = "(define (domain tinytrans)
      (:requirements :strips :typing)
      (:types loc pkg cap)
      (:predicates (tat ?l - loc) (pat ?p - pkg ?l - loc) (pin ?p - pkg)
                   (cap ?c - cap) (nxt ?a ?b - cap))
      (:action mv :parameters (?a ?b - loc)
        :precondition (tat ?a) :effect (and (not (tat ?a)) (tat ?b)))
      (:action pick :parameters (?p - pkg ?l - loc ?a ?b - cap)
        :precondition (and (tat ?l) (pat ?p ?l) (nxt ?a ?b) (cap ?b))
        :effect (and (not (pat ?p ?l)) (pin ?p) (cap ?a) (not (cap ?b))))
      (:action drop :parameters (?p - pkg ?l - loc ?a ?b - cap)
        :precondition (and (tat ?l) (pin ?p) (nxt ?a ?b) (cap ?a))
        :effect (and (not (pin ?p)) (pat ?p ?l) (cap ?b) (not (cap ?a)))))";
    // Three locations, so each package's pat/pin mutex group is a STAR
    // (pin borders every location) and is rightly rejected as a counter —
    // only the capacity chain qualifies, as in the real transport corpus.
    const TPROB: &str = "(define (problem tt1) (:domain tinytrans)
      (:objects l1 l2 l3 - loc p1 p2 p3 - pkg c0 c1 c2 - cap)
      (:init (tat l1) (pat p1 l1) (pat p2 l1) (pat p3 l1)
             (cap c2) (nxt c0 c1) (nxt c1 c2))
      (:goal (and (pat p1 l2) (pat p2 l2) (pat p3 l3))))";

    #[test]
    fn trip_bound_reads_demand_over_capacity() {
        let d = parse_domain(TDOM).expect("domain");
        let p = parse_problem(TPROB).expect("problem");
        let task = match ground(&d, &p, 1) {
            Outcome::Task(t) => t,
            _ => panic!("expected a task"),
        };
        let groups = crate::invariants::synthesize(&d, &task);
        let tb = trip_bound(&task, &groups, &task.init_bits)
            .expect("capacity chain + linked goals detected");
        assert_eq!(tb.pool, 2, "one truck, capacity 2");
        assert_eq!(tb.goals.len(), 3, "all three deliveries are linked");
        assert_eq!(tb.trips(&task.init_bits), 2, "ceil(3/2) rounds at init");
    }
}
