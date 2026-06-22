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
use crate::heuristic::{relaxed, relaxed_helpful, Scratch};
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

impl TimedPlan {
    /// Render in the IPC temporal plan format: `t: (action args) [duration]`.
    pub fn to_ipc(&self) -> String {
        let mut s = String::new();
        for step in &self.steps {
            s.push_str(&format!(
                "{:.3}: ({}) [{:.3}]\n",
                step.time,
                step.action.to_lowercase(),
                step.duration.unwrap_or(0.001),
            ));
        }
        s
    }
}

#[derive(Clone, Copy)]
enum Kind {
    /// durative start: resolved duration (constant or parameter-dependent) + the
    /// matching end op index
    Start {
        dur: f64,
        end_op: usize,
    },
    End,
    Classical,
    /// a start whose duration/end can't be resolved (undefined duration fluent,
    /// non-positive value, or missing end op); never applied
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
    /// number of happenings to reach this node (depth `g`, for the heap key).
    g: u32,
    /// FF helpful start/classical ops for this state under pruning (empty = no
    /// restriction / fall back to a full scan). Only populated in the pruned pass.
    helpful: Vec<u32>,
}

fn tkey(task: &PackedTask, n: &TNode) -> (StateKey, Vec<(i64, usize)>) {
    let ag = n
        .agenda
        .iter()
        .map(|&(t, o)| ((t * 1000.0).round() as i64, o))
        .collect();
    (task.state_key(&n.state), ag)
}

/// Evaluate a (possibly parameter-dependent) duration for one grounded
/// snap-action. The action's parameters are bound positionally to the grounded
/// args; fluents are read from the INITIAL state — IPC temporal durations depend
/// on static fluents like `(= ?duration (/ (distance ?a ?b) (speed ?v)))`, which
/// keep their init value. Returns None for a non-positive duration, an undefined
/// fluent, or division by zero (the caller then skips the action).
fn eval_duration(snap: &SnapInfo, args: &[&str], task: &PackedTask, init: &State) -> Option<f64> {
    let bind: HashMap<&str, &str> = snap
        .params
        .iter()
        .map(|(p, _)| p.as_str())
        .zip(args.iter().copied())
        .collect();
    let d = eval_expr(&snap.duration, &bind, task, init)?;
    if d.is_finite() && d > 0.0 {
        Some(d)
    } else {
        None
    }
}

fn eval_expr(e: &Expr, bind: &HashMap<&str, &str>, task: &PackedTask, init: &State) -> Option<f64> {
    match e {
        Expr::Num(n) => Some(*n),
        Expr::Fluent(name, terms) => {
            let mut disp = String::from("(");
            disp.push_str(name);
            for t in terms {
                disp.push(' ');
                match t {
                    Term::Const(c) => disp.push_str(c),
                    Term::Var(v) => disp.push_str(bind.get(v.as_str())?),
                }
            }
            disp.push(')');
            let id = task.fluent_id(&disp)?;
            init.fdef[id].then(|| init.fv[id])
        }
        Expr::Add(a, b) => Some(eval_expr(a, bind, task, init)? + eval_expr(b, bind, task, init)?),
        Expr::Sub(a, b) => Some(eval_expr(a, bind, task, init)? - eval_expr(b, bind, task, init)?),
        Expr::Mul(a, b) => Some(eval_expr(a, bind, task, init)? * eval_expr(b, bind, task, init)?),
        Expr::Div(a, b) => {
            let d = eval_expr(b, bind, task, init)?;
            if d == 0.0 {
                return None;
            }
            Some(eval_expr(a, bind, task, init)? / d)
        }
        Expr::Neg(a) => Some(-eval_expr(a, bind, task, init)?),
    }
}

/// Solve a temporal (durative-action) problem by decision-epoch forward search.
/// Returns a timed plan, or None if unsolved within the node budget. Durations
/// may be constants or parameter-dependent (evaluated against the initial state);
/// the `over all` invariant is enforced at the start and end happenings via the
/// snap preconditions.
pub fn solve(domain: &Domain, problem: &Problem, threads: usize) -> Option<TimedPlan> {
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

    // Resolve each grounded snap-action's duration (constant OR parameter-dependent,
    // evaluated against the initial state) and pair starts with their end op.
    let init = task.initial();
    let snap_by_start: HashMap<&str, &SnapInfo> = c
        .snaps
        .iter()
        .map(|s| (s.start_action.as_str(), s))
        .collect();
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
            if let Some(snap) = snap_by_start.get(head) {
                let args: Vec<&str> = disp.split_whitespace().skip(1).collect();
                let end_disp = disp.replacen("-START", "-END", 1);
                match (
                    eval_duration(snap, &args, &task, &init),
                    by_display.get(end_disp.as_str()),
                ) {
                    (Some(dur), Some(&end_op)) => Kind::Start { dur, end_op },
                    _ => Kind::Skip,
                }
            } else if end_names.contains(head) {
                Kind::End
            } else {
                Kind::Classical
            }
        })
        .collect();

    // Two-phase decision-epoch search: a fast pass that restricts start/classical
    // expansion to FF helpful actions (collapses the per-epoch AND per-idle-agent
    // branching), then the unrestricted pass on failure — so completeness holds by
    // construction (phase 2 is the full search). Heap key = W_G*g + W_H*h + agenda.
    temporal_search(&task, &kind, true).or_else(|| temporal_search(&task, &kind, false))
}

/// `h`, plus — under `prune` — the Skip-filtered helpful start/classical ops for
/// `s`. `None` iff `s` is a relaxed dead end (so this also gates dead ends).
fn eval_node(
    task: &PackedTask,
    kind: &[Kind],
    sc: &mut Scratch,
    s: &State,
    prune: bool,
) -> Option<(i32, Vec<u32>)> {
    if prune {
        let (h, helpful) = relaxed_helpful(
            task,
            sc,
            &s.bits,
            &s.fv,
            &s.fdef,
            &task.goal_pos,
            &task.goal_num,
        )?;
        let hf = helpful
            .into_iter()
            .filter(|&oi| matches!(kind[oi as usize], Kind::Start { .. } | Kind::Classical))
            .collect();
        Some((h, hf))
    } else {
        let h = relaxed(task, sc, &s.bits, &s.fv, &s.fdef)?;
        Some((h, Vec::new()))
    }
}

/// Evaluate, dedup, and enqueue a candidate node with the weighted heap key.
#[allow(clippy::too_many_arguments)]
fn push_node(
    task: &PackedTask,
    kind: &[Kind],
    sc: &mut Scratch,
    nodes: &mut Vec<TNode>,
    heap: &mut BinaryHeap<Reverse<(i64, usize)>>,
    visited: &mut HashSet<(StateKey, Vec<(i64, usize)>)>,
    prune: bool,
    mut n: TNode,
) {
    // Gentle h-weight (1g + 3h, vs the classical 1g+5h) keeps required-concurrency
    // branches in contention; the unit g breaks the flat-h plateau on long chains.
    // AGENDA_W is 0: penalizing open intervals suppresses the very parallelism we
    // want (it serialized the crew/floor cases) — keep it off.
    const W_G: i64 = 1;
    const W_H: i64 = 3;
    const AGENDA_W: i64 = 0;
    if let Some((h, helpful)) = eval_node(task, kind, sc, &n.state, prune) {
        let k = tkey(task, &n);
        if visited.insert(k) {
            n.helpful = helpful;
            // Phase 1 (prune): weighted g+h to break the flat-h plateau on long
            // chains. Phase 2 (full): the ORIGINAL pure-h key — byte-for-byte the
            // old complete search, so nothing it solved before can regress.
            let key = if prune {
                W_G * n.g as i64 + W_H * h as i64 + AGENDA_W * n.agenda.len() as i64
            } else {
                h as i64
            };
            let idx = nodes.len();
            nodes.push(n);
            heap.push(Reverse((key, idx)));
        }
    }
}

/// One decision-epoch search pass. `prune` restricts block-(a) expansion to the
/// node's helpful ops (with a per-node full-scan fallback so no node with a legal
/// successor is stranded); `false` is the full, complete search.
fn temporal_search(task: &PackedTask, kind: &[Kind], prune: bool) -> Option<TimedPlan> {
    const MAX_NODES: usize = 400_000;
    let init = task.initial();
    let mut sc = Scratch::new(task);

    let (_h0, hf0) = eval_node(task, kind, &mut sc, &init, prune)?; // also dead-end gate
    let mut nodes = vec![TNode {
        state: init,
        time: 0.0,
        agenda: Vec::new(),
        father: usize::MAX,
        ev: None,
        g: 0,
        helpful: hf0,
    }];
    let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    heap.push(Reverse((0, 0)));
    let mut visited: HashSet<(StateKey, Vec<(i64, usize)>)> = HashSet::new();
    visited.insert(tkey(task, &nodes[0]));

    while let Some(Reverse((_k, ni))) = heap.pop() {
        if task.goal_met(&nodes[ni].state) && nodes[ni].agenda.is_empty() {
            let plan = reconstruct(task, &nodes, ni, kind);
            return Some(epsilon_separate(task, plan));
        }
        if nodes.len() > MAX_NODES {
            break;
        }
        let time = nodes[ni].time;
        let pg = nodes[ni].g;

        // (a) start a durative action / apply a classical action — restricted to
        // the node's helpful set under pruning (else a full scan).
        let candidates: Vec<usize> = if prune && !nodes[ni].helpful.is_empty() {
            nodes[ni].helpful.iter().map(|&o| o as usize).collect()
        } else {
            (0..task.n_ops).collect()
        };
        for oi in candidates {
            match kind[oi] {
                Kind::Start { dur, end_op } => {
                    if task.op_applicable(oi, &nodes[ni].state) {
                        let ns = task.apply(oi, &nodes[ni].state);
                        let mut ag = nodes[ni].agenda.clone();
                        let te = time + dur;
                        let pos = ag.partition_point(|x| x.0 <= te);
                        ag.insert(pos, (te, end_op));
                        push_node(
                            task,
                            kind,
                            &mut sc,
                            &mut nodes,
                            &mut heap,
                            &mut visited,
                            prune,
                            TNode {
                                state: ns,
                                time,
                                agenda: ag,
                                father: ni,
                                ev: Some((oi, time)),
                                g: pg + 1,
                                helpful: Vec::new(),
                            },
                        );
                    }
                }
                Kind::Classical => {
                    if task.op_applicable(oi, &nodes[ni].state) {
                        let ns = task.apply(oi, &nodes[ni].state);
                        let ag = nodes[ni].agenda.clone();
                        push_node(
                            task,
                            kind,
                            &mut sc,
                            &mut nodes,
                            &mut heap,
                            &mut visited,
                            prune,
                            TNode {
                                state: ns,
                                time,
                                agenda: ag,
                                father: ni,
                                ev: Some((oi, time)),
                                g: pg + 1,
                                helpful: Vec::new(),
                            },
                        );
                    }
                }
                Kind::End | Kind::Skip => {}
            }
        }

        // (b) advance time: fire the earliest pending end (always considered).
        if let Some(&(te, end_op)) = nodes[ni].agenda.first() {
            if task.op_applicable(end_op, &nodes[ni].state) {
                let ns = task.apply(end_op, &nodes[ni].state);
                let ag = nodes[ni].agenda[1..].to_vec();
                push_node(
                    task,
                    kind,
                    &mut sc,
                    &mut nodes,
                    &mut heap,
                    &mut visited,
                    prune,
                    TNode {
                        state: ns,
                        time: te,
                        agenda: ag,
                        father: ni,
                        ev: Some((end_op, te)),
                        g: pg + 1,
                        helpful: Vec::new(),
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
fn reconstruct(task: &PackedTask, nodes: &[TNode], goal: usize, kind: &[Kind]) -> TimedPlan {
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
        // Use the durations resolved in `solve` so constant and parameter-dependent
        // durative actions render identically. END events are implied by their start.
        let (name, duration) = match kind[op] {
            Kind::End => {
                makespan = makespan.max(t);
                continue;
            }
            Kind::Start { dur, .. } => {
                makespan = makespan.max(t + dur);
                (head.trim_end_matches("-START"), Some(dur))
            }
            _ => {
                makespan = makespan.max(t);
                (head, None)
            }
        };
        let action = if args.is_empty() {
            name.to_string()
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

// ---------------------------------------------------------------------------
// Temporal plan validation (independent of the search).
// ---------------------------------------------------------------------------

/// Validate a [`TimedPlan`] against the temporal semantics, independently of how
/// it was produced: expand each durative step into a START happening at `t` and
/// an END happening at `t + duration`, order all happenings by time (ends before
/// starts at equal time), and simulate over the same snap-action compilation —
/// checking each happening's precondition + `over all` invariant holds, applying
/// its effects, cross-checking each duration against the domain expression, and
/// finally that the goal holds. Returns `Ok(())` if executable and goal-reaching,
/// else a human-readable reason. A cross-check on the search (and on any
/// externally-supplied plan).
pub fn validate(domain: &Domain, problem: &Problem, plan: &TimedPlan) -> Result<(), String> {
    let c = compile(domain, problem);
    let task = match ground(&c.domain, &c.problem, 1) {
        Outcome::Task(t) => t,
        Outcome::GoalTrue => {
            return if plan.steps.is_empty() {
                Ok(())
            } else {
                Err("goal is already true but the plan is non-empty".into())
            }
        }
        _ => return Err("problem grounds to unsolvable".into()),
    };
    let init = task.initial();
    let snap_by_start: HashMap<&str, &SnapInfo> = c
        .snaps
        .iter()
        .map(|s| (s.start_action.as_str(), s))
        .collect();
    let find = |disp: &str| {
        task.op_display
            .iter()
            .position(|d| d == disp)
            .ok_or_else(|| format!("plan references unknown action `{disp}`"))
    };

    struct Happening {
        time: f64,
        op: usize,
        is_start: bool,
    }
    let mut happenings: Vec<Happening> = Vec::new();
    for step in &plan.steps {
        let mut it = step.action.splitn(2, ' ');
        let head = it.next().unwrap_or("");
        let rest = it.next();
        let with = |suffix: &str| match rest {
            Some(r) => format!("{head}{suffix} {r}"),
            None => format!("{head}{suffix}"),
        };
        match step.duration {
            Some(dur) => {
                let start_name = format!("{head}-START");
                let snap = snap_by_start
                    .get(start_name.as_str())
                    .ok_or_else(|| format!("`{head}` is not a durative action"))?;
                // cross-check the stated duration against the domain's expression
                let args: Vec<&str> = rest
                    .map(|r| r.split_whitespace().collect())
                    .unwrap_or_default();
                if let Some(expected) = eval_duration(snap, &args, &task, &init) {
                    if (expected - dur).abs() > 1e-6 {
                        return Err(format!(
                            "`{}` has duration {dur} but the domain says {expected}",
                            step.action
                        ));
                    }
                }
                happenings.push(Happening {
                    time: step.time,
                    op: find(&with("-START"))?,
                    is_start: true,
                });
                happenings.push(Happening {
                    time: step.time + dur,
                    op: find(&with("-END"))?,
                    is_start: false,
                });
            }
            None => happenings.push(Happening {
                time: step.time,
                op: find(&step.action)?,
                is_start: true,
            }),
        }
    }

    // execute in time order; at equal time, ends (free tokens/resources) first
    happenings.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.is_start.cmp(&b.is_start))
    });
    let mut state = init.clone();
    for h in &happenings {
        if !task.op_applicable(h.op, &state) {
            return Err(format!(
                "at t={:.3}, `{}` is not applicable (precondition or invariant violated)",
                h.time, task.op_display[h.op]
            ));
        }
        state = task.apply(h.op, &state);
    }
    if !task.goal_met(&state) {
        return Err("the plan does not achieve the goal".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ε-separation: make plans valid under PDDL2.1 continuous-time semantics.
// ---------------------------------------------------------------------------

/// PDDL2.1 separation between mutex happenings (the IPC convention).
const EPS: f64 = 0.001;

fn slices_intersect(a: &[u32], b: &[u32]) -> bool {
    // slices are tiny (a handful of facts), so the quadratic scan is fine
    a.iter().any(|x| b.contains(x))
}

/// Do two grounded happenings interfere (PDDL2.1 mutex)? True if one's add/del
/// clashes with the other's precondition, add, or delete on a shared fact —
/// requiring them to be ε-separated rather than simultaneous.
fn ops_mutex(task: &PackedTask, o1: usize, o2: usize) -> bool {
    let (p1, a1, d1) = (
        task.pre_pos.slice(o1),
        task.add.slice(o1),
        task.del.slice(o1),
    );
    let (p2, a2, d2) = (
        task.pre_pos.slice(o2),
        task.add.slice(o2),
        task.del.slice(o2),
    );
    slices_intersect(a1, d2)
        || slices_intersect(d1, a2)
        || slices_intersect(a1, p2)
        || slices_intersect(p1, a2)
        || slices_intersect(d1, p2)
        || slices_intersect(p1, d2)
}

/// Re-time a plan so mutex happenings are ε-separated (PDDL2.1 / VAL validity):
/// the decision-epoch search coincides dependent happenings (e.g. one action
/// starting the instant another's at-end effect lands), which VAL rejects. We
/// model the plan's happenings as a simple temporal network — preserve the
/// execution order, pin each end at start+duration, force ε between mutex pairs —
/// and solve the earliest-time schedule by longest paths (Bellman–Ford). On any
/// inconsistency or for very large plans the original plan is returned unchanged.
fn epsilon_separate(task: &PackedTask, plan: TimedPlan) -> TimedPlan {
    // happening: (op id, owning step index, is_start)
    struct H {
        op: usize,
        step: usize,
        is_start: bool,
        time: f64,
    }
    let find = |disp: &str| task.op_display.iter().position(|d| d == disp);
    let mut hs: Vec<H> = Vec::new();
    for (si, step) in plan.steps.iter().enumerate() {
        let mut it = step.action.splitn(2, ' ');
        let head = it.next().unwrap_or("");
        let rest = it.next();
        match step.duration {
            Some(dur) => {
                let sd = match rest {
                    Some(r) => format!("{head}-START {r}"),
                    None => format!("{head}-START"),
                };
                let ed = match rest {
                    Some(r) => format!("{head}-END {r}"),
                    None => format!("{head}-END"),
                };
                match (find(&sd), find(&ed)) {
                    (Some(so), Some(eo)) => {
                        hs.push(H {
                            op: so,
                            step: si,
                            is_start: true,
                            time: step.time,
                        });
                        hs.push(H {
                            op: eo,
                            step: si,
                            is_start: false,
                            time: step.time + dur,
                        });
                    }
                    _ => return plan, // can't map -> leave as-is
                }
            }
            None => match find(&step.action) {
                Some(o) => hs.push(H {
                    op: o,
                    step: si,
                    is_start: true,
                    time: step.time,
                }),
                None => return plan,
            },
        }
    }
    let n = hs.len();
    if n == 0 || n > 600 {
        return plan; // nothing to do, or too large to schedule cheaply
    }
    // execution order: by time, ends before starts at equal time
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        hs[a]
            .time
            .partial_cmp(&hs[b].time)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(hs[a].is_start.cmp(&hs[b].is_start))
    });

    // STN edges: t[v] >= t[u] + w
    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    // preserve order (weak)
    for w in order.windows(2) {
        edges.push((w[0], w[1], 0.0));
    }
    // ε between mutex happenings (in execution order)
    for i in 0..n {
        for j in (i + 1)..n {
            let (u, v) = (order[i], order[j]);
            if ops_mutex(task, hs[u].op, hs[v].op) {
                edges.push((u, v, EPS));
            }
        }
    }
    // duration equality: end = start + dur  (two inequalities)
    for si in 0..plan.steps.len() {
        if let Some(dur) = plan.steps[si].duration {
            let (mut s, mut e) = (None, None);
            for (hi, h) in hs.iter().enumerate() {
                if h.step == si {
                    if h.is_start {
                        s = Some(hi)
                    } else {
                        e = Some(hi)
                    }
                }
            }
            if let (Some(s), Some(e)) = (s, e) {
                edges.push((s, e, dur));
                edges.push((e, s, -dur));
            }
        }
    }

    // longest-path (earliest feasible times) via Bellman–Ford
    let mut t = vec![0.0f64; n];
    for _ in 0..n {
        let mut changed = false;
        for &(u, v, w) in &edges {
            if t[v] < t[u] + w - 1e-12 {
                t[v] = t[u] + w;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // positive-cycle check: another pass must not improve
    for &(u, v, w) in &edges {
        if t[v] < t[u] + w - 1e-12 {
            return plan; // inconsistent ordering -> keep original
        }
    }

    // re-time the steps from the scheduled start happenings
    let mut steps = plan.steps;
    for (hi, h) in hs.iter().enumerate() {
        if h.is_start {
            steps[h.step].time = t[hi];
        }
    }
    let makespan = steps
        .iter()
        .map(|s| s.time + s.duration.unwrap_or(0.0))
        .fold(0.0f64, f64::max);
    TimedPlan { steps, makespan }
}
