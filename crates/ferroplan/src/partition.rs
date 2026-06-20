//! Goal partitioning.
//!
//! v1 uses the FINEST granularity — one subgoal per goal item — and lets the
//! resolver coarsen dynamically by merging conflicting groups (which is exactly
//! the dynamic grain-size control SGPlan uses; see docs/sgplan6-spec.md §2,§5).
//! A future phase can seed a better initial partition from the goal-interaction
//! graph (guidance variables + METIS min-cut).

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
    let (lo, hi) = (i.min(nb), i.max(nb));
    let removed = groups.remove(hi);
    groups[lo].pos.extend(removed.pos);
    groups[lo].num.extend(removed.num);
    lo
}
