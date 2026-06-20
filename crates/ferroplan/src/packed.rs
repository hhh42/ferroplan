//! Data-oriented grounded task (Structure-of-Arrays / CSR) and bitset state.
//!
//! Operators are stored column-wise in CSR arrays (`flat` + `off`) rather than
//! as a `Vec` of structs, so the hot loops (applicability, successor generation,
//! heuristic relaxation) stream contiguous memory and parallelise cleanly over
//! immutable shared task data.

use crate::bitset;
use crate::types::{eval_numpre, AssignOp, NumEff, NumPre};

/// A grounded ADL conditional effect `(when condition effect)`: if `cond_pos`
/// hold, `cond_neg` are absent, and `cond_num` are satisfied IN THE SOURCE STATE,
/// then `add`/`del`/`num` are applied (simultaneously with the unconditional
/// effects, all evaluated against the source state).
#[derive(Clone, Debug, Default)]
pub struct CondEff {
    pub cond_pos: Vec<u32>,
    pub cond_neg: Vec<u32>,
    pub cond_num: Vec<NumPre>,
    pub add: Vec<u32>,
    pub del: Vec<u32>,
    pub num: Vec<NumEff>,
}

/// Compressed-sparse-row container: item `i` owns `flat[off[i]..off[i+1]]`.
#[derive(Clone, Debug, Default)]
pub struct Csr<T> {
    pub flat: Vec<T>,
    pub off: Vec<u32>,
}

impl<T> Csr<T> {
    pub fn slice(&self, i: usize) -> &[T] {
        &self.flat[self.off[i] as usize..self.off[i + 1] as usize]
    }
}

/// Builder that appends one row at a time, tracking offsets.
pub struct CsrBuilder<T> {
    pub flat: Vec<T>,
    pub off: Vec<u32>,
}
impl<T> Default for CsrBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CsrBuilder<T> {
    pub fn new() -> Self {
        CsrBuilder {
            flat: Vec::new(),
            off: vec![0],
        }
    }
    pub fn push_row(&mut self, items: impl IntoIterator<Item = T>) {
        self.flat.extend(items);
        self.off.push(self.flat.len() as u32);
    }
    pub fn finish(self) -> Csr<T> {
        Csr {
            flat: self.flat,
            off: self.off,
        }
    }
}

/// The grounded planning task in data-oriented form.
pub struct PackedTask {
    pub n_facts: usize,
    pub words: usize,
    pub n_ops: usize,

    /// Per-op pretty name for the plan line, e.g. `WALK A0 P0 P1`.
    pub op_display: Vec<String>,

    pub pre_pos: Csr<u32>,
    pub add: Csr<u32>,
    pub del: Csr<u32>,
    pub pre_num: Csr<NumPre>,
    pub num_eff: Csr<NumEff>,
    /// Per-op ADL conditional effects (empty for the common STRIPS/numeric case).
    pub cond: Csr<CondEff>,

    /// fact id -> ops that add it (achiever lookup, avoids O(n_ops) scans).
    pub add_by_fact: Csr<u32>,
    /// fluent id -> ops with a numeric effect on it (numeric-achiever lookup).
    pub neff_by_fluent: Csr<u32>,
    /// fluent id -> read by some numeric precondition or goal (widening filter).
    pub relevant_fluent: Vec<bool>,
    /// the relevant fluent ids (sorted) — the compact `state_key` value vector.
    pub rel_fluents: Vec<u32>,

    pub init_bits: Vec<u64>,
    pub fv0: Vec<f64>,
    pub fdef0: Vec<bool>,

    pub goal_pos: Vec<u32>,
    pub goal_num: Vec<NumPre>,

    pub fact_names: Vec<String>,
    /// fluent id -> display string `(NAME ARGS)` (for metric/cost-fluent lookup).
    pub fluent_names: Vec<String>,

    // timing-footer stats
    pub n_easy: usize,
    pub n_hard: usize,
    pub n_reach_facts: usize,
    pub n_reach_actions: usize,
    pub n_relevant_fluents: usize,
}

// PackedTask is read-only during search, so sharing &PackedTask across threads
// is sound. (All fields are Send + Sync.)

impl PackedTask {
    #[inline]
    pub fn op_applicable(&self, oi: usize, s: &State) -> bool {
        self.pre_pos
            .slice(oi)
            .iter()
            .all(|&f| bitset::test(&s.bits, f as usize))
            && self
                .pre_num
                .slice(oi)
                .iter()
                .all(|np| eval_numpre(np, &s.fv, &s.fdef).unwrap_or(false))
    }

    /// Does conditional effect `ce` fire in source state `s`?
    #[inline]
    fn cond_holds(&self, ce: &CondEff, s: &State) -> bool {
        ce.cond_pos
            .iter()
            .all(|&f| bitset::test(&s.bits, f as usize))
            && ce
                .cond_neg
                .iter()
                .all(|&f| !bitset::test(&s.bits, f as usize))
            && ce
                .cond_num
                .iter()
                .all(|np| eval_numpre(np, &s.fv, &s.fdef).unwrap_or(false))
    }

    /// Apply op `oi` to `s`, returning the successor (assumes applicable).
    /// All effects — unconditional and any firing conditional effects — are
    /// evaluated against the SOURCE state and applied simultaneously (dels then
    /// adds so add wins on conflict; numeric deltas summed from the source).
    pub fn apply(&self, oi: usize, s: &State) -> State {
        let mut ns = s.clone();
        let conds = self.cond.slice(oi);
        let firing: Vec<bool> = conds.iter().map(|ce| self.cond_holds(ce, s)).collect();

        // numeric deltas (from source): unconditional + firing conditional
        let mut deltas: Vec<(usize, AssignOp, f64)> = self
            .num_eff
            .slice(oi)
            .iter()
            .map(|ne| {
                (
                    ne.target as usize,
                    ne.op,
                    ne.value.eval(&s.fv, &s.fdef).unwrap_or(0.0),
                )
            })
            .collect();
        for (ce, &fire) in conds.iter().zip(&firing) {
            if fire {
                for ne in &ce.num {
                    deltas.push((
                        ne.target as usize,
                        ne.op,
                        ne.value.eval(&s.fv, &s.fdef).unwrap_or(0.0),
                    ));
                }
            }
        }

        // logical: all dels first, then all adds
        for &f in self.del.slice(oi) {
            bitset::clear(&mut ns.bits, f as usize);
        }
        for (ce, &fire) in conds.iter().zip(&firing) {
            if fire {
                for &f in &ce.del {
                    bitset::clear(&mut ns.bits, f as usize);
                }
            }
        }
        for &f in self.add.slice(oi) {
            bitset::set(&mut ns.bits, f as usize);
        }
        for (ce, &fire) in conds.iter().zip(&firing) {
            if fire {
                for &f in &ce.add {
                    bitset::set(&mut ns.bits, f as usize);
                }
            }
        }

        for (t, aop, v) in deltas {
            match aop {
                AssignOp::Assign => {
                    ns.fv[t] = v;
                    ns.fdef[t] = true;
                }
                AssignOp::Increase => ns.fv[t] += v,
                AssignOp::Decrease => ns.fv[t] -= v,
                AssignOp::ScaleUp => ns.fv[t] *= v,
                AssignOp::ScaleDown => ns.fv[t] /= v,
            }
        }
        ns
    }

    /// Look up a fluent id by display string, e.g. `(TOTAL-COST)`.
    pub fn fluent_id(&self, disp: &str) -> Option<usize> {
        self.fluent_names.iter().position(|s| s == disp)
    }

    /// Look up a fact id by display string, e.g. `(AT A0 P1)`.
    pub fn fact_id(&self, disp: &str) -> Option<usize> {
        self.fact_names.iter().position(|s| s == disp)
    }

    pub fn initial(&self) -> State {
        State {
            bits: self.init_bits.clone(),
            fv: self.fv0.clone(),
            fdef: self.fdef0.clone(),
        }
    }

    pub fn goal_met(&self, s: &State) -> bool {
        self.goal_met_with(s, &self.goal_pos, &self.goal_num)
    }

    /// Visited-set key: facts + only the RELEVANT fluent values. A fluent is
    /// irrelevant iff it appears in no precondition/goal AND (transitively) in no
    /// RHS of any effect that writes a relevant fluent — i.e. a purely write-only
    /// accumulator (walkedTime/drivenTime/fuelUsed). Such a fluent cannot change
    /// applicability or goal satisfaction now or later, so two states differing
    /// only in it are behaviourally identical and must dedup; otherwise an
    /// unbounded accumulator yields infinitely many "distinct" states and the
    /// search never terminates on unsolvable problems. `relevant_fluent` is built
    /// as the transitive closure in ground.rs, which makes this sound.
    pub fn state_key(&self, s: &State) -> StateKey {
        // Compact: only the RELEVANT fluents (usually few) go in the key, in a
        // fixed order. Irrelevant/undefined ones never distinguish states, so
        // omitting them is exact and shrinks the cloned+hashed key dramatically
        // (pure-STRIPS keys carry no vals at all).
        let vals: Vec<i64> = self
            .rel_fluents
            .iter()
            .map(|&i| {
                let i = i as usize;
                if s.fdef[i] {
                    (s.fv[i] * 1e6).round() as i64
                } else {
                    0
                }
            })
            .collect();
        StateKey {
            bits: s.bits.clone(),
            vals,
        }
    }

    /// Visited key for the branch-and-bound bounded search: the compact key plus
    /// the cost fluent's value appended, so equal-fact/different-cost states stay
    /// distinct (the cost fluent is read by no precond/goal, so it isn't in
    /// `rel_fluents`). One code path for both init and successors.
    pub fn state_key_with_cost(&self, s: &State, cost_fluent: Option<usize>) -> StateKey {
        let mut k = self.state_key(s);
        if let Some(cf) = cost_fluent {
            k.vals.push(if s.fdef[cf] {
                (s.fv[cf] * 1e6).round() as i64
            } else {
                0
            });
        }
        k
    }

    /// Goal test against an arbitrary (sub)goal — used by the subplanner API.
    pub fn goal_met_with(&self, s: &State, goal_pos: &[u32], goal_num: &[NumPre]) -> bool {
        goal_pos.iter().all(|&f| bitset::test(&s.bits, f as usize))
            && goal_num
                .iter()
                .all(|np| eval_numpre(np, &s.fv, &s.fdef).unwrap_or(false))
    }
}

/// A search state: fact bitset + dense fluent values.
#[derive(Clone)]
pub struct State {
    pub bits: Vec<u64>,
    pub fv: Vec<f64>,
    pub fdef: Vec<bool>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StateKey {
    pub bits: Vec<u64>,
    pub vals: Vec<i64>,
}
