//! Plan introspection (0.18 Phase 3): the planner made legible for any
//! solved instance. Three views, each answering the question a person
//! actually asks of a plan:
//!
//! - **Causal links** (classical): for every step, WHICH earlier step (or
//!   the initial state) provided each of its positive preconditions — the
//!   last-achiever chain that makes "why is this step here" readable.
//!   Numeric preconditions are out of scope (they are conditions on
//!   accumulated quantities, not single-provider facts).
//! - **Invariant spans** (temporal): for every durative step, the `over
//!   all` conditions that must hold across its interval — rendered from
//!   the ORIGINAL action schema with the step's arguments substituted, so
//!   the spans state exactly what VAL checks over `[start, end]`, not an
//!   internal compiled form.
//! - **Preference breakdown** (PDDL3): per preference INSTANCE — name,
//!   satisfied?, `is-violated` weight — goal preferences evaluated in the
//!   final replayed state and soft trajectory-constraint preferences by
//!   the independent [`crate::verify`] oracle's folds.
//!
//! Everything replays over the same grounding pipeline as the solver
//! (`derived::compile` + `ground_task`), so plan steps match op displays
//! byte-for-byte; an unmatchable or inapplicable step is an error, never
//! a silent skip.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::api::Plan;
use crate::bitset;
use crate::ground::ground_task;
use crate::types::{CompOp, Expr, Formula, Term, TimeSpec};

/// One provider edge: `consumer`'s precondition `fact` was last made true
/// by `provider` (a plan step index), or by the initial state (`None`).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CausalLink {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<usize>,
    pub consumer: usize,
    pub fact: String,
}

/// One durative step's invariant: the `over all` conditions in force
/// across `[start, end]` (absolute plan times).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InvariantSpan {
    pub step: usize,
    pub action: String,
    pub start: f64,
    pub end: f64,
    pub conditions: Vec<String>,
}

/// One preference instance's verdict and its `is-violated` weight.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefReport {
    pub name: String,
    pub satisfied: bool,
    pub weight: f64,
}

/// The introspection payload: which views apply depends on the problem
/// class (`kind` = `"classical"` or `"temporal"`).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Explanation {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causal_links: Vec<CausalLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invariant_spans: Vec<InvariantSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preferences: Vec<PrefReport>,
}

/// Explain a plan for its domain+problem. Temporal problems get invariant
/// spans; classical ones get causal links, plus the preference breakdown
/// when the problem carries PDDL3 preferences.
pub fn explain(domain_src: &str, problem_src: &str, plan: &Plan) -> Result<Explanation, String> {
    let domain = crate::parser::parse_domain(domain_src).map_err(|e| format!("domain: {e}"))?;
    let problem = crate::parser::parse_problem(problem_src).map_err(|e| format!("problem: {e}"))?;
    if crate::temporal::is_temporal(&domain) {
        return Ok(Explanation {
            kind: "temporal".into(),
            causal_links: Vec::new(),
            invariant_spans: invariant_spans(&domain, plan),
            preferences: Vec::new(),
        });
    }
    let (causal_links, preferences) =
        classical_views(domain_src, problem_src, &domain, &problem, plan)?;
    Ok(Explanation {
        kind: "classical".into(),
        causal_links,
        invariant_spans: Vec::new(),
        preferences,
    })
}

/// The temporal view: each durative step's `over all` conditions with the
/// step's arguments substituted, spanning `[time, time + duration]`.
fn invariant_spans(domain: &crate::types::Domain, plan: &Plan) -> Vec<InvariantSpan> {
    let mut spans = Vec::new();
    for step in &plan.steps {
        let Some(dur) = step.duration else { continue };
        let Some(da) = domain.durative_actions.iter().find(|d| {
            d.name.eq_ignore_ascii_case(&step.action) && d.params.len() == step.args.len()
        }) else {
            continue;
        };
        let sub: HashMap<&str, &str> = da
            .params
            .iter()
            .map(|(p, _)| p.as_str())
            .zip(step.args.iter().map(|a| a.as_str()))
            .collect();
        let mut conditions = Vec::new();
        for (spec, phi) in &da.conditions {
            if *spec == TimeSpec::All {
                flatten_conditions(phi, &sub, &mut conditions);
            }
        }
        if conditions.is_empty() {
            continue;
        }
        let start = step.time.unwrap_or(0.0);
        spans.push(InvariantSpan {
            step: step.index,
            action: display(&step.action, &step.args),
            start,
            end: start + dur,
            conditions,
        });
    }
    spans
}

/// The classical views: replay the plan over the solver's grounding,
/// recording each precondition's last achiever, then score preferences.
fn classical_views(
    domain_src: &str,
    problem_src: &str,
    domain: &crate::types::Domain,
    problem: &crate::types::Problem,
    plan: &Plan,
) -> Result<(Vec<CausalLink>, Vec<PrefReport>), String> {
    // The solver's own pipeline, so op displays match plan steps exactly.
    let (cdomain, cproblem) = crate::derived::compile(domain, problem)?;
    let task = ground_task(&cdomain, &cproblem, 1).ok_or("grounding failed (empty type)")?;

    // provider[fact] = the last plan step that added it (None = initial).
    let mut provider: Vec<Option<usize>> = vec![None; task.fact_names.len()];
    let mut links = Vec::new();
    let mut s = task.initial();
    for (k, step) in plan.steps.iter().enumerate() {
        let disp = display(&step.action, &step.args);
        let oi = task
            .op_display
            .iter()
            .position(|d| *d == disp)
            .ok_or_else(|| format!("plan step `{disp}` not a grounded op"))?;
        if !task.op_applicable(oi, &s) {
            return Err(format!("plan step `{disp}` not applicable at step {k}"));
        }
        for &f in task.pre_pos.slice(oi) {
            // Only report facts that can change hands: a static fact's
            // "provider" is always the initial state — noise, not insight.
            if bitset::test(&s.bits, f as usize) {
                links.push(CausalLink {
                    provider: provider[f as usize],
                    consumer: k,
                    fact: task.fact_names[f as usize].clone(),
                });
            }
        }
        s = task.apply(oi, &s);
        for &f in task.add.slice(oi) {
            provider[f as usize] = Some(k);
        }
    }
    // Keep the chain readable: drop initial-state links for facts NO step
    // ever provided anywhere (statics like road maps), keep initial-state
    // links for genuinely mutable facts (they show consumption order).
    let ever_provided: std::collections::HashSet<String> = links
        .iter()
        .filter(|l| l.provider.is_some())
        .map(|l| l.fact.clone())
        .collect();
    links.retain(|l| l.provider.is_some() || ever_provided.contains(&l.fact));

    // Preference breakdown: goal preferences in the final state; soft
    // trajectory-constraint preferences via the verify oracle's folds.
    let mut preferences = Vec::new();
    if crate::pddl3::has_preferences(problem) {
        let weights = crate::pddl3::pref_weights(&cdomain, &cproblem);
        let objs = crate::ground::objects_by_type(&cdomain, &cproblem);
        for (name, phi) in &crate::pddl3::preferences(&cproblem.goal, &objs) {
            let phi = crate::constraints::expand_quantifiers(phi, &objs);
            preferences.push(PrefReport {
                name: name.clone(),
                satisfied: eval_in(&task, &s, &phi),
                weight: weights.get(name).copied().unwrap_or(0.0),
            });
        }
        let steps: Vec<(String, Vec<String>)> = plan
            .steps
            .iter()
            .map(|st| (st.action.clone(), st.args.clone()))
            .collect();
        if let Ok(v) = crate::verify::verify(domain_src, problem_src, &steps) {
            for (name, accepted) in v.constraint_prefs {
                let weight = weights.get(&name).copied().unwrap_or(0.0);
                preferences.push(PrefReport {
                    name,
                    satisfied: accepted,
                    weight,
                });
            }
        }
    }
    Ok((links, preferences))
}

fn display(action: &str, args: &[String]) -> String {
    if args.is_empty() {
        action.to_string()
    } else {
        format!("{} {}", action, args.join(" "))
    }
}

/// Evaluate a ground formula in a state (the verify oracle's semantics).
fn eval_in(task: &crate::packed::PackedTask, s: &crate::packed::State, f: &Formula) -> bool {
    match f {
        Formula::True => true,
        Formula::False => false,
        Formula::And(v) => v.iter().all(|x| eval_in(task, s, x)),
        Formula::Or(v) => v.iter().any(|x| eval_in(task, s, x)),
        Formula::Not(a) => !eval_in(task, s, a),
        Formula::Pref(_, inner) => eval_in(task, s, inner),
        Formula::Eq(a, b) => term(a) == term(b),
        Formula::Forall(_, inner) | Formula::Exists(_, inner) => eval_in(task, s, inner),
        Formula::Atom(p, args) => {
            let a: Vec<String> = args.iter().map(term).collect();
            let d = if a.is_empty() {
                format!("({p})")
            } else {
                format!("({} {})", p, a.join(" "))
            };
            task.fact_id(&d).is_some_and(|id| bitset::test(&s.bits, id))
        }
        Formula::Comp(op, l, r) => {
            let (Some(l), Some(r)) = (eval_expr(task, s, l), eval_expr(task, s, r)) else {
                return false;
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

fn eval_expr(task: &crate::packed::PackedTask, s: &crate::packed::State, e: &Expr) -> Option<f64> {
    Some(match e {
        Expr::Num(n) => *n,
        Expr::Fluent(name, args) => {
            let a: Vec<String> = args.iter().map(term).collect();
            let d = if a.is_empty() {
                format!("({name})")
            } else {
                format!("({} {})", name, a.join(" "))
            };
            let id = task.fluent_id(&d)?;
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

fn term(t: &Term) -> String {
    match t {
        Term::Const(c) => c.clone(),
        Term::Var(v) => v.clone(),
    }
}

/// Flatten a condition into per-conjunct display strings with the step's
/// arguments substituted (a top-level `and` reads as separate lines).
fn flatten_conditions(f: &Formula, sub: &HashMap<&str, &str>, out: &mut Vec<String>) {
    match f {
        Formula::And(v) => v.iter().for_each(|x| flatten_conditions(x, sub, out)),
        Formula::True => {}
        other => out.push(render_formula(other, sub)),
    }
}

fn render_formula(f: &Formula, sub: &HashMap<&str, &str>) -> String {
    let t = |x: &Term| -> String {
        match x {
            Term::Const(c) => c.clone(),
            Term::Var(v) => sub
                .get(v.as_str())
                .map_or_else(|| v.clone(), |s| s.to_string()),
        }
    };
    match f {
        Formula::True => "(and)".into(),
        Formula::False => "(or)".into(),
        Formula::Atom(p, args) => {
            let a: Vec<String> = args.iter().map(&t).collect();
            if a.is_empty() {
                format!("({p})")
            } else {
                format!("({} {})", p, a.join(" "))
            }
        }
        Formula::Not(inner) => format!("(not {})", render_formula(inner, sub)),
        Formula::And(v) => format!(
            "(and {})",
            v.iter()
                .map(|x| render_formula(x, sub))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Formula::Or(v) => format!(
            "(or {})",
            v.iter()
                .map(|x| render_formula(x, sub))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Formula::Pref(n, inner) => format!(
            "(preference {} {})",
            n.as_deref().unwrap_or(""),
            render_formula(inner, sub)
        ),
        Formula::Eq(a, b) => format!("(= {} {})", t(a), t(b)),
        Formula::Forall(_, inner) => format!("(forall ... {})", render_formula(inner, sub)),
        Formula::Exists(_, inner) => format!("(exists ... {})", render_formula(inner, sub)),
        Formula::Comp(op, l, r) => {
            let o = match op {
                CompOp::Lt => "<",
                CompOp::Le => "<=",
                CompOp::Eq => "=",
                CompOp::Ge => ">=",
                CompOp::Gt => ">",
            };
            format!("({} {} {})", o, render_expr(l, sub), render_expr(r, sub))
        }
    }
}

fn render_expr(e: &Expr, sub: &HashMap<&str, &str>) -> String {
    let t = |x: &Term| -> String {
        match x {
            Term::Const(c) => c.clone(),
            Term::Var(v) => sub
                .get(v.as_str())
                .map_or_else(|| v.clone(), |s| s.to_string()),
        }
    };
    match e {
        Expr::Num(n) => format!("{n}"),
        Expr::Fluent(name, args) => {
            let a: Vec<String> = args.iter().map(&t).collect();
            if a.is_empty() {
                format!("({name})")
            } else {
                format!("({} {})", name, a.join(" "))
            }
        }
        Expr::Add(a, b) => format!("(+ {} {})", render_expr(a, sub), render_expr(b, sub)),
        Expr::Sub(a, b) => format!("(- {} {})", render_expr(a, sub), render_expr(b, sub)),
        Expr::Mul(a, b) => format!("(* {} {})", render_expr(a, sub), render_expr(b, sub)),
        Expr::Div(a, b) => format!("(/ {} {})", render_expr(a, sub), render_expr(b, sub)),
        Expr::Neg(a) => format!("(- {})", render_expr(a, sub)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Step;

    const CHAIN_DOM: &str = "(define (domain chain)
      (:requirements :strips)
      (:predicates (a) (b) (c) (d))
      (:action MK-B :parameters () :precondition (a) :effect (b))
      (:action MK-C :parameters () :precondition (b) :effect (c)))";
    const CHAIN_PRB: &str = "(define (problem chain-1) (:domain chain)
      (:init (a)) (:goal (c)))";

    fn step(index: usize, action: &str, args: &[&str]) -> Step {
        Step {
            index,
            action: action.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            time: None,
            duration: None,
        }
    }

    fn plan_of(steps: Vec<Step>) -> Plan {
        let length = steps.len();
        Plan {
            steps,
            length,
            metric: None,
            makespan: None,
        }
    }

    /// The classical view: MK-C's precondition (B) links back to MK-B (its
    /// provider), while the static init fact (A) — never provided by any
    /// step — is dropped as noise.
    #[test]
    fn causal_links_name_the_last_achiever() {
        let plan = plan_of(vec![step(0, "MK-B", &[]), step(1, "MK-C", &[])]);
        let ex = explain(CHAIN_DOM, CHAIN_PRB, &plan).unwrap();
        assert_eq!(ex.kind, "classical");
        assert_eq!(ex.causal_links.len(), 1, "links: {:?}", ex.causal_links);
        let l = &ex.causal_links[0];
        assert_eq!(l.provider, Some(0));
        assert_eq!(l.consumer, 1);
        assert!(
            l.fact.to_ascii_uppercase().contains('B'),
            "fact: {}",
            l.fact
        );
    }

    /// An unmatchable plan step is an error, never a silent skip.
    #[test]
    fn bogus_step_is_an_error() {
        let plan = plan_of(vec![step(0, "MK-Z", &[])]);
        assert!(explain(CHAIN_DOM, CHAIN_PRB, &plan).is_err());
    }

    /// The temporal view: each MEND_FUSE carries its `over all (light m1)`
    /// invariant across exactly its own interval, args substituted.
    #[test]
    fn invariant_spans_cover_the_interval() {
        let dom = include_str!("../../../benchmarks/bench/eps-cross-domain.pddl");
        let prb = include_str!("../../../benchmarks/bench/eps-cross-p01.pddl");
        let sol = crate::api::solve(dom, prb, &crate::Options::default()).unwrap();
        assert!(sol.solved);
        let plan = sol.plan.unwrap();
        let ex = explain(dom, prb, &plan).unwrap();
        assert_eq!(ex.kind, "temporal");
        let mends: Vec<_> = ex
            .invariant_spans
            .iter()
            .filter(|s| s.action.contains("MEND_FUSE"))
            .collect();
        assert_eq!(mends.len(), 2, "spans: {:?}", ex.invariant_spans);
        for m in mends {
            assert!((m.end - m.start - 2.5).abs() < 1e-9);
            assert_eq!(m.conditions.len(), 1);
            let c = m.conditions[0].to_ascii_uppercase();
            assert!(c.contains("LIGHT") && c.contains("M1"), "condition: {c}");
        }
    }

    /// The PDDL3 view: per-preference verdict with `is-violated` weights.
    #[test]
    fn preference_breakdown_names_weights() {
        let prb = "(define (problem chain-p) (:domain chain)
          (:init (a))
          (:goal (and (c) (preference p1 (b)) (preference p2 (d))))
          (:metric minimize (+ (* 2 (is-violated p1)) (* 3 (is-violated p2)))))";
        let plan = plan_of(vec![step(0, "MK-B", &[]), step(1, "MK-C", &[])]);
        let ex = explain(CHAIN_DOM, prb, &plan).unwrap();
        let by_name: std::collections::HashMap<&str, (&bool, &f64)> = ex
            .preferences
            .iter()
            .map(|p| (p.name.as_str(), (&p.satisfied, &p.weight)))
            .collect();
        let p1 = by_name.iter().find(|(k, _)| k.eq_ignore_ascii_case("p1"));
        let p2 = by_name.iter().find(|(k, _)| k.eq_ignore_ascii_case("p2"));
        let (_, (s1, w1)) = p1.expect("p1 reported");
        let (_, (s2, w2)) = p2.expect("p2 reported");
        assert!(**s1, "p1 (b) holds in the final state");
        assert!(!**s2, "p2 (d) never achievable");
        assert_eq!(**w1, 2.0);
        assert_eq!(**w2, 3.0);
    }
}
