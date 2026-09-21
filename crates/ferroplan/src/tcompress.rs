//! THE COMPRESSION RUNG (0.28 Lane T): plan a temporal task as a classical
//! one, then give the plan its times back.
//!
//! Every durative action becomes ONE instantaneous action -- condition =
//! start ∧ over-all ∧ end, effect = start then end -- the classical ladder
//! plans that, and the plan is laid out on the clock by a left-shift over
//! the ops' read/write sets. The result is judged by [`crate::temporal::validate`]
//! against the ORIGINAL pair before anyone sees it, so the rung can be
//! wrong about a task without ever being wrong about a plan.
//!
//! Why it exists. The decision-epoch search explores start/end snap
//! interleavings, which is the right space for tasks that NEED concurrency
//! and an enormous one for tasks that do not. IPC-5's temporal tracks do
//! not: SGPlan5 -- which plans them sequentially and schedules afterwards --
//! solves `pathways-metric-time` in a median 0.22 s where this engine's
//! ladder stops at the 60 s wall on all 30, and the 2026-09-20 probe
//! (the same transform, run outside the engine) converted 121 rows the
//! boards record as unsolved, every one VAL-valid against the original
//! task: `pipesworld-metric-time` 10 → 42 of 50 against SGPlan5's 30.
//!
//! What it does not do. It declines tasks with required concurrency
//! ([`crate::sat::requires_concurrency`]), timed initial literals, or
//! trajectory constraints -- a compressed action has no inside for an
//! `always` to look at, and `within` needs the clock the classical search
//! does not have. And it is a BOUNDED bet ([`WALL_FRAC`]): the compressed
//! task can be as hard classically as the original was temporally
//! (`rovers-metric-time`, `tpp-metric-time` -- the numeric-guidance residue
//! recorded in docs/roadmap-0.28.md), and a rung that cannot solve must not
//! take the ladder's wall with it.

use crate::packed::{PackedTask, State};
use crate::temporal::{TimedPlan, TimedStep, EPS};
use crate::types::{
    Action, Domain, DurativeAction, Effect, Expr, Formula, NExpr, Problem, Sym, Term, TimeSpec,
};
use std::collections::HashMap;

/// The share of the remaining wall the FIRST attempt may spend
/// (`FF_TCOMPRESS_WALL_FRAC`). 81 of the probe's 124 conversions landed
/// inside 5 s and 94 inside 15 s; the rows a larger slice would add are
/// the ones [`solve`]'s second call -- after a ladder that gave up with
/// wall to spare -- picks up.
pub const WALL_FRAC: f64 = 0.25;

/// Why the rung does not apply, or `None` if it does. Lifted and cheap:
/// this runs on every temporal solve.
pub fn declines(domain: &Domain, problem: &Problem) -> Option<&'static str> {
    if std::env::var("FF_NO_TCOMPRESS").is_ok() {
        return Some("FF_NO_TCOMPRESS");
    }
    if domain.durative_actions.is_empty() {
        return Some("no durative actions");
    }
    if !domain.constraints.is_empty() || !problem.constraints.is_empty() {
        return Some("trajectory constraints");
    }
    if !problem.til.is_empty() {
        return Some("timed initial literals");
    }
    if domain
        .durative_actions
        .iter()
        .any(|da| da.duration.chosen().is_none())
    {
        return Some("an unconstrained duration");
    }
    if crate::sat::requires_concurrency(domain, problem) {
        return Some("required concurrency");
    }
    None
}

fn flatten<'a>(e: &'a Effect, out: &mut Vec<&'a Effect>) {
    match e {
        Effect::And(v) => v.iter().for_each(|x| flatten(x, out)),
        other => out.push(other),
    }
}

/// Drop from an AT-END condition the literals the action's own START effects
/// establish: `(at start (busy ?m))` + `(at end (busy ?m))` is a token the
/// action holds for itself, not something the world must provide
/// beforehand. Top-level conjuncts only; anything else is kept, and a
/// condition this leaves wrongly strict costs completeness, never soundness.
///
/// OVER-ALL conditions are deliberately NOT treated this way. PDDL2.1 reads
/// the invariant over the OPEN interval, so a self-established one is legal
/// there -- but this engine's snap compile (`temporal::compile`) also asks
/// for the invariant in the START snap's precondition, and the validator
/// judges by that rule. A compressed action more permissive than the
/// validator only produces plans it will refuse.
fn minus_self_established(
    f: &Formula,
    adds: &[(&Sym, &Vec<Term>)],
    dels: &[(&Sym, &Vec<Term>)],
) -> Formula {
    let own = |f: &Formula| match f {
        Formula::Atom(p, a) => adds.iter().any(|(q, b)| *q == p && *b == a),
        Formula::Not(inner) => match inner.as_ref() {
            Formula::Atom(p, a) => dels.iter().any(|(q, b)| *q == p && *b == a),
            _ => false,
        },
        _ => false,
    };
    match f {
        Formula::And(v) => Formula::And(
            v.iter()
                .filter(|x| !own(x))
                .map(|x| minus_self_established(x, adds, dels))
                .collect(),
        ),
        other if own(other) => Formula::True,
        other => other.clone(),
    }
}

/// One durative action as one classical action.
fn compress(da: &DurativeAction) -> Action {
    let dur = da
        .duration
        .chosen()
        .expect("declines() refuses unconstrained durations");
    let with_dur = |e: &Expr| crate::temporal::expr_subst_duration(e, dur);

    let mut start: Vec<&Effect> = Vec::new();
    let mut end: Vec<&Effect> = Vec::new();
    for (ts, e) in &da.effects {
        flatten(
            e,
            if *ts == TimeSpec::Start {
                &mut start
            } else {
                &mut end
            },
        );
    }
    let adds: Vec<(&Sym, &Vec<Term>)> = start
        .iter()
        .filter_map(|e| match e {
            Effect::Add(p, a) => Some((p, a)),
            _ => None,
        })
        .collect();
    let dels: Vec<(&Sym, &Vec<Term>)> = start
        .iter()
        .filter_map(|e| match e {
            Effect::Del(p, a) => Some((p, a)),
            _ => None,
        })
        .collect();

    let precond = Formula::And(
        da.conditions
            .iter()
            .map(|(ts, f)| {
                let f = crate::temporal::formula_map_exprs(f, &with_dur);
                if *ts == TimeSpec::End {
                    minus_self_established(&f, &adds, &dels)
                } else {
                    f
                }
            })
            .collect(),
    );

    // Start, then end. A classical effect list applies its deletes before
    // its adds, so "deleted at start, added at end" already nets to the
    // add. The other order does not: "added at start, deleted at end" (a
    // token held for the duration) must net to the DELETE, so the start add
    // is dropped -- its fact still sits in the op's write set through the
    // delete, which is what the scheduler needs to know.
    let end_dels: Vec<(&Sym, &Vec<Term>)> = end
        .iter()
        .filter_map(|e| match e {
            Effect::Del(p, a) => Some((p, a)),
            _ => None,
        })
        .collect();
    let effect = Effect::And(
        start
            .iter()
            .filter(|e| match e {
                Effect::Add(p, a) => !end_dels.iter().any(|(q, b)| *q == p && *b == a),
                _ => true,
            })
            .chain(end.iter())
            .map(|e| crate::temporal::effect_map_exprs(e, &with_dur))
            .collect(),
    );
    Action {
        name: da.name.clone(),
        params: da.params.clone(),
        precond,
        effect,
        monitored: false,
    }
}

/// The classical pair the ladder plans. The metric goes: `total-time` is
/// not a classical quantity, and this rung prices coverage -- the clock
/// comes back in [`lay_out`].
pub fn compile(domain: &Domain, problem: &Problem) -> (Domain, Problem) {
    let mut d = domain.clone();
    d.actions
        .extend(domain.durative_actions.iter().map(compress));
    d.durative_actions.clear();
    let mut p = problem.clone();
    p.metric = None;
    (d, p)
}

fn nexpr_fluents(e: &NExpr, out: &mut Vec<u32>) {
    match e {
        NExpr::Num(_) => {}
        NExpr::Fluent(f) => out.push(*f),
        NExpr::Add(a, b) | NExpr::Sub(a, b) | NExpr::Mul(a, b) | NExpr::Div(a, b) => {
            nexpr_fluents(a, out);
            nexpr_fluents(b, out);
        }
        NExpr::Neg(a) => nexpr_fluents(a, out),
    }
}

fn expr_fluents(e: &Expr, bind: &HashMap<&str, &str>, task: &PackedTask, out: &mut Vec<u32>) {
    match e {
        Expr::Num(_) => {}
        Expr::Fluent(name, terms) => {
            let mut disp = format!("({name}");
            for t in terms {
                disp.push(' ');
                match t {
                    Term::Const(c) => disp.push_str(c),
                    Term::Var(v) => match bind.get(v.as_str()) {
                        Some(o) => disp.push_str(o),
                        None => return,
                    },
                }
            }
            disp.push(')');
            if let Some(id) = task.fluent_id(&disp) {
                out.push(id as u32);
            }
        }
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
            expr_fluents(a, bind, task, out);
            expr_fluents(b, bind, task, out);
        }
        Expr::Neg(a) => expr_fluents(a, bind, task, out),
    }
}

/// What one grounded op reads and writes -- the whole of what the
/// left-shift is allowed to know about it.
#[derive(Default)]
struct Touch {
    read_facts: Vec<u32>,
    write_facts: Vec<u32>,
    read_fluents: Vec<u32>,
    write_fluents: Vec<u32>,
}

fn touch(task: &PackedTask, oi: usize) -> Touch {
    let mut t = Touch::default();
    t.read_facts.extend_from_slice(task.pre_pos.slice(oi));
    t.write_facts.extend_from_slice(task.add.slice(oi));
    t.write_facts.extend_from_slice(task.del.slice(oi));
    for np in task.pre_num.slice(oi) {
        nexpr_fluents(&np.lhs, &mut t.read_fluents);
        nexpr_fluents(&np.rhs, &mut t.read_fluents);
    }
    for ne in task.num_eff.slice(oi) {
        t.write_fluents.push(ne.target);
        nexpr_fluents(&ne.value, &mut t.read_fluents);
    }
    for ce in task.cond_effs(oi) {
        t.read_facts.extend_from_slice(&ce.cond_pos);
        t.read_facts.extend_from_slice(&ce.cond_neg);
        t.write_facts.extend_from_slice(&ce.add);
        t.write_facts.extend_from_slice(&ce.del);
        for np in &ce.cond_num {
            nexpr_fluents(&np.lhs, &mut t.read_fluents);
            nexpr_fluents(&np.rhs, &mut t.read_fluents);
        }
        for ne in &ce.num {
            t.write_fluents.push(ne.target);
            nexpr_fluents(&ne.value, &mut t.read_fluents);
        }
    }
    t
}

/// Put a classical plan over the compressed task back on the clock.
///
/// `shift = false` is the plan laid end to end: valid whenever the
/// compression was, since every action then runs alone. `shift = true`
/// left-shifts it: a step starts ε after the last earlier step it
/// INTERFERES with has ended -- one that wrote something it reads or
/// writes, or read something it writes -- and steps that touch nothing in
/// common overlap freely. Interference is judged on whole ops, start and
/// end together, which is coarser than the snap-level truth and errs only
/// toward serializing. Durations are read in the state each step starts
/// from, in plan order; a step whose duration reads a fluent is ordered
/// after that fluent's writers like any other reader.
///
/// `None` if a step is not an action of `domain` or its duration does not
/// evaluate to a non-negative number.
pub fn lay_out(
    domain: &Domain,
    task: &PackedTask,
    ops: &[usize],
    shift: bool,
) -> Option<TimedPlan> {
    let durative: HashMap<String, &DurativeAction> = domain
        .durative_actions
        .iter()
        .map(|da| (da.name.to_ascii_uppercase(), da))
        .collect();
    let mut fact_written = vec![0.0f64; task.n_facts];
    let mut fact_read = vec![0.0f64; task.n_facts];
    let n_fluents = task.initial().fv.len();
    let mut fluent_written = vec![0.0f64; n_fluents];
    let mut fluent_read = vec![0.0f64; n_fluents];

    let mut state: State = task.initial();
    let mut steps = Vec::with_capacity(ops.len());
    let mut prev_end = 0.0f64;
    let mut makespan = 0.0f64;
    for &oi in ops {
        let name = &task.op_display[oi];
        let mut it = name.split_whitespace();
        let head = it.next()?.to_ascii_uppercase();
        let args: Vec<&str> = it.collect();
        let mut t = touch(task, oi);
        let duration = match durative.get(&head) {
            Some(da) => {
                if da.params.len() != args.len() {
                    return None;
                }
                let bind: HashMap<&str, &str> = da
                    .params
                    .iter()
                    .map(|(p, _)| p.as_str())
                    .zip(args.iter().copied())
                    .collect();
                let e = da.duration.chosen()?;
                expr_fluents(e, &bind, task, &mut t.read_fluents);
                let d = crate::temporal::eval_expr(e, &bind, task, &state)?;
                if !d.is_finite() || d < 0.0 {
                    return None;
                }
                Some(d)
            }
            // An instantaneous action of a temporal domain: a happening.
            None => None,
        };
        let after = if shift {
            let mut after = 0.0f64;
            for &f in t.read_facts.iter().chain(&t.write_facts) {
                after = after.max(fact_written[f as usize]);
            }
            for &f in &t.write_facts {
                after = after.max(fact_read[f as usize]);
            }
            for &f in t.read_fluents.iter().chain(&t.write_fluents) {
                after = after.max(fluent_written[f as usize]);
            }
            for &f in &t.write_fluents {
                after = after.max(fluent_read[f as usize]);
            }
            after
        } else {
            prev_end
        };
        // Millisecond grid: ε-separated happenings stay ε-separated through
        // the validator's (and VAL's) rounding.
        let time = ((after + EPS) / EPS).round() * EPS;
        let end = time + duration.unwrap_or(0.0);
        for &f in &t.write_facts {
            fact_written[f as usize] = fact_written[f as usize].max(end);
        }
        for &f in &t.read_facts {
            fact_read[f as usize] = fact_read[f as usize].max(end);
        }
        for &f in &t.write_fluents {
            fluent_written[f as usize] = fluent_written[f as usize].max(end);
        }
        for &f in &t.read_fluents {
            fluent_read[f as usize] = fluent_read[f as usize].max(end);
        }
        prev_end = prev_end.max(end);
        makespan = makespan.max(end);
        if !task.op_applicable(oi, &state) {
            return None;
        }
        state = task.apply(oi, &state);
        steps.push(TimedStep {
            time,
            action: name.clone(),
            duration,
        });
    }
    // The schedule is built in PLAN order; a plan reads in TIME order.
    steps.sort_by(|a, b| a.time.total_cmp(&b.time));
    Some(TimedPlan { steps, makespan })
}

fn subst_effect(e: &Effect, b: &HashMap<Sym, Sym>) -> Effect {
    use crate::pddl3::{subst_expr, subst_formula, subst_term};
    let terms = |a: &[Term]| a.iter().map(|t| subst_term(t, b)).collect::<Vec<_>>();
    match e {
        Effect::Add(p, a) => Effect::Add(p.clone(), terms(a)),
        Effect::Del(p, a) => Effect::Del(p.clone(), terms(a)),
        Effect::Num(op, f, a, v) => Effect::Num(*op, f.clone(), terms(a), subst_expr(v, b)),
        Effect::And(v) => Effect::And(v.iter().map(|x| subst_effect(x, b)).collect()),
        Effect::When(c, inner) => {
            Effect::When(subst_formula(c, b), Box::new(subst_effect(inner, b)))
        }
        // an inner quantifier may shadow a parameter: leave its own vars alone
        Effect::Forall(vars, inner) => {
            let mut b2 = b.clone();
            for (v, _) in vars {
                b2.remove(v);
            }
            Effect::Forall(vars.clone(), Box::new(subst_effect(inner, &b2)))
        }
    }
}

/// [`temporal::validate`] at the PLAN's size instead of the task's.
///
/// The validator grounds the snap-compiled task it replays on, and for the
/// domains this rung exists for that grounding is the very thing the ladder
/// could not afford: `pipesworld-metric-time` i20's plan is found in 4 s and
/// its validation was still grounding 96 s later (the board records the
/// same blow-up as `mem-cap`). So the domain is specialised first -- one
/// parameterless durative action per distinct ground step, its parameters
/// substituted -- and the plan renamed to match. Each specialised action IS
/// the ground instance the step named, so the verdict is the original
/// validator's, on the original semantics, at a grounding of `steps` ops.
fn validate_at_plan_size(
    domain: &Domain,
    problem: &Problem,
    plan: &TimedPlan,
) -> Result<(), String> {
    let by_name: HashMap<String, &DurativeAction> = domain
        .durative_actions
        .iter()
        .map(|da| (da.name.to_ascii_uppercase(), da))
        .collect();
    let mut special = domain.clone();
    special.durative_actions.clear();
    let mut renamed: HashMap<&str, String> = HashMap::new();
    let mut steps = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let mut it = step.action.split_whitespace();
        let head = it.next().unwrap_or("").to_ascii_uppercase();
        let Some(da) = by_name.get(&head).filter(|_| step.duration.is_some()) else {
            steps.push(step.clone()); // an instantaneous action: replayed as it is
            continue;
        };
        let args: Vec<&str> = it.collect();
        if args.len() != da.params.len() {
            return Err(format!("`{}` has the wrong arity", step.action));
        }
        let next = renamed.len();
        let name = renamed.entry(step.action.as_str()).or_insert_with(|| {
            let b: HashMap<Sym, Sym> = da
                .params
                .iter()
                .zip(&args)
                .map(|((v, _), a)| (v.clone(), a.to_string()))
                .collect();
            let name = format!("{head}--STEP{next}");
            special.durative_actions.push(DurativeAction {
                name: name.clone(),
                params: Vec::new(),
                duration: crate::types::Duration {
                    min: da
                        .duration
                        .min
                        .as_ref()
                        .map(|e| crate::pddl3::subst_expr(e, &b)),
                    max: da
                        .duration
                        .max
                        .as_ref()
                        .map(|e| crate::pddl3::subst_expr(e, &b)),
                },
                conditions: da
                    .conditions
                    .iter()
                    .map(|(ts, f)| (*ts, crate::pddl3::subst_formula(f, &b)))
                    .collect(),
                effects: da
                    .effects
                    .iter()
                    .map(|(ts, e)| (*ts, subst_effect(e, &b)))
                    .collect(),
            });
            name
        });
        steps.push(TimedStep {
            time: step.time,
            action: name.clone(),
            duration: step.duration,
        });
    }
    crate::temporal::validate(
        &special,
        problem,
        &TimedPlan {
            steps,
            makespan: plan.makespan,
        },
    )
}

/// How much one attempt at the rung may spend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bet {
    /// BEFORE the decision-epoch ladder: a bounded bet, because a rung that
    /// cannot solve must not take the ladder's resources with it. Under an
    /// armed wall that is a share of the remaining wall ([`WALL_FRAC`]).
    /// With NO wall it has to be bounded some other way, and deterministically
    /// -- the classical ladder's own caps are minutes long, and the first cut
    /// of this rung spent 32 s failing on a task the ladder below solves in
    /// 40 ms (its demand tiers guide numeric accumulation the classical
    /// relaxation does not). So: [`UNWALLED_EVALS`] per classical rung.
    First,
    /// AFTER a ladder that found nothing: whatever is left, uncapped.
    Rest,
}

/// The unwalled first bet's per-rung evaluation cap. 76 of the probe's 124
/// conversions fit inside 5,000 in TOTAL and about half inside this; the rest are what [`Bet::Rest`] is for.
pub const UNWALLED_EVALS: usize = 2_000;

/// Run the rung. `None` = no validated plan; the caller carries on down the
/// temporal ladder exactly as if the rung were not there.
pub fn solve(domain: &Domain, problem: &Problem, threads: usize, bet: Bet) -> Option<TimedPlan> {
    let dbg = std::env::var("FF_WALL_DEBUG").is_ok();
    let t0 = crate::clock::Clock::now();
    let (cd, cp) = compile(domain, problem);
    // The bound covers the BET -- grounding and searching the compressed
    // task -- and ends there: validating a plan already found is not
    // optional work, and must not be cut by the rung's own deadline.
    let walled = crate::search::wall_remaining_secs();
    let frac = crate::search::wall_frac_env("FF_TCOMPRESS_WALL_FRAC", WALL_FRAC);
    let slice = match (bet, walled) {
        (Bet::First, Some(rem)) if frac < 1.0 => {
            crate::search::tighten_deadline(rem * (1.0 - frac))
        }
        _ => None,
    };
    let cfg = match (bet, walled) {
        (Bet::First, None) => crate::search::SearchCfg {
            max_eval: UNWALLED_EVALS,
            ..Default::default()
        },
        _ => crate::search::SearchCfg::default(),
    };
    let found = match crate::ground::ground(&cd, &cp, threads) {
        crate::ground::Outcome::Task(task) => {
            let o = crate::search::plan(&task, threads, cfg, true, None);
            if dbg {
                eprintln!(
                    "wall: compression rung: {} after {} evals, {:.2} s",
                    if o.ops.is_some() { "a plan" } else { "no plan" },
                    o.evaluated,
                    t0.elapsed_secs()
                );
            }
            o.ops.map(|ops| (task, ops))
        }
        crate::ground::Outcome::GoalTrue => {
            return Some(TimedPlan {
                steps: Vec::new(),
                makespan: 0.0,
            })
        }
        _ => {
            if dbg {
                eprintln!("wall: compression rung: the compressed task did not ground");
            }
            None
        }
    };
    drop(slice);
    let (task, ops) = found?;
    // The left-shifted layout goes first because it is the one worth
    // having; the end-to-end layout is the fallback a too-eager shift
    // leaves behind.
    for shift in [true, false] {
        let Some(plan) = lay_out(domain, &task, &ops, shift) else {
            continue;
        };
        match validate_at_plan_size(domain, problem, &plan) {
            Ok(()) => {
                if dbg {
                    eprintln!(
                        "wall: compression rung: {} steps, makespan {:.3} ({}), {:.2} s",
                        plan.steps.len(),
                        plan.makespan,
                        if shift { "left-shifted" } else { "end to end" },
                        t0.elapsed_secs()
                    );
                }
                return Some(plan);
            }
            Err(why) => {
                if dbg {
                    eprintln!(
                        "wall: compression rung: {} layout refused by the validator: {why}",
                        if shift { "left-shifted" } else { "end-to-end" }
                    );
                }
            }
        }
    }
    None
}
