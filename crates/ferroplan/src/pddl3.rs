//! PDDL3.0 soft-goal preferences + metric optimization (phase 1).
//!
//! Compilation (Keyder & Geffner, "Soft goals can be compiled away", JAIR 2009):
//! each `(preference p phi)` in the goal becomes
//!   - a 0-ary fact `collected_p`,
//!   - `collect_p`: precond `phi`, effect `collected_p`, cost 0   (satisfy it), and
//!   - `forgo_p`:   effect `collected_p` + `(increase (total-cost) w_p)`  (pay to skip),
//! with `collected_p` added to the HARD goal. Minimizing `total-cost` then
//! minimizes the weighted preference violation (+ any existing action costs).
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
use crate::types::{Action, AssignOp, Domain, Effect, Expr, Formula, MetricDir, Problem, Term};

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
            .map_or(false, |(_, e)| expr_has_is_violated(e))
}

/// Split a goal into hard conjuncts and (name, formula) preferences.
fn split_goal(g: &Formula, hard: &mut Vec<Formula>, prefs: &mut Vec<(String, Formula)>, ctr: &mut usize) {
    match g {
        Formula::And(v) => v.iter().for_each(|f| split_goal(f, hard, prefs, ctr)),
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

/// Extract the (name, formula) preferences from a goal (for independent scoring).
pub fn preferences(goal: &Formula) -> Vec<(String, Formula)> {
    let mut hard = Vec::new();
    let mut prefs = Vec::new();
    let mut ctr = 0;
    split_goal(goal, &mut hard, &mut prefs, &mut ctr);
    prefs
}

/// Effective metric weight per preference name (default 1 with no metric, else
/// the `(is-violated name)` coefficient, 0 if unreferenced).
pub fn pref_weights(problem: &Problem) -> HashMap<String, f64> {
    let mut w = HashMap::new();
    let mut tc = 0.0;
    let mut other = false;
    let absent = problem.metric.is_none();
    if let Some((_, e)) = &problem.metric {
        extract(e, 1.0, &mut w, &mut tc, &mut other);
    }
    let mut out = HashMap::new();
    for (n, _) in preferences(&problem.goal) {
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
                let good = matches!(op, AssignOp::Increase) && matches!(val, Expr::Num(n) if *n >= 0.0);
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
    let mut hard = Vec::new();
    let mut prefs = Vec::new();
    let mut ctr = 0;
    split_goal(&problem.goal, &mut hard, &mut prefs, &mut ctr);

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
        let raw = w.get(name).copied().unwrap_or(if metric_absent { 1.0 } else { 0.0 });
        if raw < 0.0 {
            any_negative = true;
        }
        d.actions.push(Action {
            name: forgo,
            params: vec![],
            precond: Formula::Atom(ENDED.to_string(), vec![]),
            effect: Effect::And(vec![
                Effect::Add(col.clone(), vec![]),
                Effect::Num(AssignOp::Increase, COST.to_string(), vec![], Expr::Num(raw.max(0.0))),
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
    } else if any_negative {
        Some("negative preference weight (cannot be encoded monotonically)".into())
    } else if !(tc == 0.0 || tc == 1.0) {
        Some(format!("scaled total-cost coefficient ({}) is not supported", tc))
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
pub fn metric_optimize(task: &PackedTask, cost_fluent: usize, threads: usize) -> Option<MetricResult> {
    const MAX_ITERS: usize = 10_000;
    let init = task.initial();
    let mut bound = f64::INFINITY;
    let mut best: Option<(Vec<usize>, f64)> = None;
    let mut iterations = 0;
    let mut proven = false;

    while iterations < MAX_ITERS {
        iterations += 1;
        let (opt, capped) =
            solve_subgoal_bounded(task, &init, &task.goal_pos, &task.goal_num, cost_fluent, bound, threads);
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
    best.map(|(ops, cost)| MetricResult { ops, cost, iterations, proven })
}
