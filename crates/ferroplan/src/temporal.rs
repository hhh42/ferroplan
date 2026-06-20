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

use crate::types::{Action, Domain, Effect, Expr, Formula, Problem, Sym, Term, TimeSpec};

/// Temporal metadata for one durative action, paired with its snap-actions.
#[derive(Clone, Debug)]
pub struct SnapInfo {
    /// Name of the generated start snap-action (e.g. `MOVE-START`).
    pub start_action: Sym,
    /// Name of the generated end snap-action (e.g. `MOVE-END`).
    pub end_action: Sym,
    /// `RUNNING-…` token predicate that pairs a start with its end.
    pub running_pred: Sym,
    /// Duration expression (over the action's parameters / fluents).
    pub duration: Expr,
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

        // start snap: (at-start conditions + invariant) -> at-start effects + token.
        // The invariant is also checked at both endpoints (a sound approximation
        // of `over all`: an interval violation surfaces when the END precondition
        // fails, e.g. a concurrent action removing the invariant fact).
        let start_pre = and_formulas(vec![
            pick_conditions(da, TimeSpec::Start),
            invariant.clone(),
        ]);
        let mut start_eff = pick_effects(da, TimeSpec::Start);
        start_eff.push(Effect::Add(running.clone(), run_args.clone()));
        d.actions.push(Action {
            name: start_name.clone(),
            params: da.params.clone(),
            precond: start_pre,
            effect: and_effects(start_eff),
        });

        // end snap: (at-end conditions + invariant + token) -> at-end effects, drop token
        let end_pre = and_formulas(vec![
            pick_conditions(da, TimeSpec::End),
            invariant.clone(),
            Formula::Atom(running.clone(), run_args.clone()),
        ]);
        let mut end_eff = pick_effects(da, TimeSpec::End);
        end_eff.push(Effect::Del(running.clone(), run_args.clone()));
        d.actions.push(Action {
            name: end_name.clone(),
            params: da.params.clone(),
            precond: end_pre,
            effect: and_effects(end_eff),
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
    TemporalCompiled {
        domain: d,
        problem: problem.clone(),
        snaps,
    }
}

// ---------------------------------------------------------------------------
// T3: decision-epoch temporal search.
// ---------------------------------------------------------------------------

use crate::ground::{ground, Outcome};
use crate::heuristic::{relaxed, Scratch};
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

#[derive(Clone, Copy)]
enum Kind {
    /// durative start: fixed duration + the matching end op index
    Start {
        dur: f64,
        end_op: usize,
    },
    End,
    Classical,
    /// a start whose duration/end we can't resolve (e.g. non-constant duration);
    /// never applied (the domain feature is unsupported in this first cut)
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
}

fn tkey(task: &PackedTask, n: &TNode) -> (StateKey, Vec<(i64, usize)>) {
    let ag = n
        .agenda
        .iter()
        .map(|&(t, o)| ((t * 1000.0).round() as i64, o))
        .collect();
    (task.state_key(&n.state), ag)
}

/// Solve a temporal (durative-action) problem by decision-epoch forward search.
/// Returns a timed plan, or None if unsolved within the node budget. First cut:
/// fixed (constant) durations; the `over all` invariant is enforced at the start
/// and end happenings via the snap preconditions.
pub fn solve(domain: &Domain, problem: &Problem, threads: usize) -> Option<TimedPlan> {
    const MAX_NODES: usize = 400_000;
    let c = compile(domain, problem);
    let task = match ground(&c.domain, &c.problem, threads) {
        Outcome::Task(t) => t,
        Outcome::GoalTrue => {
            return Some(TimedPlan {
                steps: Vec::new(),
                makespan: 0.0,
            })
        }
        _ => return None,
    };

    // fixed duration per start-action name
    let mut dur_by_start: HashMap<&str, f64> = HashMap::new();
    for s in &c.snaps {
        if let Expr::Num(n) = s.duration {
            dur_by_start.insert(s.start_action.as_str(), n);
        }
    }
    let start_names: HashSet<&str> = c.snaps.iter().map(|s| s.start_action.as_str()).collect();
    let end_names: HashSet<&str> = c.snaps.iter().map(|s| s.end_action.as_str()).collect();
    let by_display: HashMap<&str, usize> = task
        .op_display
        .iter()
        .enumerate()
        .map(|(i, d)| (d.as_str(), i))
        .collect();

    let kind: Vec<Kind> = (0..task.n_ops)
        .map(|oi| {
            let disp = &task.op_display[oi];
            let head = disp.split_whitespace().next().unwrap_or("");
            if start_names.contains(head) {
                let end_disp = disp.replacen("-START", "-END", 1);
                match (dur_by_start.get(head), by_display.get(end_disp.as_str())) {
                    (Some(&dur), Some(&end_op)) => Kind::Start { dur, end_op },
                    _ => Kind::Skip,
                }
            } else if end_names.contains(head) {
                Kind::End
            } else {
                Kind::Classical
            }
        })
        .collect();

    let mut sc = Scratch::new(&task);
    let init = task.initial();
    relaxed(&task, &mut sc, &init.bits, &init.fv, &init.fdef)?; // dead end -> None
    let mut nodes = vec![TNode {
        state: init,
        time: 0.0,
        agenda: Vec::new(),
        father: usize::MAX,
        ev: None,
    }];
    let mut heap: BinaryHeap<Reverse<(i32, usize)>> = BinaryHeap::new();
    heap.push(Reverse((0, 0)));
    let mut visited: HashSet<(StateKey, Vec<(i64, usize)>)> = HashSet::new();
    visited.insert(tkey(&task, &nodes[0]));

    let push = |nodes: &mut Vec<TNode>,
                heap: &mut BinaryHeap<Reverse<(i32, usize)>>,
                visited: &mut HashSet<(StateKey, Vec<(i64, usize)>)>,
                sc: &mut Scratch,
                n: TNode| {
        if let Some(h) = relaxed(&task, sc, &n.state.bits, &n.state.fv, &n.state.fdef) {
            let k = tkey(&task, &n);
            if visited.insert(k) {
                let idx = nodes.len();
                nodes.push(n);
                heap.push(Reverse((h, idx)));
            }
        }
    };

    while let Some(Reverse((_h, ni))) = heap.pop() {
        if task.goal_met(&nodes[ni].state) && nodes[ni].agenda.is_empty() {
            return Some(reconstruct(&task, &nodes, ni, &c));
        }
        if nodes.len() > MAX_NODES {
            break;
        }
        let time = nodes[ni].time;

        // (a) start a durative action / apply an instantaneous classical action
        for (oi, k) in kind.iter().enumerate() {
            match *k {
                Kind::Start { dur, end_op } => {
                    if task.op_applicable(oi, &nodes[ni].state) {
                        let ns = task.apply(oi, &nodes[ni].state);
                        let mut ag = nodes[ni].agenda.clone();
                        let te = time + dur;
                        let pos = ag.partition_point(|x| x.0 <= te);
                        ag.insert(pos, (te, end_op));
                        push(
                            &mut nodes,
                            &mut heap,
                            &mut visited,
                            &mut sc,
                            TNode {
                                state: ns,
                                time,
                                agenda: ag,
                                father: ni,
                                ev: Some((oi, time)),
                            },
                        );
                    }
                }
                Kind::Classical => {
                    if task.op_applicable(oi, &nodes[ni].state) {
                        let ns = task.apply(oi, &nodes[ni].state);
                        let ag = nodes[ni].agenda.clone();
                        push(
                            &mut nodes,
                            &mut heap,
                            &mut visited,
                            &mut sc,
                            TNode {
                                state: ns,
                                time,
                                agenda: ag,
                                father: ni,
                                ev: Some((oi, time)),
                            },
                        );
                    }
                }
                Kind::End | Kind::Skip => {}
            }
        }

        // (b) advance time: fire the earliest pending end (if its end conditions +
        // invariant still hold). If not applicable, this schedule is invalid.
        if let Some(&(te, end_op)) = nodes[ni].agenda.first() {
            if task.op_applicable(end_op, &nodes[ni].state) {
                let ns = task.apply(end_op, &nodes[ni].state);
                let ag = nodes[ni].agenda[1..].to_vec();
                push(
                    &mut nodes,
                    &mut heap,
                    &mut visited,
                    &mut sc,
                    TNode {
                        state: ns,
                        time: te,
                        agenda: ag,
                        father: ni,
                        ev: Some((end_op, te)),
                    },
                );
            }
        }
    }
    None
}

/// Walk the father chain into a timed plan: each START becomes a durative step
/// with its duration (the END is implied); END events are dropped; classical
/// actions appear instantaneously.
fn reconstruct(task: &PackedTask, nodes: &[TNode], goal: usize, c: &TemporalCompiled) -> TimedPlan {
    let dur_by_start: HashMap<&str, f64> = c
        .snaps
        .iter()
        .filter_map(|s| match s.duration {
            Expr::Num(n) => Some((s.start_action.as_str(), n)),
            _ => None,
        })
        .collect();
    let start_names: HashSet<&str> = c.snaps.iter().map(|s| s.start_action.as_str()).collect();
    let end_names: HashSet<&str> = c.snaps.iter().map(|s| s.end_action.as_str()).collect();

    let mut events: Vec<(usize, f64)> = Vec::new();
    let mut cur = goal;
    while let Some((op, t)) = nodes[cur].ev {
        events.push((op, t));
        cur = nodes[cur].father;
    }
    events.reverse();

    let mut steps = Vec::new();
    let mut makespan = 0.0f64;
    for (op, t) in events {
        let disp = &task.op_display[op];
        let head = disp.split_whitespace().next().unwrap_or("");
        let args = disp
            .split_whitespace()
            .skip(1)
            .collect::<Vec<_>>()
            .join(" ");
        if end_names.contains(head) {
            // implied by the matching start's duration
            makespan = makespan.max(t);
            continue;
        }
        let (name, duration) = if start_names.contains(head) {
            let base = head.trim_end_matches("-START");
            let dur = dur_by_start.get(head).copied();
            makespan = makespan.max(t + dur.unwrap_or(0.0));
            (base.to_string(), dur)
        } else {
            makespan = makespan.max(t);
            (head.to_string(), None)
        };
        let action = if args.is_empty() {
            name
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
