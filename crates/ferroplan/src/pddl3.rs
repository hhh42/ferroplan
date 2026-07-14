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
use crate::search::{plan, solve_subgoal_bounded, ClosureCost, PrefPhi, SatGuidance, SearchCfg};
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

/// PDDL3 *trajectory* constraints — the modal `(:constraints ...)` operators
/// (`always`, `sometime`, `at-most-once`, `sometime-after`/`-before`, `within`,
/// `hold-during`/`-after`) — are parsed into the AST (`Domain`/`Problem.constraints`)
/// but not yet enforced by any solving path. Rather than silently accept and drop a
/// user's hard constraint, every public entrypoint rejects a domain or problem that
/// carries one. Returns the rejection message when trajectory constraints are
/// present, or `None` when there are none to enforce.
///
/// This is distinct from goal `(preference ...)` SOFT goals (handled by the PDDL3
/// metric path): those live in the goal formula, not in `.constraints`, and are
/// unaffected.
pub(crate) fn unsupported_constraints(domain: &Domain, problem: &Problem) -> Option<String> {
    if domain.constraints.is_empty() && problem.constraints.is_empty() {
        return None;
    }
    Some(
        "PDDL3 trajectory constraints (:constraints — always / sometime / \
         at-most-once / sometime-after / sometime-before / within / hold-during / \
         hold-after) are parsed but not yet enforced; ferroplan cannot honor them \
         and will not silently ignore them. Remove the (:constraints ...) block, or \
         model the requirement as hard goals or PDDL3 goal preferences."
            .to_string(),
    )
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

/// Predicates never added nor deleted by any action effect — their truth is
/// fixed by the initial state. The static complement of [`modified_functions`].
fn static_predicates(domain: &Domain) -> HashSet<String> {
    fn walk(e: &Effect, out: &mut HashSet<String>) {
        match e {
            Effect::And(v) => v.iter().for_each(|x| walk(x, out)),
            Effect::Add(name, _) | Effect::Del(name, _) => {
                out.insert(name.clone());
            }
            Effect::When(_, e) | Effect::Forall(_, e) => walk(e, out),
            Effect::Num(..) => {}
        }
    }
    let mut modified = HashSet::new();
    for a in &domain.actions {
        walk(&a.effect, &mut modified);
    }
    domain
        .predicates
        .iter()
        .map(|(n, _)| n.clone())
        .filter(|n| !modified.contains(n))
        .collect()
}

/// Partially evaluate a (mostly ground) preference formula against the facts
/// that can never change: fully-ground atoms of STATIC predicates are decided
/// by init membership, ground `(= a b)` is decided by symbol equality, and the
/// connectives fold. Everything else (numeric comparisons, atoms still carrying
/// quantified variables, dynamic predicates) is left untouched — conservative,
/// so the result is equivalent in every REACHABLE state. A preference whose phi
/// folds to `True` can never be violated (metric contribution identically 0),
/// which is what lets `compile()` drop it before the Keyder–Geffner expansion.
fn peval_static(
    f: &Formula,
    statics: &HashSet<String>,
    init: &HashSet<(Sym, Vec<Sym>)>,
) -> Formula {
    match f {
        Formula::Atom(p, args) => {
            if !statics.contains(p) {
                return f.clone();
            }
            let consts: Option<Vec<Sym>> = args
                .iter()
                .map(|t| match t {
                    Term::Const(c) => Some(c.clone()),
                    Term::Var(_) => None,
                })
                .collect();
            match consts {
                Some(cs) => {
                    if init.contains(&(p.clone(), cs)) {
                        Formula::True
                    } else {
                        Formula::False
                    }
                }
                None => f.clone(), // still quantified — leave for grounding
            }
        }
        Formula::Eq(Term::Const(a), Term::Const(b)) => {
            if a == b {
                Formula::True
            } else {
                Formula::False
            }
        }
        Formula::Not(inner) => match peval_static(inner, statics, init) {
            Formula::True => Formula::False,
            Formula::False => Formula::True,
            other => Formula::Not(Box::new(other)),
        },
        Formula::And(v) => {
            let mut rest = Vec::new();
            for x in v {
                match peval_static(x, statics, init) {
                    Formula::True => {}
                    Formula::False => return Formula::False,
                    other => rest.push(other),
                }
            }
            if rest.is_empty() {
                Formula::True
            } else {
                Formula::And(rest)
            }
        }
        Formula::Or(v) => {
            let mut rest = Vec::new();
            for x in v {
                match peval_static(x, statics, init) {
                    Formula::True => return Formula::True,
                    Formula::False => {}
                    other => rest.push(other),
                }
            }
            if rest.is_empty() {
                Formula::False
            } else {
                Formula::Or(rest)
            }
        }
        // `forall . True` and `exists . False` hold/fail vacuously even over an
        // empty binding domain; the dual cases depend on domain non-emptiness,
        // so they stay wrapped (conservative).
        Formula::Forall(vars, inner) => match peval_static(inner, statics, init) {
            Formula::True => Formula::True,
            other => Formula::Forall(vars.clone(), Box::new(other)),
        },
        Formula::Exists(vars, inner) => match peval_static(inner, statics, init) {
            Formula::False => Formula::False,
            other => Formula::Exists(vars.clone(), Box::new(other)),
        },
        _ => f.clone(),
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
    /// True when a numeric metric term was FOLDED into total-cost (mirrored
    /// `increase` effects on real actions, e.g. rovers' traverse costs). The
    /// metric optimizer routes such tasks to the legacy compiled-goal B&B —
    /// real-action cost gives it a genuine gradient, and the closure search
    /// measures worse there (continuous tightening churn).
    pub folded_metric: bool,
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
    let n_prefs_total = prefs.len();

    // STATIC SIMPLIFICATION: a preference whose phi is statically TRUE (e.g. an
    // `imply` whose antecedent tests a static relation like storage's
    // `(connected s1 s2)` on an unconnected pair) can never be violated —
    // contributing exactly 0 to the metric in every reachable state — so it
    // never needs collect/forgo ops or a hard-goal fact. IPC-5 storage's
    // quadratic forall-preference expands to crates²·storeareas² instances, of
    // which ~90%+ are statically satisfied; dropping them here is what makes
    // p03+ (1601/4211 raw instances) searchable at all. Survivors keep the
    // simplified phi (cheaper DNF at grounding). The independent verifier
    // scores from the ORIGINAL goal, so reported metrics are unaffected.
    // `FF_PREF_NO_STATIC=1` restores the blind expansion.
    if std::env::var("FF_PREF_NO_STATIC").is_err() {
        let statics = static_predicates(domain);
        let init: HashSet<(Sym, Vec<Sym>)> = problem.init_atoms.iter().cloned().collect();
        prefs = prefs
            .into_iter()
            .filter_map(|(name, phi)| match peval_static(&phi, &statics, &init) {
                Formula::True => None,
                simplified => Some((name, simplified)),
            })
            .collect();
        if std::env::var("FF_RES_DEBUG").is_ok() && prefs.len() < n_prefs_total {
            eprintln!(
                "[P3] static simplification: dropped {} of {} preference instance(s)",
                n_prefs_total - prefs.len(),
                n_prefs_total
            );
        }
    }

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
    let mut folded_metric = false;
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
                folded_metric = true;
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
        // full pre-simplification count: statically-satisfied instances are
        // still real preferences (satisfied ones), so reporting stays stable
        n_prefs: n_prefs_total,
        warn_other: metric_other,
        unsupported,
        synthetic,
        forgos,
        folded_metric,
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

/// Default `SearchCfg::w_c` for the FOLDED-numeric-metric legacy B&B. ZERO ON
/// PURPOSE: the 2026-07 rovers p01–p08 sweep (w_c ∈ {0, 0.25, 0.5, 1, 2, 5})
/// showed every non-zero weight COLLAPSES quality to the all-forgo floor
/// (p01: 935.3 → 1162.1; p07: no result at all) — accumulated cost only grows
/// along a path, so cost-ordering buries the deep goal-reaching prefixes the
/// tightening B&B needs behind shallow cheap ones, and the bounded searches
/// stop finding ANY plan under the incumbent. The closure-path probe was
/// neutral (identical metrics). The rovers gains came from the escalating
/// retry instead (p02 659.3→596.7, p05 649.9→523.3). `w_c` stays available
/// for experiments via `FF_PREF_COST_WEIGHT`.
const COST_WEIGHT_FOLDED_DEFAULT: f64 = 0.0;

pub fn metric_optimize(
    task: &PackedTask,
    cost_fluent: usize,
    forgos: &[(usize, f64)],
    groups: &[Vec<u32>],
    folded_metric: bool,
    threads: usize,
) -> Option<MetricResult> {
    const MAX_ITERS: usize = 10_000;
    let init = task.initial();

    // 1. SATISFACTION GUIDANCE (pure analysis, built before any search) — the
    // earlier "force-collect" variants all failed
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
    // Cost-aware open-list ordering (see `SearchCfg::w_c`) — experimental,
    // default OFF everywhere: the sweep that was meant to pick a folded-metric
    // default found it collapses rovers instead (see COST_WEIGHT_FOLDED_DEFAULT
    // for the measured post-mortem). `FF_PREF_COST_WEIGHT` enables it for
    // experiments on either metric loop.
    let cost_w = std::env::var("FF_PREF_COST_WEIGHT")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(if folded_metric {
            COST_WEIGHT_FOLDED_DEFAULT
        } else {
            0.0
        });

    // FULL ESPC (FF_ESPC): an adaptive per-trigger penalty-resolution outer loop.
    // It re-solves under fixed penalties, raises the penalty on triggers whose
    // deliveries were missed, and keeps the best plan as an anytime incumbent,
    // terminating at a saddle point / stall / budget. Auto-tunes the penalty per
    // instance (no manual weight) and never claims optimality. See `crate::espc`
    // and docs/espc-preferences-spec.md. Seeded by the same compiled-goal EHC
    // pass as always — this branch is deliberately untouched by the closure path.
    if crate::features::espc() && !sat.deadline.is_empty() {
        let mut best: Option<(Vec<usize>, f64)> = None;
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
            best = Some((ops, cost));
        }
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

    // 2. EXACT-CLOSURE METRIC SEARCH (the default for pure-preference metrics):
    // search REAL states only and close the preference bookkeeping with the
    // exact phase tail, instead of searching a hard goal made of hundreds/
    // thousands of `P3COLLECTED-i` facts with a satisfaction-blind heuristic
    // (the storage p03+ wall, and the tpp budget sink). Precondition-preference
    // variant costs on real ops are fine — they accrue in `g`, which the
    // acceptance test sums with the closure exactly. What is NOT routed here is
    // FOLDED numeric metrics (rovers' mirrored traverse costs) route here TOO
    // since 0.5: the 0.4.0 verdict that the closure search measures worse on
    // them (tiny-epsilon tightening churn to MAX_ITERS, a poorer incumbent
    // than the EHC seed) was an artifact of first-improvement restarts — with
    // anytime sweeps the closure path dominates the legacy B&B on every
    // rovers instance (p01 935.3→811.3 ties SGPlan5, p04 485.5→418.7 and p06
    // 664.6→655.7 beat it, p05 483.6 ties; the domain flips to a lead under
    // both quality conventions). `FF_PREF_NUMLEGACY=1` restores the pre-0.5
    // split (folded → legacy); `FF_PREF_COMPILED=1` routes EVERYTHING legacy.
    // Also falls back when the closure search cannot produce an incumbent.
    let numlegacy = folded_metric && std::env::var("FF_PREF_NUMLEGACY").is_ok();
    if !forgos.is_empty() && !numlegacy && std::env::var("FF_PREF_COMPILED").is_err() {
        if let Some(tail) = build_phase_tail(task, forgos) {
            if let Some(r) = metric_optimize_closure(
                task,
                cost_fluent,
                forgos,
                &tail,
                &sat,
                groups,
                threads,
                refine_cfg.with_cost_weight(cost_w),
            ) {
                return Some(r);
            }
        }
    }

    // 3. LEGACY compiled-goal path: EHC seed on the full compiled goal, then the
    // bounded polish B&B from the incumbent. Reached only via `FF_PREF_COMPILED=1`
    // or the closure fallback above. The tightening loop shares the closure
    // path's deterministic eval-count budget and capped-failure escalation
    // (see `metric_optimize_closure`); the 1.5M EHC seed stays outside the
    // budget, mirroring the closure path's free init-tail incumbent.
    let mut bound = f64::INFINITY;
    let mut best: Option<(Vec<usize>, f64)> = None;
    let mut iterations = 0;
    let mut proven = false;
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

    // FORGO-AWARE SECOND SEED (completion pricing) — experimental, opt-in via
    // `FF_PREF_SEED=1`, default OFF after measuring NEUTRAL (2026-07): the
    // idea is to price what a preference COSTS to deliver (the relaxation is
    // blind to it — on rovers a forced traverse round-trip can cost more than
    // the preference's weight, and prefix-cost ordering `w_c` was a measured
    // dead end because cost only grows along a path). Estimate each
    // preference's delivery cost with a cost-aware relaxed plan from the
    // initial state (`heuristic::relaxed_plan_cost`), pre-forgo any preference
    // whose estimate exceeds its weight (forbid its P3COLLECT ops) in ONE
    // extra seeded solve, and keep the cheaper of the two incumbents.
    // MEASURED: on rovers the estimates fire correctly (p01: est 157 vs
    // weight 76.5 → pre-forgo) but the plain EHC seed already lands at the
    // same incumbent cost, and final metrics are identical with the seed on
    // or off (p01–p08). The residual rovers gap lives in the B&B's reachable
    // trade curve, not the seed bound — the diversified restart ladder is
    // what moved it (p04 559.9 → 485.5). Machinery kept for experiments;
    // a wrong estimate can never hurt quality (min of both seeds).
    if !forgos.is_empty() && std::env::var("FF_PREF_SEED").is_ok() {
        let mut sc = crate::heuristic::Scratch::new(task);
        let collect = collect_ops(task);
        let mut banned: Vec<bool> = vec![false; task.n_ops];
        let mut any = false;
        for (i, (_, weight)) in forgos.iter().enumerate() {
            let Some(ops_i) = collect.get(&i) else {
                continue;
            };
            // completion estimate = the cheapest disjunct's relaxed-plan cost
            let mut est = f64::INFINITY;
            for &oi in ops_i {
                let pos: Vec<u32> = task
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
                let c = crate::heuristic::relaxed_plan_cost(
                    task,
                    &mut sc,
                    &init.bits,
                    &init.fv,
                    &init.fdef,
                    &pos,
                    task.pre_num.slice(oi),
                    cost_fluent,
                )
                .unwrap_or(f64::INFINITY);
                est = est.min(c);
            }
            if std::env::var("FF_RES_DEBUG").is_ok() {
                eprintln!("[seed] pref {i}: completion est {est:.1} vs weight {weight:.1}");
            }
            if est > *weight {
                for &oi in ops_i {
                    banned[oi] = true;
                }
                any = true;
            }
        }
        if any {
            let (seeded, _evaluated) = crate::search::solve_subgoal_guided(
                task,
                &init,
                &task.goal_pos,
                &task.goal_num,
                &banned,
                threads,
                SearchCfg::from_weights(1.0, 5.0, Some(1_500_000)),
                None,
            );
            if std::env::var("FF_RES_DEBUG").is_ok() {
                let c = seeded.as_ref().map(|o| plan_cost(task, o, cost_fluent));
                eprintln!("[seed] seeded solve: {c:?} vs EHC incumbent {bound:.1}");
            }
            if let Some(ops) = seeded {
                let cost = plan_cost(task, &ops, cost_fluent);
                if cost < bound {
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
            }
        }
    }
    let budget = pref_eval_budget();
    let no_escalate = std::env::var("FF_PREF_NO_ESCALATE").is_ok();
    // Anytime in-sweep tightening + the diversified restart ladder, exactly as
    // in the closure loop (acceptance here is plain `final cost < bound` — no
    // closure term). Same hatches (`FF_PREF_GREEDY`, `FF_PREF_NO_RESTARTS`).
    const PROFILES: &[(f64, f64)] = &[(1.0, 2.0), (1.0, 8.0), (2.0, 3.0), (0.0, 1.0)];
    let anytime = std::env::var("FF_PREF_GREEDY").is_err();
    let restarts = anytime && std::env::var("FF_PREF_NO_RESTARTS").is_err();
    let mut spent = 0usize;
    let mut escalated = false;
    let mut rung = 0usize;
    while iterations < MAX_ITERS && spent < budget {
        iterations += 1;
        let cap = if escalated {
            budget - spent
        } else if rung > 0 {
            // Half-size diversification rungs — see the closure loop.
            (refine_cfg.max_eval / 2).min(budget - spent)
        } else {
            refine_cfg.max_eval.min(budget - spent)
        };
        let iter_cfg = if rung == 0 {
            SearchCfg {
                max_eval: cap,
                anytime,
                ..refine_cfg.with_cost_weight(cost_w)
            }
        } else {
            let (wg, wh) = PROFILES[rung - 1];
            SearchCfg {
                max_eval: cap,
                anytime,
                ..SearchCfg::from_weights(wg, wh, Some(cap)).with_cost_weight(cost_w)
            }
        };
        let (opt, evaluated, capped) = solve_subgoal_bounded(
            task,
            &init,
            &task.goal_pos,
            &task.goal_num,
            cost_fluent,
            bound,
            threads,
            iter_cfg,
            Some(&sat),
        );
        spent += evaluated;
        match opt {
            Some(ops) => {
                escalated = false;
                rung = 0; // an improvement re-arms the ladder
                let cost = plan_cost(task, &ops, cost_fluent);
                best = Some((ops, cost));
                if cost <= 0.0 {
                    proven = true; // cannot beat zero cost
                    break;
                }
                bound = cost; // next plan must be strictly cheaper (prune cost >= bound)
            }
            None => {
                // A capped failure is inconclusive: diversify the open-list
                // order first (same bound, different region), then retry the
                // same bound with all remaining budget (see the closure
                // loop's rationale). Proven optimal IFF a retry-exempt
                // failure exhausted the space un-capped.
                if capped && !no_escalate {
                    if restarts && rung < PROFILES.len() && budget > spent {
                        rung += 1;
                        continue;
                    }
                    if budget.saturating_sub(spent) > cap {
                        escalated = true;
                        rung = 0;
                        continue;
                    }
                }
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

/// The shared deterministic tightening budget (evaluated-state count) for both
/// metric B&B loops — never wall-clock, so results are thread-independent.
fn pref_eval_budget() -> usize {
    std::env::var("FF_PREF_EVAL_BUDGET")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(2_000_000)
}

/// PARTITIONED CLOSURE SEED — ESPC increment 3, generalized past deadline
/// pairs: compose a high-quality incumbent from per-component stages BEFORE
/// the monolithic tightening loop runs. Measured motivation: the remaining
/// tpp/pathways/trucks tails are DIRECTION-bound (identical metrics at 4× the
/// eval budget), so the fix is a structurally different plan constructor, not
/// more budget. The construction:
///
/// 1. Candidate selection — for every unsatisfied preference, price its
///    cheapest positive disjunct with a cost-aware relaxed plan from the
///    initial state (`heuristic::relaxed_plan_cost`); keep it iff the estimate
///    does not exceed its violation weight (deliverable at a profit). Real
///    hard-goal facts enter as MANDATORY candidates.
/// 2. Components — union-find over the candidates' facts through the
///    invariant-synthesis mutex variables (two candidates interact iff their
///    facts share a variable; ungrouped facts are private). Needs ≥ 2
///    components to differ from the monolithic path.
/// 3. Composition — one P3-masked, satisfaction-guided stage per component on
///    the evolving state (deterministic order: min fact id). Mandatory facts
///    of DONE components are protected (ops deleting them are forbidden). An
///    infeasible stage drops its priciest optional preference and retries;
///    a stage that cannot even meet its mandatory facts aborts the seed.
/// 4. The exact phase tail closes the bookkeeping and the composed plan
///    becomes the tightening loop's starting incumbent iff it beats the
///    init-tail one. Stage evals are charged against the SAME deterministic
///    budget the loop spends.
///
/// `FF_PREF_MONO=1` disables the composed seed (monolithic path, bit-compat).
#[allow(clippy::too_many_arguments)]
fn compose_pref_seed(
    task: &PackedTask,
    cost_fluent: usize,
    groups: &[Vec<u32>],
    forgos: &[(usize, f64)],
    tail: &PhaseTail,
    sat: &SatGuidance,
    p3_mask: &[bool],
    threads: usize,
    cfg: SearchCfg,
    budget: usize,
) -> Option<(Vec<usize>, f64, usize)> {
    use crate::types::eval_numpre;
    let init = task.initial();
    let mut spent = 0usize;

    // 1. Candidates: mandatory real goals + profitably-satisfiable preferences.
    struct Cand {
        pos: Vec<u32>,
        num: Vec<crate::types::NumPre>,
        est: f64,
        value: f64, // weight - est (mandatory: +inf); the mutex-conflict tiebreak
        mandatory: bool,
    }
    let mut cands: Vec<Cand> = Vec::new();
    for &g in task.goal_pos.iter().filter(|&&f| {
        !task.fact_names[f as usize]
            .to_ascii_uppercase()
            .starts_with("(P3")
    }) {
        cands.push(Cand {
            pos: vec![g],
            num: Vec::new(),
            est: 0.0,
            value: f64::INFINITY,
            mandatory: true,
        });
    }
    let collect = collect_ops(task);
    let mut sc = crate::heuristic::Scratch::new(task);
    for (i, _) in forgos.iter().enumerate() {
        let Some(ops_i) = collect.get(&i) else {
            continue;
        };
        let weight = forgos[i].1;
        let mut cheapest: Option<(Vec<u32>, Vec<crate::types::NumPre>, f64)> = None;
        let mut already = false;
        for &oi in ops_i {
            let pos: Vec<u32> = task
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
            let num = task.pre_num.slice(oi).to_vec();
            let true_now = pos
                .iter()
                .all(|&f| crate::bitset::test(&init.bits, f as usize))
                && num
                    .iter()
                    .all(|np| eval_numpre(np, &init.fv, &init.fdef) == Some(true));
            if true_now {
                already = true; // phi holds at init: nothing to chase
                break;
            }
            let est = crate::heuristic::relaxed_plan_cost(
                task,
                &mut sc,
                &init.bits,
                &init.fv,
                &init.fdef,
                &pos,
                &num,
                cost_fluent,
            )
            .unwrap_or(f64::INFINITY);
            if cheapest.as_ref().map_or(true, |(_, _, c)| est < *c) {
                cheapest = Some((pos, num, est));
            }
        }
        if already {
            continue;
        }
        if let Some((pos, num, est)) = cheapest {
            if est <= weight && !pos.is_empty() {
                cands.push(Cand {
                    pos,
                    num,
                    est,
                    value: weight - est,
                    mandatory: false,
                });
            }
        }
    }
    let dbg = std::env::var("FF_RES_DEBUG").is_ok();
    if cands.len() < 2 {
        if dbg {
            eprintln!("[seed3] skip: {} candidate(s)", cands.len());
        }
        return None;
    }

    let mut var_of: crate::hash::FxHashMap<u32, usize> = crate::hash::FxHashMap::default();
    for (gi, g) in groups.iter().enumerate() {
        for &f in g {
            var_of.insert(f, gi);
        }
    }

    // 1b. Mutex-conflict pruning: two OPTIONAL candidates claiming DIFFERENT
    // facts of the same mutex group are jointly unsatisfiable at the end
    // (tpp's per-goods `stored g level1..4` ladder — at most one level holds),
    // so a stage goal containing both is infeasible by construction and only
    // burns budget. Keep the best-value claimant per (group, distinct-fact)
    // conflict; the tail forgoes the dropped ones. Deterministic (groups in
    // index order, min-index tiebreak). Mandatory candidates always win.
    {
        let mut claimed: crate::hash::FxHashMap<usize, (u32, usize)> =
            crate::hash::FxHashMap::default();
        let mut drop = vec![false; cands.len()];
        for ci in 0..cands.len() {
            for k in 0..cands[ci].pos.len() {
                let f = cands[ci].pos[k];
                let Some(&gi) = var_of.get(&f) else { continue };
                match claimed.entry(gi) {
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert((f, ci));
                    }
                    std::collections::hash_map::Entry::Occupied(mut o) => {
                        let (held_f, held_ci) = *o.get();
                        if held_f == f || drop[held_ci] {
                            if drop[held_ci] {
                                o.insert((f, ci));
                            }
                            continue; // same fact: compatible
                        }
                        // Different facts of one group: keep the better value.
                        let (a, b) = (held_ci, ci);
                        let better_b = cands[b].value.partial_cmp(&cands[a].value)
                            == Some(std::cmp::Ordering::Greater);
                        if better_b {
                            drop[a] = true;
                            o.insert((f, b));
                        } else {
                            drop[b] = true;
                        }
                    }
                }
            }
        }
        let before = cands.len();
        let mut keep = drop.iter().map(|d| !d);
        cands.retain(|_| keep.next().unwrap());
        if dbg && cands.len() != before {
            eprintln!(
                "[seed3] mutex pruning: {before} -> {} candidate(s)",
                cands.len()
            );
        }
        if cands.len() < 2 {
            return None;
        }
    }

    // 2. Union-find through mutex variables.
    let var = |f: u32| var_of.get(&f).copied().unwrap_or(groups.len() + f as usize);
    let mut uf: Vec<usize> = (0..cands.len()).collect();
    fn find(uf: &mut [usize], mut x: usize) -> usize {
        while uf[x] != x {
            uf[x] = uf[uf[x]];
            x = uf[x];
        }
        x
    }
    let mut owner: crate::hash::FxHashMap<usize, usize> = crate::hash::FxHashMap::default();
    for (ci, cand) in cands.iter().enumerate() {
        for &f in &cand.pos {
            let v = var(f);
            match owner.entry(v) {
                std::collections::hash_map::Entry::Occupied(o) => {
                    let a = find(&mut uf, ci);
                    let b = find(&mut uf, *o.get());
                    uf[a.max(b)] = a.min(b);
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(ci);
                }
            }
        }
    }
    let mut comp_of: crate::hash::FxHashMap<usize, Vec<usize>> = crate::hash::FxHashMap::default();
    for ci in 0..cands.len() {
        let r = find(&mut uf, ci);
        comp_of.entry(r).or_default().push(ci);
    }
    if comp_of.len() < 2 {
        if dbg {
            eprintln!(
                "[seed3] skip: {} candidate(s) collapse into {} component(s)",
                cands.len(),
                comp_of.len()
            );
        }
        return None;
    }
    let mut comps: Vec<Vec<usize>> = comp_of.into_values().collect();
    comps.sort_by_key(|members| {
        members
            .iter()
            .flat_map(|&ci| cands[ci].pos.iter().copied())
            .min()
            .unwrap_or(u32::MAX)
    });

    // 3. Compose: one protected, sat-guided stage per component. Stage
    // attempts run at a TENTH of the loop's per-probe cap — a stage that
    // needs more is not composing cheaply and gets its priciest preference
    // dropped instead — and the whole composition may spend at most a
    // QUARTER of the budget (an infeasible-joint-goal component would
    // otherwise burn the tightening loop's entire allowance on retries).
    let stage_cap = (cfg.max_eval / 10).max(1_000);
    let seed_budget = budget / 4;
    let mut state = init.clone();
    let mut plan: Vec<usize> = Vec::new();
    let mut protected: crate::hash::FxHashSet<u32> = crate::hash::FxHashSet::default();
    for members in &comps {
        let mut alive: Vec<usize> = members.clone();
        loop {
            // Facts still to achieve for the alive members in the CURRENT state.
            let satisfied = |ci: usize| {
                cands[ci]
                    .pos
                    .iter()
                    .all(|&f| crate::bitset::test(&state.bits, f as usize))
                    && cands[ci]
                        .num
                        .iter()
                        .all(|np| eval_numpre(np, &state.fv, &state.fdef) == Some(true))
            };
            alive.retain(|&ci| !satisfied(ci));
            if alive.is_empty() {
                break;
            }
            let mut goal: Vec<u32> = alive
                .iter()
                .flat_map(|&ci| cands[ci].pos.iter().copied())
                .collect();
            goal.sort_unstable();
            goal.dedup();
            let nums: Vec<crate::types::NumPre> = alive
                .iter()
                .flat_map(|&ci| cands[ci].num.iter().cloned())
                .collect();
            let forbidden: Vec<bool> = (0..task.n_ops)
                .map(|oi| {
                    p3_mask.get(oi).copied().unwrap_or(false)
                        || task.del.slice(oi).iter().any(|f| protected.contains(f))
                })
                .collect();
            if spent >= seed_budget {
                if dbg {
                    eprintln!("[seed3] abort: seed budget exhausted mid-composition");
                }
                return None;
            }
            let stage_cfg = SearchCfg {
                max_eval: stage_cap.min(seed_budget - spent),
                ..cfg
            };
            let (ops, evaluated) = crate::search::solve_subgoal_guided(
                task,
                &state,
                &goal,
                &nums,
                &forbidden,
                threads,
                stage_cfg,
                Some(sat),
            );
            spent += evaluated;
            match ops {
                Some(ops) => {
                    for &oi in &ops {
                        state = task.apply(oi, &state);
                    }
                    plan.extend(ops);
                    break;
                }
                None => {
                    // Drop the priciest optional member and retry; a stage that
                    // cannot meet its mandatory facts sinks the whole seed.
                    let worst = alive
                        .iter()
                        .copied()
                        .filter(|&ci| !cands[ci].mandatory)
                        .max_by(|&a, &b| {
                            cands[a]
                                .est
                                .partial_cmp(&cands[b].est)
                                .unwrap_or(std::cmp::Ordering::Equal)
                                .then(a.cmp(&b))
                        });
                    match worst {
                        Some(w) => alive.retain(|&ci| ci != w),
                        None => {
                            if dbg {
                                eprintln!("[seed3] abort: mandatory facts infeasible in a stage");
                            }
                            return None;
                        }
                    }
                }
            }
        }
        for &ci in members {
            if cands[ci].mandatory {
                protected.extend(cands[ci].pos.iter().copied());
            }
        }
    }

    // 4. Close the bookkeeping and validate the composed plan.
    let Some(tail_ops) = apply_tail(task, &mut state, tail) else {
        if dbg {
            eprintln!("[seed3] abort: phase tail failed on the composed state");
        }
        return None;
    };
    plan.extend(tail_ops);
    if !task.goal_met_with(&state, &task.goal_pos, &task.goal_num) {
        if dbg {
            eprintln!("[seed3] abort: composed state fails the full goal");
        }
        return None; // never expected; an invalid seed must not become the incumbent
    }
    let cost = plan_cost(task, &plan, cost_fluent);
    Some((plan, cost, spent))
}

/// The exact-closure metric optimizer (see `metric_optimize` step 2): anytime
/// B&B where each iteration searches REAL states under a metric-bounded
/// acceptance test (`cost-so-far + closure(state) < bound`, closure = the exact
/// weight the phase tail will forgo), then appends the tail. Every valid
/// compiled plan is a real prefix + `P3END` + a collect/forgo permutation whose
/// optimal closure IS the tail, so un-capped exhaustion proves optimality.
///
/// The first incumbent is the tail applied directly to the initial state
/// (whenever the real hard goal already holds there — always, on the pure-
/// preference IPC-5 tracks), so even the largest instances report a metric
/// instantly. The tightening budget is a DETERMINISTIC evaluated-state count
/// (`FF_PREF_EVAL_BUDGET`, default 2M) — never wall-clock — so results are
/// thread-count independent. Returns `None` (→ legacy fallback) only when no
/// incumbent could be produced at all.
#[allow(clippy::too_many_arguments)]
fn metric_optimize_closure(
    task: &PackedTask,
    cost_fluent: usize,
    forgos: &[(usize, f64)],
    tail: &PhaseTail,
    sat: &SatGuidance,
    groups: &[Vec<u32>],
    threads: usize,
    cfg: SearchCfg,
) -> Option<MetricResult> {
    const MAX_ITERS: usize = 10_000;
    let real_pos: Vec<u32> = task
        .goal_pos
        .iter()
        .copied()
        .filter(|&f| {
            !task.fact_names[f as usize]
                .to_ascii_uppercase()
                .starts_with("(P3")
        })
        .collect();
    let closure = build_closure_cost(task, forgos);
    // The search explores real states only: every synthetic op is masked.
    let forbidden: Vec<bool> = (0..task.n_ops)
        .map(|oi| {
            let n = task.op_display[oi].to_ascii_uppercase();
            n == "P3END" || n.starts_with("P3COLLECT-") || n.starts_with("P3FORGO-")
        })
        .collect();
    let init = task.initial();

    // Trivial incumbent: close the initial state directly. Instant coverage —
    // this is what puts storage p05-p08 on the board at all.
    let mut best: Option<(Vec<usize>, f64)> = None;
    if task.goal_met_with(&init, &real_pos, &task.goal_num) {
        let mut s = init.clone();
        if let Some(tail_ops) = apply_tail(task, &mut s, tail) {
            if task.goal_met_with(&s, &task.goal_pos, &task.goal_num) {
                let cost = plan_cost(task, &tail_ops, cost_fluent);
                if cost <= 0.0 {
                    return Some(MetricResult {
                        ops: tail_ops,
                        cost,
                        iterations: 0,
                        proven: true,
                    });
                }
                best = Some((tail_ops, cost));
            }
        }
    }

    let budget = pref_eval_budget();
    let mut spent = 0usize;
    let mut iterations = 0usize;
    let mut proven = false;

    // PARTITIONED CLOSURE SEED (increment 3) — experimental, opt-in via
    // `FF_PREF_SEED3=1`, default OFF after measuring NEUTRAL (2026-07): with
    // mutex-conflict pruning the composition genuinely works (tpp p05 composes
    // a 99 incumbent vs the init-tail 105; p07 120 vs 135; pathways p05 9 vs
    // 10.2) — but the anytime+ladder tightening loop reaches the same final
    // metric from either starting bound on every instance measured, and an
    // aborted composition wastes up to a quarter of the budget. The mechanism
    // is kept as the substrate for real per-stage λ pricing (the un-built rest
    // of increment 3); composition-as-seeding alone does not move finals.
    if std::env::var("FF_PREF_SEED3").is_ok() && task.goal_num.is_empty() {
        let dbg = std::env::var("FF_RES_DEBUG").is_ok();
        match compose_pref_seed(
            task,
            cost_fluent,
            groups,
            forgos,
            tail,
            sat,
            &forbidden,
            threads,
            cfg,
            budget,
        ) {
            Some((ops, cost, evals)) => {
                spent += evals;
                if best.as_ref().map_or(true, |(_, c)| cost < *c) {
                    if dbg {
                        eprintln!(
                            "[seed3] composed incumbent {cost} (was {:?}), {evals} evals",
                            best.as_ref().map(|(_, c)| *c)
                        );
                    }
                    if cost <= 0.0 {
                        return Some(MetricResult {
                            ops,
                            cost,
                            iterations: 0,
                            proven: true,
                        });
                    }
                    best = Some((ops, cost));
                } else if dbg {
                    eprintln!(
                        "[seed3] composed {cost} LOST to incumbent {:?} ({evals} evals)",
                        best.as_ref().map(|(_, c)| *c)
                    );
                }
            }
            None => {
                if dbg {
                    eprintln!("[seed3] no composition");
                }
            }
        }
    }
    let mut bound = best.as_ref().map_or(f64::INFINITY, |(_, c)| *c);

    // ESCALATION: a tightening probe that hits its per-iteration eval cap
    // without finding a cheaper plan is INCONCLUSIVE, not a reason to abandon
    // the optimization — with budget remaining, retry the SAME bound with ALL
    // of it (not a doubling ladder: a retry at the same bound+cfg re-treads a
    // deterministic prefix, so intermediate rungs only re-pay it; `evaluated`
    // is actual usage, so a large cap that succeeds early costs only what it
    // used). This is what makes FF_PREF_EVAL_BUDGET the real contract — before
    // it, one failed 300k sweep ended the loop with millions unspent. All
    // quantities are deterministic eval counts, so t1≡t8 is preserved.
    // `FF_PREF_NO_ESCALATE=1` restores break-on-first-capped-failure.
    //
    // ANYTIME TIGHTENING (see `SearchCfg::anytime`): each sweep tightens its
    // bound in place on every acceptance and keeps draining, so a restart (and
    // its deterministic prefix re-tread) happens once per CAP instead of once
    // per improvement. `FF_PREF_GREEDY=1` restores first-improvement sweeps.
    //
    // DIVERSIFIED RESTART LADDER: a capped no-improvement sweep is not just
    // inconclusive — it says the current h-ordering cannot reach a better
    // plan in this region (measured: pouring the whole budget into the same
    // direction re-treads the same prefix and changes nothing). Before the
    // final all-remaining escalation, rotate the open-list weights through a
    // fixed profile ladder — each rung orders the frontier differently
    // (h-greedier / g-heavier / pure-h), exploring a genuinely different
    // region under the SAME bound. Fully deterministic (fixed profiles, eval-
    // count budgets); an improvement resets the ladder to the default rung.
    // `FF_PREF_NO_RESTARTS=1` disables the ladder alone.
    const PROFILES: &[(f64, f64)] = &[(1.0, 2.0), (1.0, 8.0), (2.0, 3.0), (0.0, 1.0)];
    let no_escalate = std::env::var("FF_PREF_NO_ESCALATE").is_ok();
    let anytime = std::env::var("FF_PREF_GREEDY").is_err();
    let restarts = anytime && std::env::var("FF_PREF_NO_RESTARTS").is_err();
    let mut escalated = false;
    let mut rung = 0usize; // 0 = default profile; 1..=len = PROFILES
    while iterations < MAX_ITERS && spent < budget {
        iterations += 1;
        let cap = if escalated {
            budget - spent
        } else if rung > 0 {
            // Diversification rungs run at HALF the probe cap: they exist to
            // find a different region fast, and the budget they don't spend
            // is what keeps the final full-budget escalation strong (measured:
            // full-size rungs starved it and gave back tpp p04 / trucks p07).
            (cfg.max_eval / 2).min(budget - spent)
        } else {
            cfg.max_eval.min(budget - spent)
        };
        let iter_cfg = if rung == 0 {
            SearchCfg {
                max_eval: cap,
                anytime,
                ..cfg
            }
        } else {
            let (wg, wh) = PROFILES[rung - 1];
            SearchCfg {
                max_eval: cap,
                anytime,
                w_c: cfg.w_c,
                ..SearchCfg::from_weights(wg, wh, Some(cap))
            }
        };
        let (opt, evaluated, capped) = crate::search::solve_closure_bounded(
            task,
            &real_pos,
            &task.goal_num,
            cost_fluent,
            bound,
            &closure,
            &forbidden,
            threads,
            iter_cfg,
            Some(sat),
        );
        spent += evaluated;
        match opt {
            Some(mut ops) => {
                escalated = false;
                rung = 0; // an improvement re-arms the whole ladder
                let mut s = init.clone();
                for &oi in &ops {
                    s = task.apply(oi, &s);
                }
                let Some(tail_ops) = apply_tail(task, &mut s, tail) else {
                    break; // never expected (P3 ops are masked); keep the incumbent
                };
                if !task.goal_met_with(&s, &task.goal_pos, &task.goal_num) {
                    break; // safety: an invalid composition must not become the incumbent
                }
                ops.extend(tail_ops);
                let cost = plan_cost(task, &ops, cost_fluent);
                best = Some((ops, cost));
                if cost <= 0.0 {
                    proven = true;
                    break;
                }
                bound = cost;
            }
            None => {
                if capped && !no_escalate {
                    // Diversify first: same bound, different open-list order.
                    if restarts && rung < PROFILES.len() && budget > spent {
                        rung += 1;
                        continue;
                    }
                    if budget.saturating_sub(spent) > cap {
                        escalated = true; // ladder spent: all remaining budget, default order
                        rung = 0;
                        continue;
                    }
                }
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

/// `P3COLLECT-i` op ids per preference index (ascending) — the shared scan
/// behind the satisfaction guidance, the deadline guidance, the phase tail,
/// and the closure cost. A preference whose phi's DNF has several disjuncts
/// grounds to SEVERAL ops all named `P3COLLECT-i` — one per disjunct — so the
/// value is a Vec: phi holds iff ANY of them is applicable. (A single-op map
/// here once silently kept an arbitrary disjunct, which would make the tail
/// forgo satisfied preferences on `imply`/`exists` phis.)
fn collect_ops(task: &PackedTask) -> std::collections::HashMap<usize, Vec<usize>> {
    let mut collect_op: std::collections::HashMap<usize, Vec<usize>> =
        std::collections::HashMap::new();
    for oi in 0..task.n_ops {
        if let Some(rest) = task.op_display[oi]
            .to_ascii_uppercase()
            .strip_prefix("P3COLLECT-")
        {
            if let Ok(i) = rest.trim().parse::<usize>() {
                collect_op.entry(i).or_default().push(oi);
            }
        }
    }
    collect_op
}

/// Op ids for the deterministic post-search **phase tail**: `P3END` freezes the
/// state, then each preference is closed in fixed order — its first applicable
/// `P3COLLECT-i` disjunct op when phi holds (free), else `P3FORGO-i` (pays the
/// weight). Exact, not heuristic: after `P3END` the state is frozen and each
/// preference's collected fact is independent, so collect-iff-applicable is the
/// optimal closure of whatever final state the search reached. Used by the
/// default closure-metric optimizer and the partitioned-ESPC composition.
/// `None` only when the compile has no `P3END` (not a preference task).
pub struct PhaseTail {
    pub end_op: usize,
    /// `(collect_ops [one per phi disjunct, possibly empty = always-forgo],
    /// forgo_op)` per preference, in preference order.
    pub prefs: Vec<(Vec<usize>, usize)>,
}

pub(crate) fn build_phase_tail(task: &PackedTask, forgos: &[(usize, f64)]) -> Option<PhaseTail> {
    let end_op = (0..task.n_ops).find(|&oi| task.op_display[oi].eq_ignore_ascii_case("P3END"))?;
    let mut collect = collect_ops(task);
    let mut prefs = Vec::with_capacity(forgos.len());
    for (i, &(forgo_op, _)) in forgos.iter().enumerate() {
        prefs.push((collect.remove(&i).unwrap_or_default(), forgo_op));
    }
    Some(PhaseTail { end_op, prefs })
}

/// Build the exact closure-cost table ([`ClosureCost`]) from the compiled
/// `P3COLLECT-i` ops: one DNF disjunct per collect op (its positive
/// precondition minus the `P3*` control facts, plus its numeric precondition),
/// weighted by the preference's forgo cost. Zero-weight preferences are
/// omitted — forgoing them is free, so they never contribute to the metric.
pub(crate) fn build_closure_cost(task: &PackedTask, forgos: &[(usize, f64)]) -> ClosureCost {
    let mut collect = collect_ops(task);
    let mut prefs = Vec::new();
    for (i, &(_, weight)) in forgos.iter().enumerate() {
        if weight <= 0.0 {
            continue;
        }
        let disjuncts: Vec<(Vec<u32>, Vec<crate::types::NumPre>)> = collect
            .remove(&i)
            .unwrap_or_default()
            .into_iter()
            .map(|oi| {
                let pos: Vec<u32> = task
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
                (pos, task.pre_num.slice(oi).to_vec())
            })
            .collect();
        prefs.push((weight, PrefPhi { disjuncts }));
    }
    ClosureCost { prefs }
}

/// Close the preference bookkeeping on `state` with the exact phase tail: apply
/// `P3END` (freezing the planning phase), then per preference in fixed order the
/// first applicable `P3COLLECT-i` disjunct (free) else `P3FORGO-i` (pays the
/// weight). Returns the tail ops and advances `state` through them. `None` when
/// an op is inapplicable (e.g. a searched plan already fired `P3END`) — callers
/// treat the composition as invalid and fall back, so this can't corrupt a plan.
pub(crate) fn apply_tail(
    task: &PackedTask,
    state: &mut crate::packed::State,
    tail: &PhaseTail,
) -> Option<Vec<usize>> {
    let mut ops = Vec::with_capacity(1 + tail.prefs.len());
    if !task.op_applicable(tail.end_op, state) {
        return None;
    }
    *state = task.apply(tail.end_op, state);
    ops.push(tail.end_op);
    for (collects, forgo) in &tail.prefs {
        let oi = collects
            .iter()
            .copied()
            .find(|&c| task.op_applicable(c, state))
            .unwrap_or(*forgo);
        if !task.op_applicable(oi, state) {
            return None;
        }
        *state = task.apply(oi, state);
        ops.push(oi);
    }
    Some(ops)
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

/// Build the metric satisfaction guidance: for each preference, its full phi
/// in DNF ([`PrefPhi`], one disjunct per `P3COLLECT-i` op — so `imply`/`exists`
/// preferences guide correctly) and a heap penalty scaled from its forgo
/// weight. Two exclusions keep the gradient honest:
/// - phi unachievable (no collect ops) or trivially true — a constant penalty
///   can't order anything;
/// - phi already satisfied in the INITIAL state (unless `FF_PREF_BARRIER=1`) —
///   penalizing its transient dips erects a wall in front of every improving
///   trajectory (tpp: the weight-16 `p4A` must dip during any real delivery),
///   while its real protection is the exact closure acceptance on the FINAL
///   state. Guidance should pull toward the not-yet-earned, not punish transit.
fn build_sat_guidance(task: &PackedTask, forgos: &[(usize, f64)]) -> SatGuidance {
    let mut collect_op = collect_ops(task);
    let init = task.initial();
    let keep_barrier = std::env::var("FF_PREF_BARRIER").is_ok();
    let mut prefs = Vec::new();
    for (i, (_, weight)) in forgos.iter().enumerate() {
        let disjuncts: Vec<(Vec<u32>, Vec<crate::types::NumPre>)> = collect_op
            .remove(&i)
            .unwrap_or_default()
            .into_iter()
            .map(|oi| {
                let pos: Vec<u32> = task
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
                (pos, task.pre_num.slice(oi).to_vec())
            })
            .collect();
        if disjuncts.is_empty() || disjuncts.iter().any(|(p, n)| p.is_empty() && n.is_empty()) {
            continue; // unachievable or trivially-true phi: constant, can't guide
        }
        let phi = PrefPhi { disjuncts };
        if !keep_barrier && phi.holds(&init) {
            continue; // init-satisfied: no barrier on its transient dips
        }
        prefs.push((phi, (weight * 100.0).round().max(1.0) as i64));
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
        let Some(ops) = collect_op.get(&i) else {
            continue;
        };
        let w = (*weight).round().max(1.0) as i64;
        // union of the pref's disjunct facts, deduped so a fact shared by
        // several disjuncts of the SAME pref is valued once (single-disjunct
        // phi — openstacks — is unchanged).
        let mut facts: Vec<u32> = ops
            .iter()
            .flat_map(|&oi| task.pre_pos.slice(oi))
            .copied()
            .filter(|&f| {
                !task.fact_names[f as usize]
                    .to_ascii_uppercase()
                    .starts_with("(P3")
            })
            .collect();
        facts.sort_unstable();
        facts.dedup();
        for f in facts {
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
