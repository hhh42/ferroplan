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
//! Scope: metrics linear in `(is-violated …)` and `(total-cost)` (the IPC-5
//! "simple-preferences" shape); other fluent terms are flagged, not optimized.

use std::collections::{HashMap, HashSet};

use crate::packed::PackedTask;
use crate::search::solve_subgoal_bounded;
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

/// Accumulate metric weights: `is-violated p` -> w[p], `total-cost` coeff, and
/// whether any unsupported fluent term appeared.
fn extract(e: &Expr, scale: f64, w: &mut HashMap<String, f64>, tc: &mut f64, other: &mut bool) {
    match e {
        Expr::Num(_) => {}
        Expr::Fluent(name, args) => {
            if name == "IS-VIOLATED" {
                if let Some(Term::Const(p)) = args.first() {
                    *w.entry(p.clone()).or_insert(0.0) += scale;
                }
            } else if name == COST {
                *tc += scale;
            } else {
                *other = true;
            }
        }
        Expr::Add(a, b) => {
            extract(a, scale, w, tc, other);
            extract(b, scale, w, tc, other);
        }
        Expr::Sub(a, b) => {
            extract(a, scale, w, tc, other);
            extract(b, -scale, w, tc, other);
        }
        Expr::Neg(a) => extract(a, -scale, w, tc, other),
        Expr::Mul(a, b) => match (&**a, &**b) {
            (Expr::Num(c), _) => extract(b, scale * c, w, tc, other),
            (_, Expr::Num(c)) => extract(a, scale * c, w, tc, other),
            _ => *other = true,
        },
        Expr::Div(_, _) => *other = true,
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
    let mut other = false;
    let absent = problem.metric.is_none();
    if let Some((_, e)) = &problem.metric {
        extract(e, 1.0, &mut w, &mut tc, &mut other);
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
    let mut other = false;
    let minimize = match &problem.metric {
        Some((MetricDir::Minimize, e)) => {
            extract(e, 1.0, &mut w, &mut tc, &mut other);
            true
        }
        Some((MetricDir::Maximize, e)) => {
            extract(e, 1.0, &mut w, &mut tc, &mut other);
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
        warn_other: other,
        unsupported,
        synthetic,
    }
}

/// Final value of the cost fluent after executing `ops` from the initial state.
fn plan_cost(task: &PackedTask, ops: &[usize], cf: usize) -> f64 {
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

/// Anytime branch-and-bound: minimize `cost_fluent` by repeatedly solving with a
/// tightening upper bound. Returns the best plan found, or None if the hard goal
/// is unreachable. Bounded by `MAX_ITERS`.
pub fn metric_optimize(
    task: &PackedTask,
    cost_fluent: usize,
    threads: usize,
) -> Option<MetricResult> {
    const MAX_ITERS: usize = 10_000;
    let init = task.initial();
    let mut bound = f64::INFINITY;
    let mut best: Option<(Vec<usize>, f64)> = None;
    let mut iterations = 0;
    let mut proven = false;

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
