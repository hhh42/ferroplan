//! Multi-valued variables over the propositional bitset (0.31 Lane C.0).
//!
//! The abstraction heuristics of Lane C -- pattern databases, and
//! saturated cost partitioning over them -- want a SAS+ view of the task:
//! a finite set of variables, each with a finite domain, a state an
//! assignment. The engine's state is a fact bitset and stays one; what
//! `invariants::synthesize` already finds is exactly what a SAS+
//! translation would give as variables -- sound mutex groups, sets of facts
//! of which at most one is true. A [`Variables`] is a DISJOINT COVER of the
//! facts by those groups, largest first, every uncovered fact its own
//! binary variable, and every variable carrying one extra value, `none`,
//! for "no fact of the group is true" (the groups are at-most-one, never
//! exactly-one -- an invariant synthesis that proved exactly-one would
//! drop the value; this one does not try).
//!
//! The cover is read-only machinery: `value(v, bits)` is a handful of bit
//! tests, and nothing here touches the search.

use crate::bitset;
use crate::packed::PackedTask;
use crate::types::Domain;

#[derive(Clone, Debug)]
pub struct Variables {
    /// Per variable, its facts; value `i` is "fact `vars[v][i]` is true",
    /// value `vars[v].len()` is `none`.
    pub vars: Vec<Vec<u32>>,
    /// fact -> its variable.
    pub fact_var: Vec<u32>,
    /// fact -> its value index in its variable.
    pub fact_val: Vec<u32>,
}

impl Variables {
    /// The cover from the task's mutex groups.
    pub fn from_task(domain: &Domain, task: &PackedTask) -> Variables {
        let groups = crate::invariants::synthesize(domain, task);
        Variables::from_groups(task.n_facts, &groups)
    }

    /// The cover from any family of sound groups: a greedy disjoint
    /// selection, largest first (ties by first fact, so the cover is
    /// deterministic), then singletons for what is left.
    pub fn from_groups(n_facts: usize, groups: &[Vec<u32>]) -> Variables {
        let mut order: Vec<&Vec<u32>> = groups.iter().filter(|g| g.len() >= 2).collect();
        order.sort_by_key(|g| (std::cmp::Reverse(g.len()), g.first().copied().unwrap_or(0)));
        let mut covered = vec![false; n_facts];
        let mut vars: Vec<Vec<u32>> = Vec::new();
        for g in order {
            if g.iter().any(|&f| covered[f as usize]) {
                continue;
            }
            let mut facts = g.clone();
            facts.sort_unstable();
            facts.dedup();
            for &f in &facts {
                covered[f as usize] = true;
            }
            vars.push(facts);
        }
        for (f, &c) in covered.iter().enumerate() {
            if !c {
                vars.push(vec![f as u32]);
            }
        }
        let mut fact_var = vec![u32::MAX; n_facts];
        let mut fact_val = vec![u32::MAX; n_facts];
        for (v, facts) in vars.iter().enumerate() {
            for (i, &f) in facts.iter().enumerate() {
                fact_var[f as usize] = v as u32;
                fact_val[f as usize] = i as u32;
            }
        }
        Variables {
            vars,
            fact_var,
            fact_val,
        }
    }

    pub fn len(&self) -> usize {
        self.vars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }

    /// Domain size of variable `v`: its facts plus `none`.
    pub fn size(&self, v: usize) -> u32 {
        self.vars[v].len() as u32 + 1
    }

    /// The `none` value of variable `v`.
    pub fn none(&self, v: usize) -> u32 {
        self.vars[v].len() as u32
    }

    /// The value of variable `v` in a state: the index of its true fact,
    /// or `none`.
    pub fn value(&self, v: usize, bits: &[u64]) -> u32 {
        for (i, &f) in self.vars[v].iter().enumerate() {
            if bitset::test(bits, f as usize) {
                return i as u32;
            }
        }
        self.none(v)
    }

    /// How many variables are multi-valued (a group of two or more facts).
    pub fn multi_valued(&self) -> usize {
        self.vars.iter().filter(|v| v.len() >= 2).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disjoint_cover_with_singletons() {
        // facts 0..6; groups {0,1,2}, {2,3} (overlaps 2, loses to the
        // larger), {4,5}.
        let v = Variables::from_groups(7, &[vec![2, 3], vec![0, 1, 2], vec![4, 5]]);
        assert_eq!(v.vars, vec![vec![0, 1, 2], vec![4, 5], vec![3], vec![6]]);
        assert_eq!(v.fact_var[2], 0);
        assert_eq!(v.fact_val[2], 2);
        assert_eq!(v.fact_var[3], 2);
        assert_eq!(v.size(0), 4);
        assert_eq!(v.none(0), 3);
        assert_eq!(v.multi_valued(), 2);
        let mut bits = vec![0u64; 1];
        assert_eq!(v.value(0, &bits), 3);
        bitset::set(&mut bits, 1);
        assert_eq!(v.value(0, &bits), 1);
        bitset::set(&mut bits, 6);
        assert_eq!(v.value(3, &bits), 0);
        assert_eq!(v.value(2, &bits), 1);
    }
}
