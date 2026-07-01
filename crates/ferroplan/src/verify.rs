//! Independent PDDL3 plan/metric verifier (conformance oracle).
//!
//! Re-derives, from scratch over ffdp's verified execution engine, what a plan
//! actually achieves: it grounds the ORIGINAL problem (soft goals ignored),
//! replays the plan, checks the hard goals hold in the final state, evaluates
//! each preference's formula in that final state, and recomputes the metric.
//! Used to confirm a planner's REPORTED metric equals the plan's true metric —
//! authoritative and self-contained (no external VAL/cmake dependency).

use crate::ground::ground_task;
use crate::packed::{PackedTask, State};
use crate::types::{CompOp, Expr, Formula, Term};

use crate::pddl3;

#[derive(Debug)]
pub struct Verified {
    pub metric: f64,
    pub hard_goal_met: bool,
    pub satisfied: usize,
    pub violated: usize,
}

fn disp(pred: &str, args: &[Term]) -> String {
    let a: Vec<String> = args
        .iter()
        .map(|t| match t {
            Term::Const(c) => c.clone(),
            Term::Var(v) => v.clone(),
        })
        .collect();
    if a.is_empty() {
        format!("({})", pred)
    } else {
        format!("({} {})", pred, a.join(" "))
    }
}

fn eval_expr(task: &PackedTask, s: &State, e: &Expr) -> Option<f64> {
    Some(match e {
        Expr::Num(n) => *n,
        Expr::Fluent(name, args) => {
            let id = task.fluent_id(&disp(name, args))?;
            if s.fdef[id] {
                s.fv[id]
            } else {
                return None;
            }
        }
        Expr::Add(a, b) => eval_expr(task, s, a)? + eval_expr(task, s, b)?,
        Expr::Sub(a, b) => eval_expr(task, s, a)? - eval_expr(task, s, b)?,
        Expr::Mul(a, b) => eval_expr(task, s, a)? * eval_expr(task, s, b)?,
        Expr::Div(a, b) => eval_expr(task, s, a)? / eval_expr(task, s, b)?,
        Expr::Neg(a) => -eval_expr(task, s, a)?,
    })
}

/// Evaluate a ground formula in a concrete state.
fn eval_formula(task: &PackedTask, s: &State, f: &Formula) -> bool {
    match f {
        Formula::True => true,
        Formula::False => false,
        Formula::And(v) => v.iter().all(|x| eval_formula(task, s, x)),
        Formula::Or(v) => v.iter().any(|x| eval_formula(task, s, x)),
        Formula::Not(a) => !eval_formula(task, s, a),
        Formula::Pref(_, inner) => eval_formula(task, s, inner),
        Formula::Eq(a, b) => {
            let t = |x: &Term| match x {
                Term::Const(c) => c.clone(),
                Term::Var(v) => v.clone(),
            };
            t(a) == t(b)
        }
        // quantified preferences are out of the phase-1 verifier's scope (they
        // don't appear in the supported metric class); evaluate the body best-effort
        Formula::Forall(_, inner) | Formula::Exists(_, inner) => eval_formula(task, s, inner),
        Formula::Atom(p, args) => match task.fact_id(&disp(p, args)) {
            Some(id) => s.bits[id / 64] >> (id % 64) & 1 != 0,
            None => false, // fact never grounded -> never true
        },
        Formula::Comp(op, l, r) => {
            let (l, r) = match (eval_expr(task, s, l), eval_expr(task, s, r)) {
                (Some(l), Some(r)) => (l, r),
                _ => return false,
            };
            match op {
                CompOp::Lt => l < r,
                CompOp::Le => l <= r,
                CompOp::Eq => (l - r).abs() < 1e-6,
                CompOp::Ge => l >= r,
                CompOp::Gt => l > r,
            }
        }
    }
}

/// Independently verify a plan and compute its true PDDL3 metric.
/// `plan` is the executed action sequence as `(NAME, [ARGS])` (uppercased).
pub fn verify(
    domain_src: &str,
    problem_src: &str,
    plan: &[(String, Vec<String>)],
) -> Result<Verified, String> {
    let domain = crate::parser::parse_domain(domain_src).map_err(|e| format!("domain: {}", e))?;
    let problem =
        crate::parser::parse_problem(problem_src).map_err(|e| format!("problem: {}", e))?;
    // Compile `:derived` axioms away, like every solve path — replaying against the
    // raw problem would miss the derived init facts and reject valid plans.
    let (domain, problem) = crate::derived::compile(&domain, &problem)?;
    // ground the ORIGINAL problem (soft goals ignored), forcing a Task even when
    // the hard goal is trivial/empty (preference-only problems) so we can replay.
    let task = match ground_task(&domain, &problem, 1) {
        Some(t) => t,
        None => return Err("grounding failed (empty type)".into()),
    };

    // replay the plan over the original-grounded task
    let mut s = task.initial();
    for (name, args) in plan {
        let want: Vec<&str> = args.iter().map(|x| x.as_str()).collect();
        let oi = (0..task.n_ops).find(|&oi| {
            let d = &task.op_display[oi];
            let mut it = d.split_whitespace();
            it.next() == Some(name.as_str()) && it.eq(want.iter().copied())
        });
        let oi = match oi {
            Some(oi) => oi,
            None => {
                return Err(format!(
                    "plan action `{} {}` not a grounded op",
                    name,
                    args.join(" ")
                ))
            }
        };
        if !task.op_applicable(oi, &s) {
            return Err(format!(
                "plan action `{} {}` not applicable",
                name,
                args.join(" ")
            ));
        }
        s = task.apply(oi, &s);
    }

    let hard_goal_met = task.goal_met(&s);

    // score preferences in the FINAL state
    let weights = pddl3::pref_weights(&domain, &problem);
    let objs = crate::ground::objects_by_type(&domain, &problem);
    let prefs = pddl3::preferences(&problem.goal, &objs);
    let mut metric = 0.0;
    let (mut sat, mut vio) = (0usize, 0usize);
    for (name, phi) in &prefs {
        if eval_formula(&task, &s, phi) {
            sat += 1;
        } else {
            vio += 1;
            metric += weights.get(name).copied().unwrap_or(0.0);
        }
    }
    Ok(Verified {
        metric,
        hard_goal_met,
        satisfied: sat,
        violated: vio,
    })
}
