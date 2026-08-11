//! FF relaxed-plan heuristic over the packed task, allocation-free on the hot
//! path: all working buffers live in a reusable `Scratch` that a worker thread
//! creates once and resets per evaluation (cleared, never re-allocated). This
//! removes the per-state allocation churn — and the global-allocator contention
//! it caused across worker threads — which was the main limit on both raw speed
//! and parallel scaling.
//!
//! Same algorithm as the (oracle-verified) metricff heuristic: a delete-relaxed
//! planning graph with monotone numeric interval bounds, two-phase layering,
//! lowest-layer achiever selection, and numeric repetition counting. The
//! best-first engine only needs `h`, so the helpful-action set is not computed.

use crate::bitset;
use crate::packed::PackedTask;
use crate::types::{eval_numpre, AssignOp, CompOp, NExpr, NumEff, NumPre};

pub(crate) const LAYER_CAP: u32 = 2000;
const INF: u32 = u32::MAX;

/// Why an RPG build stopped (0.22 Phase 4 L0). The exit reason is the
/// numeric-admissible bound's whole payload: with a passing
/// [`numeric_interval_audit`], layer `k`'s facts+intervals CONTAIN every
/// state reachable by a real plan of length ≤ k (induction over `widen`:
/// each real step's effect is covered by that op's widening in the round
/// after it becomes applicable, and applied ops re-widen every round), so
/// the first goal-satisfiable layer LOWER-BOUNDS plan length.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RpgExit {
    /// The goal became (relaxed+widened) satisfiable at loop entry with
    /// this many completed widening rounds — a lower bound on plan LENGTH.
    /// The off-by-one convention, pinned by the numopt-p04 fixture: ops
    /// applied in round ℓ land their adds at fact layer ℓ+1, and the goal
    /// check sees them entering round ℓ+1 — a gap-60 chain of +1 pumps
    /// reads GoalAt(60), never 59 or 61.
    GoalAt(u32),
    /// Fixpoint without the goal: under a passing audit the widened RPG
    /// over-approximates reachability, so the goal is truly unreachable
    /// (h = ∞, a safe prune).
    Fixpoint,
    /// LAYER_CAP rounds without fixpoint or goal: the goal needs MORE than
    /// LAYER_CAP steps (still a valid lower bound; never a prune).
    Cap,
}

/// Measurement-only phase accumulators (FF_RES_DEBUG; printed by the search
/// cap dump). Atomic because h runs on worker threads.
pub static T_RESET: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static T_BUILD: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static T_EXTRACT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Slack for the TRPG window comparisons — refusal needs a REAL overrun,
/// never a float hair (the temporal emission's own ε is 0.001; a millionth
/// stays well under it).
const TRPG_EPS: f64 = 1e-6;

/// One over-all-invariant window an END op's relaxed payout is gated on
/// (TRPG-lite, 0.23 Phase 4 probe 2, opt-in `FF_TRPG=1`). Two shapes, each
/// with its own sound comparison:
///
/// - **Envelope** (`providers` non-empty): every adder of `fact` is a rigid
///   durative START whose own paired END withdraws it — TMS's `(ready ?k)`,
///   match-cellar's `(light ?m)`. The window can be RE-OPENED at any later
///   time by refiring a provider, so absolute clocks prove nothing here;
///   what no delay can change is the WIDTH. The gate compares the gated
///   END's own duration against the widest APPLIED provider's duration —
///   a 15-long bake can never fit an 8-long kiln window, whenever it fires.
/// - **Static** (`providers` empty, `close` finite): `fact` is closed by a
///   TIL — an exogenous, mandatory delete at an absolute time. With no
///   adders at all the FIRST delete is final (`close` = min); with only
///   TIL adders the last window's close is the optimistic bound (`close` =
///   max). The gate compares the END's earliest fire time against `close`.
///
/// Mixed shapes (classical adders, conditional adds, state-dependent
/// provider durations, `fact` initially true alongside providers) build NO
/// window — the relaxation stays time-blind for them, the optimistic side.
#[derive(Clone, Debug)]
pub struct TrpgWindow {
    /// The grounded over-all invariant fact this window guards.
    pub fact: u32,
    /// Envelope providers: (START op id, rigid duration). Empty = static.
    pub providers: Vec<(u32, f64)>,
    /// Absolute close time for provider-free (TIL-closed) windows;
    /// `f64::INFINITY` when `providers` carry the width instead.
    pub close: f64,
}

/// TRPG-lite tables (0.23 Phase 4 probe 2, opt-in `FF_TRPG=1`): the
/// time-stamped relaxation's per-task constants, built ONCE per temporal
/// task by `temporal::trpg_info` from the snap pairing (`Kind`), the
/// grounded interval invariants (`InvMap`), and the TIL agenda. Armed by
/// PRESENCE on the task (`PackedTask::trpg`, the `pair_end` precedent) and
/// consumed only by the timed variant `relaxed_helpful` selects — the
/// classical groundings never build the table, so the flag is provably
/// inert there, and the temporal COMPLETE passes keep the time-blind
/// heuristic (they enter through `relaxed_to`), so completeness never
/// rests on the gate's approximations.
pub struct TrpgInfo {
    /// op id -> paired START op for END snaps with a RIGID duration;
    /// `u32::MAX` otherwise. An END's fire time anchors at the start's
    /// fire time plus `lag` (the end cannot land before start + duration).
    pub start_of: Vec<u32>,
    /// op id -> the rigid duration for anchored END ops; 0.0 otherwise.
    pub lag: Vec<f64>,
    /// op id -> absolute fire floor for TIL applier ops; 0.0 otherwise
    /// (a positive TIL's add is not available before its wall time).
    pub floor: Vec<f64>,
    /// op id -> the windows gating this END's relaxed payout (empty for
    /// non-END ops and ends whose invariants match no supported shape).
    pub windows: Vec<Vec<TrpgWindow>>,
}

/// Reusable per-worker working memory for `relaxed`.
pub struct Scratch {
    reached: Vec<bool>,
    fact_layer: Vec<u32>,
    op_layer: Vec<u32>,
    /// Generation stamp for `op_layer` / `selected` / `need_fact` membership: a
    /// cell is "set this evaluation" iff its stamp == `gen`. Lets `reset()` bump
    /// `gen` in O(1) instead of clearing these arrays every evaluation.
    gen: u32,
    op_stamp: Vec<u32>,
    applicable: Vec<u32>,
    lb: Vec<f64>,
    ub: Vec<f64>,
    selected: Vec<u32>,
    need_fact: Vec<u32>,
    queue: Vec<u32>,
    /// applied ops with ≥1 relevant numeric effect (re-widened each layer).
    num_applied: Vec<u32>,
    /// applied ops with ≥1 conditional effect (re-checked each layer; their
    /// conditional adds fire once the condition becomes relaxed-reached).
    cond_ops: Vec<u32>,
    /// FF helpful actions: relaxed-plan ops applicable in the current state
    /// (op_layer 0). Populated during extraction; read by EHC.
    helpful: Vec<u32>,
    /// TRPG-lite time column (0.23 Phase 4 probe 2): earliest-achievable
    /// time per fact, valid only where `reached` and only on TIMED builds
    /// (`task.trpg` armed through `relaxed_helpful`). Empty and untouched
    /// on every other path — the flag-off byte-identity is structural.
    fact_time: Vec<f64>,
    /// Per-op fire time on timed builds (stamp-gated like `op_layer`:
    /// valid iff `op_stamp == gen`). END fires anchor at start + lag.
    op_time: Vec<f64>,
}

impl Scratch {
    pub fn new(task: &PackedTask) -> Self {
        let nfl = task.fv0.len();
        Scratch {
            reached: vec![false; task.n_facts],
            fact_layer: vec![INF; task.n_facts],
            op_layer: vec![INF; task.n_ops],
            gen: 0,
            op_stamp: vec![0; task.n_ops],
            applicable: Vec::with_capacity(task.n_ops),
            lb: vec![0.0; nfl],
            ub: vec![0.0; nfl],
            selected: vec![0; task.n_ops],
            need_fact: vec![0; task.n_facts],
            queue: Vec::with_capacity(task.n_facts),
            num_applied: Vec::with_capacity(task.n_ops),
            cond_ops: Vec::new(),
            helpful: Vec::new(),
            fact_time: Vec::new(),
            op_time: Vec::new(),
        }
    }

    fn reset(&mut self, task: &PackedTask, bits: &[u64], fv: &[f64], timed: bool) {
        for f in 0..task.n_facts {
            self.reached[f] = bitset::test(bits, f);
        }
        self.fact_layer.iter_mut().enumerate().for_each(|(f, l)| {
            *l = if self.reached[f] { 0 } else { INF };
        });
        // Bump the generation instead of clearing op_layer/selected/need_fact —
        // their stale values are unobservable because every read is stamp-gated
        // (== gen). This removes 2*n_ops + n_facts dense writes per evaluation.
        self.gen = self.gen.wrapping_add(1);
        if self.gen == 0 {
            // wrapped after ~4e9 evals: hard-clear once so stamps can't collide.
            self.op_stamp.fill(0);
            self.selected.fill(0);
            self.need_fact.fill(0);
            self.gen = 1;
        }
        self.lb.copy_from_slice(fv);
        self.ub.copy_from_slice(fv);
        self.queue.clear();
        self.num_applied.clear();
        self.cond_ops.clear();
        self.helpful.clear();
        // The TRPG time column: dense re-zero only on TIMED builds (facts
        // true in this state fire at 0.0; later reaches overwrite before
        // any read). `op_time` needs no clearing — every read is
        // stamp-gated, exactly like `op_layer`.
        if timed {
            self.fact_time.clear();
            self.fact_time.resize(task.n_facts, 0.0);
            self.op_time.resize(task.n_ops, 0.0);
        }
    }
}

/// Widen monotone bounds from op `oi`'s numeric effects on RELEVANT fluents
/// (effects on fluents that no precondition/goal reads cannot change the
/// heuristic, so skipping them is exact and also stops irrelevant unbounded
/// growth). Returns whether any relevant bound changed.
fn widen(
    neffs: &[NumEff],
    relevant: &[bool],
    lb: &mut [f64],
    ub: &mut [f64],
    def: &[bool],
) -> bool {
    let mut changed = false;
    for ne in neffs {
        let t = ne.target as usize;
        if !relevant[t] {
            continue;
        }
        if let Some((vl, vu)) = eval_iv(&ne.value, lb, ub, def) {
            let before = (lb[t], ub[t]);
            match ne.op {
                AssignOp::Increase => {
                    ub[t] += vu.max(0.0);
                    lb[t] += vl.min(0.0);
                }
                AssignOp::Decrease => {
                    lb[t] -= vu.max(0.0);
                    ub[t] -= vl.min(0.0);
                }
                AssignOp::Assign => {
                    lb[t] = lb[t].min(vl);
                    ub[t] = ub[t].max(vu);
                }
                AssignOp::ScaleUp => ub[t] *= vu.max(1.0),
                AssignOp::ScaleDown => lb[t] /= vu.max(1.0),
            }
            if (lb[t], ub[t]) != before {
                changed = true;
            }
        }
    }
    changed
}

fn op_has_relevant_neff(task: &PackedTask, oi: usize) -> bool {
    task.num_eff
        .slice(oi)
        .iter()
        .any(|ne| task.relevant_fluent[ne.target as usize])
}

fn eval_iv(e: &NExpr, lb: &[f64], ub: &[f64], def: &[bool]) -> Option<(f64, f64)> {
    Some(match e {
        NExpr::Num(n) => (*n, *n),
        NExpr::Fluent(i) => {
            let i = *i as usize;
            if !def[i] {
                return None;
            }
            (lb[i], ub[i])
        }
        NExpr::Neg(a) => {
            let (l, u) = eval_iv(a, lb, ub, def)?;
            (-u, -l)
        }
        NExpr::Add(a, b) => {
            let (al, au) = eval_iv(a, lb, ub, def)?;
            let (bl, bu) = eval_iv(b, lb, ub, def)?;
            (al + bl, au + bu)
        }
        NExpr::Sub(a, b) => {
            let (al, au) = eval_iv(a, lb, ub, def)?;
            let (bl, bu) = eval_iv(b, lb, ub, def)?;
            (al - bu, au - bl)
        }
        NExpr::Mul(a, b) => {
            let (al, au) = eval_iv(a, lb, ub, def)?;
            let (bl, bu) = eval_iv(b, lb, ub, def)?;
            let c = [al * bl, al * bu, au * bl, au * bu];
            (
                c.iter().cloned().fold(f64::INFINITY, f64::min),
                c.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            )
        }
        NExpr::Div(a, b) => {
            let (al, au) = eval_iv(a, lb, ub, def)?;
            let (bl, bu) = eval_iv(b, lb, ub, def)?;
            if bl <= 0.0 && bu >= 0.0 {
                (f64::NEG_INFINITY, f64::INFINITY)
            } else {
                let c = [al / bl, al / bu, au / bl, au / bu];
                (
                    c.iter().cloned().fold(f64::INFINITY, f64::min),
                    c.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                )
            }
        }
    })
}

fn num_sat(np: &NumPre, lb: &[f64], ub: &[f64], def: &[bool]) -> bool {
    let l = match eval_iv(&np.lhs, lb, ub, def) {
        Some(x) => x,
        None => return false,
    };
    let r = match eval_iv(&np.rhs, lb, ub, def) {
        Some(x) => x,
        None => return false,
    };
    match np.op {
        CompOp::Lt => l.0 < r.1,
        CompOp::Le => l.0 <= r.1,
        CompOp::Gt => l.1 > r.0,
        CompOp::Ge => l.1 >= r.0,
        CompOp::Eq => l.0 <= r.1 && r.0 <= l.1,
    }
}

fn goal_done(
    goal_pos: &[u32],
    goal_num: &[NumPre],
    reached: &[bool],
    lb: &[f64],
    ub: &[f64],
    def: &[bool],
) -> bool {
    goal_pos.iter().all(|&f| reached[f as usize])
        && goal_num.iter().all(|np| num_sat(np, lb, ub, def))
}

/// Build the delete-relaxed planning graph into `sc` (op_layer / reached /
/// bounds). With `to_fixpoint` it ignores the goal and runs to a fixpoint
/// (so every reachable op gets a layer); otherwise it stops once the goal is
/// relaxed-reached. `sc` must be reset first. Returns WHY it stopped
/// (0.22 Phase 4 L0) — the satisficing callers ignore it, so their paths
/// are byte-identical.
fn build_rpg(
    task: &PackedTask,
    sc: &mut Scratch,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    def: &[bool],
    to_fixpoint: bool,
    timed: bool,
) -> RpgExit {
    // TRPG-lite (0.23 Phase 4 probe 2): the time column is a SECOND FIELD
    // on this same fused loop, never a second pass — `trpg` is Some only
    // on timed builds of a temporal task whose tables were armed, and
    // every timed line below keys on it, so the flag-off path (and every
    // classical task) runs the historical bytes.
    let trpg = if timed { task.trpg.as_deref() } else { None };
    // ---- build the relaxed planning graph (two-phase, incremental) ----
    // Only UNAPPLIED ops are re-scanned each layer; applied ops never lose
    // applicability (delete-relaxed), so they are skipped — except those with
    // relevant numeric effects, which are re-widened each layer from
    // `num_applied` so monotone fluents (e.g. consumed-resources) can grow to
    // reach numeric goals. (A counter-based build — watch lists decrementing
    // per reached fact — was implemented and measured EQUIVALENT on
    // transport11: identical 20,126 evals, h wall 12.85 s vs 12.83 s. The
    // per-eval cost is the relaxation FLOOR — nearly every op fires in every
    // build — so the scan is not the term; recorded 2026-07-19, not shipped.)
    let mut layer: u32 = 0;
    loop {
        if !to_fixpoint && goal_done(goal_pos, goal_num, &sc.reached, &sc.lb, &sc.ub, def) {
            return RpgExit::GoalAt(layer);
        }
        let mut changed = false;

        // (a) re-widen bounds from previously-applied relevant-numeric ops
        for idx in 0..sc.num_applied.len() {
            let oi = sc.num_applied[idx] as usize;
            if widen(
                task.num_eff.slice(oi),
                &task.relevant_fluent,
                &mut sc.lb,
                &mut sc.ub,
                def,
            ) {
                changed = true;
            }
        }

        // (a2) conditional effects of applied ops: fire those whose condition is
        // now relaxed-reached (positive facts reached + numeric satisfied;
        // negative conditions are dropped by the delete-relaxation).
        for idx in 0..sc.cond_ops.len() {
            let oi = sc.cond_ops[idx] as usize;
            for ce in task.cond_effs(oi) {
                let pos_ok = ce.cond_pos.iter().all(|&c| sc.reached[c as usize]);
                let num_ok = ce
                    .cond_num
                    .iter()
                    .all(|np| num_sat(np, &sc.lb, &sc.ub, def));
                if pos_ok && num_ok {
                    // Timed: a conditional add lands no earlier than the op's
                    // own fire time and its condition facts' times.
                    let ct = trpg.map(|_| {
                        let mut t = sc.op_time[oi];
                        for &c in &ce.cond_pos {
                            t = t.max(sc.fact_time[c as usize]);
                        }
                        t
                    });
                    for &f in &ce.add {
                        let f = f as usize;
                        if !sc.reached[f] {
                            sc.reached[f] = true;
                            sc.fact_layer[f] = layer + 1;
                            if let Some(t) = ct {
                                sc.fact_time[f] = t;
                            }
                            changed = true;
                        }
                    }
                    if !ce.num.is_empty()
                        && widen(&ce.num, &task.relevant_fluent, &mut sc.lb, &mut sc.ub, def)
                    {
                        changed = true;
                    }
                }
            }
        }

        // (b) scan only unapplied ops for new applicability
        sc.applicable.clear();
        for oi in 0..task.n_ops {
            if sc.op_stamp[oi] == sc.gen {
                continue; // already applied this evaluation
            }
            let ok = task
                .pre_pos
                .slice(oi)
                .iter()
                .all(|&f| sc.reached[f as usize])
                && task
                    .pre_num
                    .slice(oi)
                    .iter()
                    .all(|np| num_sat(np, &sc.lb, &sc.ub, def));
            if ok {
                if let Some(tp) = trpg {
                    // Time-stamp the fire and gate the END payout. A refusal
                    // leaves the op UNAPPLIED — it is re-scanned next layer
                    // (a wider provider may yet arrive), and a stable refusal
                    // simply never delivers its adds.
                    match trpg_admit(tp, task, sc, oi) {
                        Some(t) => sc.op_time[oi] = t,
                        None => continue,
                    }
                }
                sc.op_stamp[oi] = sc.gen;
                sc.op_layer[oi] = layer;
                sc.applicable.push(oi as u32);
                changed = true;
            }
        }

        // (c) apply newly-applicable ops: reach their adds, widen + register
        for k in 0..sc.applicable.len() {
            let oi = sc.applicable[k] as usize;
            for &f in task.add.slice(oi) {
                let f = f as usize;
                if !sc.reached[f] {
                    sc.reached[f] = true;
                    sc.fact_layer[f] = layer + 1;
                    if trpg.is_some() {
                        sc.fact_time[f] = sc.op_time[oi];
                    }
                    changed = true;
                }
            }
            if op_has_relevant_neff(task, oi) {
                if widen(
                    task.num_eff.slice(oi),
                    &task.relevant_fluent,
                    &mut sc.lb,
                    &mut sc.ub,
                    def,
                ) {
                    changed = true;
                }
                sc.num_applied.push(oi as u32);
            }
            if task.n_cond_effs(oi) > 0 {
                sc.cond_ops.push(oi as u32);
            }
        }

        layer += 1;
        if !changed {
            return RpgExit::Fixpoint;
        }
        if layer > LAYER_CAP {
            return RpgExit::Cap;
        }
    }
}

/// Time-stamp one newly relaxed-applicable op and gate its payout on the
/// TRPG windows (0.23 Phase 4 probe 2). Returns the op's fire time, or
/// None when a window refuses it: the op is not applied this layer.
///
/// Fire time = max of the op's precondition fact times, its TIL floor,
/// and — for anchored ENDs — the paired start's fire plus the duration.
/// These are FIRST-REACH times: an alternative achiever discovered at a
/// later layer with an earlier time does not lower a stamp, so a static-
/// window refusal can in principle over-fire where a slower-layer path is
/// time-cheaper (recorded lite caveat). The envelope comparison dodges the
/// whole clock: providers can always refire later, so only WIDTH decides —
/// the END's own duration against the widest applied provider's — and
/// refused ops are re-scanned every layer, so a wider provider arriving
/// later un-refuses. Completeness never leans on any of this: the temporal
/// complete passes evaluate time-blind (`relaxed_to`).
fn trpg_admit(tp: &TrpgInfo, task: &PackedTask, sc: &Scratch, oi: usize) -> Option<f64> {
    let mut fire = tp.floor[oi];
    for &f in task.pre_pos.slice(oi) {
        let t = sc.fact_time[f as usize];
        if t > fire {
            fire = t;
        }
    }
    let s = tp.start_of[oi];
    if s != u32::MAX && sc.op_stamp[s as usize] == sc.gen {
        let anchored = sc.op_time[s as usize] + tp.lag[oi];
        if anchored > fire {
            fire = anchored;
        }
    }
    for w in &tp.windows[oi] {
        if w.providers.is_empty() {
            // Static (TIL-closed): the fact dies at an absolute wall time.
            if fire > w.close + TRPG_EPS {
                return None;
            }
        } else {
            // Envelope: width against width. No applied provider yet means
            // the invariant fact arrived some other way this build — stay
            // optimistic and let the payout through.
            let mut width = f64::NEG_INFINITY;
            for &(sp, d) in &w.providers {
                if sc.op_stamp[sp as usize] == sc.gen && d > width {
                    width = d;
                }
            }
            if width > f64::NEG_INFINITY && tp.lag[oi] > width + TRPG_EPS {
                return None;
            }
        }
    }
    Some(fire)
}

/// The numeric-admissible LENGTH bound (0.22 Phase 4 L0): reset `sc` on the
/// given state, build the RPG toward the task's own goal, and return the
/// exit reason — `GoalAt(k)` lower-bounds plan length from this state (see
/// [`RpgExit`]). ONLY sound behind a passing [`numeric_interval_audit`]:
/// the containment argument holds case-by-case in `widen` for
/// Increase/Decrease/Assign over defined, closure-complete fluents, and
/// FAILS for the audited-out shapes. No repetition-count shortcut lives
/// here on purpose — ceil(gap/rate-now) is inadmissible when a plan can
/// raise the rate first (sailing-wind's velocity assign is the live
/// witness); the layer bound already encodes the admissible form.
pub(crate) fn admissible_goal_layers(
    task: &PackedTask,
    sc: &mut Scratch,
    bits: &[u64],
    fv: &[f64],
    def: &[bool],
) -> RpgExit {
    sc.reset(task, bits, fv, false);
    build_rpg(task, sc, &task.goal_pos, &task.goal_num, def, false, false)
}

/// The containment audit (0.22 Phase 4 L0): rejects BY NAME the shapes
/// that break the layer bound's over-approximation, and an audit reject
/// means UNARMED — byte-identical search (the 0.20 conditional-effect
/// repair is the precedent for taking these rejects seriously).
///
/// The three shapes:
/// - **scale-up/down on a relevant-closure fluent**: `widen` is one-sided
///   there (ScaleUp only raises ub, ScaleDown only lowers lb), so a real
///   application scaling the other way ESCAPES the interval — e.g. a
///   scale-up by a factor < 1 shrinks the true value below lb forever.
/// - **relevant-closure fluent undefined at init**: `eval_iv` reads None,
///   `num_sat` stays false, and a goal a real plan reaches by
///   assign-then-read is labeled unreachable — an overestimate.
/// - **monitored / shared_cond**: trajectory-monitor transitions carry
///   negative conditions and delete semantics the relaxation drops in a
///   direction the containment induction does not cover; fail closed.
///
/// `task.relevant_fluent` IS the relevant closure (ground.rs builds it as
/// pre_num/cond_num/goal reads plus the transitive closure over effect-RHS
/// reads feeding relevant targets), so every fluent whose interval feeds
/// the bound is widened — nothing escapes the audit by being read only
/// inside an effect expression.
pub(crate) fn numeric_interval_audit(task: &PackedTask) -> Result<(), String> {
    if !task.shared_cond.is_empty() && task.monitored.iter().any(|&m| m) {
        return Err("numeric bound unarmed: monitored (shared trajectory-monitor) block".into());
    }
    for &f in &task.rel_fluents {
        if !task.fdef0[f as usize] {
            return Err(format!(
                "numeric bound unarmed: relevant fluent `{}` undefined at init",
                task.fluent_names
                    .get(f as usize)
                    .map(|s| s.as_str())
                    .unwrap_or("?")
            ));
        }
    }
    let scale_reject = |ne: &NumEff| -> Option<String> {
        let t = ne.target as usize;
        if task.relevant_fluent[t] && matches!(ne.op, AssignOp::ScaleUp | AssignOp::ScaleDown) {
            Some(format!(
                "numeric bound unarmed: scale-up/down effect on relevant fluent `{}`",
                task.fluent_names.get(t).map(|s| s.as_str()).unwrap_or("?")
            ))
        } else {
            None
        }
    };
    for oi in 0..task.n_ops {
        for ne in task.num_eff.slice(oi) {
            if let Some(why) = scale_reject(ne) {
                return Err(why);
            }
        }
        for ce in task.cond.slice(oi) {
            for ne in &ce.num {
                if let Some(why) = scale_reject(ne) {
                    return Err(why);
                }
            }
        }
    }
    Ok(())
}

/// Goal-blind relaxed reachability to a FIXPOINT: returns `(fact_layer,
/// op_layer)` with `u32::MAX` for unreached entries. One RPG build; used by
/// landmark extraction ([`crate::landmarks`]), which needs every reachable
/// op's layer, not just enough layers to touch a goal.
pub fn reachability_layers(
    task: &PackedTask,
    sc: &mut Scratch,
    bits: &[u64],
    fv: &[f64],
    def: &[bool],
) -> (Vec<u32>, Vec<u32>) {
    sc.reset(task, bits, fv, false);
    build_rpg(task, sc, &[], &[], def, true, false);
    let op_layer: Vec<u32> = (0..task.n_ops)
        .map(|oi| {
            if sc.op_stamp[oi] == sc.gen {
                sc.op_layer[oi]
            } else {
                u32::MAX
            }
        })
        .collect();
    (sc.fact_layer.clone(), op_layer)
}

/// Relaxed-plan heuristic toward an ARBITRARY (sub)goal, using reusable `sc`.
/// None == dead end. This is the subplanner heuristic SGPlan-style partitioning
/// drives with per-subproblem goals. Always TIME-BLIND — the TRPG-lite
/// variant enters only through [`relaxed_helpful`] (the temporal pruned
/// pass's door), so the complete passes and every classical/subplanner
/// caller keep the historical heuristic byte-for-byte.
pub fn relaxed_to(
    task: &PackedTask,
    sc: &mut Scratch,
    bits: &[u64],
    fv: &[f64],
    def: &[bool],
    goal_pos: &[u32],
    goal_num: &[NumPre],
) -> Option<i32> {
    relaxed_to_inner(task, sc, bits, fv, def, goal_pos, goal_num, false)
}

#[allow(clippy::too_many_arguments)]
fn relaxed_to_inner(
    task: &PackedTask,
    sc: &mut Scratch,
    bits: &[u64],
    fv: &[f64],
    def: &[bool],
    goal_pos: &[u32],
    goal_num: &[NumPre],
    timed: bool,
) -> Option<i32> {
    use std::sync::atomic::Ordering::Relaxed;
    let t0 = crate::clock::Clock::now();
    sc.reset(task, bits, fv, timed);
    T_RESET.fetch_add(t0.elapsed_us() as u64, Relaxed);

    let t0 = crate::clock::Clock::now();
    build_rpg(task, sc, goal_pos, goal_num, def, false, timed);
    T_BUILD.fetch_add(t0.elapsed_us() as u64, Relaxed);

    if !goal_done(goal_pos, goal_num, &sc.reached, &sc.lb, &sc.ub, def) {
        return None;
    }
    let t0 = crate::clock::Clock::now();
    let r = relaxed_extract(task, sc, bits, fv, goal_pos, goal_num, def);
    T_EXTRACT.fetch_add(t0.elapsed_us() as u64, Relaxed);
    r
}

#[allow(clippy::too_many_arguments)]
fn relaxed_extract(
    task: &PackedTask,
    sc: &mut Scratch,
    bits: &[u64],
    fv: &[f64],
    goal_pos: &[u32],
    goal_num: &[NumPre],
    def: &[bool],
) -> Option<i32> {
    // ---- relaxed-plan extraction (count actions) ----
    let mut count: i32 = 0;
    let mut head = 0usize;
    for &g in goal_pos {
        let f = g as usize;
        if sc.need_fact[f] != sc.gen {
            sc.need_fact[f] = sc.gen;
            sc.queue.push(g);
        }
    }

    while head < sc.queue.len() {
        let f = sc.queue[head] as usize;
        head += 1;
        if bitset::test(bits, f) {
            continue;
        }
        if let Some(oi) = achiever(task, &sc.op_layer, &sc.op_stamp, sc.gen, &sc.fact_layer, f) {
            select(task, sc, oi, 1, &mut count);
            queue_cond_for(task, sc, oi, f);
        }
    }

    for np in goal_num {
        if eval_numpre(np, fv, def).unwrap_or(false) {
            continue;
        }
        if let Some((oi, reps)) = numeric_achiever(task, np, fv, def, &sc.op_stamp, sc.gen) {
            select(task, sc, oi, reps, &mut count);
            while head < sc.queue.len() {
                let f = sc.queue[head] as usize;
                head += 1;
                if bitset::test(bits, f) {
                    continue;
                }
                if let Some(o2) =
                    achiever(task, &sc.op_layer, &sc.op_stamp, sc.gen, &sc.fact_layer, f)
                {
                    select(task, sc, o2, 1, &mut count);
                    queue_cond_for(task, sc, o2, f);
                }
            }
        }
    }

    // The numeric-precondition charge (0.21 Phase 3, lever a1): the loops
    // above charge numeric distance only for TOP-LEVEL goals — a selected
    // op's own unsatisfied `pre_num` contributed ZERO, so a propositional
    // goal behind a numeric band (sailing's save_person) read h=1 across
    // the whole approach. Walk each already-selected op's `pre_num` against
    // THIS state and charge ONE level through the same achievers (charged
    // achievers' own `pre_num` is not recursed — lever a2, not taken). The
    // snapshot keeps the walk one-level by construction; a task with no
    // numeric preconditions never enters, so classical extraction is
    // byte-identical, and the temporal groundings clear `charge_pre_num`
    // (their boards are other phases' referee surface — see packed.rs).
    // `FF_NO_NUMPRE=1` restores the plateau.
    if task.charge_pre_num
        && !task.pre_num.flat.is_empty()
        && std::env::var("FF_NO_NUMPRE").is_err()
    {
        let selected_ops: Vec<usize> = (0..task.n_ops)
            .filter(|&oi| sc.selected[oi] == sc.gen)
            .collect();
        // The charge DAMPING (0.22 Phase 1, the 0.21 charge's bill), two
        // coordinated corrections behind one hatch:
        //
        //  1. SUM, not first-wins: when several selected ops' preconditions
        //     price the SAME achiever — ext-plant-watering's eleven pour
        //     ops all pricing one agent's move op — `select`'s dedup used
        //     to keep whichever gap came FIRST in op order and drop the
        //     rest, so the priced plant's identity flipped as the agent
        //     walked and h JUMPED UP on arrival at a near plant (i7:
        //     0.15 s at 0.20 → 2.9M evals unsolved at 0.21). Summing the
        //     per-precondition gaps keeps every plant's term: arrivals
        //     retire a term smoothly (no jump) and each step toward any
        //     unmet gate still descends. (MAX was probed first: monotone
        //     but too flat — it traded i5/i6/i8/i10/i16, the 0.21
        //     near-wall solves, for i7. The sum keeps all of them AND
        //     recovers i7/i13/rover i19; receipts in the 0.22 record.)
        //     Accumulate per-achiever sums, select once each after the
        //     walk; a lone achiever (sailing's fixture) is identical.
        //
        //  2. Skip preconditions the relaxed plan already pays a mover
        //     for: delivery's picks read `load + w ≤ limit` as violated in
        //     THIS state while the extraction has already selected the
        //     drop that lowers the load — charging them prices a
        //     satisfied-in-relaxation precondition, penalizes carrying,
        //     and inverts the progress gradient (i18: 9.3 s at 0.20 →
        //     timeout at 0.21). The walk runs against the extraction's
        //     selection snapshot, so the rule is deterministic and never
        //     sees the charge's own additions. Sailing keeps its whole
        //     gradient: nothing selected there moves the band fluents.
        //
        // `FF_NUMPRE_NODAMP=1` restores the 0.21 charge exactly.
        //
        // The 0.23 Phase 1 attribution hatches split the two corrections
        // so a bill row can name WHICH half it pays (the 0.22 damping
        // opened a 3-row bill — ext-plant i10/i16, sugar i18 — all
        // NODAMP-recoverable, and recovery needs the guilty half named
        // before any conditioning is designed): `FF_NUMPRE_NOSKIP=1`
        // charges even mover-covered preconditions (correction 2 off);
        // `FF_NUMPRE_NOSUM=1` selects first-wins instead of accumulating
        // (correction 1 off). Both together ≡ NODAMP.
        let damp = std::env::var("FF_NUMPRE_NODAMP").is_err();
        let skip_half = damp && std::env::var("FF_NUMPRE_NOSKIP").is_err();
        let sum_half = damp && std::env::var("FF_NUMPRE_NOSUM").is_err();
        let mut charges: Vec<(usize, i32)> = Vec::new();
        for oi in &selected_ops {
            for np in task.pre_num.slice(*oi) {
                if eval_numpre(np, fv, def).unwrap_or(false) {
                    continue;
                }
                if skip_half && selected_mover_exists(task, &selected_ops, np, fv, def) {
                    continue;
                }
                let Some((ai, reps)) = numeric_achiever(task, np, fv, def, &sc.op_stamp, sc.gen)
                else {
                    continue;
                };
                if sum_half {
                    match charges.iter_mut().find(|(a, _)| *a == ai) {
                        Some((_, r)) => *r = r.saturating_add(reps),
                        None => charges.push((ai, reps)),
                    }
                    continue;
                }
                select(task, sc, ai, reps, &mut count);
                while head < sc.queue.len() {
                    let f = sc.queue[head] as usize;
                    head += 1;
                    if bitset::test(bits, f) {
                        continue;
                    }
                    if let Some(o2) =
                        achiever(task, &sc.op_layer, &sc.op_stamp, sc.gen, &sc.fact_layer, f)
                    {
                        select(task, sc, o2, 1, &mut count);
                        queue_cond_for(task, sc, o2, f);
                    }
                }
            }
        }
        for (ai, reps) in charges {
            select(task, sc, ai, reps, &mut count);
            while head < sc.queue.len() {
                let f = sc.queue[head] as usize;
                head += 1;
                if bitset::test(bits, f) {
                    continue;
                }
                if let Some(o2) =
                    achiever(task, &sc.op_layer, &sc.op_stamp, sc.gen, &sc.fact_layer, f)
                {
                    select(task, sc, o2, 1, &mut count);
                    queue_cond_for(task, sc, o2, f);
                }
            }
        }
    }

    // The h-surgery end gate (0.21 Phase 8 probe, opt-in FF_H_ENDGATE=1):
    // h^FF pays for a snap-START the moment it fires while the interval
    // delivers nothing until its END lands — the start-credit plateau (TMS's
    // 110 floor, docs/roadmap-0.15.md). When the temporal think armed the
    // pair table, a selected START whose paired END is ALSO selected this
    // generation prices as ONE unit, paid while the END is owed. Discount
    // only — selection is untouched, so helpful sets cannot move (emptying
    // start selection replayed the 0.11 FF_LAX_HELPFUL negative). A START
    // selected only for its at-start effects keeps its full charge; a
    // reps>1 pair discounts at most 1 (recorded simplification). Composes
    // additively with the pre_num charge above; the temporal groundings
    // clear `charge_pre_num`, so the two passes never co-fire today.
    if let Some(pairs) = &task.pair_end {
        for (oi, &end) in pairs.iter().enumerate() {
            if end != u32::MAX && sc.selected[oi] == sc.gen && sc.selected[end as usize] == sc.gen {
                count -= 1;
            }
        }
    }

    Some(count)
}

/// Convenience: relaxed-plan heuristic toward the task's own goal.
pub fn relaxed(
    task: &PackedTask,
    sc: &mut Scratch,
    bits: &[u64],
    fv: &[f64],
    def: &[bool],
) -> Option<i32> {
    relaxed_to(task, sc, bits, fv, def, &task.goal_pos, &task.goal_num)
}

/// Relaxed-plan heuristic plus the FF helpful-action set (relaxed-plan ops
/// applicable in this state). Used by enforced hill-climbing to restrict
/// expansion. None == relaxed dead end. The returned op ids are in a
/// deterministic order (relaxed-plan selection order).
pub fn relaxed_helpful(
    task: &PackedTask,
    sc: &mut Scratch,
    bits: &[u64],
    fv: &[f64],
    def: &[bool],
    goal_pos: &[u32],
    goal_num: &[NumPre],
) -> Option<(i32, Vec<u32>)> {
    // TRPG-lite arming (0.23 Phase 4 probe 2): PRESENCE of the table picks
    // the time-gated build — never the env (the `pair_end` rule). Only the
    // temporal solve path builds the table (under FF_TRPG), and only its
    // PRUNED pass evaluates through here, so the complete passes stay
    // time-blind and completeness never rests on the gate.
    let h = relaxed_to_inner(
        task,
        sc,
        bits,
        fv,
        def,
        goal_pos,
        goal_num,
        task.trpg.is_some(),
    )?;
    // really applicable in THIS state (op_layer 0 is only relaxed-applicable —
    // numeric interval bounds are optimistic, so re-check exactly).
    let applicable = |oi: usize| {
        task.pre_pos
            .slice(oi)
            .iter()
            .all(|&f| bitset::test(bits, f as usize))
            && task
                .pre_num
                .slice(oi)
                .iter()
                .all(|np| eval_numpre(np, fv, def) == Some(true))
    };
    // helpful = the relaxed plan's applicable-now ops. Filter for REAL
    // applicability: on numeric domains a selected op can be relaxed-applicable
    // (op_layer 0) yet not actually applicable.
    let mut helpful: Vec<u32> = sc
        .helpful
        .iter()
        .copied()
        .filter(|&oi| applicable(oi as usize))
        .collect();
    // If that leaves nothing (typical when the relaxed plan is gated by numeric
    // preconditions), fall back to numeric subgoals: applicable ops whose numeric
    // effects touch a fluent an unsatisfied numeric precondition of a relaxed-plan
    // op reads.
    if helpful.is_empty() && h > 0 {
        let mut wanted = vec![false; fv.len()];
        let mut any = false;
        let mut tmp = Vec::new();
        for oi in 0..task.n_ops {
            if sc.selected[oi] != sc.gen {
                continue;
            }
            for np in task.pre_num.slice(oi) {
                if eval_numpre(np, fv, def) == Some(true) {
                    continue;
                }
                tmp.clear();
                np.lhs.collect_fluents(&mut tmp);
                np.rhs.collect_fluents(&mut tmp);
                for &fl in &tmp {
                    wanted[fl as usize] = true;
                    any = true;
                }
            }
        }
        if any {
            for oi in 0..task.n_ops {
                if applicable(oi)
                    && task
                        .num_eff
                        .slice(oi)
                        .iter()
                        .any(|ne| wanted[ne.target as usize])
                {
                    helpful.push(oi as u32);
                }
            }
        }
        // last resort: every really-applicable op, so EHC can act rather than
        // instantly failing (still h-guided, just unpruned for this state).
        if helpful.is_empty() {
            for oi in 0..task.n_ops {
                if applicable(oi) {
                    helpful.push(oi as u32);
                }
            }
        }
    }
    Some((h, helpful))
}

/// Lax preferred-operator fallback (0.11 Phase 2): really-applicable ops
/// whose ADD achieves a fact some relaxed-plan op still NEEDS (a positive
/// precondition not true in this state). Valid immediately after a
/// `relaxed_to`/`relaxed_helpful` call on the SAME state (reads
/// `sc.selected` at the current generation). The temporal path uses it when
/// the strict helpful set filters to nothing — its relaxed plans often lead
/// through END ops (fired by the agenda, not chosen), and the Start-only
/// filter starved the pruned pass into full scans (storage/model-train:
/// stored helpful averaged 0.0).
pub fn helpful_needed_adders(
    task: &PackedTask,
    sc: &Scratch,
    bits: &[u64],
    fv: &[f64],
    def: &[bool],
) -> Vec<u32> {
    let mut needed = vec![false; task.n_facts];
    let mut any = false;
    for oi in 0..task.n_ops {
        if sc.selected[oi] != sc.gen {
            continue;
        }
        for &f in task.pre_pos.slice(oi) {
            if !bitset::test(bits, f as usize) {
                needed[f as usize] = true;
                any = true;
            }
        }
    }
    if !any {
        return Vec::new();
    }
    let applicable = |oi: usize| {
        task.pre_pos
            .slice(oi)
            .iter()
            .all(|&f| bitset::test(bits, f as usize))
            && task
                .pre_num
                .slice(oi)
                .iter()
                .all(|np| eval_numpre(np, fv, def) == Some(true))
    };
    (0..task.n_ops)
        .filter(|&oi| applicable(oi) && task.add.slice(oi).iter().any(|&f| needed[f as usize]))
        .map(|oi| oi as u32)
        .collect()
}

/// The R-partition read-out (0.22 Phase 5B lever 1): every fact the last
/// extraction STAMPED as needed — the top-level goals, every selected
/// op's positive precondition, every queued conditional-effect condition
/// — paired with its RPG fact layer (the driver caps R lowest-layer-
/// first). Valid immediately after a `relaxed_to`/`relaxed_helpful` call
/// on the SAME state (the `helpful_needed_adders` contract: reads the
/// current-generation stamps). Read-only — the ~20 lines the roadmap
/// priced, no new bookkeeping.
pub fn extraction_need_facts(sc: &Scratch) -> Vec<(u32, u32)> {
    sc.need_fact
        .iter()
        .enumerate()
        .filter(|&(_, &g)| g == sc.gen)
        .map(|(f, _)| (f as u32, sc.fact_layer[f]))
        .collect()
}

// ARCHAEOLOGY (0.23 Phase 1): `extraction_selected_pre_num` — the
// FF_NOV_NUMFEAT rider's read-out (0.22 Phase 5B lever 4) — lived here
// and left with the rider: the flag was opt-in, no sweep ever armed it,
// and the fresh solo evidence read null. House law: an opt-in flag that
// no sweep arms produces no evidence, and no evidence means no pitch.

/// Relaxed completion COST of a subgoal from this state: run the relaxed-plan
/// extraction toward `goal_pos`/`goal_num`, then sum the SELECTED ops'
/// `increase` effects on `cost_fluent`, each evaluated against this state's
/// fluent values — exact when the increase amounts read only static fluents
/// (the IPC numeric-metric shape, e.g. rovers' traverse costs). Ops are
/// counted once (set semantics), so this UNDERestimates a plan that must
/// repeat an op — the safe direction for the forgo-aware seed (an
/// underestimate never prices a cheap preference out). None == relaxed dead
/// end (the subgoal is unreachable even ignoring deletes).
#[allow(clippy::too_many_arguments)]
pub fn relaxed_plan_cost(
    task: &PackedTask,
    sc: &mut Scratch,
    bits: &[u64],
    fv: &[f64],
    def: &[bool],
    goal_pos: &[u32],
    goal_num: &[NumPre],
    cost_fluent: usize,
) -> Option<f64> {
    relaxed_to(task, sc, bits, fv, def, goal_pos, goal_num)?;
    Some(selected_increase_sum(task, sc, fv, def, cost_fluent))
}

/// Summed `increase cost_fluent` amounts of the ops SELECTED by the last
/// relaxed-plan extraction left in `sc` (valid until the next reset), each
/// evaluated against this state's fluent values. Ops count once (set
/// semantics) — an underestimate when a plan must repeat an op.
fn selected_increase_sum(
    task: &PackedTask,
    sc: &Scratch,
    fv: &[f64],
    def: &[bool],
    cost_fluent: usize,
) -> f64 {
    let mut cost = 0.0;
    for oi in 0..task.n_ops {
        if sc.selected[oi] != sc.gen {
            continue;
        }
        for ne in task.num_eff.slice(oi) {
            if ne.target as usize == cost_fluent && ne.op == AssignOp::Increase {
                if let Some(v) = ne.value.eval(fv, def) {
                    cost += v.max(0.0);
                }
            }
        }
    }
    cost
}

/// Cost-augmented relaxed-plan heuristic: relaxed-plan LENGTH plus the summed
/// `increase cost_fluent` of the selected ops — the "cost + 1 per action"
/// shape, so search guided by it prefers cheap achievers while zero-cost
/// regions still keep a distance gradient (a pure-cost h flatlines wherever
/// remaining actions are free). Units are cost+steps; callers weight it like
/// any h. None == relaxed dead end.
#[allow(clippy::too_many_arguments)]
pub fn relaxed_costed(
    task: &PackedTask,
    sc: &mut Scratch,
    bits: &[u64],
    fv: &[f64],
    def: &[bool],
    goal_pos: &[u32],
    goal_num: &[NumPre],
    cost_fluent: usize,
) -> Option<i32> {
    let count = relaxed_to(task, sc, bits, fv, def, goal_pos, goal_num)?;
    let cost = selected_increase_sum(task, sc, fv, def, cost_fluent);
    Some(count.saturating_add(cost.min(1e9).round() as i32))
}

/// Lowest-layer op that adds fact `f` (FF prefers earliest achievers).
/// Uses the precomputed add-by-fact index instead of scanning all ops.
fn achiever(
    task: &PackedTask,
    op_layer: &[u32],
    op_stamp: &[u32],
    gen: u32,
    fact_layer: &[u32],
    f: usize,
) -> Option<usize> {
    let fl = fact_layer[f];
    if fl == INF || fl == 0 {
        return None;
    }
    let mut best = None;
    let mut best_layer = INF;
    for &oi in task.add_by_fact.slice(f) {
        let oi = oi as usize;
        if op_stamp[oi] == gen && op_layer[oi] < fl && op_layer[oi] < best_layer {
            best_layer = op_layer[oi];
            best = Some(oi);
        }
    }
    best
}

/// When fact `f` is achieved by op `oi` through a CONDITIONAL effect (not an
/// unconditional add), queue that effect's positive condition facts as extra
/// subgoals so the relaxed plan accounts for establishing the condition.
fn queue_cond_for(task: &PackedTask, sc: &mut Scratch, oi: usize, f: usize) {
    if task.add.slice(oi).iter().any(|&x| x as usize == f) {
        return; // unconditional add — nothing extra
    }
    let mut best_layer = INF;
    let mut best: Option<&crate::packed::CondEff> = None;
    for ce in task.cond_effs(oi) {
        if ce.add.iter().any(|&x| x as usize == f) {
            let cl = ce
                .cond_pos
                .iter()
                .map(|&c| sc.fact_layer[c as usize])
                .max()
                .unwrap_or(0);
            if cl != INF && cl < best_layer {
                best_layer = cl;
                best = Some(ce);
            }
        }
    }
    if let Some(ce) = best {
        for &cf in &ce.cond_pos {
            let c = cf as usize;
            if sc.need_fact[c] != sc.gen {
                sc.need_fact[c] = sc.gen;
                sc.queue.push(cf);
            }
        }
    }
}

/// Select op `oi` (×`reps`) into the relaxed plan and queue its preconditions.
fn select(task: &PackedTask, sc: &mut Scratch, oi: usize, reps: i32, count: &mut i32) {
    if sc.selected[oi] == sc.gen {
        return;
    }
    sc.selected[oi] = sc.gen;
    // a selected op applicable in the current state (layer 0) is a helpful action.
    if sc.op_stamp[oi] == sc.gen && sc.op_layer[oi] == 0 {
        sc.helpful.push(oi as u32);
    }
    *count += reps.max(1);
    for &pf in task.pre_pos.slice(oi) {
        let f = pf as usize;
        if sc.need_fact[f] != sc.gen {
            sc.need_fact[f] = sc.gen;
            sc.queue.push(pf);
        }
    }
}

/// Fold a linear numeric expression into `Σ coeff·fluent + konst`,
/// scaled by `scale`. `false` = not linear (fluent×fluent, fluent
/// divisor) — the caller falls back to no charge, exactly as before.
/// `pub(crate)` since 0.23 Phase 5: the optimal mode's rep-folded
/// numeric labels normalize their margins through the same fold.
pub(crate) fn linearize(
    e: &NExpr,
    scale: f64,
    coeffs: &mut Vec<(u32, f64)>,
    konst: &mut f64,
) -> bool {
    fn const_of(e: &NExpr) -> Option<f64> {
        match e {
            NExpr::Num(n) => Some(*n),
            NExpr::Fluent(_) => None,
            NExpr::Add(a, b) => Some(const_of(a)? + const_of(b)?),
            NExpr::Sub(a, b) => Some(const_of(a)? - const_of(b)?),
            NExpr::Mul(a, b) => Some(const_of(a)? * const_of(b)?),
            NExpr::Div(a, b) => Some(const_of(a)? / const_of(b)?),
            NExpr::Neg(a) => Some(-const_of(a)?),
        }
    }
    match e {
        NExpr::Num(n) => {
            *konst += scale * n;
            true
        }
        NExpr::Fluent(i) => {
            coeffs.push((*i, scale));
            true
        }
        NExpr::Add(a, b) => {
            linearize(a, scale, coeffs, konst) && linearize(b, scale, coeffs, konst)
        }
        NExpr::Sub(a, b) => {
            linearize(a, scale, coeffs, konst) && linearize(b, -scale, coeffs, konst)
        }
        NExpr::Mul(a, b) => {
            if let Some(c) = const_of(a) {
                linearize(b, scale * c, coeffs, konst)
            } else if let Some(c) = const_of(b) {
                linearize(a, scale * c, coeffs, konst)
            } else {
                false
            }
        }
        NExpr::Div(a, b) => match const_of(b) {
            Some(c) if c != 0.0 => linearize(a, scale / c, coeffs, konst),
            _ => false,
        },
        NExpr::Neg(a) => linearize(a, -scale, coeffs, konst),
    }
}

/// Repetition charge for a LINEAR numeric goal the bare-fluent path
/// cannot see (0.19 Phase 3, the landscape memo's bet #2): normalize
/// `lhs ≥ rhs` to `Σ coeff·fluent + konst ≥ 0`, then find the op whose
/// combined constant-delta effects raise the combination fastest —
/// reps = ⌈gap / combo_delta⌉. Runs ONLY where the bare path returned
/// None, so every previously-charged shape keeps its exact charge (and
/// tie-break); sailing's `(≥ (+ (* 2 (x)) (y)) (d))` class gains a
/// gradient where it had a plateau. `FF_NO_NUMH=1` restores the hole.
fn numeric_achiever_linear(
    task: &PackedTask,
    np: &NumPre,
    fv: &[f64],
    def: &[bool],
    op_stamp: &[u32],
    gen: u32,
) -> Option<(usize, i32)> {
    if std::env::var("FF_NO_NUMH").is_ok() {
        return None;
    }
    let mut coeffs: Vec<(u32, f64)> = Vec::new();
    let mut konst = 0.0;
    if !linearize(&np.lhs, 1.0, &mut coeffs, &mut konst)
        || !linearize(&np.rhs, -1.0, &mut coeffs, &mut konst)
    {
        return None;
    }
    let cur: f64 = konst
        + coeffs
            .iter()
            .map(|&(f, c)| {
                if def[f as usize] {
                    c * fv[f as usize]
                } else {
                    0.0
                }
            })
            .sum::<f64>();
    let need_raise = match np.op {
        CompOp::Ge | CompOp::Gt => true,
        CompOp::Le | CompOp::Lt => false,
        // An Eq from a POINT value is one-sided — raise if below zero,
        // lower if above — so it charges like the Ge/Le pair's unmet half
        // (0.21 Phase 3 probe rider c: block-grouping's entire goal shape
        // is `(= (x b1) (x b2))`, refused outright before).
        CompOp::Eq => cur < 0.0,
    };
    // condition is `combo ≥ 0` (or ≤): gap is how far the wrong side is
    let gap = if need_raise { -cur } else { cur };
    if gap <= 0.0 {
        return None; // already satisfiable-side; the caller's eval said no, stay silent
    }
    let mut best: Option<(usize, i32)> = None;
    let mut seen_ops: Vec<u32> = Vec::new();
    for &(f, _) in &coeffs {
        for &oi in task.neff_by_fluent.slice(f as usize) {
            if op_stamp[oi as usize] != gen || seen_ops.contains(&oi) {
                continue;
            }
            seen_ops.push(oi);
            let mut combo_delta = 0.0;
            for ne in task.num_eff.slice(oi as usize) {
                let Some(&(_, coeff)) = coeffs.iter().find(|&&(f2, _)| f2 == ne.target) else {
                    continue;
                };
                let Some(v) = ne.value.eval(fv, def) else {
                    continue;
                };
                match ne.op {
                    AssignOp::Increase => combo_delta += coeff * v,
                    AssignOp::Decrease => combo_delta -= coeff * v,
                    _ => {
                        combo_delta = f64::NAN; // non-monotone writer: skip op
                        break;
                    }
                }
            }
            let toward = if need_raise {
                combo_delta
            } else {
                -combo_delta
            };
            if toward > 1e-9 && combo_delta.is_finite() {
                let reps = ((gap / toward).ceil() as i32).max(1);
                if best
                    .map(|(bo, r)| reps < r || (reps == r && (oi as usize) < bo))
                    .unwrap_or(true)
                {
                    best = Some((oi as usize, reps));
                }
            }
        }
    }
    best
}

/// Does any ALREADY-SELECTED op move `np`'s linear combination toward
/// satisfaction from this state? The damped charge (0.22 Phase 1) skips
/// such preconditions: the relaxed plan is already paying for a mover, so
/// charging them again prices a satisfied-in-relaxation precondition
/// (delivery's carrying penalty). Non-linear shapes and Assign/Scale
/// writers return false — they keep their charge, the conservative side.
fn selected_mover_exists(
    task: &PackedTask,
    selected_ops: &[usize],
    np: &NumPre,
    fv: &[f64],
    def: &[bool],
) -> bool {
    let mut coeffs: Vec<(u32, f64)> = Vec::new();
    let mut konst = 0.0;
    if !linearize(&np.lhs, 1.0, &mut coeffs, &mut konst)
        || !linearize(&np.rhs, -1.0, &mut coeffs, &mut konst)
    {
        return false;
    }
    let cur: f64 = konst
        + coeffs
            .iter()
            .map(|&(f, c)| {
                if def[f as usize] {
                    c * fv[f as usize]
                } else {
                    0.0
                }
            })
            .sum::<f64>();
    let need_raise = match np.op {
        CompOp::Ge | CompOp::Gt => true,
        CompOp::Le | CompOp::Lt => false,
        CompOp::Eq => cur < 0.0,
    };
    for &oi in selected_ops {
        let mut combo_delta = 0.0;
        for ne in task.num_eff.slice(oi) {
            let Some(&(_, coeff)) = coeffs.iter().find(|&&(f2, _)| f2 == ne.target) else {
                continue;
            };
            let Some(v) = ne.value.eval(fv, def) else {
                continue;
            };
            match ne.op {
                AssignOp::Increase => combo_delta += coeff * v,
                AssignOp::Decrease => combo_delta -= coeff * v,
                _ => {
                    combo_delta = f64::NAN;
                    break;
                }
            }
        }
        let toward = if need_raise {
            combo_delta
        } else {
            -combo_delta
        };
        if combo_delta.is_finite() && toward > 1e-9 {
            return true;
        }
    }
    false
}

fn numeric_achiever(
    task: &PackedTask,
    np: &NumPre,
    fv: &[f64],
    def: &[bool],
    op_stamp: &[u32],
    gen: u32,
) -> Option<(usize, i32)> {
    let target = match &np.lhs {
        NExpr::Fluent(i) => *i,
        _ => return numeric_achiever_linear(task, np, fv, def, op_stamp, gen),
    };
    let want = match &np.rhs {
        NExpr::Num(n) => *n,
        _ => return numeric_achiever_linear(task, np, fv, def, op_stamp, gen),
    };
    let cur = if def[target as usize] {
        fv[target as usize]
    } else {
        0.0
    };
    let need_raise = cur < want;
    let mut best: Option<(usize, i32)> = None;
    // only ops with a numeric effect on `target` can help (op-id order preserved,
    // so the min-reps tie-break is identical to the former full scan)
    for &oi in task.neff_by_fluent.slice(target as usize) {
        let oi = oi as usize;
        if op_stamp[oi] != gen {
            continue;
        }
        for ne in task.num_eff.slice(oi) {
            if ne.target != target {
                continue;
            }
            let delta = match ne.value.eval(fv, def) {
                Some(v) => v,
                None => continue,
            };
            let helps = match ne.op {
                AssignOp::Increase => need_raise && delta > 0.0,
                AssignOp::Decrease => !need_raise && delta > 0.0,
                _ => false,
            };
            if helps {
                let reps = (((want - cur).abs() / delta.abs().max(1e-9)).ceil() as i32).max(1);
                if best.map(|(_, r)| reps < r).unwrap_or(true) {
                    best = Some((oi, reps));
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    //! The numeric-precondition charge (0.21 Phase 3): extraction used to
    //! charge numeric distance only for TOP-LEVEL goals, so a propositional
    //! goal gated by a numeric band read h=1 across the whole approach.

    use super::*;

    fn task_of(dom: &str, prb: &str) -> PackedTask {
        let d = crate::parser::parse_domain(dom).unwrap();
        let p = crate::parser::parse_problem(prb).unwrap();
        crate::ground::ground_task(&d, &p, 1).unwrap()
    }

    fn h_init(task: &PackedTask) -> Option<i32> {
        let init = task.initial();
        let mut sc = Scratch::new(task);
        relaxed(task, &mut sc, &init.bits, &init.fv, &init.fdef)
    }

    /// The sailing-band pin: save's two unmet LOWER band sides each price
    /// their best +3/step raiser against this state — h(init) =
    /// 1 (save) + ceil(30/3) (x+y side) + ceil(30/3) (y-x side) = 21.
    /// Before the charge this read 1 (the plateau EHC died on).
    #[test]
    fn sailing_band_h_init_carries_the_band_charge() {
        let task = task_of(
            include_str!("../../../benchmarks/bench/sailing-band-domain.pddl"),
            include_str!("../../../benchmarks/bench/sailing-band-i1.pddl"),
        );
        assert_eq!(h_init(&task), Some(21), "1 + ceil(30/3) + ceil(30/3)");
    }

    /// The NEGATIVE control (markettrader shape): a cyclic resource flow —
    /// buy consumes cash, sell restores it, profit requires the lap. The
    /// charge keeps h FINITE and ~35 (the goal's ceil(480/14) sell charge
    /// plus one-level lap costs) while the real plan is ~960 lap steps —
    /// two orders of under-pricing. Cycle-blindness stays, and the boards'
    /// markettrader ceiling is recorded, not fixed.
    #[test]
    fn trader_cycle_h_stays_finite_and_blind() {
        let task = task_of(
            include_str!("../../../benchmarks/bench/trader-cycle-domain.pddl"),
            include_str!("../../../benchmarks/bench/trader-cycle-i1.pddl"),
        );
        let h = h_init(&task).expect("relaxed-reachable");
        assert!(
            h < 100,
            "h(init) = {h}: the relaxation must keep under-pricing the cycle"
        );
    }

    /// One test mutates `FF_NUMPRE_NODAMP`; both watering pins take this
    /// lock so the parallel runner cannot race the hatch.
    static DAMP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// The charge's BILL, distilled (0.22 Phase 1): two pour ops price the
    /// ONE shared raiser (move_right) — plant A at gap 1, plant B at gap 9.
    const WATER_DOM: &str = "
    (define (domain watering-mini)
      (:requirements :typing :numeric-fluents)
      (:types agent plant - object)
      (:functions (x ?o - object) (carrying ?a - agent)
                  (poured ?p - plant) (maxx))
      (:action move_right :parameters (?a - agent)
        :precondition (<= (+ (x ?a) 1) (maxx))
        :effect (increase (x ?a) 1))
      (:action move_left :parameters (?a - agent)
        :precondition (>= (- (x ?a) 1) 0)
        :effect (decrease (x ?a) 1))
      (:action pour :parameters (?a - agent ?p - plant)
        :precondition (and (= (x ?a) (x ?p)) (>= (carrying ?a) 1))
        :effect (and (decrease (carrying ?a) 1) (increase (poured ?p) 1))))";
    const WATER_PRB: &str = "
    (define (problem watering-mini-1) (:domain watering-mini)
      (:objects a1 - agent pa pb - plant)
      (:init (= (x a1) 0) (= (x pa) 1) (= (x pb) 9)
             (= (carrying a1) 5) (= (poured pa) 0) (= (poured pb) 0)
             (= (maxx) 12))
      (:goal (and (= (poured pa) 1) (= (poured pb) 1))))";

    fn water_h_pair() -> (i32, i32) {
        let task = task_of(WATER_DOM, WATER_PRB);
        let mr = (0..task.n_ops)
            .find(|&oi| task.op_display[oi].to_uppercase().contains("MOVE_RIGHT"))
            .expect("move_right grounds");
        let init = task.initial();
        let stepped = task.apply(mr, &init);
        let mut sc = Scratch::new(&task);
        let h0 = relaxed(&task, &mut sc, &init.bits, &init.fv, &init.fdef).unwrap();
        let h1 = relaxed(&task, &mut sc, &stepped.bits, &stepped.fv, &stepped.fdef).unwrap();
        (h0, h1)
    }

    /// The damped charge prices the shared raiser at the SUM of the gaps:
    /// h(init) = 2 pours + (1 + 9) = 12, and one step right DESCENDS to
    /// 2 + (0 + 8) = 10 — arriving at plant A retires its term smoothly,
    /// never re-points the charge upward. (A MAX damping was probed first
    /// and pinned here as (11, 10): monotone but too flat — it traded the
    /// 0.21 near-wall ext-plant-watering solves for i7; the sum keeps
    /// both. The 0.22 record carries the receipts.)
    #[test]
    fn damped_charge_prices_shared_achiever_at_gap_sum() {
        let _g = DAMP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(water_h_pair(), (12, 10), "(h_init, h_after_right)");
    }

    /// The 0.21 first-wins charge under the hatch: plant A's gap-1 charge
    /// wins at init (h = 3), and the step ONTO plant A re-points the charge
    /// at plant B's gap 8 (h = 10) — h jumps UP on progress, the exact
    /// gradient inversion that took ext-plant-watering i7 from 0.15 s at
    /// 0.20 to 2.9M evals unsolved at 0.21.
    #[test]
    fn nodamp_hatch_restores_the_first_wins_jump() {
        let _g = DAMP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("FF_NUMPRE_NODAMP", "1");
        let pair = water_h_pair();
        std::env::remove_var("FF_NUMPRE_NODAMP");
        assert_eq!(pair, (3, 10), "(h_init, h_after_right): the jump");
    }

    /// The 0.23 attribution hatches disable exactly ONE half each, pinned
    /// on the mini where the OTHER half is inert (no selected op moves the
    /// position combos, and `carrying ≥ 1` is satisfied, so the mover-skip
    /// never fires here): `FF_NUMPRE_NOSUM=1` alone restores the first-wins
    /// jump (≡ the NODAMP pair), and `FF_NUMPRE_NOSKIP=1` alone leaves the
    /// summed pricing untouched (≡ the damped pair). The halves' live
    /// receipts are fo-sailing i8's fixture pair in tests/numpre.rs — the
    /// SUM half is load-bearing there and the skip half is byte-inert.
    #[test]
    fn split_hatches_disable_exactly_one_half_each() {
        let _g = DAMP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("FF_NUMPRE_NOSUM", "1");
        let nosum = water_h_pair();
        std::env::remove_var("FF_NUMPRE_NOSUM");
        assert_eq!(nosum, (3, 10), "NOSUM alone is first-wins on this mini");
        std::env::set_var("FF_NUMPRE_NOSKIP", "1");
        let noskip = water_h_pair();
        std::env::remove_var("FF_NUMPRE_NOSKIP");
        assert_eq!(noskip, (12, 10), "NOSKIP alone keeps the summed pricing");
    }

    // ---- 0.22 Phase 4 L0: the numeric-admissible layer bound + audit ----

    fn goal_layers_init(task: &PackedTask) -> RpgExit {
        let init = task.initial();
        let mut sc = Scratch::new(task);
        admissible_goal_layers(task, &mut sc, &init.bits, &init.fv, &init.fdef)
    }

    /// The numopt admissibility pins: h_num(init) ≤ the known optimum on
    /// every problem the 0.21 soundness fixtures pinned, with the EXACT
    /// layer asserted so the counter's arithmetic can never drift quietly.
    /// p01: optimum LENGTH 3 (bigpump pump fire); each layer widens charge
    /// by +6 (pump and bigpump both re-widen), fire arms at layer 1, done
    /// lands at fact layer 2 ⇒ GoalAt(2). p02: charge ≥ 6 at layer 1
    /// (optimum 2 steps). p03: GoalAt(2) = the optimum exactly.
    #[test]
    fn numopt_layer_bounds_are_admissible() {
        let dom = include_str!("../../../benchmarks/bench/numopt-domain.pddl");
        let cases = [
            ("../../../benchmarks/bench/numopt-p01.pddl", 2, 3),
            ("../../../benchmarks/bench/numopt-p02.pddl", 1, 2),
            ("../../../benchmarks/bench/numopt-p03.pddl", 2, 2),
        ];
        let probs = [
            include_str!("../../../benchmarks/bench/numopt-p01.pddl"),
            include_str!("../../../benchmarks/bench/numopt-p02.pddl"),
            include_str!("../../../benchmarks/bench/numopt-p03.pddl"),
        ];
        for (i, prb) in probs.iter().enumerate() {
            let (name, layers, optimum) = cases[i];
            let task = task_of(dom, prb);
            assert_eq!(numeric_interval_audit(&task), Ok(()), "{name}");
            let got = goal_layers_init(&task);
            assert_eq!(got, RpgExit::GoalAt(layers), "{name}");
            assert!(layers <= optimum, "{name}: bound must not exceed optimum");
        }
    }

    /// The gap-60 pump chain (numopt-p04): +1 per layer from charge 0
    /// toward 60 makes the counter's arithmetic transparent — GoalAt(60),
    /// exactly. 61 would be INADMISSIBLE (the true optimum IS 60 pumps),
    /// 59 a silent weakening; the off-by-one is pinned forever.
    #[test]
    fn pump_chain_layer_counter_off_by_one_is_pinned() {
        let task = task_of(
            include_str!("../../../benchmarks/bench/numopt-pump-domain.pddl"),
            include_str!("../../../benchmarks/bench/numopt-p04.pddl"),
        );
        assert_eq!(numeric_interval_audit(&task), Ok(()));
        assert_eq!(goal_layers_init(&task), RpgExit::GoalAt(60));
    }

    /// sailing-band on the ADMISSIBLE side (0.22 Phase 4): the audit
    /// passes (increase/decrease by constants, everything defined), and
    /// the layer bound reads GoalAt(4) — both +3/step band sides sit 30
    /// short, all five movers re-widen every layer (±4.5 on x, +3/−2 on
    /// y), so x+y's ub crosses −30 after 4 rounds and save fires — a
    /// LOWER bound two orders under the charged h's exact 21, and that is
    /// the design: weak-but-admissible against the proof boards' currency.
    #[test]
    fn sailing_band_layer_bound_is_admissible() {
        let task = task_of(
            include_str!("../../../benchmarks/bench/sailing-band-domain.pddl"),
            include_str!("../../../benchmarks/bench/sailing-band-i1.pddl"),
        );
        assert_eq!(numeric_interval_audit(&task), Ok(()));
        match goal_layers_init(&task) {
            RpgExit::GoalAt(l) => {
                assert_eq!(l, 4, "the widening arithmetic moved");
                assert!(l <= 21, "must lower-bound the 21-step optimum");
            }
            other => panic!("expected GoalAt, got {other:?}"),
        }
    }

    /// trader-cycle stays the NEGATIVE control on the admissible side
    /// too: the audit passes and the bound is sound, but a cyclic economy
    /// prices at a handful of layers (every sell re-widens cash each
    /// round) against a ~2,400-step real optimum — the bound may never be
    /// used as evidence of guidance on cycle domains.
    #[test]
    fn trader_cycle_layer_bound_is_weak_but_sound() {
        let task = task_of(
            include_str!("../../../benchmarks/bench/trader-cycle-domain.pddl"),
            include_str!("../../../benchmarks/bench/trader-cycle-i1.pddl"),
        );
        assert_eq!(numeric_interval_audit(&task), Ok(()));
        match goal_layers_init(&task) {
            RpgExit::GoalAt(l) => assert!(
                (1..=20).contains(&l),
                "GoalAt({l}): admissible (≤ 2,400) and honestly blind to the lap"
            ),
            other => panic!("expected GoalAt, got {other:?}"),
        }
    }

    /// Audit reject #1, by name: a scale-up on a relevant fluent. widen's
    /// ScaleUp only raises ub — a factor-0.25 scale-up SHRINKS the true
    /// value below lb forever, the goal (<= x 0.5) reads unreachable at
    /// every layer, and an unaudited arm would certify PROVEN UNSOLVABLE
    /// on a 1-step task (tests/optimal.rs carries the end-to-end teeth).
    #[test]
    fn audit_rejects_scale_on_relevant_fluent() {
        let task = task_of(
            "(define (domain sc) (:requirements :numeric-fluents)
              (:functions (x))
              (:action shrink :parameters ()
                :precondition (and) :effect (scale-up (x) 0.25)))",
            "(define (problem p) (:domain sc)
              (:init (= (x) 1)) (:goal (<= (x) 0.5)))",
        );
        let err = numeric_interval_audit(&task).unwrap_err();
        assert!(err.contains("scale-up/down"), "{err}");
    }

    /// Audit reject #2, by name: a relevant fluent undefined at init.
    /// eval_iv reads None, num_sat stays false, and a goal a real plan
    /// reaches by define-then-read is labeled unreachable — an
    /// overestimate the audit fails closed on. The grounder NORMALIZES
    /// most of this class away (its definedness fixpoint marks
    /// assign-reachable fluents defined-at-0.0, ground.rs:1540 — engine
    /// semantics the RPG then matches), so the residual shape is a
    /// runtime-only definition the fixpoint could not prove; the pin
    /// constructs it directly on the packed task rather than hunting a
    /// PDDL incantation the normalizer might learn to close later.
    #[test]
    fn audit_rejects_undefined_at_init_relevant_fluent() {
        let mut task = task_of(
            include_str!("../../../benchmarks/bench/numopt-domain.pddl"),
            include_str!("../../../benchmarks/bench/numopt-p02.pddl"),
        );
        assert_eq!(numeric_interval_audit(&task), Ok(()), "baseline is clean");
        let charge = task.rel_fluents[0] as usize;
        task.fdef0[charge] = false;
        let err = numeric_interval_audit(&task).unwrap_err();
        assert!(err.contains("undefined at init"), "{err}");
    }

    /// Audit reject #3, by name: the shared trajectory-monitor block.
    /// Monitor transitions carry negative conditions and delete semantics
    /// the containment induction does not cover; fail closed.
    #[test]
    fn audit_rejects_monitored_tasks() {
        let d = crate::parser::parse_domain(
            "(define (domain mon) (:requirements :numeric-fluents)
              (:predicates (loaded) (done))
              (:functions (fuel))
              (:action load :parameters ()
                :precondition (and) :effect (loaded))
              (:action go :parameters ()
                :precondition (and (loaded) (>= (fuel) 1))
                :effect (and (done) (decrease (fuel) 1))))",
        )
        .unwrap();
        let p = crate::parser::parse_problem(
            "(define (problem m) (:domain mon)
              (:init (= (fuel) 3)) (:goal (done))
              (:constraints (at-most-once (loaded))))",
        )
        .unwrap();
        let (d, p) = crate::derived::compile(&d, &p).expect("derived");
        let (d, p) = match crate::constraints::gate(&d, &p).expect("gate") {
            Some(pair) => pair,
            None => panic!("constraints must compile to monitors"),
        };
        let task = crate::ground::ground_task(&d, &p, 1).expect("ground");
        assert!(
            task.monitored.iter().any(|&m| m),
            "fixture must actually carry a monitor block"
        );
        let err = numeric_interval_audit(&task).unwrap_err();
        assert!(err.contains("monitored"), "{err}");
    }
}
