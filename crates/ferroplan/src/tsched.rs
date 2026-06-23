//! Concurrent scheduling phase for temporal plans.
//!
//! ferroplan's temporal search finds *what* to do but lays the actions out
//! sequentially (it's guided by action count, not makespan) — so more workers never
//! shortened the schedule. This pass takes a found [`TimedPlan`] and **repacks it
//! onto the domain's actor-objects**: one job per actor at a time, each action
//! starting as early as its inputs (consumed resources) and prerequisites (the
//! build-order predicates) allow. Independent work then overlaps across actors, so
//! more actors ⇒ shorter makespan — the parallelism the planner couldn't show.
//!
//! It's the planner's *scheduling* phase (search = causal structure, this = who does
//! what, when), gated by [`crate::features::tconc`]. The result is always run through
//! [`crate::temporal::validate`]; if the concurrent schedule doesn't validate (or
//! isn't shorter), the original sequential plan is returned unchanged. So it can only
//! help, never produce a wrong plan.
//!
//! **Convention.** The *actor* is the first parameter of each durative action (e.g.
//! `(?w - worker …)`); actors are the problem objects of that type. Effects that
//! depend on *which* actor (a `(when (lumberjack ?w) …)`) would change when an action
//! is reassigned, so a domain meant for this pass keeps workers interchangeable.

use crate::temporal::{validate, TimedPlan, TimedStep};
use crate::types::{AssignOp, Domain, Effect, Formula, Problem, Sym, Term, TimeSpec};
use std::collections::HashMap;

const EPS: f64 = 0.001;

/// Repack `plan` onto the domain's actor objects to minimise makespan. Returns a
/// shorter, validated concurrent plan, or `None` to keep the original.
pub fn reschedule(domain: &Domain, problem: &Problem, plan: &TimedPlan) -> Option<TimedPlan> {
    if plan.steps.is_empty() {
        return None;
    }
    let actor_type = actor_type(domain)?;
    let actors: Vec<String> = problem
        .objects
        .iter()
        .filter(|(_, t)| up(t) == up(&actor_type))
        .map(|(o, _)| up(o))
        .collect();
    if actors.len() < 2 {
        return None; // a single actor can't overlap — nothing to gain
    }
    let by_name: HashMap<String, &crate::types::DurativeAction> = domain
        .durative_actions
        .iter()
        .map(|da| (up(&da.name), da))
        .collect();

    // running resource-event timeline and fact-availability times
    let mut events: Vec<(f64, String, f64)> = Vec::new(); // (time, resource-key, delta)
    let init_res: HashMap<String, f64> = problem
        .init_fluents
        .iter()
        .map(|((f, a), v)| (res_key(f, a), *v))
        .collect();
    let mut fact_ready: HashMap<String, f64> = problem
        .init_atoms
        .iter()
        .map(|(p, a)| (atom_key(p, a), 0.0))
        .collect();
    let mut actor_free = vec![0.0f64; actors.len()];

    let balance = |events: &[(f64, String, f64)], key: &str, t: f64| -> f64 {
        init_res.get(key).copied().unwrap_or(0.0)
            + events
                .iter()
                .filter(|(et, k, _)| k == key && *et <= t + EPS / 2.0)
                .map(|(_, _, d)| d)
                .sum::<f64>()
    };

    let mut out: Vec<TimedStep> = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let mut it = step.action.split_whitespace();
        let head = up(it.next().unwrap_or(""));
        let args: Vec<String> = it.map(up).collect();
        let dur = step.duration.unwrap_or(0.0);

        // Non-durative or unknown action: keep it where it is (rare in a crew domain).
        let Some(da) = by_name.get(&head) else {
            out.push(step.clone());
            continue;
        };
        if da.params.is_empty() || up(&da.params[0].1) != up(&actor_type) {
            out.push(step.clone());
            continue;
        }
        let actor_var = up(&da.params[0].0);
        // bind non-actor params positionally (actor param left to the scheduler)
        let mut bind: HashMap<String, String> = HashMap::new();
        for (i, (p, _)) in da.params.iter().enumerate() {
            if let Some(a) = args.get(i) {
                bind.insert(up(p), a.clone());
            }
        }

        let consumes = collect_num(da, TimeSpec::Start, AssignOp::Decrease, &bind);
        let produces = collect_num(da, TimeSpec::End, AssignOp::Increase, &bind);
        let prereqs = collect_atoms(da, TimeSpec::Start, &bind, &actor_var);
        let adds = collect_added(da, TimeSpec::End, &bind, &actor_var);

        // earliest the prerequisites (build-order predicates) hold
        let prereq_t = prereqs
            .iter()
            .map(|a| fact_ready.get(a).copied().unwrap_or(0.0))
            .fold(0.0f64, f64::max);
        // pick the actor that frees earliest
        let ai = (0..actor_free.len())
            .min_by(|&a, &b| actor_free[a].total_cmp(&actor_free[b]))
            .unwrap();
        let lb = prereq_t.max(actor_free[ai]);

        // earliest time ≥ lb at which every consumed resource has enough balance
        let mut cands: Vec<f64> = events
            .iter()
            .map(|(t, _, _)| *t)
            .filter(|t| *t >= lb - EPS)
            .collect();
        cands.push(lb);
        cands.sort_by(f64::total_cmp);
        let start = cands
            .into_iter()
            .find(|&t| {
                consumes
                    .iter()
                    .all(|(k, amt)| balance(&events, k, t) >= *amt - 1e-6)
            })
            .unwrap_or(lb);

        // commit: consume at start, produce at end, occupy the actor, mark adds ready
        for (k, amt) in &consumes {
            events.push((start, k.clone(), -*amt));
        }
        let end = start + dur;
        for (k, amt) in &produces {
            events.push((end, k.clone(), *amt));
        }
        for a in &adds {
            let e = fact_ready.entry(a.clone()).or_insert(end);
            if end < *e {
                *e = end;
            }
        }
        actor_free[ai] = end;

        // emit the action with the chosen actor substituted into arg 0
        let mut new_args = args.clone();
        if !new_args.is_empty() {
            new_args[0] = actors[ai].clone();
        }
        out.push(TimedStep {
            time: start,
            action: if new_args.is_empty() {
                head.clone()
            } else {
                format!("{} {}", head, new_args.join(" "))
            },
            duration: step.duration,
        });
    }

    out.sort_by(|a, b| a.time.total_cmp(&b.time));
    let makespan = out
        .iter()
        .map(|s| s.time + s.duration.unwrap_or(0.0))
        .fold(0.0f64, f64::max);
    let rescheduled = TimedPlan {
        steps: out,
        makespan,
    };

    // only accept a genuine, *valid* improvement
    if makespan + EPS < plan.makespan && validate(domain, problem, &rescheduled).is_ok() {
        Some(rescheduled)
    } else {
        None
    }
}

/// A copy of `problem` keeping only the FIRST actor object (and dropping any init
/// facts that mention the others), so the causal search runs on a tractable, single-
/// actor reduction — the search is flaky with many symmetric actors, but a lone
/// worker can do the whole job sequentially. [`reschedule`] then distributes that
/// plan across the full crew. Returns the problem unchanged if there's no actor type
/// or fewer than two actors.
pub fn single_actor_problem(domain: &Domain, problem: &Problem) -> Problem {
    let Some(at) = actor_type(domain) else {
        return problem.clone();
    };
    let actors: Vec<&Sym> = problem
        .objects
        .iter()
        .filter(|(_, t)| up(t) == up(&at))
        .map(|(o, _)| o)
        .collect();
    if actors.len() < 2 {
        return problem.clone();
    }
    let keep = up(actors[0]);
    let dropped: std::collections::HashSet<String> = actors.iter().skip(1).map(|o| up(o)).collect();
    let mut p = problem.clone();
    p.objects.retain(|(o, t)| up(t) != up(&at) || up(o) == keep);
    p.init_atoms
        .retain(|(_, args)| !args.iter().any(|a| dropped.contains(&up(a))));
    p.init_fluents
        .retain(|((_, args), _)| !args.iter().any(|a| dropped.contains(&up(a))));
    p
}

/// The actor type = the first-parameter type shared by the durative actions.
fn actor_type(domain: &Domain) -> Option<Sym> {
    domain
        .durative_actions
        .iter()
        .find_map(|da| da.params.first().map(|(_, t)| t.clone()))
}

fn up(s: &str) -> String {
    s.to_ascii_uppercase()
}

fn res_key(fluent: &str, args: &[Sym]) -> String {
    if args.is_empty() {
        up(fluent)
    } else {
        format!(
            "{} {}",
            up(fluent),
            args.iter().map(|a| up(a)).collect::<Vec<_>>().join(" ")
        )
    }
}

fn atom_key(pred: &str, args: &[Sym]) -> String {
    res_key(pred, args)
}

/// Bind a fluent/atom arg list against the param binding (objects pass through).
fn bind_args(args: &[Term], bind: &HashMap<String, String>) -> Option<Vec<Sym>> {
    args.iter()
        .map(|t| match t {
            Term::Var(v) => bind.get(&up(v)).cloned(),
            Term::Const(c) => Some(up(c)),
        })
        .collect()
}

/// Evaluate a constant numeric expression (the only kind used for craft amounts).
fn const_expr(e: &crate::types::Expr) -> Option<f64> {
    use crate::types::Expr::*;
    match e {
        Num(n) => Some(*n),
        Neg(a) => const_expr(a).map(|x| -x),
        Add(a, b) => Some(const_expr(a)? + const_expr(b)?),
        Sub(a, b) => Some(const_expr(a)? - const_expr(b)?),
        Mul(a, b) => Some(const_expr(a)? * const_expr(b)?),
        Div(a, b) => Some(const_expr(a)? / const_expr(b)?),
        Fluent(..) => None,
    }
}

/// Collect numeric effects of one assign-op at a given time-spec, as resource→amount.
fn collect_num(
    da: &crate::types::DurativeAction,
    when: TimeSpec,
    op: AssignOp,
    bind: &HashMap<String, String>,
) -> Vec<(String, f64)> {
    let mut acc = Vec::new();
    for (t, eff) in &da.effects {
        if *t == when {
            walk_num(eff, op, bind, &mut acc);
        }
    }
    acc
}

fn walk_num(
    eff: &Effect,
    op: AssignOp,
    bind: &HashMap<String, String>,
    acc: &mut Vec<(String, f64)>,
) {
    match eff {
        Effect::And(es) => es.iter().for_each(|e| walk_num(e, op, bind, acc)),
        Effect::Num(o, f, args, expr) if *o == op => {
            if let (Some(a), Some(v)) = (bind_args(args, bind), const_expr(expr)) {
                acc.push((res_key(f, &a), v));
            }
        }
        // conditional/universal effects are actor-dependent or unbounded; a crew
        // domain avoids them. Skipping is safe — validate() guards the result.
        _ => {}
    }
}

/// Collect positive atoms in the precondition at a time-spec, excluding any that
/// mention the actor variable (those are satisfied by whichever actor we assign).
fn collect_atoms(
    da: &crate::types::DurativeAction,
    when: TimeSpec,
    bind: &HashMap<String, String>,
    actor_var: &str,
) -> Vec<String> {
    let mut acc = Vec::new();
    for (t, f) in &da.conditions {
        if *t == when {
            walk_atoms(f, bind, actor_var, &mut acc);
        }
    }
    acc
}

fn walk_atoms(f: &Formula, bind: &HashMap<String, String>, actor_var: &str, acc: &mut Vec<String>) {
    match f {
        Formula::And(fs) => fs.iter().for_each(|x| walk_atoms(x, bind, actor_var, acc)),
        Formula::Atom(p, args) => {
            let mentions_actor = args
                .iter()
                .any(|t| matches!(t, Term::Var(v) if up(v) == actor_var));
            if !mentions_actor {
                if let Some(a) = bind_args(args, bind) {
                    acc.push(atom_key(p, &a));
                }
            }
        }
        _ => {}
    }
}

/// Collect atoms ADDED at a time-spec (excluding actor-mentioning ones).
fn collect_added(
    da: &crate::types::DurativeAction,
    when: TimeSpec,
    bind: &HashMap<String, String>,
    actor_var: &str,
) -> Vec<String> {
    let mut acc = Vec::new();
    for (t, eff) in &da.effects {
        if *t == when {
            walk_added(eff, bind, actor_var, &mut acc);
        }
    }
    acc
}

fn walk_added(
    eff: &Effect,
    bind: &HashMap<String, String>,
    actor_var: &str,
    acc: &mut Vec<String>,
) {
    match eff {
        Effect::And(es) => es.iter().for_each(|e| walk_added(e, bind, actor_var, acc)),
        Effect::Add(p, args) => {
            let mentions_actor = args
                .iter()
                .any(|t| matches!(t, Term::Var(v) if up(v) == actor_var));
            if !mentions_actor {
                if let Some(a) = bind_args(args, bind) {
                    acc.push(atom_key(p, &a));
                }
            }
        }
        _ => {}
    }
}
