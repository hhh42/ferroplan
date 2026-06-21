//! Goal partitioning.
//!
//! v1 uses the FINEST granularity — one subgoal per goal item — and lets the
//! resolver coarsen dynamically by merging conflicting groups (which is exactly
//! the dynamic grain-size control SGPlan uses; see docs/sgplan6-spec.md §2,§5).
//! A future phase can seed a better initial partition from the goal-interaction
//! graph (guidance variables + METIS min-cut).

use std::collections::BTreeSet;

use crate::hash::FxHashMap;
use crate::packed::PackedTask;
use crate::types::NumPre;

/// One subproblem's goal: positive fact ids + numeric comparisons.
#[derive(Clone)]
pub struct Subgoal {
    pub pos: Vec<u32>,
    pub num: Vec<NumPre>,
}

impl Subgoal {
    pub fn is_empty(&self) -> bool {
        self.pos.is_empty() && self.num.is_empty()
    }
}

/// Finest partition: each positive goal fact and each numeric goal is its own group.
pub fn partition(task: &PackedTask) -> Vec<Subgoal> {
    let mut groups = Vec::new();
    for &f in &task.goal_pos {
        groups.push(Subgoal {
            pos: vec![f],
            num: vec![],
        });
    }
    for np in &task.goal_num {
        groups.push(Subgoal {
            pos: vec![],
            num: vec![np.clone()],
        });
    }
    // a goal with no items at all (already-true / empty) -> single empty group
    if groups.is_empty() {
        groups.push(Subgoal {
            pos: vec![],
            num: vec![],
        });
    }
    groups
}

/// Merge group `i` with an adjacent group (coarsening). Returns the kept index.
/// No-op when there is nothing to merge (keeps the always-terminate invariant
/// even on misuse; a release-stripped debug_assert previously left a usize
/// underflow path here).
pub fn merge_with_neighbor(groups: &mut Vec<Subgoal>, i: usize) -> usize {
    if groups.len() <= 1 {
        return 0;
    }
    let nb = if i + 1 < groups.len() { i + 1 } else { i - 1 };
    merge_at(groups, i, nb)
}

/// Merge two specific groups (semantic coarsening — used to coalesce the actual
/// conflicting pair rather than a positional neighbor). Returns the kept index.
pub fn merge_at(groups: &mut Vec<Subgoal>, i: usize, j: usize) -> usize {
    if i == j || i >= groups.len() || j >= groups.len() || groups.len() <= 1 {
        return i.min(groups.len().saturating_sub(1));
    }
    let (lo, hi) = (i.min(j), i.max(j));
    let removed = groups.remove(hi);
    groups[lo].pos.extend(removed.pos);
    groups[lo].num.extend(removed.num);
    lo
}

fn uf_find(uf: &mut [usize], x: usize) -> usize {
    let mut r = x;
    while uf[r] != r {
        r = uf[r];
    }
    let mut c = x;
    while uf[c] != c {
        let p = uf[c];
        uf[c] = r;
        c = p;
    }
    r
}

fn uf_union(uf: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (uf_find(uf, a), uf_find(uf, b));
    if ra != rb {
        let (lo, hi) = (ra.min(rb), ra.max(rb));
        uf[hi] = lo;
    }
}

/// Seed the initial partition from a **goal-interaction graph** over mutex
/// variables: two goal facts are linked when some operator achieves (adds) one's
/// variable while disturbing (deleting) the other's. Connected components become
/// the initial subgoal groups; numeric goals stay singletons. Falls back to the
/// finest partition when `groups` is empty. Sound regardless of grain — the
/// resolver still coarsens on conflict.
pub fn interaction_partition(task: &PackedTask, groups: &[Vec<u32>]) -> Vec<Subgoal> {
    if groups.is_empty() || task.goal_pos.is_empty() {
        return partition(task);
    }
    // fact id -> variable id (mutex group index); ungrouped facts are unique.
    let mut var_of: FxHashMap<u32, usize> = FxHashMap::default();
    for (gi, g) in groups.iter().enumerate() {
        for &f in g {
            var_of.insert(f, gi);
        }
    }
    let base = groups.len();
    let var = |f: u32| -> usize { var_of.get(&f).copied().unwrap_or(base + f as usize) };

    // each goal fact is a node; map its variable -> node index
    let n = task.goal_pos.len();
    let mut var_to_goal: FxHashMap<usize, usize> = FxHashMap::default();
    for (gi, &f) in task.goal_pos.iter().enumerate() {
        var_to_goal.entry(var(f)).or_insert(gi);
    }

    let mut uf: Vec<usize> = (0..n).collect();
    for oi in 0..task.n_ops {
        let added: BTreeSet<usize> = task
            .add
            .slice(oi)
            .iter()
            .filter_map(|&f| var_to_goal.get(&var(f)).copied())
            .collect();
        let deleted: BTreeSet<usize> = task
            .del
            .slice(oi)
            .iter()
            .filter_map(|&f| var_to_goal.get(&var(f)).copied())
            .collect();
        for &a in &added {
            for &d in &deleted {
                if a != d {
                    uf_union(&mut uf, a, d);
                }
            }
        }
    }

    let mut comp: FxHashMap<usize, Vec<u32>> = FxHashMap::default();
    for gi in 0..n {
        let r = uf_find(&mut uf, gi);
        comp.entry(r).or_default().push(task.goal_pos[gi]);
    }
    let mut out: Vec<Subgoal> = comp
        .into_values()
        .map(|pos| Subgoal { pos, num: vec![] })
        .collect();
    for np in &task.goal_num {
        out.push(Subgoal {
            pos: vec![],
            num: vec![np.clone()],
        });
    }
    if out.is_empty() {
        out.push(Subgoal {
            pos: vec![],
            num: vec![],
        });
    }
    out
}
