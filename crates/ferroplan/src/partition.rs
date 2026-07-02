//! Goal partitioning.
//!
//! v1 uses the FINEST granularity — one subgoal per goal item — and lets the
//! resolver coarsen dynamically by merging conflicting groups (which is exactly
//! the dynamic grain-size control SGPlan uses; see docs/sgplan6-spec.md §2,§5).
//! A future phase can seed a better initial partition from the goal-interaction
//! graph (guidance variables + METIS min-cut).

use std::collections::BTreeSet;

use crate::hash::{FxHashMap, FxHashSet};
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
    let mut out = interaction_partition_of(task, groups, &task.goal_pos, &FxHashSet::default());
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

/// [`interaction_partition`]'s core, generalized for the partitioned-ESPC path
/// (`crate::espc`): components over an EXPLICIT positive-goal subset, with
/// designated **shared guidance variables excluded from edge formation** — a goal
/// fact sitting on an excluded variable still becomes a component, but the shared
/// variable is never a merge reason (it is priced as a global constraint by the
/// λ schedule instead, per docs/espc-preferences-spec.md "increment 2"). Numeric
/// goals and the empty-goal fallback are the caller's business. With
/// `goals = &task.goal_pos` and no exclusions this is exactly the old
/// `interaction_partition` body (unit-tested identical), preserving the component
/// order the classical resolver iterates in.
pub fn interaction_partition_of(
    task: &PackedTask,
    groups: &[Vec<u32>],
    goals: &[u32],
    excluded_vars: &FxHashSet<usize>,
) -> Vec<Subgoal> {
    // fact id -> variable id (mutex group index); ungrouped facts are unique.
    let mut var_of: FxHashMap<u32, usize> = FxHashMap::default();
    for (gi, g) in groups.iter().enumerate() {
        for &f in g {
            var_of.insert(f, gi);
        }
    }
    let base = groups.len();
    let var = |f: u32| -> usize { var_of.get(&f).copied().unwrap_or(base + f as usize) };

    // each goal fact is a node; map its variable -> node index, EXCEPT excluded
    // (shared/guidance) variables, which must never carry an interaction edge.
    let n = goals.len();
    let mut var_to_goal: FxHashMap<usize, usize> = FxHashMap::default();
    for (gi, &f) in goals.iter().enumerate() {
        let v = var(f);
        if v < base && excluded_vars.contains(&v) {
            continue;
        }
        var_to_goal.entry(v).or_insert(gi);
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
    for (gi, &f) in goals.iter().enumerate() {
        let r = uf_find(&mut uf, gi);
        comp.entry(r).or_default().push(f);
    }
    comp.into_values()
        .map(|pos| Subgoal { pos, num: vec![] })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::{ground, Outcome};
    use crate::parser::{parse_domain, parse_problem};

    // Two goals coupled ONLY through a token mutex variable: `grab` achieves
    // (done1) while deleting (tok-a) — the variable that goal (tok-b) sits on.
    const DOM: &str = "
    (define (domain t) (:requirements :strips)
      (:predicates (done1) (tok-a) (tok-b))
      (:action grab :precondition (tok-a)
        :effect (and (done1) (not (tok-a)) (tok-b)))
      (:action swap :precondition (tok-b)
        :effect (and (not (tok-b)) (tok-a))))";
    const PRB: &str = "(define (problem p) (:domain t)
      (:init (tok-a)) (:goal (and (done1) (tok-b))))";

    fn task_and_ids() -> (crate::packed::PackedTask, u32, u32, u32) {
        let d = parse_domain(DOM).expect("domain parses");
        let p = parse_problem(PRB).expect("problem parses");
        let task = match ground(&d, &p, 1) {
            Outcome::Task(t) => t,
            _ => panic!("grounds to a task"),
        };
        let fid = |name: &str| {
            task.fact_names
                .iter()
                .position(|n| n == name)
                .unwrap_or_else(|| panic!("fact {name} not found in {:?}", task.fact_names))
                as u32
        };
        let (d1, ta, tb) = (fid("(DONE1)"), fid("(TOK-A)"), fid("(TOK-B)"));
        (task, d1, ta, tb)
    }

    #[test]
    fn shared_variable_merges_goals_by_default() {
        let (task, _d1, ta, tb) = task_and_ids();
        let groups = vec![vec![ta, tb]]; // the token mutex variable
                                         // grab adds (done1) [goal 1's var] and deletes (tok-a) [goal 2's var] -> edge.
        let comps = interaction_partition_of(&task, &groups, &task.goal_pos, &FxHashSet::default());
        assert_eq!(comps.len(), 1, "coupled goals merge into one component");
        assert_eq!(comps[0].pos.len(), 2);
    }

    #[test]
    fn excluded_shared_variable_never_merges() {
        let (task, _d1, ta, tb) = task_and_ids();
        let groups = vec![vec![ta, tb]];
        let mut excluded = FxHashSet::default();
        excluded.insert(0usize); // the token variable is a global-constraint var
        let comps = interaction_partition_of(&task, &groups, &task.goal_pos, &excluded);
        assert_eq!(
            comps.len(),
            2,
            "an excluded guidance variable must not be a merge reason"
        );
    }

    #[test]
    fn interaction_partition_matches_of_with_defaults() {
        let (task, _d1, ta, tb) = task_and_ids();
        let groups = vec![vec![ta, tb]];
        let old = interaction_partition(&task, &groups);
        let new = interaction_partition_of(&task, &groups, &task.goal_pos, &FxHashSet::default());
        // no numeric goals in this task, so the wrapper adds nothing on top
        let key = |sg: &Subgoal| {
            let mut v = sg.pos.clone();
            v.sort_unstable();
            v
        };
        let mut a: Vec<_> = old.iter().map(key).collect();
        let mut b: Vec<_> = new.iter().map(key).collect();
        a.sort();
        b.sort();
        assert_eq!(a, b, "wrapper and core must produce identical components");
    }
}
