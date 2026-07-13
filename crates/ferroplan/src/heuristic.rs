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

const LAYER_CAP: u32 = 2000;
const INF: u32 = u32::MAX;

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
        }
    }

    fn reset(&mut self, task: &PackedTask, bits: &[u64], fv: &[f64]) {
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
/// relaxed-reached. `sc` must be reset first.
fn build_rpg(
    task: &PackedTask,
    sc: &mut Scratch,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    def: &[bool],
    to_fixpoint: bool,
) {
    // ---- build the relaxed planning graph (two-phase, incremental) ----
    // Only UNAPPLIED ops are re-scanned each layer; applied ops never lose
    // applicability (delete-relaxed), so they are skipped — except those with
    // relevant numeric effects, which are re-widened each layer from
    // `num_applied` so monotone fluents (e.g. consumed-resources) can grow to
    // reach numeric goals.
    let mut layer: u32 = 0;
    loop {
        if !to_fixpoint && goal_done(goal_pos, goal_num, &sc.reached, &sc.lb, &sc.ub, def) {
            break;
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
            for ce in task.cond.slice(oi) {
                let pos_ok = ce.cond_pos.iter().all(|&c| sc.reached[c as usize]);
                let num_ok = ce
                    .cond_num
                    .iter()
                    .all(|np| num_sat(np, &sc.lb, &sc.ub, def));
                if pos_ok && num_ok {
                    for &f in &ce.add {
                        let f = f as usize;
                        if !sc.reached[f] {
                            sc.reached[f] = true;
                            sc.fact_layer[f] = layer + 1;
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
            if !task.cond.slice(oi).is_empty() {
                sc.cond_ops.push(oi as u32);
            }
        }

        layer += 1;
        if !changed || layer > LAYER_CAP {
            break;
        }
    }
}

/// Relaxed-plan heuristic toward an ARBITRARY (sub)goal, using reusable `sc`.
/// None == dead end. This is the subplanner heuristic SGPlan-style partitioning
/// drives with per-subproblem goals.
pub fn relaxed_to(
    task: &PackedTask,
    sc: &mut Scratch,
    bits: &[u64],
    fv: &[f64],
    def: &[bool],
    goal_pos: &[u32],
    goal_num: &[NumPre],
) -> Option<i32> {
    sc.reset(task, bits, fv);

    build_rpg(task, sc, goal_pos, goal_num, def, false);

    if !goal_done(goal_pos, goal_num, &sc.reached, &sc.lb, &sc.ub, def) {
        return None;
    }

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
    let h = relaxed_to(task, sc, bits, fv, def, goal_pos, goal_num)?;
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
    Some(cost)
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
    let mut best: Option<usize> = None;
    for (ci, ce) in task.cond.slice(oi).iter().enumerate() {
        if ce.add.iter().any(|&x| x as usize == f) {
            let cl = ce
                .cond_pos
                .iter()
                .map(|&c| sc.fact_layer[c as usize])
                .max()
                .unwrap_or(0);
            if cl != INF && cl < best_layer {
                best_layer = cl;
                best = Some(ci);
            }
        }
    }
    if let Some(ci) = best {
        for &cf in &task.cond.slice(oi)[ci].cond_pos {
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
        _ => return None,
    };
    let want = match &np.rhs {
        NExpr::Num(n) => *n,
        _ => return None,
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
