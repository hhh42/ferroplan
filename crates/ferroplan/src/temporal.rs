//! PDDL2.1 temporal planning — durative actions (EPIC-Temporal).
//!
//! T2 (this module's [`compile`]): each `:durative-action` is split into two
//! instantaneous CLASSICAL actions so the existing grounder/heuristic can be
//! reused. `A-START` takes the action's `at start` conditions as its
//! precondition and applies its `at start` effects plus a `(RUNNING-A ?params)`
//! token; `A-END` requires the `at end` conditions and that token, applies the
//! `at end` effects, and deletes the token.
//!
//! The `over all` invariant and the duration are not expressible in classical
//! STRIPS, so they are kept in a side table ([`SnapInfo`]) that the decision-epoch
//! temporal search (T3) consumes: it only lets `A-END` fire `duration` after the
//! matching `A-START`, and checks the invariant holds across the interval.

use crate::types::{
    eval_numpre, Action, AssignOp, CompOp, Domain, Duration, Effect, Expr, Formula, NExpr, NumEff,
    NumPre, Problem, Sym, Term, TimeSpec,
};

/// Temporal metadata for one durative action, paired with its snap-actions.
#[derive(Clone, Debug)]
pub struct SnapInfo {
    /// Name of the generated start snap-action (e.g. `MOVE-START`).
    pub start_action: Sym,
    /// Name of the generated end snap-action (e.g. `MOVE-END`).
    pub end_action: Sym,
    /// `RUNNING-…` token predicate that pairs a start with its end.
    pub running_pred: Sym,
    /// Duration constraint (fixed `=` or an inequality range) over the action's
    /// parameters / fluents.
    pub duration: Duration,
    /// `over all` invariant that must hold across the action's execution.
    pub invariant: Formula,
    /// The action's typed parameters (for grounding the duration/invariant).
    pub params: Vec<(Sym, Sym)>,
}

/// The result of compiling durative actions to classical snap-actions.
pub struct TemporalCompiled {
    /// Domain with `durative_actions` replaced by classical start/end actions.
    pub domain: Domain,
    pub problem: Problem,
    /// One entry per original durative action.
    pub snaps: Vec<SnapInfo>,
    /// Timed initial literals as `(absolute time, synthetic applier action name)`.
    /// Each name is a 0-arg classical action added to `domain` whose effect asserts /
    /// retracts the literal; the search fires it from the agenda at `time`.
    pub til_ops: Vec<(f64, Sym)>,
}

/// Does this domain use durative actions (i.e. is it a temporal problem)?
pub fn is_temporal(domain: &Domain) -> bool {
    !domain.durative_actions.is_empty()
}

fn and_formulas(parts: Vec<Formula>) -> Formula {
    match parts.len() {
        0 => Formula::True,
        1 => parts.into_iter().next().unwrap(),
        _ => Formula::And(parts),
    }
}

fn and_effects(mut parts: Vec<Effect>) -> Effect {
    if parts.len() == 1 {
        parts.pop().unwrap()
    } else {
        Effect::And(parts)
    }
}

fn pick_conditions(da: &crate::types::DurativeAction, when: TimeSpec) -> Formula {
    and_formulas(
        da.conditions
            .iter()
            .filter(|(t, _)| *t == when)
            .map(|(_, f)| f.clone())
            .collect(),
    )
}

fn pick_effects(da: &crate::types::DurativeAction, when: TimeSpec) -> Vec<Effect> {
    da.effects
        .iter()
        .filter(|(t, _)| *t == when)
        .map(|(_, e)| e.clone())
        .collect()
}

/// Compile a temporal domain (durative actions) into a classical domain of
/// snap-actions plus the [`SnapInfo`] side table.
/// Does this expression reference the `?duration` pseudo-fluent?
fn expr_has_duration(e: &Expr) -> bool {
    match e {
        Expr::Num(_) => false,
        Expr::Fluent(f, _) => f == crate::types::DURATION_PSEUDO,
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
            expr_has_duration(a) || expr_has_duration(b)
        }
        Expr::Neg(a) => expr_has_duration(a),
    }
}

/// Substitute the `?duration` pseudo-fluent with the action's duration
/// expression (PDDL2.1 duration-dependent effects/conditions).
fn expr_subst_duration(e: &Expr, dur: &Expr) -> Expr {
    match e {
        Expr::Num(n) => Expr::Num(*n),
        Expr::Fluent(f, _) if f == crate::types::DURATION_PSEUDO => dur.clone(),
        Expr::Fluent(f, a) => Expr::Fluent(f.clone(), a.clone()),
        Expr::Add(a, b) => Expr::Add(
            Box::new(expr_subst_duration(a, dur)),
            Box::new(expr_subst_duration(b, dur)),
        ),
        Expr::Sub(a, b) => Expr::Sub(
            Box::new(expr_subst_duration(a, dur)),
            Box::new(expr_subst_duration(b, dur)),
        ),
        Expr::Mul(a, b) => Expr::Mul(
            Box::new(expr_subst_duration(a, dur)),
            Box::new(expr_subst_duration(b, dur)),
        ),
        Expr::Div(a, b) => Expr::Div(
            Box::new(expr_subst_duration(a, dur)),
            Box::new(expr_subst_duration(b, dur)),
        ),
        Expr::Neg(a) => Expr::Neg(Box::new(expr_subst_duration(a, dur))),
    }
}

fn formula_map_exprs(f: &Formula, m: &impl Fn(&Expr) -> Expr) -> Formula {
    match f {
        Formula::Comp(op, l, r) => Formula::Comp(*op, m(l), m(r)),
        Formula::And(v) => Formula::And(v.iter().map(|x| formula_map_exprs(x, m)).collect()),
        Formula::Or(v) => Formula::Or(v.iter().map(|x| formula_map_exprs(x, m)).collect()),
        Formula::Not(inner) => Formula::Not(Box::new(formula_map_exprs(inner, m))),
        Formula::Forall(vs, inner) => {
            Formula::Forall(vs.clone(), Box::new(formula_map_exprs(inner, m)))
        }
        Formula::Exists(vs, inner) => {
            Formula::Exists(vs.clone(), Box::new(formula_map_exprs(inner, m)))
        }
        Formula::Pref(n, inner) => Formula::Pref(n.clone(), Box::new(formula_map_exprs(inner, m))),
        other => other.clone(),
    }
}

fn effect_map_exprs(e: &Effect, m: &impl Fn(&Expr) -> Expr) -> Effect {
    match e {
        Effect::Num(op, f, a, v) => Effect::Num(*op, f.clone(), a.clone(), m(v)),
        Effect::And(v) => Effect::And(v.iter().map(|x| effect_map_exprs(x, m)).collect()),
        Effect::When(c, inner) => Effect::When(
            formula_map_exprs(c, m),
            Box::new(effect_map_exprs(inner, m)),
        ),
        Effect::Forall(vs, inner) => {
            Effect::Forall(vs.clone(), Box::new(effect_map_exprs(inner, m)))
        }
        other => other.clone(),
    }
}

fn formula_has_duration(f: &Formula) -> bool {
    match f {
        Formula::Comp(_, l, r) => expr_has_duration(l) || expr_has_duration(r),
        Formula::And(v) | Formula::Or(v) => v.iter().any(formula_has_duration),
        Formula::Not(inner) | Formula::Forall(_, inner) | Formula::Exists(_, inner) => {
            formula_has_duration(inner)
        }
        Formula::Pref(_, inner) => formula_has_duration(inner),
        _ => false,
    }
}

fn effect_has_duration(e: &Effect) -> bool {
    match e {
        Effect::Num(_, _, _, v) => expr_has_duration(v),
        Effect::And(v) => v.iter().any(effect_has_duration),
        Effect::When(c, inner) => formula_has_duration(c) || effect_has_duration(inner),
        Effect::Forall(_, inner) => effect_has_duration(inner),
        _ => false,
    }
}

/// Fluent NAMES any action (classical or durative, any time spec) assigns —
/// the name-level over-approximation behind the end-side `?duration` scope
/// check in [`compile`].
fn assigned_fluent_names(domain: &Domain) -> HashSet<&Sym> {
    fn rec<'a>(e: &'a Effect, out: &mut HashSet<&'a Sym>) {
        match e {
            Effect::Num(_, f, _, _) => {
                out.insert(f);
            }
            Effect::And(v) => v.iter().for_each(|x| rec(x, out)),
            Effect::When(_, inner) | Effect::Forall(_, inner) => rec(inner, out),
            _ => {}
        }
    }
    let mut out = HashSet::new();
    for a in &domain.actions {
        rec(&a.effect, &mut out);
    }
    for da in &domain.durative_actions {
        for e in &da.effects {
            rec(&e.1, &mut out);
        }
    }
    out
}

/// Does the duration expression read any assigned (dynamic) fluent name?
fn duration_reads_assigned(e: &Expr, assigned: &HashSet<&Sym>) -> bool {
    match e {
        Expr::Num(_) => false,
        Expr::Fluent(f, _) => assigned.contains(f),
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
            duration_reads_assigned(a, assigned) || duration_reads_assigned(b, assigned)
        }
        Expr::Neg(a) => duration_reads_assigned(a, assigned),
    }
}

pub fn compile(domain: &Domain, problem: &Problem) -> TemporalCompiled {
    let mut d = domain.clone();
    let mut snaps = Vec::new();

    for da in &domain.durative_actions {
        let running = format!("RUNNING-{}", da.name);
        let start_name = format!("{}-START", da.name);
        let end_name = format!("{}-END", da.name);
        let run_args: Vec<Term> = da
            .params
            .iter()
            .map(|(p, _)| Term::Var(p.clone()))
            .collect();
        let run_types: Vec<Sym> = da.params.iter().map(|(_, t)| t.clone()).collect();

        d.predicates.push((running.clone(), run_types));
        let invariant = pick_conditions(da, TimeSpec::All);

        // PDDL2.1 `?duration` inside conditions/effects: substitute the
        // action's duration expression (the parser emits the `?DURATION`
        // pseudo-fluent). At-START substitution is exact — the effect/
        // condition evaluates against the same state the duration was fixed
        // in. At-END (or invariant) references are only exact when the
        // duration reads no fluent any action assigns (else intervening
        // effects could change the read between START and END); actions
        // outside that scope are SKIPPED — never compiled wrong.
        let start_cond0 = pick_conditions(da, TimeSpec::Start);
        let start_effs0 = pick_effects(da, TimeSpec::Start);
        let end_cond0 = pick_conditions(da, TimeSpec::End);
        let end_effs0 = pick_effects(da, TimeSpec::End);
        let uses_dur = |f: &Formula, effs: &[Effect]| {
            formula_has_duration(f) || effs.iter().any(effect_has_duration)
        };
        let start_uses = uses_dur(&start_cond0, &start_effs0);
        let end_uses = uses_dur(&end_cond0, &end_effs0) || formula_has_duration(&invariant);
        let dur_expr = if start_uses || end_uses {
            match da.duration.chosen() {
                Some(e) if !expr_has_duration(e) => {
                    if end_uses {
                        let assigned: HashSet<&Sym> = assigned_fluent_names(domain);
                        if duration_reads_assigned(e, &assigned) {
                            continue; // end-side `?duration` over a dynamic read: unsupported
                        }
                    }
                    Some(e.clone())
                }
                _ => continue, // `?duration` used but no usable duration bound
            }
        } else {
            None
        };
        let subst_f = |f: &Formula| match &dur_expr {
            Some(dexp) => formula_map_exprs(f, &|e| expr_subst_duration(e, dexp)),
            None => f.clone(),
        };
        let subst_e = |eff: &Effect| match &dur_expr {
            Some(dexp) => effect_map_exprs(eff, &|e| expr_subst_duration(e, dexp)),
            None => eff.clone(),
        };
        let invariant = subst_f(&invariant);

        // start snap: (at-start conditions + invariant) -> at-start effects + token.
        // The invariant is also checked at both endpoints. Endpoint checks alone
        // were UNSOUND — a delete + re-add BETWEEN the endpoints (kiln-gap
        // fixture) passed both and failed VAL; the search's per-happening
        // transition guard ([`InvMap`], `inv_ok`) closes that for conjunctive
        // propositional invariants (0.14) and numeric comparison conjuncts
        // (0.15, fuel-gap fixture — only actual true→false flips block).
        let start_pre = and_formulas(vec![subst_f(&start_cond0), invariant.clone()]);
        let mut start_eff: Vec<Effect> = start_effs0.iter().map(&subst_e).collect();
        start_eff.push(Effect::Add(running.clone(), run_args.clone()));
        d.actions.push(Action {
            name: start_name.clone(),
            params: da.params.clone(),
            precond: start_pre,
            effect: and_effects(start_eff),
            monitored: false,
        });

        // end snap: (at-end conditions + invariant + token) -> at-end effects, drop token
        let end_pre = and_formulas(vec![
            subst_f(&end_cond0),
            invariant.clone(),
            Formula::Atom(running.clone(), run_args.clone()),
        ]);
        let mut end_eff: Vec<Effect> = end_effs0.iter().map(&subst_e).collect();
        end_eff.push(Effect::Del(running.clone(), run_args.clone()));
        d.actions.push(Action {
            name: end_name.clone(),
            params: da.params.clone(),
            precond: end_pre,
            effect: and_effects(end_eff),
            monitored: false,
        });

        snaps.push(SnapInfo {
            start_action: start_name,
            end_action: end_name,
            running_pred: running,
            duration: da.duration.clone(),
            invariant,
            params: da.params.clone(),
        });
    }

    d.durative_actions.clear(); // now expressed as classical snap-actions

    // Timed initial literals → one synthetic 0-arg classical action each (precond
    // True, effect asserts/retracts the literal). This (a) registers the literal's
    // fact with the grounder and (b) makes a positive TIL relaxed-reachable, so a goal
    // achievable only via a TIL isn't pruned as a dead end. The search never *starts*
    // these (classified `Kind::Til`); it fires them from a pre-seeded agenda at `time`.
    let mut til_ops = Vec::new();
    for (k, t) in problem.til.iter().enumerate() {
        let name = format!("TIL-{k}");
        let args: Vec<Term> = t.args.iter().map(|a| Term::Const(a.clone())).collect();
        let eff = if t.add {
            Effect::Add(t.pred.clone(), args)
        } else {
            Effect::Del(t.pred.clone(), args)
        };
        d.actions.push(Action {
            name: name.clone(),
            params: Vec::new(),
            precond: Formula::True,
            effect: eff,
            monitored: false,
        });
        til_ops.push((t.time, name));
    }

    TemporalCompiled {
        domain: d,
        problem: problem.clone(),
        snaps,
        til_ops,
    }
}

// ---------------------------------------------------------------------------
// T3: decision-epoch temporal search.
// ---------------------------------------------------------------------------

use crate::features::DemandMode;
use crate::ground::{ground_stratified, Outcome};
use crate::hash::FxHashMap;
use crate::heuristic::{relaxed_helpful, relaxed_to, Scratch};
use crate::packed::{PackedTask, State, StateKey};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// One action in a timed plan (a durative action is one step with its duration;
/// the end snap is implied).
#[derive(Clone, Debug)]
pub struct TimedStep {
    pub time: f64,
    pub action: String,
    pub duration: Option<f64>,
}

/// A timed (temporal) plan.
#[derive(Clone, Debug)]
pub struct TimedPlan {
    pub steps: Vec<TimedStep>,
    pub makespan: f64,
}

impl TimedPlan {
    /// Render in the IPC temporal plan format: `t: (action args) [duration]`.
    pub fn to_ipc(&self) -> String {
        let mut s = String::new();
        for step in &self.steps {
            s.push_str(&format!(
                "{:.3}: ({}) [{:.3}]\n",
                step.time,
                step.action.to_lowercase(),
                step.duration.unwrap_or(0.001),
            ));
        }
        s
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Kind {
    /// durative start: resolved duration (constant or parameter-dependent) + the
    /// matching end op index
    Start {
        dur: f64,
        end_op: usize,
        /// `u32::MAX` = fixed `dur` (the historical init-resolved path);
        /// otherwise an index into the state-dependent duration table
        /// (`build_kind`'s second return), evaluated per expansion.
        dexp: u32,
    },
    End,
    Classical,
    /// a synthetic timed-initial-literal applier: never started by the search (block
    /// (a) skips it), only fired from the pre-seeded agenda at its absolute time.
    Til,
    /// a start whose duration/end can't be resolved (undefined duration fluent,
    /// non-positive value, or missing end op); never applied
    Skip,
}

struct TNode {
    state: State,
    time: f64,
    /// pending ends as (absolute_end_time, end_op), kept sorted ascending.
    agenda: Vec<(f64, usize)>,
    father: usize,
    /// (op applied, time) that produced this node; None for the root.
    ev: Option<(usize, f64)>,
    /// number of happenings to reach this node (depth `g`, for the heap key).
    g: u32,
    /// FF helpful start/classical ops for this state under pruning (empty = no
    /// restriction / fall back to a full scan). Only populated in the pruned pass.
    helpful: Vec<u32>,
    /// Cumulative-availability per demand resource (init stock + everything produced
    /// along this path, clamped to the demand). Empty unless FF_TDEMAND is on. Tracks
    /// production rather than current stock so the gradient survives consumption.
    met: Vec<i32>,
    /// Fact landmarks accepted on the path to this node (bitset over the
    /// landmark list index) — the temporal LAMA term (0.11 Phase 1). Empty
    /// outside the pruned pass / under FF_NO_TLAMA.
    lm_accepted: Vec<u64>,
}

/// Mark landmarks true in `state` as accepted (the `lama.rs` shape).
fn lm_accept_into(accepted: &mut [u64], lms: &[u32], state: &State) {
    for (i, &f) in lms.iter().enumerate() {
        if accepted[i >> 6] & (1 << (i & 63)) == 0 && crate::bitset::test(&state.bits, f as usize) {
            accepted[i >> 6] |= 1 << (i & 63);
        }
    }
}

fn lm_unaccepted(accepted: &[u64], n: usize) -> i64 {
    n as i64 - accepted.iter().map(|w| w.count_ones() as i64).sum::<i64>()
}

/// Visited key. `relative` keys the agenda by (end − node.time) deltas
/// instead of absolute end times: on a TIL-free task the transition system
/// is SHIFT-INVARIANT (durations read fluents, never the clock; the goal is
/// state-only), so two nodes with equal state and equal pending-end deltas
/// have identical futures — absolute times only retime the plan. Absolute
/// keys made every retimed permutation a "new" state (turn-and-open stored
/// 175k+ nodes on a ~1k-fact instance). With TILs the clock is semantic
/// (a future TIL fires at an absolute time), so the caller passes
/// `relative = til_events.is_empty()`; `FF_TEMPORAL_ABS_KEY=1` restores
/// absolute keys everywhere.
fn tkey(
    task: &PackedTask,
    n: &TNode,
    relative: bool,
    orbit: Option<&crate::orbits::OrbitMap>,
) -> (StateKey, Vec<(i64, usize)>) {
    let base = if relative { n.time } else { 0.0 };
    let ag: Vec<(i64, usize)> = n
        .agenda
        .iter()
        .map(|&(t, o)| (((t - base) * 1000.0).round() as i64, o))
        .collect();
    match orbit {
        // Orbit canonicalization (0.14 ext Phase 10): states differing only
        // by a permutation of interchangeable members share one key.
        Some(om) => om.canonical_key(task, &n.state, &ag),
        None => (task.state_key(&n.state), ag),
    }
}

/// Evaluate a (possibly parameter-dependent) duration for one grounded
/// snap-action. The action's parameters are bound positionally to the grounded
/// args; fluents are read from the INITIAL state — IPC temporal durations depend
/// on static fluents like `(= ?duration (/ (distance ?a ?b) (speed ?v)))`, which
/// keep their init value. Returns None for a non-positive duration, an undefined
/// fluent, or division by zero (the caller then skips the action).
fn eval_duration(snap: &SnapInfo, args: &[&str], task: &PackedTask, init: &State) -> Option<f64> {
    let bind = duration_bind(snap, args);
    // Commit to the shortest feasible duration (the lower bound; the upper bound only
    // for a sole `<=`). Inequality slack is given up here in exchange for a single
    // resolved duration the decision-epoch search can schedule — see `validate`, which
    // accepts the whole `[min, max]` range.
    let d = eval_expr(snap.duration.chosen()?, &bind, task, init)?;
    if d.is_finite() && d > 0.0 {
        Some(d)
    } else {
        None
    }
}

/// Evaluate the `[min, max]` duration bounds against the initial state (for the
/// validator). An open side stays `None` (unbounded). A bound that fails to evaluate
/// (undefined fluent, div-by-zero) also yields `None` for that side.
fn eval_duration_bounds(
    snap: &SnapInfo,
    args: &[&str],
    task: &PackedTask,
    init: &State,
) -> (Option<f64>, Option<f64>) {
    let bind = duration_bind(snap, args);
    let ev = |o: &Option<Expr>| o.as_ref().and_then(|e| eval_expr(e, &bind, task, init));
    (ev(&snap.duration.min), ev(&snap.duration.max))
}

/// Bind a snap-action's parameters positionally to the grounded args.
fn duration_bind<'a>(snap: &'a SnapInfo, args: &[&'a str]) -> HashMap<&'a str, &'a str> {
    snap.params
        .iter()
        .map(|(p, _)| p.as_str())
        .zip(args.iter().copied())
        .collect()
}

fn eval_expr(e: &Expr, bind: &HashMap<&str, &str>, task: &PackedTask, init: &State) -> Option<f64> {
    match e {
        Expr::Num(n) => Some(*n),
        Expr::Fluent(name, terms) => {
            let mut disp = String::from("(");
            disp.push_str(name);
            for t in terms {
                disp.push(' ');
                match t {
                    Term::Const(c) => disp.push_str(c),
                    Term::Var(v) => disp.push_str(bind.get(v.as_str())?),
                }
            }
            disp.push(')');
            let id = task.fluent_id(&disp)?;
            init.fdef[id].then(|| init.fv[id])
        }
        Expr::Add(a, b) => Some(eval_expr(a, bind, task, init)? + eval_expr(b, bind, task, init)?),
        Expr::Sub(a, b) => Some(eval_expr(a, bind, task, init)? - eval_expr(b, bind, task, init)?),
        Expr::Mul(a, b) => Some(eval_expr(a, bind, task, init)? * eval_expr(b, bind, task, init)?),
        Expr::Div(a, b) => {
            let d = eval_expr(b, bind, task, init)?;
            if d == 0.0 {
                return None;
            }
            Some(eval_expr(a, bind, task, init)? / d)
        }
        Expr::Neg(a) => Some(-eval_expr(a, bind, task, init)?),
    }
}

/// Solve a temporal (durative-action) problem by decision-epoch forward search.
/// Returns a timed plan, or None if unsolved within the node budget. Durations
/// may be constants or parameter-dependent (evaluated against the initial state);
/// the `over all` invariant is enforced at the endpoints via the snap
/// preconditions AND on every happening in between via the grounded
/// invariant transition guard (a delete + re-add between the endpoints
/// used to slip through — the kiln-gap fixture pins the fix).
pub fn solve(domain: &Domain, problem: &Problem, threads: usize) -> Option<TimedPlan> {
    let ambient = crate::features::demand_mode();
    if let Some(plan) = solve_monolithic(domain, problem, threads, ambient) {
        return Some(plan);
    }
    // On-failure escalation ladder (see `features::escalate`). Each rung runs only
    // after the previous failed, so nothing that solves above can change — the
    // ladder converts failures into solves at the cost of extra time on failures.
    // Gated off by FF_NO_ESCALATE, and by FF_NO_TDEMAND (the master pre-v0.2
    // opt-out — escalating from `Off` would contradict it). Measured (cabin):
    // crew-solo/pair + skilled-specialists solve at the Full rung; order-8/12
    // solve at the decomposer rung.
    if ambient == DemandMode::Off || !crate::features::escalate() {
        return None;
    }
    if ambient != DemandMode::Full {
        if let Some(plan) = solve_monolithic(domain, problem, threads, DemandMode::Full) {
            return Some(plan);
        }
    }
    // Decomposer rung — the ladder variant, which skips the decomposer's own
    // monolithic fallbacks (this ladder already ran that exact search at both
    // tiers) and thus also cannot recurse. Passes this ladder's own rung-0 tier
    // so the premise can't drift with the process-global override.
    crate::tresolve::solve_after_ladder(domain, problem, threads, ambient)
}

/// The monolithic temporal search at an explicit demand `tier` — the scheduling
/// phase + the plain decision-epoch search, WITHOUT the escalation ladder. This is
/// the primitive `solve` builds its ladder from and the decomposer's single-group
/// fallback (`tresolve`) terminates on.
pub(crate) fn solve_monolithic(
    domain: &Domain,
    problem: &Problem,
    threads: usize,
    tier: DemandMode,
) -> Option<TimedPlan> {
    // Concurrent scheduling phase (gated). The multi-actor search is flaky, so we
    // search a SINGLE-actor reduction (tractable) and then repack that plan onto the
    // full crew — one job per worker, resources permitting — to minimise makespan.
    // Validated + only-if-shorter inside `reschedule`, so it can only improve things;
    // if the reduction finds nothing we fall through to a normal solve.
    if crate::features::tconc() {
        // ≥2 actors ⇒ the reduction is a *super-worker* (all skills), so its plan is
        // only valid for `problem` once reassigned to real skilled workers; <2 ⇒ the
        // reduction is `problem` itself, so its plan is valid as-is.
        let reduced = crate::tsched::n_actors(domain, problem) >= 2;
        let solo = crate::tsched::single_actor_problem(domain, problem);
        if let Some(plan) = solve_inner(domain, &solo, threads, tier) {
            if let Some(rp) = crate::tsched::reschedule(domain, problem, &plan) {
                return Some(rp);
            }
            if !reduced {
                return Some(plan);
            }
            // reduced but couldn't validly reschedule (e.g. a task needs a skill no
            // single worker has) — fall through to an honest full-problem search.
        }
    }
    solve_inner(domain, problem, threads, tier)
}

/// Search a temporal plan for `problem` as-is (no scheduling phase).
fn solve_inner(
    domain: &Domain,
    problem: &Problem,
    threads: usize,
    tier: DemandMode,
) -> Option<TimedPlan> {
    let c = compile(domain, problem);
    let task = match ground_stratified(&c.domain, &c.problem, threads) {
        Outcome::Task(t) => t,
        Outcome::GoalTrue => {
            return Some(TimedPlan {
                steps: Vec::new(),
                makespan: 0.0,
            })
        }
        _ => return None,
    };

    let (kind, dur_exprs, inv) = build_kind(&task, &c);
    // Resolve each TIL's synthetic applier to its grounded op id (0-arg ⇒ op display
    // is the action name). A TIL whose op didn't ground is silently dropped.
    let by_display: HashMap<&str, usize> = task
        .op_display
        .iter()
        .enumerate()
        .map(|(i, d)| (d.as_str(), i))
        .collect();
    let til_events: Vec<(f64, usize)> = c
        .til_ops
        .iter()
        .filter_map(|(t, name)| by_display.get(name.as_str()).map(|&oi| (*t, oi)))
        .collect();

    // Object-symmetry orbits (0.14 ext Phase 10): detected against the COMPILED
    // lifted pair (op displays are snap-action names). None = no usable symmetry.
    let orbit = crate::orbits::detect(&c.domain, &c.problem, &task);

    // FF_TEVAL_BUDGET caps search evaluations — the deterministic measuring
    // stick for A/B probes (eval budgets, never wall clock). Default unlimited.
    let cap: usize = std::env::var("FF_TEVAL_BUDGET")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    let mut budget = cap;
    let r = solve_from_seeded_orbit(
        &task,
        &kind,
        &dur_exprs,
        &inv,
        &task.initial(),
        &task.goal_pos,
        &task.goal_num,
        &[],
        &til_events,
        threads,
        tier,
        &mut budget,
        crate::search::NODE_CAP_TARGET_BYTES,
        false,
        orbit.as_ref(),
    );
    if std::env::var("FF_ORBIT_DEBUG").is_ok() {
        eprintln!(
            "orbit: solve_inner evals {} solved {}",
            cap - budget,
            r.is_some()
        );
    }
    r
}

/// Grounded `over all` invariant facts per END op id: (positive, negative)
/// atoms that must hold strictly INSIDE the interval — the search refuses
/// any happening that deletes a positive (or adds a negative) one while
/// the interval is pending (kiln-gap fixture: endpoint-only checking
/// accepted a bake spanning a delete+re-add outage; VAL rejects it).
/// Ops with non-conjunctive invariants are absent (endpoint-only, as
/// before). Numeric conjuncts (0.15 Phase 2, the fuel-gap fixture) carry
/// their grounded comparison plus the fluent ids it reads: a happening
/// that changes a read fluent re-evaluates the comparison on the
/// post-happening state, and only an actual true→false flip blocks —
/// a fuel decrease that stays above its floor sails through.
pub(crate) type InvMap =
    crate::hash::FxHashMap<usize, (Vec<u32>, Vec<u32>, Vec<(NumPre, Vec<u32>)>)>;

/// Classify every grounded op as a durative Start (with resolved duration + paired
/// end op), End, Classical, or Skip (unresolvable). Shared by `solve` and the
/// decomposer (`tresolve`), built once per grounded task. The third result
/// is the [`InvMap`] of grounded interval invariants.
pub(crate) fn build_kind(
    task: &PackedTask,
    c: &TemporalCompiled,
) -> (Vec<Kind>, Vec<NExpr>, InvMap) {
    // Durations are constant or parameter-dependent. A duration reading only
    // UNMODIFIED fluents is resolved once against the initial state (the
    // historical path, bit-identical). One reading a fluent some op assigns
    // (model-train's `(- (tail-segment-position ?pred) (head-segment-position
    // ?t))`) is STATE-DEPENDENT: its grounded `NExpr` goes in the side table
    // and the search evaluates it per expansion against the node's state.
    let init = task.initial();
    let modified = modified_fluents(task);
    let snap_by_start: HashMap<&str, &SnapInfo> = c
        .snaps
        .iter()
        .map(|s| (s.start_action.as_str(), s))
        .collect();
    let end_names: HashSet<&str> = c.snaps.iter().map(|s| s.end_action.as_str()).collect();
    let til_names: HashSet<&str> = c.til_ops.iter().map(|(_, n)| n.as_str()).collect();
    let by_display: HashMap<&str, usize> = task
        .op_display
        .iter()
        .enumerate()
        .map(|(i, d)| (d.as_str(), i))
        .collect();
    let mut dur_exprs: Vec<NExpr> = Vec::new();
    let mut inv: InvMap = InvMap::default();
    let kinds = (0..task.n_ops)
        .map(|oi| {
            let disp = &task.op_display[oi];
            let head = disp.split_whitespace().next().unwrap_or("");
            if let Some(snap) = snap_by_start.get(head) {
                let args: Vec<&str> = disp.split_whitespace().skip(1).collect();
                let end_disp = disp.replacen("-START", "-END", 1);
                let end_op = match by_display.get(end_disp.as_str()) {
                    Some(&e) => e,
                    None => return Kind::Skip,
                };
                if !matches!(snap.invariant, Formula::True) {
                    let bind = duration_bind(snap, &args);
                    let mut pos = Vec::new();
                    let mut neg = Vec::new();
                    let mut num = Vec::new();
                    if ground_inv(
                        &snap.invariant,
                        &bind,
                        task,
                        &modified,
                        &mut pos,
                        &mut neg,
                        &mut num,
                    ) && !(pos.is_empty() && neg.is_empty() && num.is_empty())
                    {
                        inv.insert(end_op, (pos, neg, num));
                    }
                }
                let nexpr = snap
                    .duration
                    .chosen()
                    .and_then(|e| ground_duration_nexpr(e, &duration_bind(snap, &args), task));
                let state_dep = nexpr.as_ref().is_some_and(|ne| {
                    let mut v = Vec::new();
                    ne.collect_fluents(&mut v);
                    v.iter().any(|&f| modified[f as usize])
                });
                if state_dep {
                    let idx = dur_exprs.len() as u32;
                    dur_exprs.push(nexpr.unwrap());
                    // dur is unused for state-dependent starts (resolved per
                    // expansion); no init positivity gate — it may only
                    // become positive later.
                    Kind::Start {
                        dur: 0.0,
                        end_op,
                        dexp: idx,
                    }
                } else {
                    match eval_duration(snap, &args, task, &init) {
                        Some(dur) => Kind::Start {
                            dur,
                            end_op,
                            dexp: u32::MAX,
                        },
                        None => Kind::Skip,
                    }
                }
            } else if end_names.contains(head) {
                Kind::End
            } else if til_names.contains(head) {
                Kind::Til
            } else {
                Kind::Classical
            }
        })
        .collect();
    (kinds, dur_exprs, inv)
}

/// Collect a conjunctive invariant's grounded (positive, negative) fact
/// atoms and numeric comparisons. `true` = the shape is supported (static
/// `Eq` conjuncts are passed over; a numeric conjunct whose expressions
/// don't ground makes the whole op fall back to endpoint-only); `false`
/// = disjunctive/quantified structure, caller keeps endpoint-only checking
/// for the whole op. An atom that grounded to no task fact is skipped:
/// statically-true facts have no deleter, never-reachable ones no adder.
fn ground_inv(
    f: &Formula,
    bind: &HashMap<&str, &str>,
    task: &PackedTask,
    modified: &[bool],
    pos: &mut Vec<u32>,
    neg: &mut Vec<u32>,
    num: &mut Vec<(NumPre, Vec<u32>)>,
) -> bool {
    match f {
        Formula::True | Formula::Eq(..) => true,
        Formula::Comp(op, lhs, rhs) => {
            let (Some(l), Some(r)) = (
                ground_duration_nexpr(lhs, bind, task),
                ground_duration_nexpr(rhs, bind, task),
            ) else {
                return false;
            };
            let np = NumPre {
                op: *op,
                lhs: l,
                rhs: r,
            };
            let mut reads = Vec::new();
            np.lhs.collect_fluents(&mut reads);
            np.rhs.collect_fluents(&mut reads);
            // A comparison over UNWRITTEN fluents can never flip — skip it
            // (start_pre / end_pre already check it at the endpoints).
            if reads.iter().any(|&f| modified[f as usize]) {
                num.push((np, reads));
            }
            true
        }
        Formula::And(fs) => fs
            .iter()
            .all(|g| ground_inv(g, bind, task, modified, pos, neg, num)),
        Formula::Atom(p, args) => {
            if let Some(fid) = ground_atom_id(p, args, bind, task) {
                pos.push(fid);
            }
            true
        }
        Formula::Not(g) => match &**g {
            Formula::Atom(p, args) => {
                if let Some(fid) = ground_atom_id(p, args, bind, task) {
                    neg.push(fid);
                }
                true
            }
            _ => false,
        },
        _ => false,
    }
}

fn ground_atom_id(
    p: &crate::types::Sym,
    args: &[Term],
    bind: &HashMap<&str, &str>,
    task: &PackedTask,
) -> Option<u32> {
    let mut disp = String::from("(");
    disp.push_str(p);
    for t in args {
        disp.push(' ');
        match t {
            Term::Const(c) => disp.push_str(c),
            Term::Var(v) => disp.push_str(bind.get(v.as_str())?),
        }
    }
    disp.push(')');
    task.fact_id(&disp).map(|x| x as u32)
}

/// Fluents any op assigns (numeric effects, incl. conditional) — a duration
/// reading one is STATE-DEPENDENT (resolved per expansion / at start time),
/// not fixable against init. Shared by `build_kind` and `validate`.
fn modified_fluents(task: &PackedTask) -> Vec<bool> {
    let mut modified = vec![false; task.fv0.len()];
    for oi in 0..task.n_ops {
        for ne in task.num_eff.slice(oi) {
            modified[ne.target as usize] = true;
        }
        for ce in task.cond_effs(oi) {
            for ne in &ce.num {
                modified[ne.target as usize] = true;
            }
        }
    }
    modified
}

/// Ground a duration expression to a fluent-id `NExpr` (params bound
/// positionally). `None` if a referenced fluent didn't ground.
fn ground_duration_nexpr(e: &Expr, bind: &HashMap<&str, &str>, task: &PackedTask) -> Option<NExpr> {
    Some(match e {
        Expr::Num(n) => NExpr::Num(*n),
        Expr::Fluent(name, terms) => {
            let mut disp = String::from("(");
            disp.push_str(name);
            for t in terms {
                disp.push(' ');
                match t {
                    Term::Const(c) => disp.push_str(c),
                    Term::Var(v) => disp.push_str(bind.get(v.as_str())?),
                }
            }
            disp.push(')');
            NExpr::Fluent(task.fluent_id(&disp)? as u32)
        }
        Expr::Add(a, b) => NExpr::Add(
            Box::new(ground_duration_nexpr(a, bind, task)?),
            Box::new(ground_duration_nexpr(b, bind, task)?),
        ),
        Expr::Sub(a, b) => NExpr::Sub(
            Box::new(ground_duration_nexpr(a, bind, task)?),
            Box::new(ground_duration_nexpr(b, bind, task)?),
        ),
        Expr::Mul(a, b) => NExpr::Mul(
            Box::new(ground_duration_nexpr(a, bind, task)?),
            Box::new(ground_duration_nexpr(b, bind, task)?),
        ),
        Expr::Div(a, b) => NExpr::Div(
            Box::new(ground_duration_nexpr(a, bind, task)?),
            Box::new(ground_duration_nexpr(b, bind, task)?),
        ),
        Expr::Neg(a) => NExpr::Neg(Box::new(ground_duration_nexpr(a, bind, task)?)),
    })
}

/// Goal-relevance op mask (`true` = keep). Backward closure from the goal: an op is
/// relevant if it ADDS or DELETES a relevant fact, or INCREASES a relevant resource
/// (incl. via a conditional effect); marking it pulls its preconditions (positive
/// facts, numeric `>=` thresholds) and consumed resources into the relevant set,
/// transitively. Ops that cannot contribute — e.g. `forage-food`/`gather-herbs` when
/// food/herbs are in no recipe the goal needs: unbounded accumulators that otherwise
/// explode the complete search with food=1,2,3,… states — are dropped. Applied to
/// BOTH phases: phase 1 (helpful) is usually stuck under delete-relaxation anyway;
/// the win is letting the COMPLETE phase 2 solve within the relevant subspace instead
/// of drowning in irrelevant accumulation. Sound (completeness-preserving) because a
/// pruned op neither produces nor consumes nor toggles anything any solution needs —
/// the `del`-of-relevant clause conservatively keeps re-enablers of negative
/// preconditions. Necessary travel is kept: a relevant op's `(at a l)` precond makes
/// the travel achieving it relevant, transitively along the route.
fn relevant_op_mask(
    task: &PackedTask,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    tight: bool,
) -> Vec<bool> {
    let mut rel_fact: crate::hash::FxHashSet<u32> = goal_pos.iter().copied().collect();
    let mut rel_res: crate::hash::FxHashSet<u32> = goal_num
        .iter()
        .filter_map(|np| as_threshold(np).map(|(t, _)| t))
        .collect();
    // TIGHT mode: a resource is "produced" only by its single best-yield producer, so
    // marking (say) `planks` relevant pulls in `saw-planks` but NOT the alternative
    // producer `haul-cargo` — which would otherwise drag the whole logistics subsystem
    // into the relevant set and re-explode. best_end[r] = that producer's op id.
    let best_end: Vec<Option<usize>> = if tight {
        (0..task.fv0.len())
            .map(|r| best_producer(task, r as u32).map(|(o, _)| o))
            .collect()
    } else {
        Vec::new()
    };
    let produces = |oi: usize, t: u32| -> bool {
        !tight || best_end.get(t as usize).copied().flatten() == Some(oi)
    };
    let mut relevant = vec![false; task.n_ops];
    loop {
        let mut changed = false;
        // range loop: the body both reads task slices by `oi` and writes `relevant[oi]`.
        #[allow(clippy::needless_range_loop)]
        for oi in 0..task.n_ops {
            if relevant[oi] {
                continue;
            }
            let touches_fact = task.add.slice(oi).iter().any(|f| rel_fact.contains(f))
                || task.del.slice(oi).iter().any(|f| rel_fact.contains(f));
            let inc_res = task.num_eff.slice(oi).iter().any(|ne| {
                matches!(ne.op, AssignOp::Increase)
                    && rel_res.contains(&ne.target)
                    && produces(oi, ne.target)
            });
            let cond_rel = task.cond_effs(oi).any(|ce| {
                ce.add.iter().any(|f| rel_fact.contains(f))
                    || ce.del.iter().any(|f| rel_fact.contains(f))
                    || ce.num.iter().any(|ne| {
                        matches!(ne.op, AssignOp::Increase) && rel_res.contains(&ne.target)
                    })
            });
            if touches_fact || inc_res || cond_rel {
                relevant[oi] = true;
                changed = true;
                for &f in task.pre_pos.slice(oi) {
                    rel_fact.insert(f);
                }
                for np in task.pre_num.slice(oi) {
                    if let Some((t, _)) = as_threshold(np) {
                        rel_res.insert(t);
                    }
                }
                for ne in task.num_eff.slice(oi) {
                    if matches!(ne.op, AssignOp::Decrease) {
                        rel_res.insert(ne.target);
                    }
                }
                for ce in task.cond_effs(oi) {
                    for &f in &ce.cond_pos {
                        rel_fact.insert(f);
                    }
                    for ne in &ce.num {
                        if matches!(ne.op, AssignOp::Decrease) {
                            rel_res.insert(ne.target);
                        }
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    relevant
}

/// Solve toward an ARBITRARY `(goal_pos, goal_num)` from an arbitrary `start` state
/// over a shared grounded temporal task — the reusable subplanner the decomposer
/// (`tresolve`) calls per contract. `forbidden` masks ops (sibling protection; empty
/// = none). `tier` is the demand tier this pass runs at — threaded explicitly so
/// the escalation ladder can retry at `Full` without touching process-global state.
/// `solve_monolithic` is the whole-task wrapper (start = init, goal = task goal, no
/// forbidden); `temporal::solve` is that plus the on-failure escalation ladder.
///
/// Multi-pass decision-epoch search: a fast pass restricting start/classical
/// expansion to FF helpful actions, then unrestricted complete passes on failure
/// (tight-masked → sound-masked → unmasked; see the pruning block below).
/// Phase-1 key = W_G*g + W_H*h + W_L*(unmet numeric landmarks) + the converging-
/// resource demand deficit; the complete passes use the original pure-h key.
#[allow(clippy::too_many_arguments)]
pub(crate) fn solve_from(
    task: &PackedTask,
    kind: &[Kind],
    dur_exprs: &[NExpr],
    inv: &InvMap,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    forbidden: &[bool],
    til_events: &[(f64, usize)],
    threads: usize,
    tier: DemandMode,
    budget: &mut usize,
    node_bytes: usize,
) -> Option<TimedPlan> {
    solve_from_seeded_orbit(
        task, kind, dur_exprs, inv, start, goal_pos, goal_num, forbidden, til_events, threads,
        tier, budget, node_bytes, false, None,
    )
}

/// [`solve_from`] with `seed_til_h`: seed every node's heuristic state with
/// the ADD effects of its still-pending TIL events, so an outage the agenda
/// will REPAIR does not read as a relaxed dead end (a think can wait
/// through it — 0.14 Phase 3). Session-only: the CLI/corpus paths pass
/// `false` and stay byte-identical.
#[allow(clippy::too_many_arguments)]
pub(crate) fn solve_from_seeded(
    task: &PackedTask,
    kind: &[Kind],
    dur_exprs: &[NExpr],
    inv: &InvMap,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    forbidden: &[bool],
    til_events: &[(f64, usize)],
    threads: usize,
    tier: DemandMode,
    budget: &mut usize,
    node_bytes: usize,
    seed_til_h: bool,
) -> Option<TimedPlan> {
    solve_from_seeded_orbit(
        task, kind, dur_exprs, inv, start, goal_pos, goal_num, forbidden, til_events, threads,
        tier, budget, node_bytes, seed_til_h, None,
    )
}

/// [`solve_from_seeded`] with orbit canonicalization of the visited key
/// (0.14 ext Phase 10) — passed from callers that hold the LIFTED
/// domain/problem needed for detection.
#[allow(clippy::too_many_arguments)]
pub(crate) fn solve_from_seeded_orbit(
    task: &PackedTask,
    kind: &[Kind],
    dur_exprs: &[NExpr],
    inv: &InvMap,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    forbidden: &[bool],
    til_events: &[(f64, usize)],
    threads: usize,
    tier: DemandMode,
    budget: &mut usize,
    node_bytes: usize,
    seed_til_h: bool,
    orbit: Option<&crate::orbits::OrbitMap>,
) -> Option<TimedPlan> {
    // Fail fast on statically unproducible goals — nothing any pass could reach.
    if statically_unsolvable(task, start, goal_pos, goal_num) {
        return None;
    }
    // Landmarks are ALWAYS on (phase-1 key), so seed them from the numeric goal ONLY
    // — keeping the default path byte-identical. The predicate-goal thresholds (which
    // would change default ordering) ride the FF_TDEMAND-gated demand seed instead.
    let landmarks = extract_landmarks(task, goal_num);
    // Converging-resource demand guidance (FF_TDEMAND, default OFF → empty → the
    // phase-1 key is bit-identical to the prior temporal search). Phase 2 (the
    // complete pure-h pass) is unaffected regardless, so completeness is preserved.
    let demand = if tier != DemandMode::Off {
        let w = std::env::var("FF_TDEMAND_W")
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(3);
        // demand seed = numeric goal (always) + numeric thresholds implied by
        // PREDICATE goals' achievers (Full tier only — so `(built-wall)` drives the
        // blocks>=4 chain). The predicate half is gated off by default because it
        // reads a renewable-pool guard (e.g. `(>= (avail) 1)`) as accumulation demand
        // and serializes concurrency domains; the numeric half is the measured win.
        let mut seed: Vec<NumPre> = goal_num.to_vec();
        if tier == DemandMode::Full {
            seed.extend(predicate_goal_thresholds(task, kind, goal_pos));
        }
        let d = compute_demand(task, kind, &seed, w);
        if std::env::var("FF_RES_DEBUG").is_ok() {
            let pretty: Vec<(String, i32)> = d
                .res
                .iter()
                .map(|&(f, a)| (task.fluent_names[f as usize].clone(), a))
                .collect();
            eprintln!("[TDEMAND] w={w} total={} resources={:?}", d.total, pretty);
        }
        d
    } else {
        Demand::empty()
    };
    // Goal-relevance pruning (default-on with the demand tiers; `FF_NOREL` disables
    // pruning alone, `FF_NO_TDEMAND` restores the pristine pre-v0.2 path entirely).
    // Two masks: SOUND (every producer of each relevant resource) and TIGHT (only the
    // single best-yield producer — drops alternative-recipe subsystems like logistics
    // for `planks`). Four passes: helpful (sound) → full+tight → full+sound →
    // full+unmasked. The tight pass solves the conjunctive/structural builds without
    // exploding; the sound pass solves within the relevant subspace instead of
    // drowning in irrelevant unbounded accumulators (food=1,2,3,…); the final
    // unmasked pass makes completeness UNCONDITIONAL — even a hypothetical mask bug
    // cannot lose coverage, it can only cost time on unsolvable inputs.
    // Graduated from the Full tier in v0.3.0: `flour >= 2` on a fully-featured hub
    // (rpg-world bread-line) needs pruning to solve at all — the default search
    // exhausted its node budget in the irrelevant-accumulator swamp.
    let on = tier != DemandMode::Off && std::env::var("FF_NOREL").is_err();
    let sound = if on {
        relevant_op_mask(task, goal_pos, goal_num, false)
    } else {
        Vec::new()
    };
    let tight = if on {
        relevant_op_mask(task, goal_pos, goal_num, true)
    } else {
        Vec::new()
    };
    if on && std::env::var("FF_RES_DEBUG").is_ok() {
        eprintln!(
            "[TREL] sound {}/{}  tight {}/{}",
            sound.iter().filter(|&&b| b).count(),
            sound.len(),
            tight.iter().filter(|&&b| b).count(),
            tight.len()
        );
    }
    // The budget spans the WHOLE pass ladder (a think bounds everything);
    // RefCell keeps the closure's reborrow simple in serial control flow.
    let budget = std::cell::RefCell::new(budget);
    let go = |rel: &[bool], prune: bool, tlama: bool| {
        temporal_search(
            task,
            kind,
            dur_exprs,
            inv,
            &landmarks,
            &demand,
            start,
            goal_pos,
            goal_num,
            forbidden,
            rel,
            til_events,
            prune,
            tlama,
            threads,
            &mut budget.borrow_mut(),
            node_bytes,
            seed_til_h,
            orbit,
        )
    };
    go(&sound, true, false)
        // The TLAMA rung (0.11 Phase 1) — MEASURED NEGATIVE, opt-in via
        // FF_TLAMA=1. Three variants, none positive: the key-term mixed
        // into the pruned pass fought h (crew 50/50 → 36/50); the unbounded
        // rung taxed the wall (sokoban-t −3); bounded at 50k nodes it
        // yielded zero new coverage anywhere. The recorded diagnosis: snap
        // tasks' fact landmarks are dominated by RUNNING-token chains that
        // accept in path order REGARDLESS of choices, so the unaccepted
        // count carries almost no branching signal on these walls — unlike
        // barman's classical landmarks, which order deep resource chains.
        .or_else(|| {
            if std::env::var("FF_TLAMA").is_ok() {
                go(&sound, true, true)
            } else {
                None
            }
        })
        .or_else(|| if on { go(&tight, false, false) } else { None })
        .or_else(|| go(&sound, false, false))
        // Unmasked complete backstop — only distinct from the previous pass when
        // pruning is on (off ⇒ `sound` is already empty ⇒ pass 3 was unmasked).
        .or_else(|| if on { go(&[], false, false) } else { None })
}

/// Static unproducibility: is some goal conjunct impossible to ever achieve because
/// NOTHING in the grounded task can move it? Two sound, instant checks:
/// - a positive goal fact not true in `start` that **no op adds** (plain or
///   conditional effect — TIL appliers are ops too, so exogenous adds count);
/// - a `>=`/`>` numeric threshold not met in `start` whose fluent **no op can
///   possibly raise** (an effect "can raise" unless it provably never does:
///   `increase` by a constant ≤ 0 or `decrease` by a constant ≥ 0; `assign`/
///   `scale-up`/`scale-down`/non-constant deltas all count as potential raisers).
///
/// A `true` here means every search pass would exhaust its budget and fail anyway —
/// e.g. rpg-world `bread-line` goals on `(bread) >= 2` but no action produces
/// `bread`, and the search burned ~45 s across passes proving it. This check makes
/// such instances (and decomposer contracts) fail in microseconds. It never changes
/// a *found* plan, only converts an exhaustive failure into an instant one.
pub(crate) fn statically_unsolvable(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
) -> bool {
    let fact_true = |f: u32| (start.bits[f as usize / 64] >> (f as usize % 64)) & 1 == 1;
    let never_raises = |ne: &NumEff| match (&ne.op, &ne.value) {
        (AssignOp::Increase, NExpr::Num(w)) => w <= &0.0,
        (AssignOp::Decrease, NExpr::Num(w)) => w >= &0.0,
        _ => false,
    };
    let some_op_adds = |g: u32| {
        (0..task.n_ops).any(|oi| {
            task.add.slice(oi).contains(&g) || task.cond_effs(oi).any(|ce| ce.add.contains(&g))
        })
    };
    let some_op_raises = |t: u32| {
        (0..task.n_ops).any(|oi| {
            task.num_eff
                .slice(oi)
                .iter()
                .any(|ne| ne.target == t && !never_raises(ne))
                || task
                    .cond
                    .slice(oi)
                    .iter()
                    .any(|ce| ce.num.iter().any(|ne| ne.target == t && !never_raises(ne)))
        })
    };
    for &g in goal_pos {
        if !fact_true(g) && !some_op_adds(g) {
            return true;
        }
    }
    for np in goal_num {
        if let Some((t, _)) = as_threshold(np) {
            let already = eval_numpre(np, &start.fv, &start.fdef) == Some(true);
            if !already && !some_op_raises(t) {
                return true;
            }
        }
    }
    false
}

/// A numeric `>=`/`>` threshold `(fluent, value)`, or `None` if `np` isn't of that
/// canonical recipe-gate shape.
fn as_threshold(np: &NumPre) -> Option<(u32, f64)> {
    match (&np.op, &np.lhs, &np.rhs) {
        (CompOp::Ge | CompOp::Gt, NExpr::Fluent(t), NExpr::Num(w)) => Some((*t, *w)),
        _ => None,
    }
}

/// Numeric `>=` thresholds implied by the achievers of the PREDICATE goal facts: for
/// each goal fact, the op that adds it (the END snap) gates on numeric preconditions
/// that live on the matching START snap — bridge END->START via the RUNNING token
/// exactly as `extract_landmarks` does, and collect those `>=` preconds. Lets a
/// predicate goal like `(built-wall)` seed the `blocks>=4` demand chain (Stage 0).
fn predicate_goal_thresholds(task: &PackedTask, kind: &[Kind], goal_pos: &[u32]) -> Vec<NumPre> {
    let mut out: Vec<NumPre> = Vec::new();
    let collect_thr = |oi: usize, out: &mut Vec<NumPre>| {
        for pre in task.pre_num.slice(oi) {
            if as_threshold(pre).is_some() {
                out.push(pre.clone());
            }
        }
    };
    for &gf in goal_pos {
        for &oi in task.add_by_fact.slice(gf as usize) {
            let oi = oi as usize;
            collect_thr(oi, &mut out); // classical / direct numeric precond
            for &f in task.pre_pos.slice(oi) {
                for &start in task.add_by_fact.slice(f as usize) {
                    if matches!(kind[start as usize], Kind::Start { .. }) {
                        collect_thr(start as usize, &mut out); // bridged START precond
                    }
                }
            }
        }
    }
    out
}

/// Numeric-threshold landmarks: the transitive closure of the `>=` preconditions of
/// the ops that *increase* each goal fluent. The delete-relaxed extraction drops
/// these (it never recurses on `pre_num`), so on a converging DAG — where two inputs
/// are separate numeric thresholds feeding one join — `h` goes flat. Counting how
/// many a state has NOT met gives each converging input its own descending term in
/// the phase-1 key, restoring the gradient the FF count lacks.
fn extract_landmarks(task: &PackedTask, seed: &[NumPre]) -> Vec<NumPre> {
    let mut out: Vec<NumPre> = Vec::new();
    let mut seen: HashSet<(u32, u64)> = HashSet::new();
    let mut work: Vec<NumPre> = seed.to_vec();
    let mut iters = 0usize;
    while let Some(np) = work.pop() {
        iters += 1;
        if iters > 8000 {
            break; // safety cap against accumulator cycles
        }
        let Some((t, w)) = as_threshold(&np) else {
            continue;
        };
        if !seen.insert((t, w.to_bits())) {
            continue;
        }
        out.push(np.clone());
        let add_pre_num = |oi: usize, work: &mut Vec<NumPre>| {
            for pre in task.pre_num.slice(oi) {
                if as_threshold(pre).is_some() {
                    work.push(pre.clone());
                }
            }
        };
        // recurse toward the recipe inputs of ops that INCREASE fluent `t`.
        for &oi in task.neff_by_fluent.slice(t as usize) {
            let oi = oi as usize;
            let increases = task
                .num_eff
                .slice(oi)
                .iter()
                .any(|ne| ne.target == t && matches!(ne.op, AssignOp::Increase));
            if !increases {
                continue;
            }
            // (a) classical case: numeric preconds sit on the increasing op itself.
            add_pre_num(oi, &mut work);
            // (b) snap-compiled case: the increase is on the END snap, but the
            // recipe's numeric inputs are on the matching START snap — bridge via the
            // RUNNING token (END requires it, START adds it).
            for &f in task.pre_pos.slice(oi) {
                for &start in task.add_by_fact.slice(f as usize) {
                    add_pre_num(start as usize, &mut work);
                }
            }
        }
    }
    out
}

/// Summed DEFICIT of the landmark thresholds in `(fv, fdef)` — for each `(fluent >=
/// want)` landmark, how far below `want` the fluent is. Unlike a binary met/unmet
/// count this gives a gradient across MULTIPLE rounds (e.g. steel>=2 descends 2→1→0),
/// so deep/wide converging accumulation gets guidance, not just single assembly.
fn landmark_deficit(landmarks: &[NumPre], fv: &[f64], fdef: &[bool]) -> i64 {
    landmarks
        .iter()
        .map(|np| match as_threshold(np) {
            Some((t, want)) => {
                let cur = if fdef[t as usize] {
                    fv[t as usize]
                } else {
                    0.0
                };
                (want - cur).max(0.0).ceil() as i64
            }
            None => 0,
        })
        .sum()
}

/// Total resource DEMAND implied by the numeric goal, regressed down the recipe
/// DAG. A `(fluent >= want)` goal needs `want` of that fluent; its best (highest-
/// yield) producer needs `ceil(want / yield)` applications, each consuming its
/// inputs — recurse. Unlike the per-recipe landmark thresholds (`ingots >= 1`), this
/// captures the FULL multi-round quantity (`steel >= 2` ⇒ ingots/coal/ore ≥ 2,
/// logs ≥ 4) that the delete-relaxed heuristic — which reuses each consumed unit —
/// never demands. `weight` 0 / empty `res` ⇒ the whole term is inert (heap key
/// bit-identical), so the default temporal path is unchanged.
struct Demand {
    res: Vec<(u32, i32)>,
    idx: FxHashMap<u32, usize>,
    total: i32,
    weight: i64,
}

impl Demand {
    fn empty() -> Self {
        Demand {
            res: Vec::new(),
            idx: FxHashMap::default(),
            total: 0,
            weight: 0,
        }
    }
}

/// The highest base-yield op that increases fluent `t` (raw resources have a gather
/// producer with no numeric inputs, so regression bottoms out there). Conditional
/// (role-bonus) increases are ignored — the base yield is the safe estimate.
fn best_producer(task: &PackedTask, t: u32) -> Option<(usize, i32)> {
    let mut best: Option<(usize, i32)> = None;
    for &oi in task.neff_by_fluent.slice(t as usize) {
        let oi = oi as usize;
        for ne in task.num_eff.slice(oi) {
            if ne.target == t && matches!(ne.op, AssignOp::Increase) {
                let y = ne.value.eval(&task.fv0, &task.fdef0).unwrap_or(0.0);
                if y > 0.0 {
                    let yi = y.ceil() as i32;
                    if best.map_or(true, |(_, by)| yi > by) {
                        best = Some((oi, yi));
                    }
                }
            }
        }
    }
    best
}

fn compute_demand(task: &PackedTask, kind: &[Kind], seed: &[NumPre], weight: i64) -> Demand {
    use crate::hash::FxHashSet;
    const MAX_ITERS: usize = 20_000;
    const CAP: i32 = 100_000; // guard against cyclic/regenerating recipe blowup
    let mut need: FxHashMap<u32, i32> = FxHashMap::default();
    let mut work: Vec<(u32, i32)> = seed
        .iter()
        .filter_map(|np| as_threshold(np).map(|(t, w)| (t, w.ceil().max(0.0) as i32)))
        .collect();
    let mut iters = 0usize;
    while let Some((t, amt)) = work.pop() {
        iters += 1;
        if iters > MAX_ITERS {
            break;
        }
        if amt <= 0 {
            continue;
        }
        let cur = need.entry(t).or_insert(0);
        let target = (*cur + amt).min(CAP);
        let delta = target - *cur; // only propagate the marginal new demand
        if delta <= 0 {
            continue;
        }
        *cur = target;
        let Some((oi, yield_t)) = best_producer(task, t) else {
            continue; // raw resource — bottoms out (stays in `need`)
        };
        let apps = (delta + yield_t - 1) / yield_t; // ceil
                                                    // Inputs = the producer's own decreases PLUS, for snap-compiled durative
                                                    // recipes, the matching START snap's decreases (the increase is on the END
                                                    // snap; the consume is on the START that adds a RUNNING token END requires).
                                                    // Bridge exactly as extract_landmarks does, filtering adders to START snaps.
        let mut consumers: FxHashSet<usize> = FxHashSet::default();
        consumers.insert(oi);
        for &f in task.pre_pos.slice(oi) {
            for &start in task.add_by_fact.slice(f as usize) {
                if matches!(kind[start as usize], Kind::Start { .. }) {
                    consumers.insert(start as usize);
                }
            }
        }
        for op in consumers {
            for ne in task.num_eff.slice(op) {
                if matches!(ne.op, AssignOp::Decrease) {
                    let c = ne.value.eval(&task.fv0, &task.fdef0).unwrap_or(0.0);
                    if c > 0.0 {
                        work.push((ne.target, apps.saturating_mul(c.ceil() as i32)));
                    }
                }
            }
        }
    }
    let mut res: Vec<(u32, i32)> = need.into_iter().collect();
    res.sort_unstable(); // deterministic order, independent of hashmap iteration
    let mut idx = FxHashMap::default();
    let mut total = 0i32;
    for (i, &(f, a)) in res.iter().enumerate() {
        idx.insert(f, i);
        total += a;
    }
    Demand {
        res,
        idx,
        total,
        weight,
    }
}

/// The set of resources in the demand-closure of `goal_num` (the recipe inputs that
/// producing the goal consumes, transitively) — used by the decomposer to order
/// contracts so a goal that is itself an input to another goal is produced LAST.
pub(crate) fn demand_resources(task: &PackedTask, kind: &[Kind], goal_num: &[NumPre]) -> Vec<u32> {
    compute_demand(task, kind, goal_num, 1)
        .res
        .into_iter()
        .map(|(f, _)| f)
        .collect()
}

/// Root availability: initial stock of each demand resource, clamped to its demand.
fn met_root(demand: &Demand, task: &PackedTask) -> Vec<i32> {
    demand
        .res
        .iter()
        .map(|&(f, a)| {
            let cur = if task.fdef0[f as usize] {
                task.fv0[f as usize]
            } else {
                0.0
            };
            (cur.max(0.0) as i32).min(a)
        })
        .collect()
}

/// Child availability: parent's, plus op `oi`'s (unconditional) increases on demand
/// resources, clamped. Production-only (consumption never lowers it) — so a consumed
/// intermediate still counts as delivered, the key to the multi-round gradient.
fn met_child(parent: &[i32], demand: &Demand, task: &PackedTask, oi: usize) -> Vec<i32> {
    if demand.res.is_empty() {
        return Vec::new();
    }
    let mut m = parent.to_vec();
    for ne in task.num_eff.slice(oi) {
        if matches!(ne.op, AssignOp::Increase) {
            if let Some(&i) = demand.idx.get(&ne.target) {
                let v = ne.value.eval(&task.fv0, &task.fdef0).unwrap_or(0.0);
                if v > 0.0 {
                    m[i] = (m[i] + v.ceil() as i32).min(demand.res[i].1);
                }
            }
        }
    }
    m
}

#[inline]
fn demand_deficit(met: &[i32], demand: &Demand) -> i64 {
    (demand.total - met.iter().sum::<i32>()) as i64
}

/// `h`, plus — under `prune` — the Skip-filtered helpful start/classical ops for
/// `s`. `None` iff `s` is a relaxed dead end (so this also gates dead ends).
#[allow(clippy::too_many_arguments)]
fn eval_node(
    task: &PackedTask,
    kind: &[Kind],
    sc: &mut Scratch,
    s: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    prune: bool,
) -> Option<(i32, Vec<u32>)> {
    if prune {
        let (h, helpful) = relaxed_helpful(task, sc, &s.bits, &s.fv, &s.fdef, goal_pos, goal_num)?;
        let mut hf: Vec<u32> = helpful
            .into_iter()
            .filter(|&oi| matches!(kind[oi as usize], Kind::Start { .. } | Kind::Classical))
            .collect();
        // Drift repair (0.11 Phase 2) — MEASURED NEGATIVE, opt-in via
        // FF_LAX_HELPFUL=1. The mechanism was real (the Start-only filter
        // empties a nonempty set when relaxed plans lead through
        // agenda-fired ENDs; storage's stored helpful averaged 0.0) but the
        // repair RESTRICTS block (a) where the empty set previously meant a
        // FULL SCAN — zero new solves anywhere, sokoban-t −3 (the full scan
        // was finding what the lax set misses). Restriction and repair
        // pull opposite ways here; recorded in roadmap-0.11.
        if hf.is_empty() && std::env::var("FF_LAX_HELPFUL").is_ok() {
            hf = crate::heuristic::helpful_needed_adders(task, sc, &s.bits, &s.fv, &s.fdef)
                .into_iter()
                .filter(|&oi| matches!(kind[oi as usize], Kind::Start { .. } | Kind::Classical))
                .collect();
        }
        Some((h, hf))
    } else {
        // relaxed_to with the task goal == the old `relaxed`; with a subgoal it
        // targets the contract (used by solve_from). Byte-identical for the default.
        let h = relaxed_to(task, sc, &s.bits, &s.fv, &s.fdef, goal_pos, goal_num)?;
        Some((h, Vec::new()))
    }
}

/// Dedup and enqueue a candidate node whose heuristic `(h, helpful)` is already
/// computed. Both the serial path ([`push_node`]) and the batched parallel path
/// funnel through this on ONE thread, in input order — that serial funnel is what
/// keeps parallel evaluation byte-identical to the sequential search.
#[allow(clippy::too_many_arguments)]
fn enqueue_evaluated(
    orbit: Option<&crate::orbits::OrbitMap>,
    task: &PackedTask,
    nodes: &mut Vec<TNode>,
    heap: &mut BinaryHeap<Reverse<(i64, usize)>>,
    visited: &mut HashSet<(StateKey, Vec<(i64, usize)>)>,
    landmarks: &[NumPre],
    lms: &[u32],
    demand: &Demand,
    prune: bool,
    relative: bool,
    n: TNode,
    h: i32,
    helpful: Vec<u32>,
) {
    let k = tkey(task, &n, relative, orbit);
    if visited.insert(k) {
        enqueue_committed(
            task, nodes, heap, landmarks, lms, demand, prune, n, h, helpful,
        );
    }
}

/// [`enqueue_evaluated`] after the visited check — callers on the orbit
/// path dedup BEFORE paying for the heuristic (a canonical key is ~4x
/// cheaper than an eval on machine-shop-sized tasks) and land here.
#[allow(clippy::too_many_arguments)]
fn enqueue_committed(
    task: &PackedTask,
    nodes: &mut Vec<TNode>,
    heap: &mut BinaryHeap<Reverse<(i64, usize)>>,
    landmarks: &[NumPre],
    lms: &[u32],
    demand: &Demand,
    prune: bool,
    mut n: TNode,
    h: i32,
    helpful: Vec<u32>,
) {
    // Gentle h-weight (1g + 3h, vs the classical 1g+5h) keeps required-concurrency
    // branches in contention; the unit g breaks the flat-h plateau on long chains;
    // W_L counts unmet numeric-threshold landmarks, restoring a gradient on
    // converging DAGs (where the FF count goes flat). AGENDA_W is 0: penalizing open
    // intervals suppresses the very parallelism we want — keep it off.
    const W_G: i64 = 1;
    const W_H: i64 = 3;
    const W_L: i64 = 3;
    // Pruned-pass agenda term (0.15 Phase 1 probe, FF_TAGENDA_W_PRUNE=<w>):
    // the start-credit counter-account. A start drops h by ~1 the moment it
    // fires (its snap leaves the relaxed plan) while delivering nothing
    // until its END lands — with w == W_H the credit cancels at start and
    // pays at the end instead (key = g + 3·(h + agenda)), which is exactly
    // the accounting TMS's start-spam floor (best_h pinned at 110 across a
    // 13x budget ladder) says is missing. The historical AGENDA_W=0 stays
    // the default.
    let agenda_w: i64 = std::env::var("FF_TAGENDA_W_PRUNE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    n.helpful = helpful;
    // Cumulative-availability for the demand term: parent's plus this op's
    // production. Empty (no-op) unless FF_TDEMAND is on.
    let op = n.ev.map(|(o, _)| o).unwrap_or(usize::MAX);
    n.met = if op == usize::MAX {
        nodes[n.father].met.clone()
    } else {
        met_child(&nodes[n.father].met, demand, task, op)
    };
    // Temporal LAMA (0.11 Phase 1): landmarks accepted along the path.
    if !lms.is_empty() {
        n.lm_accepted = nodes[n.father].lm_accepted.clone();
        lm_accept_into(&mut n.lm_accepted, lms, &n.state);
    }
    // Phase 1 (prune): weighted g+h plus the unmet-landmark term AND the
    // total converging-resource demand deficit (the multi-round gradient the
    // relaxation is blind to), to break the flat-h plateau on long chains AND
    // converging DAGs. Phase 2 (full): the ORIGINAL pure-h key — byte-for-byte
    // the old complete search, so nothing it solved before can regress.
    // The TLAMA rung's key is LANDMARK-DOMINANT (the lama.rs shape:
    // unaccepted count outweighs h), not a term mixed into the pruned
    // key — see the measured lesson at the `lms` computation.
    const W_TLM: i64 = 4;
    let key = if !lms.is_empty() {
        W_G * n.g as i64
            + W_TLM * lm_unaccepted(&n.lm_accepted, lms.len()) * (W_H + 1)
            + W_H * h as i64
    } else if prune {
        W_G * n.g as i64
            + W_H * h as i64
            + W_L * landmark_deficit(landmarks, &n.state.fv, &n.state.fdef)
            + demand.weight * demand_deficit(&n.met, demand)
            + agenda_w * n.agenda.len() as i64
    } else {
        // Complete-pass agenda ordering (0.12 Phase 4 experiment,
        // FF_TAGENDA_W=<w>, default off): parc-printer-t's complete pass
        // drowns in start-spam (avg ~2,076 pending intervals per node) —
        // an ordering term de-prioritizes interval hoarding WITHOUT
        // losing completeness (ordering, never pruning). The recorded
        // AGENDA_W=0 verdict was for the PRUNED pass's key.
        let wa = std::env::var("FF_TAGENDA_W")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);
        h as i64 + wa * n.agenda.len() as i64
    };
    let idx = nodes.len();
    nodes.push(n);
    // Tie-break probe (0.15 Phase 1, FF_TLIFO=1): at EQUAL key the min-heap
    // pops the smallest index — FIFO, i.e. breadth-first across a plateau.
    // TMS's best_h sits flat at 110 while the frontier balloons; LIFO
    // commits depth-first through the plateau instead. Deterministic
    // either way (pure function of insertion order).
    let tie = if std::env::var("FF_TLIFO").is_ok() {
        usize::MAX - idx
    } else {
        idx
    };
    heap.push(Reverse((key, tie)));
}

/// May a happening move the state `old`→`new` while `pending` intervals
/// run? False iff the transition deletes a positive (or adds a negative)
/// `over all` invariant fact of some pending interval other than `skip`
/// (the agenda index being fired — its own interval closes AT this
/// happening, and end effects may touch their own invariant). Diff-based,
/// so conditional effects are exact. Invariant facts of pending intervals
/// are true by induction (their starts required them, every later
/// happening was vetted here), so `old`-true + `new`-false is precisely
/// "this happening broke it".
/// Per-op static write-set pre-filter for the transition guard (0.15
/// Phase 6): an op whose effects — unconditional, conditional, and the
/// shared monitor block alike — cannot touch any fact some invariant
/// watches (`prop`) or any fluent some numeric conjunct reads (`num`)
/// can never break a running interval, so [`inv_ok`] skips its pending
/// scan entirely. Built once per pass; both vectors are empty when the
/// task has no invariants (callers gate on `inv.is_empty()` first).
/// Keeps the guard pay-per-threat instead of pay-per-happening when
/// most ops are bystanders to every invariant — by design rather than
/// by luck of the domain's shape.
struct InvTouch {
    prop: Vec<bool>,
    num: Vec<bool>,
}

fn build_inv_touch(task: &PackedTask, inv: &InvMap) -> InvTouch {
    if inv.is_empty() {
        return InvTouch {
            prop: Vec::new(),
            num: Vec::new(),
        };
    }
    let mut watched: crate::hash::FxHashSet<u32> = Default::default();
    let mut reads: crate::hash::FxHashSet<u32> = Default::default();
    for (pos, neg, num) in inv.values() {
        watched.extend(pos.iter().copied());
        watched.extend(neg.iter().copied());
        for (_, r) in num {
            reads.extend(r.iter().copied());
        }
    }
    let mut prop = vec![false; task.n_ops];
    let mut num = vec![false; task.n_ops];
    for oi in 0..task.n_ops {
        let mut p = task
            .add
            .slice(oi)
            .iter()
            .chain(task.del.slice(oi))
            .any(|f| watched.contains(f));
        let mut n = task
            .num_eff
            .slice(oi)
            .iter()
            .any(|ne| reads.contains(&ne.target));
        for ce in task.cond_effs(oi) {
            p = p || ce.add.iter().chain(&ce.del).any(|f| watched.contains(f));
            n = n || ce.num.iter().any(|ne| reads.contains(&ne.target));
        }
        prop[oi] = p;
        num[oi] = n;
    }
    InvTouch { prop, num }
}

fn inv_ok(
    inv: &InvMap,
    touch: &InvTouch,
    applied: usize,
    pending: &[(f64, usize)],
    skip: Option<usize>,
    old: &State,
    new: &State,
) -> bool {
    if inv.is_empty() {
        return true;
    }
    let (chk_prop, chk_num) = (touch.prop[applied], touch.num[applied]);
    if !chk_prop && !chk_num {
        return true;
    }
    for (i, &(_, eop)) in pending.iter().enumerate() {
        if Some(i) == skip {
            continue;
        }
        if let Some((pos, neg, num)) = inv.get(&eop) {
            if chk_prop
                && (pos.iter().any(|&f| {
                    crate::bitset::test(&old.bits, f as usize)
                        && !crate::bitset::test(&new.bits, f as usize)
                }) || neg.iter().any(|&f| {
                    !crate::bitset::test(&old.bits, f as usize)
                        && crate::bitset::test(&new.bits, f as usize)
                }))
            {
                return false;
            }
            // Numeric conjuncts (fuel-gap): only an actual true→false FLIP
            // blocks, and only when a read fluent moved — a drain that
            // stays above its floor is untouched.
            if chk_num {
                for (np, reads) in num {
                    let moved = reads.iter().any(|&f| {
                        let f = f as usize;
                        old.fdef[f] != new.fdef[f] || old.fv[f] != new.fv[f]
                    });
                    if moved
                        && eval_numpre(np, &old.fv, &old.fdef) == Some(true)
                        && eval_numpre(np, &new.fv, &new.fdef) != Some(true)
                    {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// Per-pass probe counters (FF_RES_DEBUG only output; the increments are
/// unconditional u64 adds, cheap enough to keep unguarded). The 0.15
/// Phase 1 measurement eyes: where do the generated candidates actually
/// go — doomed at birth, orbit-deduped before eval, evaluated, relaxed
/// dead ends — and does the pruned pass's best h make progress at all?
#[derive(Default)]
struct TStats {
    doomed: u64,
    deduped: u64,
    evaluated: u64,
    dead_end: u64,
    b_blocked: u64,
    tie_rescue: u64,
    best_h: i32,
}

/// A node whose agenda head can NEVER legally fire is dead: the head must
/// fire for time to advance, but its unconditional deletes (adds) break
/// the over-all invariant of a pending interval that outlives the head's
/// epoch — and no rescue exists: any earlier event that would clear the
/// invariant fact is vetted by the same guard, and the blocker's own end
/// fires only after the head. Pruning the subtree at birth (before paying
/// for its heuristic) is what makes the invariant semantics affordable on
/// machine-shop: every bake overrunning its kiln window dies here instead
/// of spawning a doomed (a)-subtree. Goal states are never pruned — a
/// doomed node has a blocked non-TIL end pending, which the goal test
/// already rejects.
fn doomed(task: &PackedTask, inv: &InvMap, touch: &InvTouch, n: &TNode) -> bool {
    if inv.is_empty() {
        return false;
    }
    let Some(&(te, hop)) = n.agenda.first() else {
        return false;
    };
    // Write-set pre-filter: a head that can't touch any watched fact can't
    // doom anything (touch.prop covers the unconditional dels/adds used
    // below and more — conservative).
    if !touch.prop[hop] {
        return false;
    }
    let hdel = task.del.slice(hop);
    let hadd = task.add.slice(hop);
    if hdel.is_empty() && hadd.is_empty() {
        return false;
    }
    n.agenda.iter().skip(1).any(|&(tb, bop)| {
        tb > te + 1e-9
            && inv.get(&bop).is_some_and(|(pos, neg, _)| {
                hdel.iter()
                    .any(|f| pos.contains(f) && crate::bitset::test(&n.state.bits, *f as usize))
                    || hadd.iter().any(|f| {
                        neg.contains(f) && !crate::bitset::test(&n.state.bits, *f as usize)
                    })
            })
    })
}

/// A node's heuristic state under TIL seeding (0.14 Phase 3): the node's
/// state plus the ADD effects of its still-pending TIL events — an outage
/// the agenda will repair must not read as a relaxed dead end. `None` when
/// seeding is off or nothing on the agenda is a TIL (the common case:
/// byte-identical evaluation).
fn til_seeded_state(
    task: &PackedTask,
    kind: &[Kind],
    agenda: &[(f64, usize)],
    s: &State,
    seed: bool,
) -> Option<State> {
    if !seed {
        return None;
    }
    let mut out: Option<State> = None;
    for &(_, op) in agenda {
        if matches!(kind[op], Kind::Til) && !task.add.slice(op).is_empty() {
            let st = out.get_or_insert_with(|| s.clone());
            for &f in task.add.slice(op) {
                crate::bitset::set(&mut st.bits, f as usize);
            }
        }
    }
    out
}

/// Evaluate, dedup, and enqueue a candidate node with the weighted heap key.
#[allow(clippy::too_many_arguments)]
fn push_node(
    orbit: Option<&crate::orbits::OrbitMap>,
    task: &PackedTask,
    kind: &[Kind],
    inv: &InvMap,
    touch: &InvTouch,
    stats: &mut TStats,
    sc: &mut Scratch,
    nodes: &mut Vec<TNode>,
    heap: &mut BinaryHeap<Reverse<(i64, usize)>>,
    visited: &mut HashSet<(StateKey, Vec<(i64, usize)>)>,
    landmarks: &[NumPre],
    lms: &[u32],
    demand: &Demand,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    prune: bool,
    relative: bool,
    seed_til_h: bool,
    n: TNode,
) {
    if doomed(task, inv, touch, &n) {
        stats.doomed += 1;
        return;
    }
    // Orbit path: dedup on the canonical key BEFORE paying for the
    // heuristic — most successors of a symmetric task are permutations of
    // already-seen states, and a key is ~4x cheaper than an eval. Budget
    // is charged per candidate by the caller either way, so exploration
    // order and t1 ≡ t8 are unchanged; only wall clock improves. (Dead-end
    // evals also become visited here, unlike the no-orbit flow — that path
    // stays byte-identical to the pre-orbit engine.)
    if orbit.is_some() {
        let k = tkey(task, &n, relative, orbit);
        if !visited.insert(k) {
            stats.deduped += 1;
            return;
        }
    }
    let hs = til_seeded_state(task, kind, &n.agenda, &n.state, seed_til_h);
    stats.evaluated += 1;
    let ev = eval_node(
        task,
        kind,
        sc,
        hs.as_ref().unwrap_or(&n.state),
        goal_pos,
        goal_num,
        prune,
    );
    if ev.is_none() {
        stats.dead_end += 1;
    }
    if let Some((h, helpful)) = ev {
        stats.best_h = stats.best_h.min(h);
        if orbit.is_some() {
            enqueue_committed(
                task, nodes, heap, landmarks, lms, demand, prune, n, h, helpful,
            );
        } else {
            enqueue_evaluated(
                orbit, task, nodes, heap, visited, landmarks, lms, demand, prune, relative, n, h,
                helpful,
            );
        }
    }
}

/// Historical per-pass stored-node ceiling — now the COUNT arm of the cap; the
/// byte arm below binds first whenever states are big.
const MAX_NODES: usize = 400_000;

/// Deterministic per-pass node cap: the classical `node_cap_for` byte model
/// extended with the temporal extras — one stored `TNode` (State + agenda) in
/// `nodes` plus one visited key (StateKey bits + relevant fluent vals + agenda
/// copy), and fixed container overhead. Sized from STATIC task dims only, so
/// it is identical across thread counts and runs (an eval-count-style budget,
/// never wall clock). The agenda estimate is TIL count + a small open-interval
/// allowance; `FF_TEMPORAL_NODE_CAP` overrides the count directly (`0`
/// disables). Bounded above by the historical 400k count cap.
fn temporal_node_cap(task: &PackedTask, til_len: usize, bytes: usize) -> usize {
    if let Ok(v) = std::env::var("FF_TEMPORAL_NODE_CAP") {
        if let Ok(n) = v.trim().parse::<usize>() {
            return if n == 0 { usize::MAX } else { n };
        }
    }
    let agenda_est = til_len + 8;
    let per_node = 2 * task.words * 8
        + task.fv0.len() * 8
        + task.fdef0.len()
        + task.rel_fluents.len() * 8
        + 2 * agenda_est * 16
        + 160;
    (bytes / per_node.max(1)).min(MAX_NODES)
}

/// One decision-epoch search pass. `prune` restricts block-(a) expansion to the
/// node's helpful ops (with a per-node full-scan fallback so no node with a legal
/// successor is stranded); `false` is the full, complete search.
#[allow(clippy::too_many_arguments)]
fn temporal_search(
    task: &PackedTask,
    kind: &[Kind],
    dur_exprs: &[NExpr],
    inv: &InvMap,
    landmarks: &[NumPre],
    demand: &Demand,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    forbidden: &[bool],
    relevant: &[bool],
    til_events: &[(f64, usize)],
    prune: bool,
    tlama: bool,
    threads: usize,
    budget: &mut usize,
    node_bytes: usize,
    seed_til_h: bool,
    orbit: Option<&crate::orbits::OrbitMap>,
) -> Option<TimedPlan> {
    // The TLAMA rung is a BOUNDED bet, like the classical ladder's 400k-eval
    // LAMA cap: a failed rung must cost seconds, not the wall — unbounded it
    // burned a full node-cap slice and pushed sokoban-t's complete-pass
    // solves past the 30 s budget (10→8/30, 2→1/20).
    const TLAMA_NODE_CAP: usize = 50_000;
    let max_nodes = if tlama {
        temporal_node_cap(task, til_events.len(), node_bytes).min(TLAMA_NODE_CAP)
    } else {
        temporal_node_cap(task, til_events.len(), node_bytes)
    };
    // Measurement only (FF_RES_DEBUG): dims at pass start, container sizes every
    // 25k stored nodes — the memory-attribution eyes for the temporal path.
    let dbg = std::env::var("FF_RES_DEBUG").is_ok();
    let orbit_gen = std::env::var("FF_ORBIT_GEN").is_ok();
    let lifo = std::env::var("FF_TLIFO").is_ok();
    let tb_free_g = std::env::var("FF_TB_FREE_G").is_ok();
    let t0 = crate::clock::Clock::now();
    if dbg {
        eprintln!(
            "[tsearch] pass start: prune={prune} masked={} words={} fv={} rel_fluents={} tils={} ops={}",
            !relevant.is_empty(),
            task.words,
            task.fv0.len(),
            task.rel_fluents.len(),
            til_events.len(),
            task.n_ops
        );
    }
    let mut next_dump = 25_000usize;
    let mut stats = TStats {
        best_h: i32::MAX,
        ..TStats::default()
    };
    // Per-op write-set pre-filter for the transition guard — one linear
    // scan of the effect tables per pass buys a constant-time bystander
    // exit in every `inv_ok`/`doomed` call below.
    let touch = build_inv_touch(task, inv);
    // Worker count for batched successor evaluation (0 = auto, like the classical
    // search). Parallelism only changes evaluation COST — see the funnel comment in
    // the expansion block; plans are identical for any value.
    let workers = if threads == 0 {
        crate::par::num_threads()
    } else {
        threads
    };
    // Root from the (possibly mid-composition) START state, but always at clock 0
    // with an agenda holding only the timed initial literals (sorted ascending): a
    // contract is solved as a fresh interval and drains its agenda before returning,
    // so it never inherits a parent's running durations.
    let init = start.clone();
    let mut sc = Scratch::new(task);

    let root_seed = til_seeded_state(task, kind, til_events, &init, seed_til_h);
    let (_h0, hf0) = eval_node(
        task,
        kind,
        &mut sc,
        root_seed.as_ref().unwrap_or(&init),
        goal_pos,
        goal_num,
        prune,
    )?; // dead-end gate
        // TMS symmetry reduction (0.13 Phase 5): keep the agenda sorted by
        // (time, op id) — CANONICAL, not arrival-ordered. N same-epoch starts of
        // interchangeable intervals (machine-shop's kilns, printer sheets) used
        // to reach the same pending MULTISET through N! arrival orders, and the
        // visited key (which contains the agenda) stored every one of them as a
        // distinct state: copies, not classes. With a canonical order, one node
        // represents the class; simultaneous ends fire in op-id order (one valid
        // serialization — symmetric ends touch different tokens and commute).
        // `FF_NO_TSYMM=1` restores arrival order.
    let symm = std::env::var("FF_NO_TSYMM").is_err();
    let mut root_agenda: Vec<(f64, usize)> = til_events.to_vec();
    root_agenda.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Shift-invariant dedup (see tkey): sound only when no TIL pins the clock.
    let relative = til_events.is_empty() && std::env::var("FF_TEMPORAL_ABS_KEY").is_err();
    // Temporal LAMA (0.11 Phase 1): fact landmarks over the snap task drive
    // a DEDICATED rung's key (landmark-dominant, LAMA-style) — measured
    // lesson: mixed into the pruned pass's key the gradient FIGHTS h where
    // they disagree (crew-planning 50/50 → 36/50, sokoban-t/floor-tile-t
    // −6), exactly why classical LAMA is a separate rung and not a term.
    // Only the `tlama` pass computes or sees them; every other pass keys as
    // 0.10 did, bit-for-bit.
    let lms: Vec<u32> = if tlama {
        crate::landmarks::landmarks_for(task, start, goal_pos)
    } else {
        Vec::new()
    };
    let lm_words = lms.len().div_ceil(64);
    if dbg && tlama {
        eprintln!("[tsearch] tlama: {} fact landmarks", lms.len());
    }
    let mut root_lm = vec![0u64; lm_words];
    if !lms.is_empty() {
        lm_accept_into(&mut root_lm, &lms, start);
    }
    let mut nodes = vec![TNode {
        state: init,
        time: 0.0,
        agenda: root_agenda,
        father: usize::MAX,
        ev: None,
        g: 0,
        helpful: hf0,
        met: met_root(demand, task),
        lm_accepted: root_lm,
    }];
    let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    heap.push(Reverse((0, if lifo { usize::MAX } else { 0 })));
    let mut visited: HashSet<(StateKey, Vec<(i64, usize)>)> = HashSet::new();
    visited.insert(tkey(task, &nodes[0], relative, orbit));

    while let Some(Reverse((_k, tie))) = heap.pop() {
        // Decode the FF_TLIFO tie encoding (see enqueue_committed).
        let ni = if lifo { usize::MAX - tie } else { tie };
        // The goal is reached once no *action* end is still pending. Unfired future
        // TILs may remain on the agenda — they're exogenous and don't gate completion.
        let ends_pending = nodes[ni]
            .agenda
            .iter()
            .any(|&(_, op)| !matches!(kind[op], Kind::Til));
        if task.goal_met_with(&nodes[ni].state, goal_pos, goal_num) && !ends_pending {
            let plan = reconstruct(task, &nodes, ni, kind, dur_exprs);
            return Some(epsilon_separate(task, plan, !til_events.is_empty()));
        }
        if dbg && nodes.len() >= next_dump {
            next_dump += 25_000;
            let ag: usize = nodes.iter().map(|n| n.agenda.len()).sum();
            let hf: usize = nodes.iter().map(|n| n.helpful.len()).sum();
            eprintln!(
                "[tsearch] nodes {} heap {} visited {} avg_agenda {:.1} avg_helpful {:.1} {}ms",
                nodes.len(),
                heap.len(),
                visited.len(),
                ag as f64 / nodes.len() as f64,
                hf as f64 / nodes.len() as f64,
                t0.elapsed_ms()
            );
            // What is the frontier hoarding? The POPPED node's pending ends,
            // op-grouped (measurement eyes for the start-spam diagnosis).
            let mut by_op: HashMap<usize, usize> = HashMap::new();
            for &(_, o) in &nodes[ni].agenda {
                *by_op.entry(o).or_default() += 1;
            }
            let mut counts: Vec<(usize, usize)> = by_op.into_iter().collect();
            counts.sort_by_key(|&(o, c)| (std::cmp::Reverse(c), o));
            let head: Vec<String> = counts
                .iter()
                .take(4)
                .map(|&(o, c)| format!("{}x {}", c, task.op_display[o]))
                .collect();
            eprintln!(
                "[tsearch]   popped: time {:.1} g {} agenda {} [{}]",
                nodes[ni].time,
                nodes[ni].g,
                nodes[ni].agenda.len(),
                head.join(", ")
            );
        }
        if nodes.len() > max_nodes || *budget == 0 {
            if dbg {
                eprintln!(
                    "[tsearch] cap hit (nodes {} / max {max_nodes}, budget left {budget}) at {}ms",
                    nodes.len(),
                    t0.elapsed_ms()
                );
                eprintln!(
                    "[tsearch] stats: doomed {} deduped {} evaluated {} dead_end {} b_blocked {} tie_rescue {} best_h {}",
                    stats.doomed, stats.deduped, stats.evaluated, stats.dead_end,
                    stats.b_blocked, stats.tie_rescue, stats.best_h
                );
            }
            break;
        }
        let time = nodes[ni].time;
        let pg = nodes[ni].g;

        // (a) start a durative action / apply a classical action — restricted to
        // the node's helpful set under pruning (else a full scan), minus any
        // forbidden ops (sibling protection; forbidding a START suffices).
        // forbidden (sibling protection) + goal-relevance pruning, both phases.
        // Empty relevance mask = keep all (default path). Sound: a non-relevant op
        // can't be on any path to this goal, so phase 2 stays complete on the
        // relevant subspace.
        let allow = |oi: usize| {
            !forbidden.get(oi).copied().unwrap_or(false)
                && (relevant.is_empty() || relevant.get(oi).copied().unwrap_or(true))
        };
        let candidates: Vec<usize> = if prune && !nodes[ni].helpful.is_empty() {
            nodes[ni]
                .helpful
                .iter()
                .map(|&o| o as usize)
                .filter(|&oi| allow(oi))
                .collect()
        } else {
            (0..task.n_ops).filter(|&oi| allow(oi)).collect()
        };
        // Successor prototypes first (cheap state application), heuristics second —
        // batched across worker threads when the frontier is big enough, then
        // funneled through `enqueue_evaluated` serially IN INPUT ORDER. The funnel
        // order matches the old per-candidate loop exactly, so heap and visited-set
        // evolve identically and the plan is byte-identical for any thread count.
        let mut protos: Vec<TNode> = Vec::new();
        // Generation-side symmetry skipping (0.15 Phase 1): the 0.14 probe
        // showed 81% of TMS candidates were orbit-permutation duplicates,
        // generated and then killed one-by-one on the canonical key. Two
        // same-template ops on members of one STABILIZER class (their
        // transposition provably fixes this node's whole state — cross-
        // member facts and pending agenda included) produce π-equivalent
        // successors, so only the first is generated; the duplicate never
        // exists. Pure function of the node → deterministic, t1 ≡ t8.
        // OPT-IN (`FF_ORBIT_GEN=1`): the 0.15 Phase 6 sweep found the
        // per-expansion stabilizer scan costs more than dedup saves on
        // orbit-rich domains that aren't start-credit-walled (match-cellar
        // lost 9 instances to it; TMS's 2.4× eval throughput bought zero
        // extra solves because its wall is h-shaped, not throughput-shaped).
        // The canonical-key pre-dedup below stays default-on — it's pay-per-
        // duplicate, not pay-per-expansion.
        let gen_classes = orbit
            .filter(|_| orbit_gen)
            .map(|om| om.stabilizer_classes(&nodes[ni].state, &nodes[ni].agenda));
        let mut seen_class: HashSet<(u32, Vec<u16>)> = HashSet::new();
        for oi in candidates {
            if let (Some(om), Some(cls)) = (orbit, gen_classes.as_ref()) {
                if let Some(k) = om.gen_key(oi, cls) {
                    if !seen_class.insert(k) {
                        continue;
                    }
                }
            }
            match kind[oi] {
                Kind::Start { dur, end_op, dexp } => {
                    if task.op_applicable(oi, &nodes[ni].state) {
                        // State-dependent duration: resolve against THIS node's
                        // state; skip the start if unresolved or non-positive.
                        let dur = if dexp == u32::MAX {
                            dur
                        } else {
                            match dur_exprs[dexp as usize]
                                .eval(&nodes[ni].state.fv, &nodes[ni].state.fdef)
                            {
                                Some(v) if v.is_finite() && v > 0.0 => v,
                                _ => continue,
                            }
                        };
                        let ns = task.apply(oi, &nodes[ni].state);
                        let te = time + dur;
                        // Identical-interval reduction (0.13 Phase 5, second
                        // arm): a start that changes NOTHING in the state
                        // while the same end op is already pending at or
                        // before `te` adds a redundant COPY of a running
                        // interval — machine-shop re-fires a lit kiln and
                        // re-bakes a baking piece forever this way, each
                        // copy minting a "new" visited key. Dropping the
                        // copy is sound for a plain-STRIPS end (numeric /
                        // conditional end effects are not idempotent): the
                        // start contributed nothing, and the copy's end can
                        // only repeat adds/deletes the earlier pending end
                        // already performs by then — in any VALID plan the
                        // extra end is a no-op, so the plan minus the copy
                        // stays valid.
                        if symm
                            && nodes[ni]
                                .agenda
                                .iter()
                                .any(|&(t, o)| o == end_op && t <= te)
                            && task.num_eff.slice(end_op).is_empty()
                            && task.cond_effs(end_op).next().is_none()
                            && ns.bits == nodes[ni].state.bits
                            && ns.fv == nodes[ni].state.fv
                            && ns.fdef == nodes[ni].state.fdef
                        {
                            continue;
                        }
                        // Over-all invariant enforcement (kiln-gap fixture):
                        // the start's at-start effects must not break a
                        // RUNNING interval's invariant, and the new
                        // interval's own invariant must hold in the state
                        // its start produces (start_pre checked the state
                        // BEFORE effects).
                        if !inv_ok(
                            inv,
                            &touch,
                            oi,
                            &nodes[ni].agenda,
                            None,
                            &nodes[ni].state,
                            &ns,
                        ) {
                            continue;
                        }
                        if let Some((ipos, ineg, inum)) = inv.get(&end_op) {
                            if ipos
                                .iter()
                                .any(|&f| !crate::bitset::test(&ns.bits, f as usize))
                                || ineg
                                    .iter()
                                    .any(|&f| crate::bitset::test(&ns.bits, f as usize))
                                || inum
                                    .iter()
                                    .any(|(np, _)| eval_numpre(np, &ns.fv, &ns.fdef) != Some(true))
                            {
                                continue;
                            }
                        }
                        let mut ag = nodes[ni].agenda.clone();
                        // Canonical (time, op) position under symmetry
                        // reduction; arrival order (after all equal times)
                        // under FF_NO_TSYMM.
                        let pos = if symm {
                            ag.partition_point(|x| x.0 < te || (x.0 == te && x.1 <= end_op))
                        } else {
                            ag.partition_point(|x| x.0 <= te)
                        };
                        ag.insert(pos, (te, end_op));
                        protos.push(TNode {
                            state: ns,
                            time,
                            agenda: ag,
                            father: ni,
                            ev: Some((oi, time)),
                            g: pg + 1,
                            helpful: Vec::new(),
                            met: Vec::new(),
                            lm_accepted: Vec::new(),
                        });
                    }
                }
                Kind::Classical => {
                    if task.op_applicable(oi, &nodes[ni].state) {
                        let ns = task.apply(oi, &nodes[ni].state);
                        // Instantaneous effects are happenings too — same
                        // running-invariant vet as starts.
                        if !inv_ok(
                            inv,
                            &touch,
                            oi,
                            &nodes[ni].agenda,
                            None,
                            &nodes[ni].state,
                            &ns,
                        ) {
                            continue;
                        }
                        let ag = nodes[ni].agenda.clone();
                        protos.push(TNode {
                            state: ns,
                            time,
                            agenda: ag,
                            father: ni,
                            ev: Some((oi, time)),
                            g: pg + 1,
                            helpful: Vec::new(),
                            met: Vec::new(),
                            lm_accepted: Vec::new(),
                        });
                    }
                }
                Kind::End | Kind::Til | Kind::Skip => {}
            }
        }
        // Small frontiers evaluate serially on the persistent Scratch (no per-round
        // allocation); big ones fan out with one fresh Scratch per worker. The
        // threshold is deliberately higher than the classical `par::MIN_PAR` (32):
        // this fans out PER EXPANSION POP, so the scoped-spawn cost recurs every
        // round — at 32-item frontiers it measurably loses (trade-bazaar +39% at
        // t8); it has to amortize against a full unpruned op scan to win.
        // Think-budget accounting (0.12 Phase 1): every proto costs one h
        // evaluation; charge the batch SERIALLY (deterministic at any thread
        // count) before evaluating. Block (b)'s time-advance eval charges 1.
        *budget = budget.saturating_sub(protos.len());
        const PAR_FRONTIER: usize = 128;
        if workers <= 1 || protos.len() < PAR_FRONTIER {
            for n in protos {
                push_node(
                    orbit,
                    task,
                    kind,
                    inv,
                    &touch,
                    &mut stats,
                    &mut sc,
                    &mut nodes,
                    &mut heap,
                    &mut visited,
                    landmarks,
                    &lms,
                    demand,
                    goal_pos,
                    goal_num,
                    prune,
                    relative,
                    seed_til_h,
                    n,
                );
            }
        } else {
            // Same order as the serial path: doomed nodes die first (no
            // eval, no visited entry), then the orbit pre-dedup.
            let mut protos = protos;
            let before = protos.len();
            protos.retain(|n| !doomed(task, inv, &touch, n));
            stats.doomed += (before - protos.len()) as u64;
            // Orbit pre-dedup, serially IN INPUT ORDER before the fan-out —
            // the same keys the funnel would compute, so any thread count
            // sees the identical visited evolution (t1 ≡ t8), and duplicate
            // permutation-states never reach the parallel evaluators.
            let protos: Vec<TNode> = if orbit.is_some() {
                let before = protos.len();
                let kept: Vec<TNode> = protos
                    .into_iter()
                    .filter(|n| visited.insert(tkey(task, n, relative, orbit)))
                    .collect();
                stats.deduped += (before - kept.len()) as u64;
                kept
            } else {
                protos
            };
            let evals: Vec<Option<(i32, Vec<u32>)>> = crate::par::par_map_with(
                &protos,
                workers,
                || Scratch::new(task),
                |wsc, n| {
                    let hs = til_seeded_state(task, kind, &n.agenda, &n.state, seed_til_h);
                    eval_node(
                        task,
                        kind,
                        wsc,
                        hs.as_ref().unwrap_or(&n.state),
                        goal_pos,
                        goal_num,
                        prune,
                    )
                },
            );
            for (n, ev) in protos.into_iter().zip(evals) {
                stats.evaluated += 1;
                if ev.is_none() {
                    stats.dead_end += 1;
                }
                if let Some((h, helpful)) = ev {
                    stats.best_h = stats.best_h.min(h);
                    if orbit.is_some() {
                        enqueue_committed(
                            task, &mut nodes, &mut heap, landmarks, &lms, demand, prune, n, h,
                            helpful,
                        );
                    } else {
                        enqueue_evaluated(
                            orbit,
                            task,
                            &mut nodes,
                            &mut heap,
                            &mut visited,
                            landmarks,
                            &lms,
                            demand,
                            prune,
                            relative,
                            n,
                            h,
                            helpful,
                        );
                    }
                }
            }
        }

        // (b) advance time: fire the earliest pending agenda event — an action END or
        // a timed initial literal. Both apply their grounded op's effect at its time.
        // TILs fire UNCONDITIONALLY: exogenous events don't ask permission (their
        // compiled ops historically carried `True` preconditions, so this is
        // behavior-preserving there — and it lets the session's scheduled-event
        // setters live behind a never-true fence that hides them from the
        // relaxation and block (a) without blocking their firing).
        *budget = budget.saturating_sub(1);
        if let Some(&(te, head_op)) = nodes[ni].agenda.first() {
            // Head inapplicable ⇒ no time-advance successor, exactly as
            // before. Head applicable but its firing would break a
            // still-running interval's over-all invariant ⇒ try the other
            // SAME-EPOCH events in agenda order (kiln-gap tie: the outage
            // TIL and the bake's end share t=8 — closing the bake first is
            // the legal order; dead-ending would lose it). Events later in
            // time are never candidates: they can't fire early.
            let head_applicable =
                matches!(kind[head_op], Kind::Til) || task.op_applicable(head_op, &nodes[ni].state);
            if head_applicable {
                for j in 0..nodes[ni].agenda.len() {
                    let (tj, eop) = nodes[ni].agenda[j];
                    if tj > te {
                        break;
                    }
                    if j > 0
                        && !(matches!(kind[eop], Kind::Til)
                            || task.op_applicable(eop, &nodes[ni].state))
                    {
                        continue;
                    }
                    let ns = task.apply(eop, &nodes[ni].state);
                    if !inv_ok(
                        inv,
                        &touch,
                        eop,
                        &nodes[ni].agenda,
                        Some(j),
                        &nodes[ni].state,
                        &ns,
                    ) {
                        if j == 0 {
                            stats.b_blocked += 1;
                        }
                        continue;
                    }
                    if j > 0 {
                        stats.tie_rescue += 1;
                    }
                    let mut ag = nodes[ni].agenda.clone();
                    ag.remove(j);
                    push_node(
                        orbit,
                        task,
                        kind,
                        inv,
                        &touch,
                        &mut stats,
                        &mut sc,
                        &mut nodes,
                        &mut heap,
                        &mut visited,
                        landmarks,
                        &lms,
                        demand,
                        goal_pos,
                        goal_num,
                        prune,
                        relative,
                        seed_til_h,
                        TNode {
                            state: ns,
                            time: tj,
                            agenda: ag,
                            father: ni,
                            ev: Some((eop, tj)),
                            // FF_TB_FREE_G probe (0.15 Phase 1): firing a due
                            // end is the WORLD moving, not a decision — not
                            // charging g lets time-advance compete with the
                            // start-spam layer on the 1g+3h key (TMS: starts
                            // each drop h by ~1, so breadth over start
                            // subsets starves block (b) and no structure
                            // ever completes).
                            g: if tb_free_g { pg } else { pg + 1 },
                            helpful: Vec::new(),
                            met: Vec::new(),
                            lm_accepted: Vec::new(),
                        },
                    );
                    break;
                }
            }
        }
    }
    None
}

/// Walk the father chain into a timed plan: each START becomes a durative step
/// with its duration (the END is implied); END events are dropped; classical
/// actions appear instantaneously.
fn reconstruct(
    task: &PackedTask,
    nodes: &[TNode],
    goal: usize,
    kind: &[Kind],
    dur_exprs: &[NExpr],
) -> TimedPlan {
    // (op, time, source-node) — the source state resolves state-dependent
    // durations exactly as the expansion did.
    let mut events: Vec<(usize, f64, usize)> = Vec::new();
    let mut cur = goal;
    while let Some((op, t)) = nodes[cur].ev {
        events.push((op, t, nodes[cur].father));
        cur = nodes[cur].father;
    }
    events.reverse();

    let mut steps = Vec::new();
    let mut makespan = 0.0f64;
    for (op, t, src) in events {
        let disp = &task.op_display[op];
        let head = disp.split_whitespace().next().unwrap_or("");
        let args = disp
            .split_whitespace()
            .skip(1)
            .collect::<Vec<_>>()
            .join(" ");
        // Use the durations resolved in `solve` so constant and parameter-dependent
        // durative actions render identically. END events are implied by their start.
        let (name, duration) = match kind[op] {
            Kind::End => {
                makespan = makespan.max(t);
                continue;
            }
            // exogenous TIL firings are not plan steps and don't define the makespan.
            Kind::Til => continue,
            Kind::Start { dur, dexp, .. } => {
                let dur = if dexp == u32::MAX {
                    dur
                } else {
                    dur_exprs[dexp as usize]
                        .eval(&nodes[src].state.fv, &nodes[src].state.fdef)
                        .unwrap_or(dur)
                };
                makespan = makespan.max(t + dur);
                (head.trim_end_matches("-START"), Some(dur))
            }
            _ => {
                makespan = makespan.max(t);
                (head, None)
            }
        };
        let action = if args.is_empty() {
            name.to_string()
        } else {
            format!("{} {}", name, args)
        };
        steps.push(TimedStep {
            time: t,
            action,
            duration,
        });
    }
    TimedPlan { steps, makespan }
}

// ---------------------------------------------------------------------------
// Temporal plan validation (independent of the search).
// ---------------------------------------------------------------------------

/// Validate a [`TimedPlan`] against the temporal semantics, independently of how
/// it was produced: expand each durative step into a START happening at `t` and
/// an END happening at `t + duration`, order all happenings by time (ends before
/// starts at equal time), and simulate over the same snap-action compilation —
/// checking each happening's precondition + `over all` invariant holds, applying
/// its effects, cross-checking each duration against the domain expression, and
/// finally that the goal holds. Returns `Ok(())` if executable and goal-reaching,
/// else a human-readable reason. A cross-check on the search (and on any
/// externally-supplied plan).
pub fn validate(domain: &Domain, problem: &Problem, plan: &TimedPlan) -> Result<(), String> {
    let c = compile(domain, problem);
    let task = match ground_stratified(&c.domain, &c.problem, 1) {
        Outcome::Task(t) => t,
        Outcome::GoalTrue => {
            return if plan.steps.is_empty() {
                Ok(())
            } else {
                Err("goal is already true but the plan is non-empty".into())
            }
        }
        _ => return Err("problem grounds to unsolvable".into()),
    };
    let init = task.initial();
    let modified = modified_fluents(&task);
    let snap_by_start: HashMap<&str, &SnapInfo> = c
        .snaps
        .iter()
        .map(|s| (s.start_action.as_str(), s))
        .collect();
    let find = |disp: &str| {
        task.op_display
            .iter()
            .position(|d| d == disp)
            .ok_or_else(|| format!("plan references unknown action `{disp}`"))
    };

    struct Happening<'a> {
        time: f64,
        op: usize,
        is_start: bool,
        /// Deferred duration cross-check for STATE-DEPENDENT durations
        /// (bounds reading fluents some op assigns): evaluated against the
        /// simulation state when this start fires, exactly as the search
        /// resolved it. `(stated duration, snap, args, step display)`.
        dur_check: Option<(f64, &'a SnapInfo, Vec<&'a str>, &'a str)>,
    }
    let mut happenings: Vec<Happening> = Vec::new();
    for step in &plan.steps {
        let mut it = step.action.splitn(2, ' ');
        let head = it.next().unwrap_or("");
        let rest = it.next();
        let with = |suffix: &str| match rest {
            Some(r) => format!("{head}{suffix} {r}"),
            None => format!("{head}{suffix}"),
        };
        match step.duration {
            Some(dur) => {
                let start_name = format!("{head}-START");
                let snap = snap_by_start
                    .get(start_name.as_str())
                    .ok_or_else(|| format!("`{head}` is not a durative action"))?;
                // cross-check the stated duration against the domain's constraint:
                // it must fall within the `[min, max]` range (a fixed `=` collapses the
                // range to a point, recovering exact-equality). Bounds reading a
                // fluent some op assigns are checked at the start happening
                // against the simulation state (init would be wrong).
                let args: Vec<&str> = rest
                    .map(|r| r.split_whitespace().collect())
                    .unwrap_or_default();
                let bind = duration_bind(snap, &args);
                let state_dep = [&snap.duration.min, &snap.duration.max]
                    .into_iter()
                    .flatten()
                    .any(|e| {
                        ground_duration_nexpr(e, &bind, &task).is_some_and(|ne| {
                            let mut v = Vec::new();
                            ne.collect_fluents(&mut v);
                            v.iter().any(|&f| modified[f as usize])
                        })
                    });
                let mut dur_check = None;
                if state_dep {
                    dur_check = Some((dur, *snap, args.clone(), step.action.as_str()));
                } else {
                    let (lo, hi) = eval_duration_bounds(snap, &args, &task, &init);
                    if let Some(min) = lo {
                        if dur < min - 1e-6 {
                            return Err(format!(
                                "`{}` has duration {dur} below the domain minimum {min}",
                                step.action
                            ));
                        }
                    }
                    if let Some(max) = hi {
                        if dur > max + 1e-6 {
                            return Err(format!(
                                "`{}` has duration {dur} above the domain maximum {max}",
                                step.action
                            ));
                        }
                    }
                }
                happenings.push(Happening {
                    time: step.time,
                    op: find(&with("-START"))?,
                    is_start: true,
                    dur_check,
                });
                happenings.push(Happening {
                    time: step.time + dur,
                    op: find(&with("-END"))?,
                    is_start: false,
                    dur_check: None,
                });
            }
            None => happenings.push(Happening {
                time: step.time,
                op: find(&step.action)?,
                is_start: true,
                dur_check: None,
            }),
        }
    }

    // Replay timed initial literals as exogenous happenings, up to the plan horizon
    // (the last action happening). A TIL strictly after the plan's end is beyond the
    // plan's interval and must not retroactively undo the end-state goal check.
    let horizon = happenings.iter().map(|h| h.time).fold(0.0f64, f64::max);
    for (t, name) in &c.til_ops {
        if *t <= horizon + EPS {
            happenings.push(Happening {
                time: *t,
                op: find(name)?,
                // fire with ends (before starts) at the same epoch, so a gate the TIL
                // opens is available to an action starting at that instant.
                is_start: false,
                dur_check: None,
            });
        }
    }

    // Execute in time order; at the SAME decision epoch, ends (which free
    // tokens/resources) fire before starts (which consume them) — ferroplan's
    // decision-epoch semantics. Key on the ε-grid-rounded time, not the raw float,
    // so a producer-END and consumer-START at the same epoch order deterministically
    // even when composition offsets introduce sub-ε float noise.
    happenings.sort_by_key(|h| ((h.time / EPS).round() as i64, h.is_start));
    let mut state = init.clone();
    for h in &happenings {
        if let Some((dur, snap, args, disp)) = &h.dur_check {
            let (lo, hi) = eval_duration_bounds(snap, args, &task, &state);
            if let Some(min) = lo {
                if *dur < min - 1e-6 {
                    return Err(format!(
                        "`{disp}` has duration {dur} below the domain minimum {min}",
                    ));
                }
            }
            if let Some(max) = hi {
                if *dur > max + 1e-6 {
                    return Err(format!(
                        "`{disp}` has duration {dur} above the domain maximum {max}",
                    ));
                }
            }
        }
        if !task.op_applicable(h.op, &state) {
            return Err(format!(
                "at t={:.3}, `{}` is not applicable (precondition or invariant violated)",
                h.time, task.op_display[h.op]
            ));
        }
        state = task.apply(h.op, &state);
    }
    if !task.goal_met(&state) {
        return Err("the plan does not achieve the goal".into());
    }
    Ok(())
}

/// Replay a composed `TimedPlan` over `state` in global-time happening order (ends
/// before starts at equal time) and return the post-state, or `None` if any
/// happening is inapplicable on the running state (a shared-resource shortfall or
/// stale precondition — the decomposer's conflict signal). Mirrors `validate`'s
/// simulation loop, minus the duration cross-check and goal check, over the SAME
/// grounded `task` whose `op_display` the plan's steps name.
pub(crate) fn treplay(task: &PackedTask, state: &State, plan: &TimedPlan) -> Option<State> {
    treplay_with_exempt(task, state, plan, &[])
}

/// [`treplay`] where ops in `exempt` (sorted) skip the applicability check —
/// the session's scheduled-event setters (0.14 Phase 3) sit behind a
/// never-true fence exactly so nothing BUT an agenda/replay fires them, and
/// exogenous events don't ask permission (the same exemption the search's
/// time-advance block applies to `Kind::Til`).
pub(crate) fn treplay_with_exempt(
    task: &PackedTask,
    state: &State,
    plan: &TimedPlan,
    exempt: &[usize],
) -> Option<State> {
    let find = |disp: &str| task.op_display.iter().position(|d| d == disp);
    struct H {
        time: f64,
        op: usize,
        is_start: bool,
    }
    let mut hs: Vec<H> = Vec::new();
    for step in &plan.steps {
        let mut it = step.action.splitn(2, ' ');
        let head = it.next().unwrap_or("");
        let rest = it.next();
        let with = |suffix: &str| match rest {
            Some(r) => format!("{head}{suffix} {r}"),
            None => format!("{head}{suffix}"),
        };
        match step.duration {
            Some(dur) => {
                hs.push(H {
                    time: step.time,
                    op: find(&with("-START"))?,
                    is_start: true,
                });
                hs.push(H {
                    time: step.time + dur,
                    op: find(&with("-END"))?,
                    is_start: false,
                });
            }
            None => {
                let op = find(&step.action)?;
                hs.push(H {
                    time: step.time,
                    // Exempt ops (injected exogenous events) and injected
                    // `-END` happenings (a session's running intervals) fire
                    // from the agenda BEFORE same-instant starts in the
                    // search; the replay sorts them with the ends to match.
                    is_start: exempt.binary_search(&op).is_err() && !head.ends_with("-END"),
                    op,
                });
            }
        }
    }
    // Same ε-grid-rounded ordering as `validate` (ends before starts at one epoch),
    // so the decomposer's per-contract replay agrees with the global validator.
    hs.sort_by_key(|h| ((h.time / EPS).round() as i64, h.is_start));
    let mut s = state.clone();
    for h in &hs {
        if exempt.binary_search(&h.op).is_err() && !task.op_applicable(h.op, &s) {
            return None;
        }
        s = task.apply(h.op, &s);
    }
    Some(s)
}

// ---------------------------------------------------------------------------
// ε-separation: make plans valid under PDDL2.1 continuous-time semantics.
// ---------------------------------------------------------------------------

/// PDDL2.1 separation between mutex happenings (the IPC convention).
pub(crate) const EPS: f64 = 0.001;

/// Re-time a plan so mutex happenings are ε-separated (PDDL2.1 / VAL validity):
/// the decision-epoch search coincides dependent happenings (e.g. one action
/// starting the instant another's at-end effect lands), which VAL rejects. We
/// model the plan's happenings as a simple temporal network — preserve the
/// execution order, pin each end at start+duration, force ε between mutex pairs —
/// and solve the earliest-time schedule by longest paths (Bellman–Ford). On any
/// inconsistency or for very large plans the original plan is returned unchanged.
fn epsilon_separate(task: &PackedTask, plan: TimedPlan, floor_to_search: bool) -> TimedPlan {
    // happening: (owning step index, is_start); op ids became unnecessary
    // when the pairwise interference test gave way to total ε-ordering, but
    // the display lookups below still gate on mappability (an unmappable
    // step means we cannot trust the schedule at all).
    struct H {
        step: usize,
        is_start: bool,
        time: f64,
    }
    let find = |disp: &str| task.op_display.iter().position(|d| d == disp);
    let mut hs: Vec<H> = Vec::new();
    for (si, step) in plan.steps.iter().enumerate() {
        let mut it = step.action.splitn(2, ' ');
        let head = it.next().unwrap_or("");
        let rest = it.next();
        match step.duration {
            Some(dur) => {
                let sd = match rest {
                    Some(r) => format!("{head}-START {r}"),
                    None => format!("{head}-START"),
                };
                let ed = match rest {
                    Some(r) => format!("{head}-END {r}"),
                    None => format!("{head}-END"),
                };
                match (find(&sd), find(&ed)) {
                    (Some(_so), Some(_eo)) => {
                        hs.push(H {
                            step: si,
                            is_start: true,
                            time: step.time,
                        });
                        hs.push(H {
                            step: si,
                            is_start: false,
                            time: step.time + dur,
                        });
                    }
                    _ => {
                        if std::env::var("FF_RES_DEBUG").is_ok() {
                            eprintln!("[eps] cannot map `{sd}`/`{ed}` -> plan left unseparated");
                        }
                        return plan; // can't map -> leave as-is
                    }
                }
            }
            None => match find(&step.action) {
                Some(_o) => hs.push(H {
                    step: si,
                    is_start: true,
                    time: step.time,
                }),
                None => return plan,
            },
        }
    }
    let n = hs.len();
    if n == 0 || n > 2000 {
        // (2000 happenings ≈ 1000 steps; the elevator tails exceeded the old
        // 600 cap and shipped UNseparated plans VAL would reject)
        return plan; // nothing to do, or too large to schedule cheaply
    }
    // execution order: by time, ends before starts at equal time
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        hs[a]
            .time
            .partial_cmp(&hs[b].time)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(hs[a].is_start.cmp(&hs[b].is_start))
    });

    // STN edges: t[v] >= t[u] + w. TOTAL ε-ordering: every consecutive pair
    // in execution order is ε apart. Concurrency lives in INTERVAL OVERLAP,
    // not shared instants, so this keeps every genuinely concurrent plan
    // concurrent while making simultaneity — the only thing any mutex
    // definition can object to — impossible by construction. It subsumes the
    // pairwise interference test this pass used to run: first fact-only
    // (missed same-instant numeric write-write — elevator-numeric), then
    // fact+numeric footprints (still missed VAL's SYNTACTIC footprint: a
    // `forall (imply (includes o4 p) (started o4))` condition reads
    // `(started o4)` in VAL's mutex test even when the imply is statically
    // vacuous — the compiled precondition is semantically minimal and
    // under-approximates it; openstacks-temporal-ADL failed 50/60 plans on
    // exactly that). Makespan cost: ≤ n·ε ≈ milliseconds.
    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    for w in order.windows(2) {
        edges.push((w[0], w[1], EPS));
    }
    // duration equality: end = start + dur  (two inequalities)
    for si in 0..plan.steps.len() {
        if let Some(dur) = plan.steps[si].duration {
            let (mut s, mut e) = (None, None);
            for (hi, h) in hs.iter().enumerate() {
                if h.step == si {
                    if h.is_start {
                        s = Some(hi)
                    } else {
                        e = Some(hi)
                    }
                }
            }
            if let (Some(s), Some(e)) = (s, e) {
                edges.push((s, e, dur));
                edges.push((e, s, -dur));
            }
        }
    }

    // longest-path (earliest feasible times) via Bellman–Ford. With timed initial
    // literals present, seed each happening at its SEARCH-assigned time as a lower
    // bound (the search already placed every happening at a TIL-feasible instant);
    // relaxation only pushes later, so a TIL-gated action can't be slid before its
    // gate. Without TILs, seed at 0 — byte-identical to the prior re-timing.
    let mut t: Vec<f64> = if floor_to_search {
        hs.iter().map(|h| h.time).collect()
    } else {
        vec![0.0f64; n]
    };
    for _ in 0..n {
        let mut changed = false;
        for &(u, v, w) in &edges {
            if t[v] < t[u] + w - 1e-12 {
                t[v] = t[u] + w;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // positive-cycle check: another pass must not improve
    for &(u, v, w) in &edges {
        if t[v] < t[u] + w - 1e-12 {
            if std::env::var("FF_RES_DEBUG").is_ok() {
                eprintln!("[eps] STN inconsistency -> plan left unseparated");
            }
            return plan; // inconsistent ordering -> keep original
        }
    }

    // re-time the steps from the scheduled start happenings, SNAPPED to the
    // ε grid: accumulated float drift (0.003 + 1.0 + 0.001 =
    // 1.0039999999999999) leaves an intended ε gap a hair UNDER ε, and VAL
    // at tolerance ε then groups the two happenings as simultaneous — the
    // openstacks-temporal-ADL sweeps failed 20/30 + 30/30 plans on exactly
    // this before snapping. Rounding to whole ε slots puts every gap on the
    // safe side.
    let mut steps = plan.steps;
    for (hi, h) in hs.iter().enumerate() {
        if h.is_start {
            steps[h.step].time = (t[hi] / EPS).round() * EPS;
        }
    }
    let makespan = steps
        .iter()
        .map(|s| s.time + s.duration.unwrap_or(0.0))
        .fold(0.0f64, f64::max);
    TimedPlan { steps, makespan }
}
