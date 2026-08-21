//! The SAT compilation wing (0.24 Phases 2+3): a bounded-layer ∃-step
//! encoding of the grounded task over the in-tree [`ferroplan_sat`] CDCL
//! solver, with a geometric horizon ramp for the classical face and
//! STN-taught CEGAR (the existing scheduler as the teacher) for the
//! temporal face.
//!
//! ## The ∃-step semantics claim, and the decode's serialization argument
//!
//! A model assigns each layer `t ∈ 0..H` a SET of ops. The encoding fixes
//! one global chain order — ascending grounded op id — and admits a set
//! at a layer only when executing its members sequentially IN THAT ORDER
//! is valid from the layer's entry state and reaches exactly the layer's
//! exit state:
//!
//! - every member's precondition holds at the layer's ENTRY state
//!   (op ⇒ pre clauses), and the Rintanen-style disabling chain (one
//!   linear accumulator per fact over that fact's deleters, in chain
//!   order) forbids any member from executing after an earlier-in-chain
//!   member deleted one of its preconditions — so each precondition still
//!   holds at the member's own position in the serialization;
//! - effect clauses pin the exit state to the union of member effects,
//!   and an add/delete conflict on one fact is unsatisfiable by those
//!   same clauses — so no delete-then-re-add (or the reverse) can hide
//!   inside a layer, and the serialized end state equals the layer's exit
//!   state exactly;
//! - explanatory frames make every change name a cause, so an empty
//!   layer copies its state (empty layers are free — no noop op needed;
//!   the frames ARE the noop bits).
//!
//! The decoder replays exactly that serialization — layers in order, ops
//! within a layer in chain order — through [`PackedTask::apply`] with an
//! applicability check per step, so every emitted plan carries its own
//! serialization proof; a replay miss is an encoder bug and is refused,
//! never shipped. This is deliberately the incomplete-per-layer fragment
//! of ∃-step semantics (preconditions are anchored at layer entry, so a
//! member cannot enable another member's precondition mid-layer): it
//! only prunes packings, never plans — any plan lays out one op per
//! layer, and the horizon ramp reaches it.
//!
//! ## The temporal face (Phase 3)
//!
//! The snap-compiled task's start/end events are the layered actions;
//! durations NEVER enter the CNF. The snap `RUNNING-*` token facts carry
//! the pairing structurally (an END requires and deletes its token), and
//! three clause families finish it: no-self-overlap (a start is blocked
//! while its own token is up), all tokens false at the final state (every
//! start ⇒ a later end within the horizon), and the interval invariants
//! from the `InvMap` (token@state ⇒ over-all fact held, plus
//! interference blockers so no co-layer op breaks an open interval). The
//! decoded event sequence goes to a simple temporal network — order edges
//! ε apart, `end = start + duration` equalities — solved by longest-path
//! Bellman–Ford. A positive cycle means the causal order cannot be
//! scheduled: the cycle's events become a refutation clause (that exact
//! event-order core, minimal-ish — the co-placement of the cycle's
//! op@layer assignments is forbidden) and the solver re-solves inside the
//! same horizon — CEGAR with the scheduler as teacher. Feasible times
//! then run the SAME emission rite as a search goal pop (the 0.22
//! topological ε-separation + duration reconciliation, via
//! `crate::temporal::emit_scheduled`) and the result is validated
//! against the ORIGINAL problem before it is ever returned.
//!
//! ## Honesty rules (the 0.21 wording bar, inherited)
//!
//! The encoder prices itself BEFORE building (exact per-layer clause and
//! literal counts) and declines over the cap with a named note — never a
//! hang. A conflict-budget trip is "no plan within horizon H", never
//! "unsolvable"; a proven-UNSAT verdict is claimed per horizon only when
//! the budget did NOT trip, and even a full-ramp UNSAT sweep stays a
//! bounded-horizon verdict.

use crate::packed::{PackedTask, State};
use crate::temporal::{InvMap, Kind, TimedPlan, TimedStep};
use crate::types::{Domain, Effect, Formula, Problem, TimeSpec};
use ferroplan_sat::{ExtendFormula, Lit, Solver};
use std::collections::{HashMap, HashSet};

/// ε between consecutive happenings in the decoded schedule — the grid
/// the emission machinery snaps to.
const EPS: f64 = 0.001;

/// The pre-registered thrash bound (Phase 3): more STN refutations than
/// this in ONE horizon is a measured negative — record and bail to ramp.
const MAX_REFUTATIONS_PER_HORIZON: usize = 100;

/// Ramp + budget knobs. Tests construct this directly; production entries
/// use [`SatCfg::from_env`] so the knobs stay probe-shaped
/// (`FF_SAT_HORIZON`, `FF_SAT_CONFLICTS`, `FF_SAT_CAP`).
#[derive(Clone, Debug)]
pub struct SatCfg {
    /// Largest horizon the geometric ramp reaches (1, 2, 4, …, ≤ this).
    pub max_horizon: usize,
    /// Conflict budget per horizon (shared across CEGAR re-solves at that
    /// horizon), spent in slices so the wall checkpoint stays live.
    pub conflicts_per_horizon: u64,
    /// Encoder self-pricing cap: estimated vars + clause literals above
    /// this ⇒ decline honestly. Additionally tightened by the remaining
    /// armed wall (see `price_cap`).
    pub cap_lits: u64,
}

impl Default for SatCfg {
    fn default() -> Self {
        SatCfg {
            max_horizon: 128,
            conflicts_per_horizon: 200_000,
            cap_lits: 50_000_000,
        }
    }
}

impl SatCfg {
    pub fn from_env() -> Self {
        let mut c = SatCfg::default();
        if let Some(v) = env_usize("FF_SAT_HORIZON") {
            c.max_horizon = v.max(1);
        }
        if let Some(v) = env_usize("FF_SAT_CONFLICTS") {
            c.conflicts_per_horizon = (v as u64).max(1);
        }
        if let Some(v) = env_usize("FF_SAT_CAP") {
            c.cap_lits = (v as u64).max(1);
        }
        c
    }
}

fn env_usize(var: &str) -> Option<usize> {
    std::env::var(var).ok().and_then(|s| s.trim().parse().ok())
}

fn wall_debug() -> bool {
    std::env::var("FF_WALL_DEBUG").is_ok()
}

/// The wing's verdict: a plan (replay-verified; temporal plans
/// additionally validated against the original problem) or the honest
/// no-plan story. A capped or declined exit never contains the word
/// "unsolvable" — the 0.21 wording bar, enforced by construction.
pub struct SatOutcome<P> {
    pub plan: Option<P>,
    /// The story: ramp trail, decline reasons, cap notices, refutation
    /// counts. Rendered into `Solution::notes` / the text path verbatim.
    pub notes: Vec<String>,
    /// Every horizon tried came back proven UNSAT with budget intact —
    /// the strongest available no-plan verdict, still bounded-horizon.
    pub proven_at_every_horizon: bool,
    pub grounded_facts: usize,
    pub grounded_actions: usize,
}

impl<P> SatOutcome<P> {
    fn declined(why: String, facts: usize, ops: usize) -> Self {
        SatOutcome {
            plan: None,
            notes: vec![format!("SAT encoder declined: {why}")],
            proven_at_every_horizon: false,
            grounded_facts: facts,
            grounded_actions: ops,
        }
    }
}

// ---------------------------------------------------------------------------
// The encoder core (shared by both faces).
// ---------------------------------------------------------------------------

/// Chain plan for one fact: the disabling accumulator is built only up to
/// the deepest prefix any requirer actually checks.
struct ChainPlan {
    /// number of accumulator vars needed (max requirer prefix depth).
    jmax: u32,
    /// (requirer slot, prefix depth j ≥ 1): requirer ⇒ ¬c_j.
    reqs: Vec<(u32, u32)>,
}

/// The horizon-independent encoding tables, built once per task and
/// reused (with exact pricing) at every rung of the ramp.
struct EncTask<'a> {
    task: &'a PackedTask,
    /// encodable op ids, ascending — the fixed chain order.
    ops: Vec<u32>,
    /// op id -> dense slot, or `u32::MAX` (not encoded).
    slot_of: Vec<u32>,
    /// per slot: effective adds / deletes (delete-then-add collapses to
    /// add, matching `PackedTask::apply`'s del-then-add rule).
    addeff: Vec<Vec<u32>>,
    deleff: Vec<Vec<u32>>,
    /// fact -> slots adding / deleting it (ascending = chain order).
    adders_of: Vec<Vec<u32>>,
    dels_of: Vec<Vec<u32>>,
    /// fact -> disabling-chain plan (only facts with a guarded requirer).
    chain: Vec<Option<ChainPlan>>,
    /// fact -> offset of its accumulator block within a layer's chain
    /// vars; `u32::MAX` when the fact has no chain.
    chain_off: Vec<u32>,
    chain_per_layer: usize,
    /// mutex-group seed pairs (at-most-one facts, pairwise).
    mutex_pairs: Vec<(u32, u32)>,
    /// temporal only: (start slot, token fact) no-self-overlap pairs; the
    /// token list doubles as the ¬token@H pairing closure.
    tokens: Vec<(u32, u32)>,
    /// temporal only: per interval — (token fact, over-all positives,
    /// over-all negatives), held at every state where the token is up.
    inv_holds: Vec<(u32, Vec<u32>, Vec<u32>)>,
    /// temporal only: (token fact, op slot) — the op's unconditional
    /// effects would break the open interval's invariant; blocked while
    /// the token is up.
    inv_blockers: Vec<(u32, u32)>,
}

/// Drop TRUE-TWIN ground ops — identical display AND identical
/// pre/add/del content — keeping the LAST twin: the op every by-display
/// pointer (`build_kind`'s `end_op` resolution) lands on. The
/// constituency is the IPC dual-typed-object trick (TMS declares
/// `kiln0 - kiln8` AND `kiln0 - kiln20`, so supertype enumeration
/// grounds every bake twice): twins are interchangeable in any plan, so
/// excluding the shadow twin loses nothing — but keeping it lets a model
/// fire an END the pairing pointers never opened (the decoded-END
/// mismatch the TMS repro caught). Same-display ops with DIFFERENT
/// content — the REACH-GOAL disjunct closers — are all kept.
fn drop_twin_ops(task: &PackedTask, ops: Vec<u32>) -> Vec<u32> {
    let mut canon: HashMap<&str, u32> = HashMap::new();
    for &oi in &ops {
        canon.insert(task.op_display[oi as usize].as_str(), oi);
    }
    ops.into_iter()
        .filter(|&oi| {
            let c = canon[task.op_display[oi as usize].as_str()];
            c == oi || !same_op_content(task, oi as usize, c as usize)
        })
        .collect()
}

/// Twin test: the propositional footprint and the (empty-or-not) shape of
/// everything else. The encodable slice has no numerics or conditional
/// effects anyway (`encodability` declines them), so slice-length equality
/// there is exact within it.
fn same_op_content(task: &PackedTask, a: usize, b: usize) -> bool {
    task.pre_pos.slice(a) == task.pre_pos.slice(b)
        && task.add.slice(a) == task.add.slice(b)
        && task.del.slice(a) == task.del.slice(b)
        && task.pre_num.slice(a).len() == task.pre_num.slice(b).len()
        && task.num_eff.slice(a).len() == task.num_eff.slice(b).len()
        && task.n_cond_effs(a) == task.n_cond_effs(b)
}

/// Why a task cannot enter the CNF — each variant is a named decline.
fn encodability(task: &PackedTask, ops: &[u32]) -> Result<(), String> {
    if !task.goal_num.is_empty() {
        return Err("numeric goal conditions".into());
    }
    let mut n_num = 0usize;
    let mut n_cond = 0usize;
    for &oi in ops {
        let oi = oi as usize;
        if !task.pre_num.slice(oi).is_empty() || !task.num_eff.slice(oi).is_empty() {
            n_num += 1;
        }
        if task.n_cond_effs(oi) > 0 {
            n_cond += 1;
        }
    }
    if n_num > 0 {
        return Err(format!("numeric preconditions/effects on {n_num} ops"));
    }
    if n_cond > 0 {
        return Err(format!("conditional effects on {n_cond} ops"));
    }
    Ok(())
}

impl<'a> EncTask<'a> {
    /// Build the tables over `ops` (already filtered to the encodable
    /// kinds). `groups` are invariants.rs mutex groups (seed clauses).
    fn build(task: &'a PackedTask, ops: Vec<u32>, groups: &[Vec<u32>]) -> EncTask<'a> {
        let f = task.n_facts;
        let mut slot_of = vec![u32::MAX; task.n_ops];
        for (s, &oi) in ops.iter().enumerate() {
            slot_of[oi as usize] = s as u32;
        }
        let n = ops.len();
        let mut addeff = Vec::with_capacity(n);
        let mut deleff = Vec::with_capacity(n);
        let mut adders_of: Vec<Vec<u32>> = vec![Vec::new(); f];
        let mut dels_of: Vec<Vec<u32>> = vec![Vec::new(); f];
        for (s, &oi) in ops.iter().enumerate() {
            let oi = oi as usize;
            let add: Vec<u32> = task.add.slice(oi).to_vec();
            let del: Vec<u32> = task
                .del
                .slice(oi)
                .iter()
                .filter(|p| !add.contains(p))
                .copied()
                .collect();
            for &p in &add {
                adders_of[p as usize].push(s as u32);
            }
            for &p in &del {
                dels_of[p as usize].push(s as u32);
            }
            addeff.push(add);
            deleff.push(del);
        }
        // Disabling chains: each requirer of a fact is guarded against the
        // deleters strictly earlier in the chain order.
        let mut chain: Vec<Option<ChainPlan>> = (0..f).map(|_| None).collect();
        for (s, &oi) in ops.iter().enumerate() {
            for &p in task.pre_pos.slice(oi as usize) {
                let dels = &dels_of[p as usize];
                if dels.is_empty() {
                    continue;
                }
                let j = dels.partition_point(|&d| (d as usize) < s) as u32;
                if j == 0 {
                    continue;
                }
                let cp = chain[p as usize].get_or_insert(ChainPlan {
                    jmax: 0,
                    reqs: Vec::new(),
                });
                cp.jmax = cp.jmax.max(j);
                cp.reqs.push((s as u32, j));
            }
        }
        let mut chain_off = vec![u32::MAX; f];
        let mut chain_per_layer = 0usize;
        for (p, cp) in chain.iter().enumerate() {
            if let Some(cp) = cp {
                chain_off[p] = chain_per_layer as u32;
                chain_per_layer += cp.jmax as usize;
            }
        }
        // Mutex seeds: sound because every group is an at-most-one
        // invariant of the reachable space and each CNF state is
        // reachable by the serialization argument — the clauses prune
        // solver search, never plans. Pairwise, small groups only (the
        // seeds are redundant hints, not semantics).
        let mut mutex_pairs = Vec::new();
        for g in groups {
            if g.len() < 2 || g.len() > 12 {
                continue;
            }
            for i in 0..g.len() {
                for j in (i + 1)..g.len() {
                    mutex_pairs.push((g[i], g[j]));
                }
            }
        }
        EncTask {
            task,
            ops,
            slot_of,
            addeff,
            deleff,
            adders_of,
            dels_of,
            chain,
            chain_off,
            chain_per_layer,
            mutex_pairs,
            tokens: Vec::new(),
            inv_holds: Vec::new(),
            inv_blockers: Vec::new(),
        }
    }

    /// Wire the temporal families (tokens, pairing closure, interval
    /// invariants). Returns a decline reason if an interval carries a
    /// numeric over-all conjunct or its token cannot be identified.
    fn add_temporal(&mut self, kind: &[Kind], inv: &InvMap) -> Result<(), String> {
        for (s, &oi) in self.ops.iter().enumerate() {
            if let Kind::Start { end_op, .. } = kind[oi as usize] {
                let token = self.find_token(oi as usize, end_op)?;
                self.tokens.push((s as u32, token));
                if let Some((pos, neg, num)) = inv.get(&end_op) {
                    if !num.is_empty() {
                        return Err("numeric over-all invariants".into());
                    }
                    if !pos.is_empty() || !neg.is_empty() {
                        self.inv_holds.push((token, pos.clone(), neg.clone()));
                        let end_slot = self.slot_of[end_op];
                        let mut blocked: HashSet<u32> = HashSet::new();
                        for &p in pos {
                            for &d in &self.dels_of[p as usize] {
                                if d != end_slot {
                                    blocked.insert(d);
                                }
                            }
                        }
                        for &q in neg {
                            for &a in &self.adders_of[q as usize] {
                                if a != end_slot {
                                    blocked.insert(a);
                                }
                            }
                        }
                        let mut blocked: Vec<u32> = blocked.into_iter().collect();
                        blocked.sort_unstable();
                        for b in blocked {
                            self.inv_blockers.push((token, b));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// The snap `RUNNING-*` token: added by the start, required and
    /// deleted by the paired end.
    fn find_token(&self, start_op: usize, end_op: usize) -> Result<u32, String> {
        let epre = self.task.pre_pos.slice(end_op);
        let edel = self.task.del.slice(end_op);
        let cands: Vec<u32> = self
            .task
            .add
            .slice(start_op)
            .iter()
            .copied()
            .filter(|p| epre.contains(p) && edel.contains(p))
            .collect();
        cands
            .iter()
            .copied()
            .find(|&p| self.task.fact_names[p as usize].starts_with("(RUNNING-"))
            .or_else(|| cands.first().copied())
            .ok_or_else(|| format!("no pairing token for `{}`", self.task.op_display[start_op]))
    }

    // -- variable layout ---------------------------------------------------

    fn n_facts(&self) -> usize {
        self.task.n_facts
    }
    fn fact_var(&self, t: usize, p: u32) -> usize {
        t * self.n_facts() + p as usize
    }
    fn op_var(&self, h: usize, t: usize, slot: u32) -> usize {
        (h + 1) * self.n_facts() + t * self.ops.len() + slot as usize
    }
    fn chain_var(&self, h: usize, t: usize, p: u32, i: u32) -> usize {
        (h + 1) * self.n_facts()
            + h * self.ops.len()
            + t * self.chain_per_layer
            + self.chain_off[p as usize] as usize
            + (i - 1) as usize
    }
    fn n_vars(&self, h: usize) -> usize {
        (h + 1) * self.n_facts() + h * (self.ops.len() + self.chain_per_layer)
    }

    // -- exact pricing -----------------------------------------------------

    /// Exact (clauses, literals) the encoding at horizon `h` will emit —
    /// computed WITHOUT building anything, so a decline costs
    /// milliseconds, never a hang.
    fn price(&self, h: usize) -> (u64, u64) {
        let f = self.n_facts() as u64;
        let hh = h as u64;
        let mut layer_cl = 0u64;
        let mut layer_lit = 0u64;
        for (s, &oi) in self.ops.iter().enumerate() {
            let pre = self.task.pre_pos.slice(oi as usize).len() as u64;
            let eff = (self.addeff[s].len() + self.deleff[s].len()) as u64;
            layer_cl += pre + eff;
            layer_lit += 2 * (pre + eff);
        }
        for p in 0..self.n_facts() {
            layer_cl += 2; // the two frames
            layer_lit += 4 + self.adders_of[p].len() as u64 + self.dels_of[p].len() as u64;
            if let Some(cp) = &self.chain[p] {
                let j = cp.jmax as u64;
                layer_cl += j + j.saturating_sub(1) + cp.reqs.len() as u64;
                layer_lit += 2 * (j + j.saturating_sub(1) + cp.reqs.len() as u64);
            }
        }
        layer_cl += (self.inv_blockers.len() + self.tokens.len()) as u64;
        layer_lit += 2 * (self.inv_blockers.len() + self.tokens.len()) as u64;
        let mut state_cl = self.mutex_pairs.len() as u64;
        for (_, pos, neg) in &self.inv_holds {
            state_cl += (pos.len() + neg.len()) as u64;
        }
        let state_lit = 2 * state_cl;
        let units = f + self.task.goal_pos.len() as u64 + self.tokens.len() as u64;
        let clauses = hh * layer_cl + (hh + 1) * state_cl + units;
        let lits = hh * layer_lit + (hh + 1) * state_lit + units;
        (clauses, lits)
    }

    // -- clause emission ---------------------------------------------------

    /// Build the full CNF at horizon `h` into a fresh solver.
    fn encode(&self, h: usize) -> Solver {
        let mut s = Solver::new();
        let pos = |v: usize| Lit::from_index(v, true);
        let neg = |v: usize| Lit::from_index(v, false);
        // S_0: exactly the initial state.
        for p in 0..self.n_facts() {
            let v = self.fact_var(0, p as u32);
            if crate::bitset::test(&self.task.init_bits, p) {
                s.add_clause(&[pos(v)]);
            } else {
                s.add_clause(&[neg(v)]);
            }
        }
        // Goal at S_H, plus every pairing token closed.
        for &g in &self.task.goal_pos {
            s.add_clause(&[pos(self.fact_var(h, g))]);
        }
        for &(_, token) in &self.tokens {
            s.add_clause(&[neg(self.fact_var(h, token))]);
        }
        // Mutex seeds + interval-invariant holds at every state.
        for t in 0..=h {
            for &(a, b) in &self.mutex_pairs {
                s.add_clause(&[neg(self.fact_var(t, a)), neg(self.fact_var(t, b))]);
            }
            for (token, ipos, ineg) in &self.inv_holds {
                let tk = neg(self.fact_var(t, *token));
                for &p in ipos {
                    s.add_clause(&[tk, pos(self.fact_var(t, p))]);
                }
                for &q in ineg {
                    s.add_clause(&[tk, neg(self.fact_var(t, q))]);
                }
            }
        }
        let mut cls: Vec<Lit> = Vec::with_capacity(8);
        for t in 0..h {
            for (slot, &oi) in self.ops.iter().enumerate() {
                let ov = self.op_var(h, t, slot as u32);
                // preconditions at layer entry
                for &p in self.task.pre_pos.slice(oi as usize) {
                    s.add_clause(&[neg(ov), pos(self.fact_var(t, p))]);
                }
                // effects at layer exit
                for &p in &self.addeff[slot] {
                    s.add_clause(&[neg(ov), pos(self.fact_var(t + 1, p))]);
                }
                for &p in &self.deleff[slot] {
                    s.add_clause(&[neg(ov), neg(self.fact_var(t + 1, p))]);
                }
            }
            // explanatory frames + disabling chains
            for p in 0..self.n_facts() {
                let pu = p as u32;
                cls.clear();
                cls.push(pos(self.fact_var(t, pu)));
                cls.push(neg(self.fact_var(t + 1, pu)));
                for &a in &self.adders_of[p] {
                    cls.push(pos(self.op_var(h, t, a)));
                }
                s.add_clause(&cls);
                cls.clear();
                cls.push(neg(self.fact_var(t, pu)));
                cls.push(pos(self.fact_var(t + 1, pu)));
                for &d in &self.dels_of[p] {
                    cls.push(pos(self.op_var(h, t, d)));
                }
                s.add_clause(&cls);
                if let Some(cp) = &self.chain[p] {
                    for i in 1..=cp.jmax {
                        let cv = pos(self.chain_var(h, t, pu, i));
                        let d = self.dels_of[p][(i - 1) as usize];
                        s.add_clause(&[neg(self.op_var(h, t, d)), cv]);
                        if i > 1 {
                            s.add_clause(&[neg(self.chain_var(h, t, pu, i - 1)), cv]);
                        }
                    }
                    for &(r, j) in &cp.reqs {
                        s.add_clause(&[
                            neg(self.op_var(h, t, r)),
                            neg(self.chain_var(h, t, pu, j)),
                        ]);
                    }
                }
            }
            // temporal: no self-overlap + open-interval interference blocks
            for &(start_slot, token) in &self.tokens {
                s.add_clause(&[
                    neg(self.op_var(h, t, start_slot)),
                    neg(self.fact_var(t, token)),
                ]);
            }
            for &(token, b) in &self.inv_blockers {
                s.add_clause(&[neg(self.fact_var(t, token)), neg(self.op_var(h, t, b))]);
            }
        }
        s
    }

    /// Decode a model into the serialized (op id, layer) sequence —
    /// layers ascending, chain order (ascending op id) within a layer.
    /// Vars a simplified model leaves unassigned read as FALSE (any value
    /// satisfies the formula; false fires no op).
    fn decode(&self, h: usize, model: &[Lit]) -> Vec<(u32, u32)> {
        let mut val = vec![false; self.n_vars(h)];
        for l in model {
            if l.index() < val.len() {
                val[l.index()] = l.is_positive();
            }
        }
        let mut out = Vec::new();
        for t in 0..h {
            for (slot, &oi) in self.ops.iter().enumerate() {
                if val[self.op_var(h, t, slot as u32)] {
                    out.push((oi, t as u32));
                }
            }
        }
        out
    }

    /// The serialization proof: replay the decoded sequence through the
    /// task's own executor. `Some(final)` iff every op applies in order
    /// and the goal holds at the end.
    fn replay(&self, seq: &[(u32, u32)]) -> Option<State> {
        let mut st = self.task.initial();
        for &(oi, _) in seq {
            if !self.task.op_applicable(oi as usize, &st) {
                return None;
            }
            st = self.task.apply(oi as usize, &st);
        }
        self.task.goal_met(&st).then_some(st)
    }
}

// ---------------------------------------------------------------------------
// The horizon ramp (shared driver).
// ---------------------------------------------------------------------------

enum RampStep<R> {
    /// SAT at this horizon — the callback's accepted payload.
    Done(R),
    /// proven UNSAT at this horizon, budget intact.
    Unsat,
    /// conflict budget tripped (or a CEGAR thrash bail) — NOT a proof.
    Capped,
    /// armed wall expired — stop the whole ramp.
    Wall,
    /// conflict-rate bail (promoted slice only): the measured rate says
    /// this horizon cannot finish its budget inside the remaining slice,
    /// and every deeper horizon is bigger — stop the whole ramp and hand
    /// the rest of the slice back to the ladder. NOT a proof.
    Hopeless,
}

/// The promoted rung's own deadline (0.24 regression fix): the router's
/// early promotion hands the wing a bounded SLICE of the wall, so a
/// horizon that grinds its conflict budget in pure SAT conflicts (no STN
/// refutations — the thrash bail never sees it; match-cellar's cut
/// instances at h32) cannot starve the ladder fallback. `None` = no
/// slice (the exhaustion-armed and `Mode::Sat` entries, and every
/// no-wall path — all byte-identical to the pre-slice wing).
type WallSlice = Option<(crate::clock::Clock, f64)>;

/// Effective pricing cap: the configured cap, tightened by the remaining
/// armed wall — and by the promoted slice when one is armed (a CNF we
/// cannot build AND solve inside the tighter of the two is a decline,
/// not a hang) — ~4M literals/second of build+solve throughput as the
/// conservative conversion.
fn price_cap(cfg: &SatCfg, slice: WallSlice) -> u64 {
    let rem = match (
        crate::search::wall_remaining_secs(),
        crate::search::deadline_remaining_secs(&slice),
    ) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, None) => a,
        (None, b) => b,
    };
    match rem {
        Some(s) if s > 0.0 => cfg.cap_lits.min((s * 4_000_000.0) as u64),
        Some(_) => 1, // wall (or slice) already spent: everything declines
        None => cfg.cap_lits,
    }
}

fn wall_expired(slice: WallSlice) -> bool {
    crate::search::rung_wallcap_on()
        && (crate::search::wall_deadline()
            .is_some_and(|d| crate::search::deadline_expired_reserving(d, 0))
            || slice.is_some_and(|d| crate::search::deadline_expired_reserving(d, 0)))
}

/// Which clock ran out, for the honest note: the process wall, or the
/// promoted rung's own slice.
fn wall_story(slice: WallSlice) -> &'static str {
    let global = crate::search::wall_deadline()
        .is_some_and(|d| crate::search::deadline_expired_reserving(d, 0));
    if !global && slice.is_some() {
        "promoted wall slice"
    } else {
        "wall"
    }
}

/// Run the solver at one horizon in conflict slices (wall checkpoints
/// between slices). `on_model` either accepts a model (final payload) or
/// returns `Err(Some(clause))` — a refutation to assert and re-solve
/// inside the same horizon — or `Err(None)` to bail to the ramp.
fn solve_horizon<R, F>(
    solver: &mut Solver,
    budget: u64,
    wall_slice: WallSlice,
    mut on_model: F,
) -> RampStep<R>
where
    F: FnMut(&[Lit]) -> Result<R, Option<Vec<Lit>>>,
{
    const SLICE: u64 = 4_096;
    // The conflict-rate bail (0.25 Wing II, the residue the 0.24 cut
    // record priced at ~15 s/row): a promoted bet's grinding horizon
    // must not spend the WHOLE slice learning "no". Once the measured
    // conflict rate says finishing this horizon's budget would eat more
    // than RATE_BAIL_FRAC of the remaining slice, the verdict is already
    // known — "no plan within budget at this horizon" — and every deeper
    // horizon is strictly bigger, so the ramp is done either way; the
    // only thing continuing buys is a bet on a mid-budget model, exactly
    // the bet the 0.24 receipts price at zero on grinding horizons.
    // Armed ONLY under a wall slice (the promoted entry) — Mode::Sat and
    // the exhaustion rung have no ladder behind them to refund.
    // `FF_NO_SAT_RATEBAIL=1` restores the 0.24 slice-spend shape.
    const RATE_BAIL_FRAC: f64 = 0.8;
    const RATE_MIN_CONFLICTS: u64 = 8_192;
    const RATE_MIN_SECS: f64 = 1.0;
    let rate_bail_armed = wall_slice.is_some() && std::env::var("FF_NO_SAT_RATEBAIL").is_err();
    let t0 = crate::clock::Clock::now();
    let mut spent = 0u64;
    loop {
        if wall_expired(wall_slice) {
            return RampStep::Wall;
        }
        if rate_bail_armed && spent >= RATE_MIN_CONFLICTS {
            let elapsed = t0.elapsed_secs();
            if elapsed >= RATE_MIN_SECS {
                let rate = spent as f64 / elapsed;
                let est = (budget - spent) as f64 / rate.max(1.0);
                let remaining =
                    crate::search::deadline_remaining_secs(&wall_slice).unwrap_or(f64::INFINITY);
                if est > remaining * RATE_BAIL_FRAC {
                    if wall_debug() {
                        eprintln!(
                            "[sat] conflict-rate bail: {spent} conflicts in \
                             {elapsed:.1}s (~{rate:.0}/s); finishing the \
                             {budget} budget needs ~{est:.0}s of the \
                             {remaining:.0}s slice left"
                        );
                    }
                    return RampStep::Hopeless;
                }
            }
        }
        let slice = SLICE.min(budget - spent);
        solver.set_conflict_limit(Some(slice));
        match solver.solve() {
            Ok(true) => {
                let model = solver.model().unwrap_or_default();
                match on_model(&model) {
                    Ok(r) => return RampStep::Done(r),
                    Err(Some(refutation)) => solver.add_clause(&refutation),
                    Err(None) => return RampStep::Capped,
                }
            }
            Ok(false) => return RampStep::Unsat,
            // Interrupted (the conflict budget slice) is the only error the
            // absorbed solver produces today; the enum is non_exhaustive, so
            // any future variant reads as a budget stop — honest either way.
            Err(_) => {
                spent += slice;
                if spent >= budget {
                    return RampStep::Capped;
                }
            }
        }
    }
}

/// The geometric ramp: 1, 2, 4, … ≤ `cfg.max_horizon`. Prices each rung
/// before building; ramps on UNSAT **or** budget exhaustion (never waits
/// for full UNSAT proofs at pre-goal horizons — the classic SATPLAN sink,
/// named and avoided). The UNSAT escape narrates itself into the notes
/// (the loud ramp escape the UNSAT-at-horizon-1 fixture pins).
fn ramp<R, F>(
    enc: &EncTask,
    cfg: &SatCfg,
    notes: &mut Vec<String>,
    proven_all: &mut bool,
    wall_slice: WallSlice,
    mut on_model: F,
) -> Option<(R, usize)>
where
    F: FnMut(&EncTask, usize, &[Lit]) -> Result<R, Option<Vec<Lit>>>,
{
    *proven_all = true;
    let mut trail = String::new();
    let mut h = 1usize;
    let mut last_h = 0usize;
    while h <= cfg.max_horizon {
        if wall_expired(wall_slice) {
            notes.push(format!(
                "SAT: {} expired before horizon {h}; no plan within horizon {last_h} \
                 (NOT a proof)",
                wall_story(wall_slice)
            ));
            *proven_all = false;
            return None;
        }
        let (clauses, lits) = enc.price(h);
        let vars = enc.n_vars(h) as u64;
        let cap = price_cap(cfg, wall_slice);
        if vars + lits > cap {
            notes.push(format!(
                "SAT encoder declined at horizon {h}: ~{vars} vars + ~{lits} clause literals \
                 ({clauses} clauses) exceed the cap {cap}; no plan within horizon {last_h}"
            ));
            *proven_all = false;
            return None;
        }
        let mut solver = enc.encode(h);
        let step = solve_horizon(&mut solver, cfg.conflicts_per_horizon, wall_slice, |m| {
            on_model(enc, h, m)
        });
        match step {
            RampStep::Done(r) => {
                if !trail.is_empty() {
                    notes.push(format!("SAT ramp: {trail}h{h} SAT"));
                }
                return Some((r, h));
            }
            RampStep::Unsat => {
                trail.push_str(&format!("h{h} UNSAT, "));
                if wall_debug() {
                    eprintln!("[sat] horizon {h} proven UNSAT -> ramping");
                }
            }
            RampStep::Capped => {
                trail.push_str(&format!("h{h} capped, "));
                *proven_all = false;
                if wall_debug() {
                    eprintln!("[sat] horizon {h} budget tripped -> ramping (no proof)");
                }
            }
            RampStep::Wall => {
                notes.push(format!(
                    "SAT: {} expired inside horizon {h}; no plan within horizon {last_h} \
                     (NOT a proof)",
                    wall_story(wall_slice)
                ));
                *proven_all = false;
                return None;
            }
            RampStep::Hopeless => {
                notes.push(format!(
                    "SAT: conflict-rate bail inside horizon {h} — the horizon cannot \
                     finish its conflict budget inside the promoted slice; remainder \
                     handed back to the ladder; no plan within horizon {last_h} \
                     (NOT a proof)"
                ));
                *proven_all = false;
                if wall_debug() {
                    eprintln!("[sat] horizon {h} conflict-rate bail -> ramp abandoned");
                }
                return None;
            }
        }
        last_h = h;
        h *= 2;
    }
    let trail = trail.trim_end_matches(", ").to_string();
    if *proven_all {
        notes.push(format!(
            "SAT: no plan within horizon {last_h} — every horizon tried was proven UNSAT \
             within budget ({trail}); a bounded-horizon verdict, not unsolvability"
        ));
    } else {
        notes.push(format!(
            "SAT: no plan within horizon {last_h} ({trail}); a budget tripped, so this is \
             NOT a proof"
        ));
    }
    None
}

// ---------------------------------------------------------------------------
// The classical face (Phase 2).
// ---------------------------------------------------------------------------

/// Solve a classical (pure-STRIPS slice) grounded task by bounded-layer
/// SAT with the horizon ramp. `groups` are invariants.rs mutex groups
/// (seed clauses). The returned op sequence is replay-verified.
pub fn solve_classical(
    task: &PackedTask,
    groups: &[Vec<u32>],
    cfg: &SatCfg,
) -> SatOutcome<Vec<usize>> {
    let facts = task.n_facts;
    let n_ops = task.n_ops;
    let ops = drop_twin_ops(task, (0..task.n_ops as u32).collect());
    if let Err(why) = encodability(task, &ops) {
        return SatOutcome::declined(why, facts, n_ops);
    }
    let enc = EncTask::build(task, ops, groups);
    let mut notes = Vec::new();
    let mut proven_all = false;
    let solved = ramp(
        &enc,
        cfg,
        &mut notes,
        &mut proven_all,
        None,
        |enc, h, model| {
            let seq = enc.decode(h, model);
            if enc.replay(&seq).is_some() {
                Ok(seq)
            } else {
                // The serialization argument failed — an encoder bug, refused
                // loudly rather than shipped (never a panic in release).
                debug_assert!(false, "SAT decode failed its serialization replay");
                Err(None)
            }
        },
    );
    match solved {
        Some((seq, h)) => {
            let layers = seq.iter().map(|&(_, t)| t + 1).max().unwrap_or(0);
            notes.insert(
                0,
                format!(
                    "SAT: plan found at horizon {h} ({} ops over {layers} layers), \
                     replay-verified",
                    seq.len()
                ),
            );
            SatOutcome {
                plan: Some(seq.into_iter().map(|(oi, _)| oi as usize).collect()),
                notes,
                proven_at_every_horizon: false,
                grounded_facts: facts,
                grounded_actions: n_ops,
            }
        }
        None => SatOutcome {
            plan: None,
            notes,
            proven_at_every_horizon: proven_all,
            grounded_facts: facts,
            grounded_actions: n_ops,
        },
    }
}

// ---------------------------------------------------------------------------
// The temporal face (Phase 3): pairing, STN, CEGAR.
// ---------------------------------------------------------------------------

/// The scheduler-as-teacher: order edges ε apart along the decoded
/// sequence, `end = start + dur` equalities per pair. Longest-path
/// Bellman–Ford; `Err(core)` carries event indices witnessing a positive
/// cycle (the refutation core, minimal-ish).
///
/// Core reduction: the raw predecessor cycle drags in every ε-order
/// intermediate (150-event cores on TMS — a clause that never recurs and
/// teaches nothing). When the cycle's SIGNED duration sum `D` is
/// non-negative, the duration-arc endpoints alone carry the
/// infeasibility: any model co-placing those events at those layers
/// reproduces their relative chain order, and the reproduced cycle's
/// ε-order terms are strictly positive, so `D ≥ 0` keeps it a positive
/// cycle regardless of which intermediates exist. `D < 0` (positivity
/// financed by accumulated ε steps — a degenerate shape) keeps the full
/// cycle as the core, exact as before.
fn stn_schedule(n: usize, pairs: &[(usize, usize, f64)]) -> Result<Vec<f64>, Vec<usize>> {
    let mut edges: Vec<(usize, usize, f64)> = Vec::with_capacity(n + 2 * pairs.len());
    for i in 1..n {
        edges.push((i - 1, i, EPS));
    }
    // (u, v) -> signed duration weight, for the core reduction below.
    let mut dur_arc: HashMap<(usize, usize), f64> = HashMap::new();
    for &(s, e, dur) in pairs {
        edges.push((s, e, dur));
        edges.push((e, s, -dur));
        dur_arc.insert((s, e), dur);
        dur_arc.insert((e, s), -dur);
    }
    let mut t = vec![0.0f64; n];
    let mut pred = vec![usize::MAX; n];
    let mut cycle_seed = usize::MAX;
    for round in 0..=n {
        let mut changed = false;
        for &(u, v, w) in &edges {
            if t[v] < t[u] + w - 1e-9 {
                t[v] = t[u] + w;
                pred[v] = u;
                changed = true;
                cycle_seed = v;
            }
        }
        if !changed {
            return Ok(t);
        }
        if round == n {
            break;
        }
    }
    // Positive cycle: walk predecessors n times to land inside it, then
    // collect it.
    let mut x = cycle_seed;
    for _ in 0..n {
        x = pred[x];
    }
    let mut cycle = vec![x];
    let mut y = pred[x];
    while y != x && cycle.len() <= n {
        cycle.push(y);
        y = pred[y];
    }
    let mut d_sum = 0.0f64;
    let mut endpoints: Vec<usize> = Vec::new();
    for (i, &v) in cycle.iter().enumerate() {
        let u = cycle[(i + 1) % cycle.len()]; // pred order: u -> v
        if let Some(&w) = dur_arc.get(&(u, v)) {
            d_sum += w;
            endpoints.push(u);
            endpoints.push(v);
        }
    }
    endpoints.sort_unstable();
    endpoints.dedup();
    if d_sum >= -1e-9 && !endpoints.is_empty() {
        Err(endpoints)
    } else {
        Err(cycle)
    }
}

/// The required-concurrency detector (Phase 3, the promotion policy):
/// TRUE iff some durative action's over-all condition needs a fact whose
/// EVERY producer provides it only DURING its own run (added at-start,
/// deleted at-end by the same action, no classical or at-end adder, not
/// init-true, no TIL adder) — the fire-kiln / match-cellar shape, where
/// decision-epoch search is structurally hopeless and the SAT rung
/// promotes EARLY on the temporal ladder. Lifted (predicate-level, no
/// grounding): cheap enough for the router, conservative both ways — a
/// false positive costs one bounded SAT attempt before the ladder, a
/// false negative defers the rung to exhaustion arming.
pub fn requires_concurrency(domain: &Domain, problem: &Problem) -> bool {
    let mut overall: HashSet<String> = HashSet::new();
    for da in &domain.durative_actions {
        for (ts, f) in &da.conditions {
            if *ts == TimeSpec::All {
                collect_pos_atoms(f, &mut overall);
            }
        }
    }
    if overall.is_empty() {
        return false;
    }
    let init_preds: HashSet<String> = problem
        .init_atoms
        .iter()
        .map(|(p, _)| p.to_ascii_uppercase())
        .collect();
    let til_preds: HashSet<String> = problem
        .til
        .iter()
        .filter(|t| t.add)
        .map(|t| t.pred.to_ascii_uppercase())
        .collect();
    for pred in &overall {
        if init_preds.contains(pred) || til_preds.contains(pred) {
            continue; // may pre-exist / arrive exogenously: not during-only
        }
        let mut envelope_adders = 0usize;
        let mut other_adders = 0usize;
        for a in &domain.actions {
            if effect_adds(&a.effect, pred) {
                other_adders += 1;
            }
        }
        for da in &domain.durative_actions {
            let adds_start = da
                .effects
                .iter()
                .any(|(ts, e)| *ts == TimeSpec::Start && effect_adds(e, pred));
            let adds_end = da
                .effects
                .iter()
                .any(|(ts, e)| *ts == TimeSpec::End && effect_adds(e, pred));
            let dels_end = da
                .effects
                .iter()
                .any(|(ts, e)| *ts == TimeSpec::End && effect_dels(e, pred));
            if adds_end {
                other_adders += 1;
            }
            if adds_start {
                // during-envelope only if the SAME action closes it, and
                // only a DIFFERENT action's over-all makes that required
                // concurrency (an action enveloping its own over-all fact
                // is decision-epoch bread and butter).
                let own_overall = da.conditions.iter().any(|(ts, f)| {
                    *ts == TimeSpec::All && {
                        let mut s = HashSet::new();
                        collect_pos_atoms(f, &mut s);
                        s.contains(pred)
                    }
                });
                if dels_end && !own_overall {
                    envelope_adders += 1;
                } else {
                    other_adders += 1;
                }
            }
        }
        if envelope_adders > 0 && other_adders == 0 {
            return true;
        }
    }
    false
}

fn collect_pos_atoms(f: &Formula, out: &mut HashSet<String>) {
    match f {
        Formula::Atom(p, _) => {
            out.insert(p.to_ascii_uppercase());
        }
        Formula::And(v) => v.iter().for_each(|x| collect_pos_atoms(x, out)),
        Formula::Forall(_, inner) => collect_pos_atoms(inner, out),
        _ => {}
    }
}

fn effect_adds(e: &Effect, pred: &str) -> bool {
    match e {
        Effect::Add(p, _) => p.eq_ignore_ascii_case(pred),
        Effect::And(v) => v.iter().any(|x| effect_adds(x, pred)),
        Effect::When(_, inner) | Effect::Forall(_, inner) => effect_adds(inner, pred),
        _ => false,
    }
}

fn effect_dels(e: &Effect, pred: &str) -> bool {
    match e {
        Effect::Del(p, _) => p.eq_ignore_ascii_case(pred),
        Effect::And(v) => v.iter().any(|x| effect_dels(x, pred)),
        Effect::When(_, inner) | Effect::Forall(_, inner) => effect_dels(inner, pred),
        _ => false,
    }
}

/// The router-facing temporal entry: a plan or nothing (the ladder keeps
/// the story). The full-story entry is [`solve_temporal`].
pub(crate) fn plan_temporal(
    domain: &Domain,
    problem: &Problem,
    threads: usize,
) -> Option<TimedPlan> {
    solve_temporal(domain, problem, threads, &SatCfg::from_env()).plan
}

/// The router-facing PROMOTED entry (0.24 regression fix): like
/// [`plan_temporal`], but the attempt is bounded by its own wall slice of
/// `budget_secs` — the promotion runs BEFORE the ladder, so an attempt
/// that cannot bail (conflict grind, not STN thrash) must not spend the
/// ladder's wall. `None` = unbounded (the no-wall contract).
pub(crate) fn plan_temporal_within(
    domain: &Domain,
    problem: &Problem,
    threads: usize,
    budget_secs: Option<f64>,
) -> Option<TimedPlan> {
    solve_temporal_within(domain, problem, threads, &SatCfg::from_env(), budget_secs).plan
}

/// The temporal face, end to end: snap compile, ground, encode with
/// pairing + interval invariants, ramp, decode, STN-schedule (CEGAR on
/// negative cycles), emit through the house ε machinery, validate against
/// the ORIGINAL problem. A validation miss refuses the plan — a red plan
/// is never shipped.
pub fn solve_temporal(
    domain: &Domain,
    problem: &Problem,
    threads: usize,
    cfg: &SatCfg,
) -> SatOutcome<TimedPlan> {
    solve_temporal_within(domain, problem, threads, cfg, None)
}

/// [`solve_temporal`] under a wall slice: the attempt additionally stops
/// (with the honest "promoted wall slice expired" note) once
/// `budget_secs` of ITS OWN clock is spent — the promoted router entry's
/// bounded-bet contract. `None` is byte-identical to [`solve_temporal`].
pub fn solve_temporal_within(
    domain: &Domain,
    problem: &Problem,
    threads: usize,
    cfg: &SatCfg,
    budget_secs: Option<f64>,
) -> SatOutcome<TimedPlan> {
    use crate::ground::Outcome;
    // The slice arms only while the 0.22 clock checkpoints do —
    // `FF_NO_RUNG_WALLCAP=1` hatches it with them (pre-slice shapes stay
    // pinnable), and pricing must not see a slice the checkpoints won't.
    let slice: WallSlice = budget_secs
        .filter(|_| crate::search::rung_wallcap_on())
        .map(|s| (crate::clock::Clock::now(), s));
    if !domain.constraints.is_empty() || !problem.constraints.is_empty() {
        return SatOutcome::declined(
            "trajectory constraints (the temporal SAT face is unconstrained-only)".into(),
            0,
            0,
        );
    }
    if !problem.til.is_empty() {
        return SatOutcome::declined(
            "timed initial literals (absolute times have no place in a duration-free CNF)".into(),
            0,
            0,
        );
    }
    let c = crate::temporal::compile(domain, problem);
    let task = match crate::ground::ground_stratified_walled(&c.domain, &c.problem, threads) {
        Outcome::Task(t) => t,
        Outcome::GoalTrue => {
            return SatOutcome {
                plan: Some(TimedPlan {
                    steps: Vec::new(),
                    makespan: 0.0,
                }),
                notes: vec!["goal already satisfied; the empty plan solves it".into()],
                proven_at_every_horizon: false,
                grounded_facts: 0,
                grounded_actions: 0,
            };
        }
        Outcome::GoalFalse(why) => {
            return SatOutcome {
                plan: None,
                notes: vec![format!("unsolvable at grounding: {why}")],
                proven_at_every_horizon: false,
                grounded_facts: 0,
                grounded_actions: 0,
            };
        }
        Outcome::WallExhausted(why) => {
            return SatOutcome {
                plan: None,
                notes: vec![format!("grounding stopped at the declared budget: {why}")],
                proven_at_every_horizon: false,
                grounded_facts: 0,
                grounded_actions: 0,
            };
        }
        _ => return SatOutcome::declined("grounding refused the snap-compiled task".into(), 0, 0),
    };
    let facts = task.n_facts;
    let n_ops = task.n_ops;
    let (kind, _dur_exprs, inv) = crate::temporal::build_kind(&task, &c);
    // Encodable events: starts (fixed durations only), ends, classical
    // happenings. Skip stays skipped — the search never applies it either.
    let mut ops: Vec<u32> = Vec::new();
    for (oi, k) in kind.iter().enumerate() {
        match *k {
            Kind::Start { dexp, .. } => {
                if dexp != u32::MAX {
                    return SatOutcome::declined(
                        "state-dependent durations (the STN needs fixed interval lengths)".into(),
                        facts,
                        n_ops,
                    );
                }
                ops.push(oi as u32);
            }
            Kind::End | Kind::Classical => ops.push(oi as u32),
            Kind::Til => {
                return SatOutcome::declined("timed initial literals".into(), facts, n_ops)
            }
            Kind::Skip => {}
        }
    }
    let ops = drop_twin_ops(&task, ops);
    if let Err(why) = encodability(&task, &ops) {
        return SatOutcome::declined(why, facts, n_ops);
    }
    let groups = crate::invariants::synthesize(&c.domain, &task);
    let mut enc = EncTask::build(&task, ops, &groups);
    if let Err(why) = enc.add_temporal(&kind, &inv) {
        return SatOutcome::declined(why, facts, n_ops);
    }

    let mut notes = Vec::new();
    let mut proven_all = false;
    let mut refutations_total = 0usize;
    let mut refutations_this_h = 0usize;
    let mut cur_h = 0usize;
    let solved: Option<(TimedPlan, usize)> = ramp(
        &enc,
        cfg,
        &mut notes,
        &mut proven_all,
        slice,
        |enc, h, model| {
            if h != cur_h {
                cur_h = h;
                refutations_this_h = 0;
            }
            let seq = enc.decode(h, model);
            if enc.replay(&seq).is_none() {
                debug_assert!(false, "SAT decode failed its serialization replay");
                return Err(None);
            }
            // Pair each START with the next END of its pair op — unique,
            // because no-self-overlap keeps at most one interval open per
            // ground action.
            let mut open: HashMap<usize, usize> = HashMap::new();
            let mut pairs: Vec<(usize, usize, f64)> = Vec::new();
            for (i, &(op, _layer)) in seq.iter().enumerate() {
                match kind[op as usize] {
                    Kind::Start { end_op, .. } => {
                        open.insert(end_op, i);
                    }
                    Kind::End => {
                        let Some(si) = open.remove(&(op as usize)) else {
                            debug_assert!(false, "END decoded without an open START");
                            return Err(None);
                        };
                        let Kind::Start { dur, .. } = kind[seq[si].0 as usize] else {
                            unreachable!("pairing points at a non-start")
                        };
                        pairs.push((si, i, dur));
                    }
                    _ => {}
                }
            }
            if !open.is_empty() {
                debug_assert!(false, "unclosed START decoded despite the token closure");
                return Err(None);
            }
            match stn_schedule(seq.len(), &pairs) {
                Ok(times) => {
                    // The raw timed plan: one durative step per START (the
                    // END is implied), classical happenings instantaneous.
                    let mut steps: Vec<TimedStep> = Vec::new();
                    let mut makespan = 0.0f64;
                    for (i, &(op, _)) in seq.iter().enumerate() {
                        let disp = &task.op_display[op as usize];
                        match kind[op as usize] {
                            Kind::Start { dur, .. } => {
                                let mut it = disp.splitn(2, ' ');
                                let head = it.next().unwrap_or("");
                                let rest = it.next();
                                let name = head.trim_end_matches("-START");
                                let action = match rest {
                                    Some(r) => format!("{name} {r}"),
                                    None => name.to_string(),
                                };
                                makespan = makespan.max(times[i] + dur);
                                steps.push(TimedStep {
                                    time: times[i],
                                    action,
                                    duration: Some(dur),
                                });
                            }
                            Kind::Classical => {
                                makespan = makespan.max(times[i]);
                                steps.push(TimedStep {
                                    time: times[i],
                                    action: disp.clone(),
                                    duration: None,
                                });
                            }
                            _ => {}
                        }
                    }
                    steps.sort_by(|a, b| a.time.total_cmp(&b.time));
                    Ok(TimedPlan { steps, makespan })
                }
                Err(cycle) => {
                    refutations_this_h += 1;
                    refutations_total += 1;
                    if wall_debug() {
                        eprintln!(
                            "[sat] horizon {h}: STN refutation #{refutations_this_h} \
                             ({} events on the cycle)",
                            cycle.len()
                        );
                    }
                    if refutations_this_h > MAX_REFUTATIONS_PER_HORIZON {
                        if wall_debug() {
                            eprintln!(
                                "[sat] horizon {h}: >{MAX_REFUTATIONS_PER_HORIZON} STN \
                                 refutations — thrash bail to ramp (the pre-registered read)"
                            );
                        }
                        return Err(None);
                    }
                    let clause: Vec<Lit> = cycle
                        .iter()
                        .map(|&i| {
                            let (op, layer) = seq[i];
                            Lit::from_index(
                                enc.op_var(h, layer as usize, enc.slot_of[op as usize]),
                                false,
                            )
                        })
                        .collect();
                    Err(Some(clause))
                }
            }
        },
    );
    if refutations_total > MAX_REFUTATIONS_PER_HORIZON {
        notes.push(format!(
            "SAT: STN-refutation thrash recorded ({refutations_total} refutations total; \
             >{MAX_REFUTATIONS_PER_HORIZON} in one horizon bailed to the ramp)"
        ));
    }
    let Some((raw, h)) = solved else {
        return SatOutcome {
            plan: None,
            notes,
            proven_at_every_horizon: proven_all,
            grounded_facts: facts,
            grounded_actions: n_ops,
        };
    };
    // The house emission rite (the 0.22 topological ε machinery runs AFTER
    // scheduling, exactly as for search plans), then the oracle.
    let plan = crate::temporal::emit_scheduled(&task, &c, &inv, raw);
    match crate::temporal::validate(domain, problem, &plan) {
        Ok(()) => {
            notes.insert(
                0,
                format!(
                    "SAT: temporal plan at horizon {h} ({} steps, makespan {:.3}), \
                     STN-scheduled ({refutations_total} scheduler refutations), \
                     validated against the original problem",
                    plan.steps.len(),
                    plan.makespan
                ),
            );
            SatOutcome {
                plan: Some(plan),
                notes,
                proven_at_every_horizon: false,
                grounded_facts: facts,
                grounded_actions: n_ops,
            }
        }
        Err(why) => {
            // Refused, never shipped red. This is an encoder/emission bug
            // by construction — say so.
            notes.insert(
                0,
                format!(
                    "SAT: decoded plan at horizon {h} FAILED the internal oracle ({why}); \
                     plan refused"
                ),
            );
            debug_assert!(false, "SAT temporal plan failed validate: {why}");
            SatOutcome {
                plan: None,
                notes,
                proven_at_every_horizon: false,
                grounded_facts: facts,
                grounded_actions: n_ops,
            }
        }
    }
}
