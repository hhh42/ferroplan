//! PDDL3 trajectory-constraint ENFORCEMENT (0.7, docs/roadmap-0.7.md).
//!
//! From 0.4.1 to 0.6 every `(:constraints ...)` block was parsed and then
//! cleanly REJECTED. 0.7 narrows that fence operator-by-operator: the six
//! untimed modal operators (`always`, `sometime`, `at-most-once`,
//! `sometime-after`, `sometime-before`, `at end`) compile into small
//! **monitor automata** over state trajectories — fresh 0-ary monitor facts
//! transitioned by `Effect::When` conditional effects appended to every real
//! action (the grounder and heuristic already handle everything this emits),
//! with acceptance conjoined into the hard goal. Anything this build cannot
//! enforce (the timed operators; soft `preference`-wrapped constraints until
//! Phase 2) keeps a rejection that NAMES the operator — the "never silently
//! ignore" contract is narrowed, never deleted.
//!
//! THE OBSERVATION OFFSET (load-bearing): `PackedTask::apply` evaluates
//! conditional-effect conditions against the SOURCE state, so a monitor
//! riding action a_k observes S_{k-1}. The trajectory S_0..S_n is covered
//! three ways — S_0 by compile-time evaluation against init (this module),
//! S_0..S_{n-1} by the per-action `When`s, and S_n by a goal-side formula.
//! For `sometime-before` the one-step lag implements "strictly earlier"
//! exactly. All transition conditions on one monitor fact are mutually
//! exclusive, so the add-wins conflict rule can never co-fire a set and a
//! clear of the same bit.
//!
//! The independent verifier does NOT use this compilation: `verify.rs` folds
//! the ORIGINAL constraint semantics over its replay (see [`Fold`]), so the
//! oracle stays independent of the compiled monitors.

use std::collections::HashMap;

use crate::pddl3::{combos, subst_formula};
use crate::types::{Constraint, Domain, Effect, Formula, Problem, Sym};

/// One ground untimed trajectory-constraint instance.
#[derive(Clone, Debug)]
pub enum Traj {
    Always(Formula),
    Sometime(Formula),
    AtMostOnce(Formula),
    SometimeAfter(Formula, Formula),
    SometimeBefore(Formula, Formula),
    AtEnd(Formula),
}

/// The expanded constraint sets of a task: `Forall` quantifiers grounded,
/// `And` flattened, hard and soft (`preference`-wrapped) separated.
pub struct Expanded {
    pub hard: Vec<Traj>,
    /// `(preference name <constraint>)` instances — parsed, named, NOT
    /// enforced in Phase 1 (they reject at the gates until Phase 2 wires
    /// them into the metric machinery).
    pub soft: Vec<(String, Traj)>,
}

/// Expand and validate a task's `(:constraints ...)` trees. Errors name the
/// unsupported operator (the timed family) or the malformed nesting.
pub fn expand(domain: &Domain, problem: &Problem) -> Result<Expanded, String> {
    let objs = crate::ground::objects_by_type(domain, problem);
    let mut out = Expanded {
        hard: Vec::new(),
        soft: Vec::new(),
    };
    for c in domain.constraints.iter().chain(problem.constraints.iter()) {
        walk(c, &objs, None, &HashMap::new(), &mut out)?;
    }
    Ok(out)
}

fn timed_err(op: &str) -> String {
    format!(
        "PDDL3 trajectory constraint `{op}` is time-bounded and not yet \
         enforced (untimed operators — always / sometime / at-most-once / \
         sometime-after / sometime-before / at-end — are). Remove it, or \
         model the requirement without a clock."
    )
}

fn walk(
    c: &Constraint,
    objs: &HashMap<Sym, Vec<Sym>>,
    pref: Option<&str>,
    binding: &HashMap<Sym, Sym>,
    out: &mut Expanded,
) -> Result<(), String> {
    let sub = |f: &Formula| subst_formula(f, binding);
    let push = |t: Traj, out: &mut Expanded| match pref {
        Some(name) => out.soft.push((name.to_string(), t)),
        None => out.hard.push(t),
    };
    match c {
        Constraint::And(v) => {
            for x in v {
                walk(x, objs, pref, binding, out)?;
            }
        }
        Constraint::Forall(vars, inner) => {
            for combo in combos(vars, objs) {
                let mut b = binding.clone();
                b.extend(combo);
                walk(inner, objs, pref, &b, out)?;
            }
        }
        Constraint::Pref(name, inner) => {
            if pref.is_some() {
                return Err(
                    "malformed (:constraints ...): a preference nested inside a \
                     preference has no PDDL3 semantics"
                        .into(),
                );
            }
            let name = name.clone().unwrap_or_else(|| "ANON-TRAJ".into());
            walk(inner, objs, Some(&name), binding, out)?;
        }
        Constraint::Always(f) => push(Traj::Always(sub(f)), out),
        Constraint::Sometime(f) => push(Traj::Sometime(sub(f)), out),
        Constraint::AtMostOnce(f) => push(Traj::AtMostOnce(sub(f)), out),
        Constraint::SometimeAfter(a, b) => push(Traj::SometimeAfter(sub(a), sub(b)), out),
        Constraint::SometimeBefore(a, b) => push(Traj::SometimeBefore(sub(a), sub(b)), out),
        Constraint::AtEnd(f) => push(Traj::AtEnd(sub(f)), out),
        Constraint::Within(_, _) => return Err(timed_err("within")),
        Constraint::AlwaysWithin(_, _, _) => return Err(timed_err("always-within")),
        Constraint::HoldDuring(_, _, _) => return Err(timed_err("hold-during")),
        Constraint::HoldAfter(_, _) => return Err(timed_err("hold-after")),
    }
    Ok(())
}

/// Incremental trajectory fold for ONE constraint instance — the verifier's
/// independent semantics (never the compiled monitors). Feed every state of
/// the replay in order (S_0 first), then ask [`Fold::accepted`].
pub struct Fold<'a> {
    traj: &'a Traj,
    ok: bool,
    seen: bool,    // sometime: φ seen; at-most-once: an episode has closed
    holding: bool, // at-most-once: currently inside a φ episode
    pending: bool, // sometime-after: φ seen, ψ still owed
    safe: bool,    // sometime-before: ψ seen strictly earlier
    first: bool,   // S_0 marker (sometime-before's strictly-earlier check)
    last: bool,    // at-end: φ in the most recent state
}

impl<'a> Fold<'a> {
    pub fn new(traj: &'a Traj) -> Self {
        Fold {
            traj,
            ok: true,
            seen: false,
            holding: false,
            pending: false,
            safe: false,
            first: true,
            last: false,
        }
    }

    /// Observe the next state of the trajectory via a formula evaluator.
    pub fn step(&mut self, holds: &mut dyn FnMut(&Formula) -> bool) {
        match self.traj {
            Traj::Always(f) => {
                if !holds(f) {
                    self.ok = false;
                }
            }
            Traj::Sometime(f) => {
                if holds(f) {
                    self.seen = true;
                }
            }
            Traj::AtMostOnce(f) => {
                let now = holds(f);
                if now && !self.holding {
                    if self.seen {
                        self.ok = false; // a second episode opened
                    }
                    self.seen = true;
                }
                self.holding = now;
            }
            Traj::SometimeAfter(a, b) => {
                let (fa, fb) = (holds(a), holds(b));
                if fb {
                    self.pending = false;
                } else if fa {
                    self.pending = true;
                }
            }
            Traj::SometimeBefore(a, b) => {
                // check φ against ψ-seen STRICTLY earlier, then record ψ.
                if holds(a) && !self.safe {
                    self.ok = false;
                }
                if holds(b) {
                    self.safe = true;
                }
            }
            Traj::AtEnd(f) => {
                self.last = holds(f);
            }
        }
        self.first = false;
    }

    /// The verdict once the final state has been observed.
    pub fn accepted(&self) -> bool {
        match self.traj {
            Traj::Always(_) => self.ok,
            Traj::Sometime(_) => self.seen,
            Traj::AtMostOnce(_) => self.ok,
            Traj::SometimeAfter(_, _) => !self.pending,
            Traj::SometimeBefore(_, _) => self.ok,
            Traj::AtEnd(_) => self.last,
        }
    }

    /// Human name of the operator (for verifier reports).
    pub fn op_name(&self) -> &'static str {
        match self.traj {
            Traj::Always(_) => "always",
            Traj::Sometime(_) => "sometime",
            Traj::AtMostOnce(_) => "at-most-once",
            Traj::SometimeAfter(_, _) => "sometime-after",
            Traj::SometimeBefore(_, _) => "sometime-before",
            Traj::AtEnd(_) => "at-end",
        }
    }
}

/// The 0.7 entrypoint gate, shared by `solve`/`decompose`/`run_planner`/
/// `run_ff` so no gate can silently diverge: `Ok(None)` = no constraints
/// (byte-identical no-op path), `Ok(Some(pair))` = hard untimed constraints
/// compiled into the rewritten task, `Err(msg)` = a NAMED rejection — the
/// timed operators, soft constraint-preferences (Phase 2), any constraint on
/// a durative-action domain (Phase 3), or the `FF_CONSTRAINTS_REJECT=1`
/// hatch, which restores the 0.4.1 blanket rejection byte-for-byte (it
/// restores *rejection*, never ignoring).
pub fn gate(domain: &Domain, problem: &Problem) -> Result<Option<(Domain, Problem)>, String> {
    if domain.constraints.is_empty() && problem.constraints.is_empty() {
        return Ok(None);
    }
    if std::env::var("FF_CONSTRAINTS_REJECT").is_ok() {
        return Err(crate::pddl3::unsupported_constraints(domain, problem)
            .unwrap_or_else(|| "trajectory constraints rejected (hatch)".into()));
    }
    if crate::temporal::is_temporal(domain) {
        return Err(
            "trajectory constraints on durative-action (temporal) domains are \
             not yet enforced (the untimed classical path is); remove the \
             (:constraints ...) block or the durative actions"
                .into(),
        );
    }
    compile(domain, problem).map(Some)
}

/// Compile the HARD untimed constraints into the domain/problem: monitor
/// predicates + per-action `When` transitions + goal conjuncts, per the
/// module-level table. Returns the rewritten pair. Errors on timed operators
/// (naming them) and, in Phase 1, on soft constraint preferences.
pub fn compile(domain: &Domain, problem: &Problem) -> Result<(Domain, Problem), String> {
    let exp = expand(domain, problem)?;
    if !exp.soft.is_empty() {
        return Err(format!(
            "soft trajectory constraints — (preference {} ...) inside \
             (:constraints ...) — are not yet enforced (hard untimed \
             constraints are). Score them via goal preferences, or drop the \
             preference wrapper to make them hard.",
            exp.soft[0].0
        ));
    }
    if exp.hard.is_empty() {
        return Ok((domain.clone(), problem.clone()));
    }

    let mut d = domain.clone();
    let mut p = problem.clone();
    // S_0 evaluation happens against the raw init atom set of the ORIGINAL
    // problem (user formulas can never reference the monitor facts we add).
    let init_holds = |f: &Formula| eval_static(f, problem);

    let mut goal_conj: Vec<Formula> = vec![p.goal.clone()];
    // Per-action transition effects, accumulated then appended to every action.
    let mut transitions: Vec<Effect> = Vec::new();
    let atom = |n: &str| Formula::Atom(n.to_string(), vec![]);
    let add = |n: &str| Effect::Add(n.to_string(), vec![]);
    let del = |n: &str| Effect::Del(n.to_string(), vec![]);
    let declare = |d: &mut Domain, p: &mut Problem, n: &str, init_true: bool| {
        d.predicates.push((n.to_string(), vec![]));
        if init_true {
            p.init_atoms.push((n.to_string(), vec![]));
        }
    };

    for (i, t) in exp.hard.iter().enumerate() {
        match t {
            Traj::Always(f) => {
                let viol = format!("TRAJ{i}-VIOL");
                declare(&mut d, &mut p, &viol, !init_holds(f));
                transitions.push(Effect::When(
                    Formula::Not(Box::new(f.clone())),
                    Box::new(add(&viol)),
                ));
                goal_conj.push(Formula::Not(Box::new(atom(&viol))));
                goal_conj.push(f.clone()); // S_n
            }
            Traj::Sometime(f) => {
                let seen = format!("TRAJ{i}-SEEN");
                declare(&mut d, &mut p, &seen, init_holds(f));
                transitions.push(Effect::When(f.clone(), Box::new(add(&seen))));
                goal_conj.push(Formula::Or(vec![atom(&seen), f.clone()]));
            }
            Traj::AtMostOnce(f) => {
                let hold = format!("TRAJ{i}-HOLD");
                let seen = format!("TRAJ{i}-SEEN");
                let viol = format!("TRAJ{i}-VIOL");
                let f0 = init_holds(f);
                declare(&mut d, &mut p, &hold, f0);
                declare(&mut d, &mut p, &seen, f0);
                declare(&mut d, &mut p, &viol, false);
                // second rising edge (φ ∧ ¬HOLD ∧ SEEN) → VIOL; then episode
                // tracking. Conditions are mutually exclusive per fact.
                transitions.push(Effect::When(
                    Formula::And(vec![
                        f.clone(),
                        Formula::Not(Box::new(atom(&hold))),
                        atom(&seen),
                    ]),
                    Box::new(add(&viol)),
                ));
                transitions.push(Effect::When(
                    Formula::And(vec![f.clone(), Formula::Not(Box::new(atom(&hold)))]),
                    Box::new(Effect::And(vec![add(&seen), add(&hold)])),
                ));
                transitions.push(Effect::When(
                    Formula::And(vec![Formula::Not(Box::new(f.clone())), atom(&hold)]),
                    Box::new(del(&hold)),
                ));
                goal_conj.push(Formula::Not(Box::new(atom(&viol))));
                // S_n rising edge: φ now, not holding into it, already seen.
                goal_conj.push(Formula::Not(Box::new(Formula::And(vec![
                    f.clone(),
                    Formula::Not(Box::new(atom(&hold))),
                    atom(&seen),
                ]))));
            }
            Traj::SometimeAfter(a, b) => {
                let pend = format!("TRAJ{i}-PEND");
                declare(&mut d, &mut p, &pend, init_holds(a) && !init_holds(b));
                transitions.push(Effect::When(b.clone(), Box::new(del(&pend))));
                transitions.push(Effect::When(
                    Formula::And(vec![a.clone(), Formula::Not(Box::new(b.clone()))]),
                    Box::new(add(&pend)),
                ));
                // accepted iff nothing pending after S_n's own φ/ψ resolve.
                goal_conj.push(Formula::Or(vec![
                    b.clone(),
                    Formula::And(vec![
                        Formula::Not(Box::new(atom(&pend))),
                        Formula::Not(Box::new(a.clone())),
                    ]),
                ]));
            }
            Traj::SometimeBefore(a, b) => {
                let safe = format!("TRAJ{i}-SAFE");
                let viol = format!("TRAJ{i}-VIOL");
                declare(&mut d, &mut p, &safe, init_holds(b));
                declare(&mut d, &mut p, &viol, init_holds(a)); // φ(S_0): nothing earlier
                                                               // source-state reads give "strictly earlier" for free.
                transitions.push(Effect::When(
                    Formula::And(vec![a.clone(), Formula::Not(Box::new(atom(&safe)))]),
                    Box::new(add(&viol)),
                ));
                transitions.push(Effect::When(b.clone(), Box::new(add(&safe))));
                goal_conj.push(Formula::Not(Box::new(atom(&viol))));
                goal_conj.push(Formula::Or(vec![
                    Formula::Not(Box::new(a.clone())),
                    atom(&safe),
                ]));
            }
            Traj::AtEnd(f) => {
                goal_conj.push(f.clone());
            }
        }
    }

    // Append the monitor transitions to every real action.
    if !transitions.is_empty() {
        for act in &mut d.actions {
            let mut v = vec![act.effect.clone()];
            v.extend(transitions.iter().cloned());
            act.effect = Effect::And(v);
        }
    }
    p.goal = Formula::And(goal_conj);
    d.constraints.clear();
    p.constraints.clear();
    Ok((d, p))
}

/// Evaluate an (assumed ground) formula against the raw init atom set —
/// S_0 for the monitor initialization. Numeric comparisons evaluate against
/// init fluents; unknown fluents make the comparison false.
fn eval_static(f: &Formula, p: &Problem) -> bool {
    match f {
        Formula::True => true,
        Formula::False => false,
        Formula::And(v) => v.iter().all(|x| eval_static(x, p)),
        Formula::Or(v) => v.iter().any(|x| eval_static(x, p)),
        Formula::Not(a) => !eval_static(a, p),
        Formula::Pref(_, a) => eval_static(a, p),
        Formula::Forall(_, a) | Formula::Exists(_, a) => eval_static(a, p),
        Formula::Eq(a, b) => a == b,
        Formula::Atom(name, args) => p.init_atoms.iter().any(|(n, a)| {
            n.eq_ignore_ascii_case(name)
                && a.len() == args.len()
                && a.iter().zip(args).all(|(x, t)| match t {
                    crate::types::Term::Const(c) => x.eq_ignore_ascii_case(c),
                    crate::types::Term::Var(_) => false,
                })
        }),
        Formula::Comp(op, l, r) => {
            let ev = |e: &crate::types::Expr| eval_init_expr(e, p);
            match (ev(l), ev(r)) {
                (Some(l), Some(r)) => match op {
                    crate::types::CompOp::Lt => l < r,
                    crate::types::CompOp::Le => l <= r,
                    crate::types::CompOp::Eq => (l - r).abs() < 1e-6,
                    crate::types::CompOp::Ge => l >= r,
                    crate::types::CompOp::Gt => l > r,
                },
                _ => false,
            }
        }
    }
}

fn eval_init_expr(e: &crate::types::Expr, p: &Problem) -> Option<f64> {
    use crate::types::Expr::*;
    Some(match e {
        Num(n) => *n,
        Fluent(name, args) => {
            let ((_, _), v) = p.init_fluents.iter().find(|((n, a), _)| {
                n.eq_ignore_ascii_case(name)
                    && a.len() == args.len()
                    && a.iter().zip(args).all(|(x, t)| match t {
                        crate::types::Term::Const(c) => x.eq_ignore_ascii_case(c),
                        crate::types::Term::Var(_) => false,
                    })
            })?;
            *v
        }
        Add(a, b) => eval_init_expr(a, p)? + eval_init_expr(b, p)?,
        Sub(a, b) => eval_init_expr(a, p)? - eval_init_expr(b, p)?,
        Mul(a, b) => eval_init_expr(a, p)? * eval_init_expr(b, p)?,
        Div(a, b) => eval_init_expr(a, p)? / eval_init_expr(b, p)?,
        Neg(a) => -eval_init_expr(a, p)?,
    })
}

#[cfg(test)]
mod grounding_cost {
    //! Heavy fixtures per docs/roadmap-0.7.md Phase 1 acceptance: the
    //! grounding cost of a hard-`(:constraints ...)` overlay on vendored
    //! IPC-5 instances — conditional-effect count and grounding wall time
    //! vs. the unconstrained input. Run with
    //! `cargo test -p ferroplan --release --lib grounding_cost -- --ignored --nocapture`

    /// Parse, gate (compiling any constraints), ground, and report
    /// `(ops, facts, conditional effects, ground millis)`. Also prints the
    /// monitor count and how many ops are synthetic REACH-GOAL disjunct ops —
    /// the goal-DNF cost of the monitors' S_n acceptance checks.
    fn measure(dom: &str, prob: &str, label: &str) -> (usize, usize, usize, u128) {
        let d = crate::parser::parse_domain(dom).expect("domain");
        let p = crate::parser::parse_problem(prob).expect("problem");
        let (d, p) = crate::derived::compile(&d, &p).expect("derived");
        let monitors = super::expand(&d, &p).expect("expand").hard.len();
        let (d, p) = match super::gate(&d, &p).expect("gate") {
            Some(pair) => pair,
            None => (d, p),
        };
        let t0 = std::time::Instant::now();
        let task = crate::ground::ground_task(&d, &p, 1).expect("ground");
        let ms = t0.elapsed().as_millis();
        let cond: usize = (0..task.n_ops).map(|oi| task.cond.slice(oi).len()).sum();
        let goal_ops = (0..task.n_ops)
            .filter(|&oi| task.op_display[oi].starts_with("REACH-GOAL"))
            .count();
        println!(
            "{label}: {} monitors, {} ops ({} REACH-GOAL), {} facts, \
             {} conditional effects, ground {} ms",
            monitors, task.n_ops, goal_ops, task.n_facts, cond, ms
        );
        (task.n_ops, task.n_facts, cond, ms)
    }

    /// Insert a `(:constraints ...)` block before the problem's final paren.
    fn overlay(prob: &str, constraints: &str) -> String {
        let i = prob.rfind(')').expect("problem has a closing paren");
        format!("{}(:constraints {}){}", &prob[..i], constraints, &prob[i..])
    }

    #[test]
    #[ignore = "heavy: grounding-cost measurement (docs/roadmap-0.7.md Phase 1)"]
    fn storage_p05_hard_overlay() {
        let base = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../benchmarks/ipc/pref/storage"
        );
        let dom = std::fs::read_to_string(format!("{base}/domain.pddl")).unwrap();
        let prob = std::fs::read_to_string(format!("{base}/p05.pddl")).unwrap();
        let (_, f0, c0, _) = measure(&dom, &prob, "storage p05 unconstrained");
        // "each hoist lifts each crate at most once" — forall expands at the
        // constraint level, so every monitor body stays ground.
        let hard = overlay(
            &prob,
            "(forall (?h - hoist ?c - crate) (at-most-once (lifting ?h ?c)))",
        );
        let (_, f1, c1, _) = measure(&dom, &hard, "storage p05 + hard overlay");
        assert!(f1 > f0, "monitor facts must appear ({f0} -> {f1})");
        assert!(c1 > c0, "monitor transitions must appear ({c0} -> {c1})");
    }

    #[test]
    #[ignore = "heavy: grounding-cost measurement (docs/roadmap-0.7.md Phase 1)"]
    fn trucks_p03_hard_overlay() {
        let base = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../benchmarks/ipc/pref/trucks"
        );
        let dom = std::fs::read_to_string(format!("{base}/domain.pddl")).unwrap();
        let prob = std::fs::read_to_string(format!("{base}/p03.pddl")).unwrap();
        let (_, f0, c0, _) = measure(&dom, &prob, "trucks p03 unconstrained");
        // "a truck parks at each location at most once"
        let hard = overlay(
            &prob,
            "(forall (?t - truck ?l - location) (at-most-once (at ?t ?l)))",
        );
        let (_, f1, c1, _) = measure(&dom, &hard, "trucks p03 + hard overlay");
        assert!(f1 > f0, "monitor facts must appear ({f0} -> {f1})");
        assert!(c1 > c0, "monitor transitions must appear ({c0} -> {c1})");
    }
}
