//! PDDL3.0 soft-goal preferences + metric optimization (phase 1).
//!
//! Compilation (Keyder & Geffner, "Soft goals can be compiled away", JAIR 2009):
//! each `(preference p phi)` in the goal becomes a 0-ary fact `collected_p`, a
//! `collect_p` action (precond `phi`, effect `collected_p`, cost 0), and a
//! `forgo_p` action (effect `collected_p` + `(increase (total-cost) w_p)`).
//! `collected_p` is added to the HARD goal, so minimizing `total-cost` minimizes
//! the weighted preference violation (plus any existing action costs).
//!
//! `w_p` is the coefficient of `(is-violated p)` in the `:metric`; preferences not
//! referenced by the metric get weight 0 (free to forgo); with no metric at all,
//! weight 1 (minimize count of violated preferences).
//!
//! Optimization is ANYTIME branch-and-bound over `total-cost` via
//! `crate::solve_subgoal_bounded` (sound because action costs are monotone).
//! Scope: metrics linear in `(is-violated …)`, `(total-cost)`, and any other
//! MONOTONE numeric fluent term (e.g. rovers' `(sum-traverse-cost)`), which
//! `compile()` folds into `total-cost` so the single-cost B&B optimizes the FULL
//! metric. Non-monotone / under-forall / divided terms are flagged, not optimized.

use std::collections::{HashMap, HashSet};

use crate::packed::PackedTask;
use crate::search::{plan, solve_subgoal_bounded, SatGuidance, SearchCfg};
use crate::types::{
    Action, AssignOp, Domain, Effect, Expr, Formula, MetricDir, Problem, Sym, Term,
};

pub const COST: &str = "TOTAL-COST";
pub const COST_DISP: &str = "(TOTAL-COST)";

/// Does this problem use PDDL3 soft goals or a metric?
pub fn is_pddl3(problem: &Problem) -> bool {
    problem.metric.is_some() || goal_has_pref(&problem.goal)
}

fn goal_has_pref(f: &Formula) -> bool {
    match f {
        Formula::Pref(_, _) => true,
        Formula::And(v) | Formula::Or(v) => v.iter().any(goal_has_pref),
        Formula::Not(a) => goal_has_pref(a),
        Formula::Forall(_, a) | Formula::Exists(_, a) => goal_has_pref(a),
        _ => false,
    }
}

fn expr_has_is_violated(e: &crate::types::Expr) -> bool {
    use crate::types::Expr::*;
    match e {
        Fluent(name, _) => name.eq_ignore_ascii_case("is-violated"),
        Add(a, b) | Sub(a, b) | Mul(a, b) | Div(a, b) => {
            expr_has_is_violated(a) || expr_has_is_violated(b)
        }
        Neg(a) => expr_has_is_violated(a),
        Num(_) => false,
    }
}

/// True iff the problem has genuine PDDL3 *preferences* (goal preferences or an
/// `is-violated` metric) — as opposed to a plain numeric `:metric`. Used to route
/// `Mode::Auto`: preferences -> PDDL3 mode, otherwise classic FF.
pub fn has_preferences(problem: &Problem) -> bool {
    goal_has_pref(&problem.goal)
        || problem
            .metric
            .as_ref()
            .is_some_and(|(_, e)| expr_has_is_violated(e))
}

// ---- formula substitution + quantifier combos (for forall-preferences) ----

fn subst_term(t: &Term, b: &HashMap<Sym, Sym>) -> Term {
    match t {
        Term::Var(v) => b
            .get(v)
            .map(|o| Term::Const(o.clone()))
            .unwrap_or_else(|| t.clone()),
        Term::Const(_) => t.clone(),
    }
}
fn subst_expr(e: &Expr, b: &HashMap<Sym, Sym>) -> Expr {
    match e {
        Expr::Num(n) => Expr::Num(*n),
        Expr::Fluent(f, a) => Expr::Fluent(f.clone(), a.iter().map(|t| subst_term(t, b)).collect()),
        Expr::Add(x, y) => Expr::Add(Box::new(subst_expr(x, b)), Box::new(subst_expr(y, b))),
        Expr::Sub(x, y) => Expr::Sub(Box::new(subst_expr(x, b)), Box::new(subst_expr(y, b))),
        Expr::Mul(x, y) => Expr::Mul(Box::new(subst_expr(x, b)), Box::new(subst_expr(y, b))),
        Expr::Div(x, y) => Expr::Div(Box::new(subst_expr(x, b)), Box::new(subst_expr(y, b))),
        Expr::Neg(x) => Expr::Neg(Box::new(subst_expr(x, b))),
    }
}
fn subst_formula(f: &Formula, b: &HashMap<Sym, Sym>) -> Formula {
    match f {
        Formula::And(v) => Formula::And(v.iter().map(|x| subst_formula(x, b)).collect()),
        Formula::Or(v) => Formula::Or(v.iter().map(|x| subst_formula(x, b)).collect()),
        Formula::Not(a) => Formula::Not(Box::new(subst_formula(a, b))),
        Formula::Atom(p, a) => {
            Formula::Atom(p.clone(), a.iter().map(|t| subst_term(t, b)).collect())
        }
        Formula::Comp(op, l, r) => Formula::Comp(*op, subst_expr(l, b), subst_expr(r, b)),
        Formula::Eq(x, y) => Formula::Eq(subst_term(x, b), subst_term(y, b)),
        Formula::Pref(n, inner) => Formula::Pref(n.clone(), Box::new(subst_formula(inner, b))),
        // inner quantifier may shadow an outer var: don't substitute its own vars
        Formula::Forall(vars, inner) | Formula::Exists(vars, inner) => {
            let mut b2 = b.clone();
            for (v, _) in vars {
                b2.remove(v);
            }
            let inner = Box::new(subst_formula(inner, &b2));
            if matches!(f, Formula::Forall(..)) {
                Formula::Forall(vars.clone(), inner)
            } else {
                Formula::Exists(vars.clone(), inner)
            }
        }
        Formula::True => Formula::True,
        Formula::False => Formula::False,
    }
}
fn combos(vars: &[(Sym, Sym)], objs: &HashMap<Sym, Vec<Sym>>) -> Vec<HashMap<Sym, Sym>> {
    let mut acc = vec![HashMap::new()];
    for (v, ty) in vars {
        let dom: &[Sym] = objs.get(ty).map(|x| x.as_slice()).unwrap_or(&[]);
        let mut next = Vec::new();
        for a in &acc {
            for o in dom {
                let mut m = a.clone();
                m.insert(v.clone(), o.clone());
                next.push(m);
            }
        }
        acc = next;
    }
    acc
}
fn contains_pref(f: &Formula) -> bool {
    match f {
        Formula::Pref(_, _) => true,
        Formula::And(v) | Formula::Or(v) => v.iter().any(contains_pref),
        Formula::Not(a) | Formula::Forall(_, a) | Formula::Exists(_, a) => contains_pref(a),
        _ => false,
    }
}

/// Pull soft `(preference name phi)` conjuncts out of an action precondition,
/// returning the hard precondition and the list of (name, phi) precond prefs.
fn extract_precond_prefs(f: &Formula, ctr: &mut usize) -> (Formula, Vec<(String, Formula)>) {
    match f {
        Formula::And(v) => {
            let mut hard = Vec::new();
            let mut prefs = Vec::new();
            for x in v {
                let (h, mut ps) = extract_precond_prefs(x, ctr);
                prefs.append(&mut ps);
                if !matches!(h, Formula::True) {
                    hard.push(h);
                }
            }
            (Formula::And(hard), prefs)
        }
        Formula::Pref(name, inner) => {
            let n = name.clone().unwrap_or_else(|| {
                let s = format!("PCPREF{}", *ctr);
                *ctr += 1;
                s
            });
            (Formula::True, vec![(n, (**inner).clone())])
        }
        other => (other.clone(), Vec::new()),
    }
}

/// Split a goal into hard conjuncts and (name, formula) preferences. A
/// `(forall (vars) ... preference ...)` is expanded into one preference INSTANCE
/// per object binding, all sharing the preference name (so `(is-violated name)`
/// counts violated instances — the PDDL3 semantics).
fn split_goal(
    g: &Formula,
    hard: &mut Vec<Formula>,
    prefs: &mut Vec<(String, Formula)>,
    ctr: &mut usize,
    objs: &HashMap<Sym, Vec<Sym>>,
) {
    match g {
        Formula::And(v) => v.iter().for_each(|f| split_goal(f, hard, prefs, ctr, objs)),
        Formula::Forall(vars, inner) if contains_pref(inner) => {
            for b in combos(vars, objs) {
                split_goal(&subst_formula(inner, &b), hard, prefs, ctr, objs);
            }
        }
        Formula::Pref(name, inner) => {
            let n = name.clone().unwrap_or_else(|| {
                let s = format!("PREF{}", *ctr);
                *ctr += 1;
                s
            });
            prefs.push((n, (**inner).clone()));
        }
        Formula::True => {}
        other => hard.push(other.clone()),
    }
}

/// Accumulate metric weights: `is-violated p` -> w[p], `total-cost` coeff,
/// `others[f]` += coeff for any other 0-ary numeric fluent `(f)` (e.g.
/// `(sum-traverse-cost)`), and `other` for genuinely unsupported shapes (n-ary
/// metric fluents, division, non-constant products).
fn extract(
    e: &Expr,
    scale: f64,
    w: &mut HashMap<String, f64>,
    tc: &mut f64,
    others: &mut HashMap<String, f64>,
    other: &mut bool,
) {
    match e {
        Expr::Num(_) => {}
        Expr::Fluent(name, args) => {
            if name == "IS-VIOLATED" {
                if let Some(Term::Const(p)) = args.first() {
                    *w.entry(p.clone()).or_insert(0.0) += scale;
                }
            } else if name == COST {
                *tc += scale;
            } else if args.is_empty() {
                *others.entry(name.clone()).or_insert(0.0) += scale;
            } else {
                *other = true;
            }
        }
        Expr::Add(a, b) => {
            extract(a, scale, w, tc, others, other);
            extract(b, scale, w, tc, others, other);
        }
        Expr::Sub(a, b) => {
            extract(a, scale, w, tc, others, other);
            extract(b, -scale, w, tc, others, other);
        }
        Expr::Neg(a) => extract(a, -scale, w, tc, others, other),
        Expr::Mul(a, b) => match (&**a, &**b) {
            (Expr::Num(c), _) => extract(b, scale * c, w, tc, others, other),
            (_, Expr::Num(c)) => extract(a, scale * c, w, tc, others, other),
            _ => *other = true,
        },
        Expr::Div(_, _) => *other = true,
    }
}

/// Functions modified by some action effect (so the complement is "static").
fn modified_functions(domain: &Domain) -> HashSet<String> {
    fn walk(e: &Effect, out: &mut HashSet<String>) {
        match e {
            Effect::And(v) => v.iter().for_each(|x| walk(x, out)),
            Effect::Num(_, name, _, _) => {
                out.insert(name.clone());
            }
            Effect::When(_, e) | Effect::Forall(_, e) => walk(e, out),
            _ => {}
        }
    }
    let mut out = HashSet::new();
    for a in &domain.actions {
        walk(&a.effect, &mut out);
    }
    out
}

/// Can metric fluent `fname` be folded into total-cost (monotone non-decreasing)?
/// Yes iff every effect on it is `(increase fname X)` where X is a non-negative
/// constant, or a STATIC function whose init values are all non-negative. Returns
/// `Some(reason)` if not foldable.
fn fluent_foldable(domain: &Domain, problem: &Problem, fname: &str) -> Option<String> {
    let modified = modified_functions(domain);
    let static_nonneg = |g: &str| -> bool {
        if modified.contains(g) {
            return false;
        }
        // every init value of g must be >= 0 (default 0 if unspecified)
        problem
            .init_fluents
            .iter()
            .filter(|((n, _), _)| n == g)
            .all(|(_, v)| *v >= 0.0)
    };
    let mut bad: Option<String> = None;
    fn walk(
        e: &Effect,
        fname: &str,
        in_forall: bool,
        static_nonneg: &dyn Fn(&str) -> bool,
        bad: &mut Option<String>,
    ) {
        match e {
            Effect::And(v) => v
                .iter()
                .for_each(|x| walk(x, fname, in_forall, static_nonneg, bad)),
            Effect::When(_, e) => walk(e, fname, in_forall, static_nonneg, bad),
            Effect::Forall(_, e) => walk(e, fname, true, static_nonneg, bad),
            Effect::Num(op, name, _, val) if name == fname => {
                // an increase inside a forall can't be mirrored term-for-term, so
                // treat it as not foldable.
                let ok = !in_forall
                    && matches!(op, AssignOp::Increase)
                    && match val {
                        Expr::Num(n) => *n >= 0.0,
                        Expr::Fluent(g, _) => static_nonneg(g),
                        _ => false,
                    };
                if !ok && bad.is_none() {
                    *bad = Some(format!(
                        "metric fluent ({fname}) is not foldable (not monotone, or under forall)"
                    ));
                }
            }
            _ => {}
        }
    }
    for a in &domain.actions {
        walk(&a.effect, fname, false, &static_nonneg, &mut bad);
    }
    bad
}

/// `coeff * x` as an Expr (avoids a `(* 1 x)` wrapper when coeff is 1).
fn scaled_expr(coeff: f64, x: &Expr) -> Expr {
    if coeff == 1.0 {
        x.clone()
    } else {
        Expr::Mul(Box::new(Expr::Num(coeff)), Box::new(x.clone()))
    }
}

/// Collect `(increase total-cost coeff*X)` mirrors for each `(increase fname X)`
/// found inside `eff`.
fn collect_cost_mirror(eff: &Effect, fname: &str, coeff: f64, out: &mut Vec<Effect>) {
    match eff {
        Effect::And(v) => v
            .iter()
            .for_each(|x| collect_cost_mirror(x, fname, coeff, out)),
        Effect::When(_, e) => collect_cost_mirror(e, fname, coeff, out),
        Effect::Num(AssignOp::Increase, name, _, val) if name == fname => {
            out.push(Effect::Num(
                AssignOp::Increase,
                COST.to_string(),
                vec![],
                scaled_expr(coeff, val),
            ));
        }
        _ => {}
    }
}

/// Extract the (name, formula) preference INSTANCES from a goal (forall-expanded
/// over `objs`) — for independent scoring and compilation.
pub fn preferences(goal: &Formula, objs: &HashMap<Sym, Vec<Sym>>) -> Vec<(String, Formula)> {
    let mut hard = Vec::new();
    let mut prefs = Vec::new();
    let mut ctr = 0;
    split_goal(goal, &mut hard, &mut prefs, &mut ctr, objs);
    prefs
}

/// Effective metric weight per preference INSTANCE name (default 1 with no
/// metric, else the `(is-violated name)` coefficient, 0 if unreferenced).
pub fn pref_weights(domain: &Domain, problem: &Problem) -> HashMap<String, f64> {
    let mut w = HashMap::new();
    let mut tc = 0.0;
    let mut others = HashMap::new();
    let mut other = false;
    let absent = problem.metric.is_none();
    if let Some((_, e)) = &problem.metric {
        extract(e, 1.0, &mut w, &mut tc, &mut others, &mut other);
    }
    let objs = crate::ground::objects_by_type(domain, problem);
    let mut out = HashMap::new();
    for (n, _) in preferences(&problem.goal, &objs) {
        let wn = w.get(&n).copied().unwrap_or(if absent { 1.0 } else { 0.0 });
        out.insert(n, wn);
    }
    out
}

pub struct Compiled {
    pub domain: Domain,
    pub problem: Problem,
    pub minimize: bool,
    pub n_prefs: usize,
    pub warn_other: bool,
    /// Set if the metric is outside the supported class (maximize / negative
    /// weight / scaled or non-monotone total-cost). The caller falls back to a
    /// satisficing plan instead of silently optimizing the wrong objective.
    pub unsupported: Option<String>,
    /// Names of the synthetic Keyder-Geffner actions (stripped from the plan).
    pub synthetic: HashSet<String>,
    /// (forgo-action name, weight) per preference instance.
    pub forgos: Vec<(String, f64)>,
}

/// Is `total-cost` monotone non-decreasing across the domain? Branch-and-bound
/// cost pruning is only sound if it is. Any decrease/scale/assign on total-cost,
/// or an increase by a non-constant/negative amount, breaks monotonicity.
fn cost_monotone(domain: &Domain) -> bool {
    fn walk(e: &Effect, ok: &mut bool) {
        match e {
            Effect::And(v) => v.iter().for_each(|x| walk(x, ok)),
            Effect::Num(op, name, _, val) if name == COST => {
                let good =
                    matches!(op, AssignOp::Increase) && matches!(val, Expr::Num(n) if *n >= 0.0);
                if !good {
                    *ok = false;
                }
            }
            _ => {}
        }
    }
    let mut ok = true;
    for a in &domain.actions {
        walk(&a.effect, &mut ok);
    }
    ok
}

/// Compile soft goals away into a classical+cost problem (Keyder–Geffner).
pub fn compile(domain: &Domain, problem: &Problem) -> Compiled {
    let objs = crate::ground::objects_by_type(domain, problem);
    let mut hard = Vec::new();
    let mut prefs = Vec::new();
    let mut ctr = 0;
    split_goal(&problem.goal, &mut hard, &mut prefs, &mut ctr, &objs);

    let mut w = HashMap::new();
    let mut tc = 0.0;
    let mut others: HashMap<String, f64> = HashMap::new();
    let mut other = false;
    let minimize = match &problem.metric {
        Some((MetricDir::Minimize, e)) => {
            extract(e, 1.0, &mut w, &mut tc, &mut others, &mut other);
            true
        }
        Some((MetricDir::Maximize, e)) => {
            extract(e, 1.0, &mut w, &mut tc, &mut others, &mut other);
            false
        }
        None => true,
    };
    let metric_absent = problem.metric.is_none();
    let _ = tc;

    let mut d = domain.clone();
    let mut p = problem.clone();
    if !d.functions.iter().any(|(n, _)| n == COST) {
        d.functions.push((COST.to_string(), vec![]));
    }
    if !p.init_fluents.iter().any(|((n, _), _)| n == COST) {
        p.init_fluents.push(((COST.to_string(), vec![]), 0.0));
    }

    // FOLD monotone numeric metric terms (e.g. `(sum-traverse-cost)` in rovers)
    // into total-cost: mirror every `(increase f X)` with `(increase total-cost
    // coeff*X)`. Then total-cost == the FULL metric, and the existing single-cost
    // B&B optimizes + reports it correctly. Terms that can't be folded (non-
    // monotone, under forall) are left out and surfaced via `warn_other`.
    let mut metric_other = other;
    for (fname, &coeff) in &others {
        if coeff == 0.0 {
            continue;
        }
        if fluent_foldable(domain, problem, fname).is_some() {
            metric_other = true; // optimize the supported part only
            continue;
        }
        for a in &mut d.actions {
            let mut mirror = Vec::new();
            collect_cost_mirror(&a.effect, fname, coeff, &mut mirror);
            if !mirror.is_empty() {
                let mut v = vec![a.effect.clone()];
                v.append(&mut mirror);
                a.effect = Effect::And(v);
            }
        }
    }

    // Precondition preferences: split each action with soft preconditions into
    // satisfied/violated variants (same name). The satisfied variant requires the
    // soft condition (free); the violated variant requires its negation and pays
    // the weight. Because the variants are mutually exclusive and the planner
    // applies exactly one per use, `(is-violated p)` counts per-application
    // violations EXACTLY (no over-count from disjunctive negations — applying one
    // grounded op charges the weight once).
    let mut pp_negative = false;
    let mut pp_overflow = false;
    let mut new_actions = Vec::new();
    let mut pctr = 0usize;
    for a in &d.actions {
        let (hard_pre, pprefs) = extract_precond_prefs(&a.precond, &mut pctr);
        if pprefs.is_empty() {
            new_actions.push(a.clone());
            continue;
        }
        let k = pprefs.len();
        if k > 6 {
            pp_overflow = true;
            new_actions.push(Action {
                precond: hard_pre,
                ..a.clone()
            });
            continue;
        }
        for mask in 0u32..(1u32 << k) {
            let mut conj = vec![hard_pre.clone()];
            let mut cost = 0.0;
            for (i, (name, phi)) in pprefs.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    conj.push(Formula::Not(Box::new(phi.clone()))); // violated
                    let raw = w
                        .get(name)
                        .copied()
                        .unwrap_or(if metric_absent { 1.0 } else { 0.0 });
                    if raw < 0.0 {
                        pp_negative = true;
                    }
                    cost += raw.max(0.0);
                } else {
                    conj.push(phi.clone()); // satisfied
                }
            }
            let mut eff = vec![a.effect.clone()];
            if cost != 0.0 {
                eff.push(Effect::Num(
                    AssignOp::Increase,
                    COST.to_string(),
                    vec![],
                    Expr::Num(cost),
                ));
            }
            new_actions.push(Action {
                name: a.name.clone(),
                params: a.params.clone(),
                precond: Formula::And(conj),
                effect: Effect::And(eff),
            });
        }
    }
    d.actions = new_actions;

    // End-marker phasing (Keyder–Geffner): simple preferences are evaluated in
    // the FINAL state, so collect/forgo must run only after planning ends — else
    // a preference could be "collected" while transiently true mid-plan. Real
    // actions require (P3PLANNING); `end` flips to (P3ENDED) and freezes the
    // state; collect_p (which checks phi) and forgo_p require (P3ENDED).
    const PLANNING: &str = "P3PLANNING";
    const ENDED: &str = "P3ENDED";
    d.predicates.push((PLANNING.to_string(), vec![]));
    d.predicates.push((ENDED.to_string(), vec![]));
    p.init_atoms.push((PLANNING.to_string(), vec![]));
    for a in &mut d.actions {
        a.precond = Formula::And(vec![
            Formula::Atom(PLANNING.to_string(), vec![]),
            a.precond.clone(),
        ]);
    }
    let mut synthetic = HashSet::new();
    synthetic.insert("P3END".to_string());
    d.actions.push(Action {
        name: "P3END".to_string(),
        params: vec![],
        precond: Formula::Atom(PLANNING.to_string(), vec![]),
        effect: Effect::And(vec![
            Effect::Del(PLANNING.to_string(), vec![]),
            Effect::Add(ENDED.to_string(), vec![]),
        ]),
    });

    let mut goal_parts = hard;
    let mut any_negative = false;
    // (forgo-action name, weight) per preference — lets the optimizer force-collect
    // high-weight preferences (forbid forgoing them) during relax-and-tighten.
    let mut forgos: Vec<(String, f64)> = Vec::new();
    for (i, (name, phi)) in prefs.iter().enumerate() {
        let col = format!("P3COLLECTED-{}", i);
        d.predicates.push((col.clone(), vec![]));
        let collect = format!("P3COLLECT-{}", i);
        let forgo = format!("P3FORGO-{}", i);
        synthetic.insert(collect.clone());
        synthetic.insert(forgo.clone());
        // collect: phi must hold in the FINAL (ended) state — free
        d.actions.push(Action {
            name: collect,
            params: vec![],
            precond: Formula::And(vec![Formula::Atom(ENDED.to_string(), vec![]), phi.clone()]),
            effect: Effect::Add(col.clone(), vec![]),
        });
        // forgo: skip it, paying its weight (clamped >= 0 to keep cost monotone;
        // a negative weight is flagged unsupported below, not silently applied)
        let raw = w
            .get(name)
            .copied()
            .unwrap_or(if metric_absent { 1.0 } else { 0.0 });
        if raw < 0.0 {
            any_negative = true;
        }
        forgos.push((format!("P3FORGO-{}", i), raw.max(0.0)));
        d.actions.push(Action {
            name: forgo,
            params: vec![],
            precond: Formula::Atom(ENDED.to_string(), vec![]),
            effect: Effect::And(vec![
                Effect::Add(col.clone(), vec![]),
                Effect::Num(
                    AssignOp::Increase,
                    COST.to_string(),
                    vec![],
                    Expr::Num(raw.max(0.0)),
                ),
            ]),
        });
        goal_parts.push(Formula::Atom(col, vec![]));
    }
    // require the end marker so the planner closes the planning phase
    goal_parts.push(Formula::Atom(ENDED.to_string(), vec![]));
    p.goal = Formula::And(goal_parts);

    // determine whether the metric is inside the supported (optimizable) class
    let unsupported = if !minimize {
        Some("metric maximization is not supported (phase 1: minimize only)".into())
    } else if any_negative || pp_negative {
        Some("negative preference weight (cannot be encoded monotonically)".into())
    } else if pp_overflow {
        Some("an action has too many precondition preferences (>6)".into())
    } else if !(tc == 0.0 || tc == 1.0) {
        Some(format!(
            "scaled total-cost coefficient ({}) is not supported",
            tc
        ))
    } else if !cost_monotone(domain) {
        Some("non-monotone total-cost (decrease/scale effects) breaks branch-and-bound".into())
    } else {
        None
    };

    Compiled {
        domain: d,
        problem: p,
        minimize,
        n_prefs: prefs.len(),
        warn_other: metric_other,
        unsupported,
        synthetic,
        forgos,
    }
}

/// Final value of the cost fluent after executing `ops` from the initial state.
pub(crate) fn plan_cost(task: &PackedTask, ops: &[usize], cf: usize) -> f64 {
    let mut s = task.initial();
    for &oi in ops {
        s = task.apply(oi, &s);
    }
    if s.fdef[cf] {
        s.fv[cf]
    } else {
        0.0
    }
}

pub struct MetricResult {
    pub ops: Vec<usize>,
    pub cost: f64,
    pub iterations: usize,
    /// True only if the search exhausted the space proving no cheaper plan
    /// exists. False if a resource bound (MAX_EVAL / MAX_ITERS) cut it short, in
    /// which case `cost` is the best found, not provably optimal.
    pub proven: bool,
}

/// Minimize `cost_fluent` (total preference-violation weight) by relax-and-tighten:
/// an EHC first incumbent, then SGPlan-style force-collect tightening (force the
/// highest-weight preferences to actually be satisfied), then a bounded B&B polish.
/// `forgos` are the (op-id, weight) of the synthetic forgo actions.
/// Default per-`occupancy²` weight for the renewable-resource guidance term.
///
/// **Off (0) by default — deliberately.** A swept experiment (FF_RES_WEIGHT ×
/// FF_RES_THRESH on openstacks p01–p05) showed a soft occupancy penalty never
/// lowers the metric: small weights *raise* it (penalizing live occupancy
/// suppresses the necessary start→make→ship pipeline — a started-but-unshipped
/// order carries both a forgone-pref penalty and occupancy cost), and large
/// thresholds are inert. This is principled: openstacks is min-open-stacks
/// scheduling (an order's products must be made while it is *started*, so orders
/// sharing a product must be open simultaneously — the MOSP/pathwidth constraint),
/// a combinatorial *peak/throughput* objective that no per-state penalty can
/// express. Closing that gap needs the ESPC partition+penalty loop or a real
/// scheduler, not this term.
///
/// The detection + concrete-state hook are kept as the **foundation** for
/// capacity-aware scheduling (numeric resources, and renewable-resource
/// feasibility in the temporal planner — where capacity is a *hard* constraint,
/// the case that actually matters for durative resource allocation). Override at
/// runtime with `FF_RES_WEIGHT` / `FF_RES_THRESH` to experiment.
const RES_WEIGHT_DEFAULT: i64 = 0;

pub fn metric_optimize(
    task: &PackedTask,
    cost_fluent: usize,
    forgos: &[(usize, f64)],
    groups: &[Vec<u32>],
    threads: usize,
) -> Option<MetricResult> {
    const MAX_ITERS: usize = 10_000;
    let init = task.initial();
    let mut bound = f64::INFINITY;
    let mut best: Option<(Vec<usize>, f64)> = None;
    let mut iterations = 0;
    let mut proven = false;

    // 1. RELAX: first incumbent via EHC-then-best-first (SGPlan's modified-FF
    // subplanner) — a fast feasible plan, so we get coverage even on hard
    // instances. Bounded so a hard instance can't hang here.
    let first = plan(
        task,
        threads,
        SearchCfg::from_weights(1.0, 5.0, Some(1_500_000)),
        true,
    );
    if let Some(ops) = first.ops {
        let cost = plan_cost(task, &ops, cost_fluent);
        if cost <= 0.0 {
            return Some(MetricResult {
                ops,
                cost,
                iterations: 0,
                proven: true,
            });
        }
        bound = cost;
        best = Some((ops, cost));
    }

    // 2. SATISFACTION GUIDANCE — the earlier "force-collect" variants all failed
    // because under delete-relaxation the free forgo action makes every preference
    // look reachable, so the heuristic was blind to satisfaction (see
    // docs/espc-preferences-spec.md). Instead, bias the B&B open list by a penalty
    // that counts preferences forgone in the CONCRETE state — this sees real
    // satisfaction and gives the search a gradient toward delivering, breaking the
    // all-forgo floor. (It still can't see the openstacks `stacks-avail` resource —
    // that needs the SAS+ partition + penalty loop — so it narrows, not closes, the
    // gap.) Built from each preference's P3COLLECT-i `phi` precondition.
    let mut sat = build_sat_guidance(task, forgos);
    // Resource-aware guidance foundation: detect any renewable "counter" resource
    // the delete-relaxed heuristic is blind to (e.g. openstacks' stacks-avail).
    // The occupancy penalty in SatGuidance is OFF by default — see
    // RES_WEIGHT_DEFAULT for why a soft penalty can't crack openstacks; this stays
    // as the substrate for capacity-aware scheduling. Empty (no-op) on domains
    // with no such resource. Tunable via FF_RES_WEIGHT / FF_RES_THRESH.
    sat.res = crate::resource::detect_resources(task, groups, &task.init_bits);
    if !sat.res.is_empty() {
        sat.res_weight = std::env::var("FF_RES_WEIGHT")
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(RES_WEIGHT_DEFAULT);
        sat.res_thresh = std::env::var("FF_RES_THRESH")
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);
        if std::env::var("FF_RES_DEBUG").is_ok() {
            let caps: Vec<usize> = sat.res.iter().map(|r| r.members.len() - 1).collect();
            eprintln!(
                "[C3] {} renewable resource(s), capacities {:?}; w={} thresh={}",
                sat.res.len(),
                caps,
                sat.res_weight,
                sat.res_thresh
            );
        }
    }

    // ESPC make-deadline guidance. Penalizes once-only conditional achievements
    // that fire without delivering (openstacks: a product made while its orders
    // still wait — a permanently locked metric loss the delete-relaxed RPG is blind
    // to). Built unconditionally (pure analysis, inert on domains without the
    // structure); only the heap WEIGHT is gated, so the default path stays
    // bit-identical until a flag is set.
    sat.deadline = build_deadline_guidance(task, forgos);
    let refine_cfg = SearchCfg::from_weights(1.0, 5.0, Some(300_000));

    // FULL ESPC (FF_ESPC): an adaptive per-trigger penalty-resolution outer loop.
    // It re-solves under fixed penalties, raises the penalty on triggers whose
    // deliveries were missed, and keeps the best plan as an anytime incumbent,
    // terminating at a saddle point / stall / budget. Auto-tunes the penalty per
    // instance (no manual weight) and never claims optimality. See `crate::espc`
    // and docs/espc-preferences-spec.md.
    if crate::features::espc() && !sat.deadline.is_empty() {
        let part = build_espc_partition(task, forgos, groups, &sat);
        return crate::espc::espc_optimize(
            task,
            cost_fluent,
            &mut sat,
            best.clone(),
            part,
            threads,
            refine_cfg,
        )
        .map(|r| MetricResult {
            ops: r.ops,
            cost: r.cost,
            iterations: r.iterations,
            proven: false, // anytime: a saddle point is not a global-optimality proof
        })
        .or_else(|| {
            best.map(|(ops, cost)| MetricResult {
                ops,
                cost,
                iterations: 0,
                proven: false,
            })
        });
    }

    // Phase-0 lever (OFF by default): a FIXED deadline penalty for manual sweeps /
    // ablation. With it unset the heap key is bit-identical to the non-ESPC path.
    if !sat.deadline.is_empty() {
        sat.deadline_weight = std::env::var("FF_DEADLINE_WEIGHT")
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);
        if std::env::var("FF_RES_DEBUG").is_ok() {
            eprintln!(
                "[ESPC] {} deadline pair(s), fixed lambda={}",
                sat.deadline.len(),
                sat.deadline_weight
            );
        }
    }

    // 3. POLISH: bounded B&B from the (now much better) incumbent — reaches the
    // true optimum on small instances; on timeout we keep the incumbent.
    while iterations < MAX_ITERS {
        iterations += 1;
        let (opt, capped) = solve_subgoal_bounded(
            task,
            &init,
            &task.goal_pos,
            &task.goal_num,
            cost_fluent,
            bound,
            threads,
            refine_cfg,
            Some(&sat),
        );
        match opt {
            Some(ops) => {
                let cost = plan_cost(task, &ops, cost_fluent);
                best = Some((ops, cost));
                if cost <= 0.0 {
                    proven = true; // cannot beat zero cost
                    break;
                }
                bound = cost; // next plan must be strictly cheaper (prune cost >= bound)
            }
            None => {
                // no plan cheaper than the incumbent: proven optimal IFF the
                // search exhausted (not stopped by the MAX_EVAL safety cap).
                proven = !capped;
                break;
            }
        }
    }
    best.map(|(ops, cost)| MetricResult {
        ops,
        cost,
        iterations,
        proven,
    })
}

/// `P3COLLECT-i` op id per preference index — the shared scan behind the
/// satisfaction guidance, the deadline guidance, and the partitioned-ESPC
/// phase tail.
fn collect_ops(task: &PackedTask) -> std::collections::HashMap<usize, usize> {
    let mut collect_op = std::collections::HashMap::new();
    for oi in 0..task.n_ops {
        if let Some(rest) = task.op_display[oi]
            .to_ascii_uppercase()
            .strip_prefix("P3COLLECT-")
        {
            if let Ok(i) = rest.trim().parse::<usize>() {
                collect_op.insert(i, oi);
            }
        }
    }
    collect_op
}

/// Op ids for the deterministic post-composition **phase tail** (partitioned
/// ESPC, `crate::espc`): `P3END` freezes the state, then each preference is
/// closed in fixed order — `P3COLLECT-i` when its `phi` holds (free), else
/// `P3FORGO-i` (pays the weight). Exact, not heuristic: after `P3END` the state
/// is frozen and each preference's collected fact is independent, so
/// collect-iff-applicable is the optimal closure of whatever final state the
/// composition reached. `None` when any preference lacks a collect op (never
/// expected on a supported compile — the caller falls back to monolithic).
pub struct PhaseTail {
    pub end_op: usize,
    /// `(collect_op, forgo_op)` per preference, in preference order.
    pub prefs: Vec<(usize, usize)>,
}

pub(crate) fn build_phase_tail(task: &PackedTask, forgos: &[(usize, f64)]) -> Option<PhaseTail> {
    let end_op = (0..task.n_ops).find(|&oi| task.op_display[oi].eq_ignore_ascii_case("P3END"))?;
    let collect = collect_ops(task);
    let mut prefs = Vec::with_capacity(forgos.len());
    for (i, &(forgo_op, _)) in forgos.iter().enumerate() {
        prefs.push((*collect.get(&i)?, forgo_op));
    }
    Some(PhaseTail { end_op, prefs })
}

/// Build the partitioned-ESPC subproblems ("increment 2", see `crate::espc`):
/// interaction components over the REAL goal (the compiled `P3*` bookkeeping
/// goals are closed by the phase tail instead), with the detected renewable
/// resource variables (openstacks' `stacks-avail` chain) excluded from edge
/// formation — that shared coupling is priced by the λ schedule as a global
/// constraint, not solved inside any one subproblem. `None` (→ monolithic loop)
/// when the compile shape is unsupported: no phase tail, numeric goals present,
/// no real positive goals, or fewer than 2 components.
fn build_espc_partition(
    task: &PackedTask,
    forgos: &[(usize, f64)],
    groups: &[Vec<u32>],
    sat: &SatGuidance,
) -> Option<crate::espc::EspcPartition> {
    if !task.goal_num.is_empty() {
        return None; // components carry positive facts only; don't drop a numeric goal
    }
    let tail = build_phase_tail(task, forgos)?;
    let real_goals: Vec<u32> = task
        .goal_pos
        .iter()
        .copied()
        .filter(|&f| {
            !task.fact_names[f as usize]
                .to_ascii_uppercase()
                .starts_with("(P3")
        })
        .collect();
    if real_goals.is_empty() {
        return None;
    }
    // Global-constraint variables: any mutex group carrying a detected renewable
    // resource member (detect_resources accepts whole groups, so member-overlap
    // identifies exactly the accepted group indices).
    let res_member: crate::hash::FxHashSet<u32> = sat
        .res
        .iter()
        .flat_map(|r| r.members.iter().map(|&(f, _)| f))
        .collect();
    let excluded: crate::hash::FxHashSet<usize> = groups
        .iter()
        .enumerate()
        .filter(|(_, g)| g.iter().any(|f| res_member.contains(f)))
        .map(|(gi, _)| gi)
        .collect();
    let mut comps =
        crate::partition::interaction_partition_of(task, groups, &real_goals, &excluded);
    // Deterministic composition order regardless of hash-map component order.
    comps.sort_by_key(|c| c.pos.iter().min().copied().unwrap_or(u32::MAX));
    if comps.len() < 2 {
        return None;
    }
    // Stage-goal enrichment map: deliverable D → the real goals structurally
    // tied to it. D's conditional-achievement CONDITION facts name the party the
    // delivery is FOR (openstacks: `delivered(o,p)` fires on `started(o)`), and
    // a goal fact claims D when one of the goal's ACHIEVER ops requires such a
    // condition fact (`ship-order(o)` adds `shipped(o)` and requires
    // `started(o)`), so the stage solving that goal also tries to earn its own
    // preferences (see `EspcPartition::assoc`).
    let deliverables: crate::hash::FxHashSet<u32> =
        sat.deadline.iter().map(|&(_, d, _)| d).collect();
    let mut by_cond: crate::hash::FxHashMap<u32, Vec<u32>> = crate::hash::FxHashMap::default();
    for oi in 0..task.n_ops {
        for ce in task.cond.slice(oi) {
            for &d in &ce.add {
                if !deliverables.contains(&d) {
                    continue;
                }
                for &c in &ce.cond_pos {
                    by_cond.entry(c).or_default().push(d);
                }
            }
        }
    }
    let mut assoc: crate::hash::FxHashMap<u32, Vec<u32>> = crate::hash::FxHashMap::default();
    for &g in &real_goals {
        let mut ds: Vec<u32> = task
            .add_by_fact
            .slice(g as usize)
            .iter()
            .flat_map(|&oi| task.pre_pos.slice(oi as usize))
            .filter_map(|p| by_cond.get(p))
            .flatten()
            .copied()
            .collect();
        if !ds.is_empty() {
            ds.sort_unstable();
            ds.dedup();
            assoc.insert(g, ds);
        }
    }
    if std::env::var("FF_RES_DEBUG").is_ok() {
        eprintln!(
            "[ESPC] partition: {} component(s) over {} real goal(s), {} excluded var(s), {} enriched goal(s)",
            comps.len(),
            real_goals.len(),
            excluded.len(),
            assoc.len()
        );
    }
    Some(crate::espc::EspcPartition { comps, tail, assoc })
}

/// Build the metric satisfaction guidance: for each preference, the fact-ids of
/// its `phi` (taken from the `P3COLLECT-i` action's precondition, minus the
/// synthetic `P3*` control facts) and a heap penalty scaled from its forgo
/// weight. Preferences with a non-atom `phi` simply contribute nothing (their
/// `P3COLLECT` precondition still works; they're just unguided).
fn build_sat_guidance(task: &PackedTask, forgos: &[(usize, f64)]) -> SatGuidance {
    let collect_op = collect_ops(task);
    let mut prefs = Vec::new();
    for (i, (_, weight)) in forgos.iter().enumerate() {
        let Some(&oi) = collect_op.get(&i) else {
            continue;
        };
        let phi: Vec<u32> = task
            .pre_pos
            .slice(oi)
            .iter()
            .copied()
            .filter(|&f| {
                !task.fact_names[f as usize]
                    .to_ascii_uppercase()
                    .starts_with("(P3")
            })
            .collect();
        if !phi.is_empty() {
            prefs.push((phi, (weight * 100.0).round().max(1.0) as i64));
        }
    }
    SatGuidance {
        prefs,
        res: Vec::new(),
        res_weight: 0,
        res_thresh: 0,
        deadline: Vec::new(),
        deadline_weight: 0,
    }
}

/// Build ESPC make-deadline guidance (see [`SatGuidance::deadline`]). For each
/// preference deliverable fact `D` (extracted from each `P3COLLECT-i` `phi`, as in
/// [`build_sat_guidance`]), locate the op whose CONDITIONAL effect adds `D` and that
/// op's unique unconditional add `M` — the once-only "trigger" (e.g. `(made p)`),
/// which fires at most once because the op requires its own trigger absent. Emit
/// `(M, D, value)` where `value` is the summed weight of the preferences that
/// require `D`, so a deliverable shared by the weight-1/2/4 chain is valued highest.
/// Returns empty on domains without this conditional-achievement structure (⇒ inert),
/// in a deterministic, hashmap-iteration-independent order.
fn build_deadline_guidance(task: &PackedTask, forgos: &[(usize, f64)]) -> Vec<(u32, u32, i64)> {
    use std::collections::{HashMap, HashSet};
    // P3COLLECT-i op per preference index (mirrors build_sat_guidance).
    let collect_op = collect_ops(task);
    // value[D] = Σ weight over preferences whose phi (P3COLLECT precondition,
    // minus synthetic P3* control facts) contains the deliverable fact D.
    let mut value: HashMap<u32, i64> = HashMap::new();
    for (i, (_, weight)) in forgos.iter().enumerate() {
        let Some(&oi) = collect_op.get(&i) else {
            continue;
        };
        let w = (*weight).round().max(1.0) as i64;
        for &f in task.pre_pos.slice(oi) {
            if task.fact_names[f as usize]
                .to_ascii_uppercase()
                .starts_with("(P3")
            {
                continue;
            }
            *value.entry(f).or_insert(0) += w;
        }
    }
    if value.is_empty() {
        return Vec::new();
    }
    // For each deliverable D, find its conditional achiever op and that op's
    // UNIQUE unconditional trigger M (skip ops with non-unique unconditional adds —
    // they aren't the clean once-only achiever this models).
    let mut out: Vec<(u32, u32, i64)> = Vec::new();
    let mut seen: HashSet<(u32, u32)> = HashSet::new();
    for oi in 0..task.n_ops {
        let uncond = task.add.slice(oi);
        if uncond.len() != 1 {
            continue;
        }
        let trigger = uncond[0];
        for ce in task.cond.slice(oi) {
            for &d in &ce.add {
                if let Some(&val) = value.get(&d) {
                    if seen.insert((trigger, d)) {
                        out.push((trigger, d, val));
                    }
                }
            }
        }
    }
    out.sort_unstable();
    out
}
