//! Data-parallel weighted best-first search.
//!
//! Each round pops a *batch* of the lowest-f nodes, expands all their applicable
//! successors, dedups against the visited set, then evaluates the FF heuristic
//! for the whole batch of successors IN PARALLEL (the dominant cost). Because
//! `par_map` preserves order and all control flow is on one thread, the plan
//! found is identical regardless of the worker count — only the wall-clock of
//! heuristic evaluation changes.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

use crate::hash::FxHashSet;
use crate::heuristic::{relaxed_helpful, relaxed_to, Scratch};
use crate::packed::{PackedTask, State, StateKey};
use crate::par;
use crate::types::NumPre;

/// Frontier batch size — FIXED (independent of thread count) so the search
/// expansion order, and thus the plan AND the evaluated-state count, are
/// identical for any worker count; threads only split each batch's h-eval.
const BATCH: usize = 256;
/// Default safety cap on evaluated states (deterministic).
pub const DEFAULT_MAX_EVAL: usize = 5_000_000;
/// Fixed-point scale for fractional heuristic weights (keeps the priority key an
/// integer, so the heap order — and thus the plan — stays deterministic).
const WEIGHT_SCALE: f64 = 256.0;

/// Tunable weighted-best-first parameters (exposed via the library `Options`).
/// `w_g`/`w_h` are pre-scaled integers (`weight * WEIGHT_SCALE`), so the default
/// `1·g + 5·h` ordering is preserved exactly while fractional weights still work.
#[derive(Clone, Copy, Debug)]
pub struct SearchCfg {
    pub w_g: i64,
    pub w_h: i64,
    pub max_eval: usize,
}

impl Default for SearchCfg {
    fn default() -> Self {
        SearchCfg::from_weights(1.0, 5.0, None)
    }
}

impl SearchCfg {
    /// Build from human-facing f64 weights. `weight_g = 1.0, weight_h = 5.0`
    /// reproduces the historical `1·g + 5·h` ordering bit-for-bit.
    ///
    /// Inputs are sanitized so a malformed weight can never collapse or overflow
    /// the integer heap key: a non-finite or negative weight falls back to that
    /// term's default, weights are clamped to a sane maximum, and if both round
    /// to zero the defaults are restored (an all-zero key would degrade to
    /// insertion order).
    pub fn from_weights(weight_g: f64, weight_h: f64, max_eval: Option<usize>) -> Self {
        let san = |w: f64, default: f64| {
            if w.is_finite() && w >= 0.0 {
                w.min(1e9)
            } else {
                default
            }
        };
        let mut w_g = (san(weight_g, 1.0) * WEIGHT_SCALE).round() as i64;
        let mut w_h = (san(weight_h, 5.0) * WEIGHT_SCALE).round() as i64;
        if w_g == 0 && w_h == 0 {
            w_g = WEIGHT_SCALE as i64;
            w_h = (5.0 * WEIGHT_SCALE) as i64;
        }
        SearchCfg {
            w_g,
            w_h,
            max_eval: max_eval.unwrap_or(DEFAULT_MAX_EVAL),
        }
    }
}

pub enum PlanResult {
    Plan {
        ops: Vec<usize>,
        advance: Vec<i32>,
        evaluated: usize,
        max_g: usize,
    },
    Unsolvable {
        evaluated: usize,
        capped: bool, // true if the MAX_EVAL safety cap was hit (not proven unsolvable)
    },
}

struct Node {
    state: State,
    father: usize,
    op: usize,
    g: usize,
}

/// Solve toward an ARBITRARY (sub)goal from an arbitrary start state over a
/// shared grounded task — the reusable subplanner entry point for SGPlan-style
/// partition-and-resolve. `search` is the whole-task convenience wrapper.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn search_from(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    cost_fluent: Option<usize>,
    cost_bound: f64,
    threads: usize,
    cfg: SearchCfg,
) -> PlanResult {
    let batch = BATCH;

    let init = start.clone();
    // early dead-end check: if the initial state is a relaxed dead end, unsolvable
    if relaxed_to(
        task,
        &mut Scratch::new(task),
        &init.bits,
        &init.fv,
        &init.fdef,
        goal_pos,
        goal_num,
    )
    .is_none()
    {
        return PlanResult::Unsolvable {
            evaluated: 1,
            capped: false,
        };
    }

    let mut nodes: Vec<Node> = vec![Node {
        state: init.clone(),
        father: usize::MAX,
        op: usize::MAX,
        g: 0,
    }];
    // Deferred evaluation: a node's priority is set from its PARENT's h at
    // insertion; its own h is computed only when it is popped. Many inserted
    // nodes are never popped, so far fewer heuristic evaluations are done.
    // The visited key excludes irrelevant fluents (termination); under
    // branch-and-bound it also appends the cost fluent so equal-fact/different-cost
    // states stay distinct (see PackedTask::state_key_with_cost).
    let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    heap.push(Reverse((0, 0))); // init popped first
    let mut visited: FxHashSet<StateKey> = FxHashSet::default();
    visited.insert(task.state_key_with_cost(&init, cost_fluent));

    let mut evaluated = 0usize;
    let mut best = i32::MAX;
    let mut advance: Vec<i32> = Vec::new();
    let mut max_g = 0usize;

    while !heap.is_empty() {
        // pop a batch of lowest-priority nodes
        let mut popped: Vec<usize> = Vec::with_capacity(batch);
        for _ in 0..batch {
            match heap.pop() {
                Some(Reverse((_, ni))) => popped.push(ni),
                None => break,
            }
        }

        // goal check (cheap, before any heuristic work)
        for &ni in &popped {
            max_g = max_g.max(nodes[ni].g);
            if task.goal_met_with(&nodes[ni].state, goal_pos, goal_num) {
                return PlanResult::Plan {
                    ops: reconstruct(&nodes, ni),
                    advance,
                    evaluated,
                    max_g,
                };
            }
        }

        // PARALLEL: evaluate h for the popped batch (the only evaluations),
        // each worker reusing one Scratch across its chunk.
        let hs: Vec<Option<i32>> = par::par_map_with(
            &popped,
            threads,
            || Scratch::new(task),
            |sc, &ni| {
                relaxed_to(
                    task,
                    sc,
                    &nodes[ni].state.bits,
                    &nodes[ni].state.fv,
                    &nodes[ni].state.fdef,
                    goal_pos,
                    goal_num,
                )
            },
        );
        evaluated += popped.len();
        if evaluated > cfg.max_eval {
            return PlanResult::Unsolvable {
                evaluated,
                capped: true,
            };
        }
        for h in hs.iter().flatten() {
            if *h < best {
                best = *h;
                advance.push(*h);
            }
        }

        // PARALLEL: expand non-dead-end popped nodes; successors carry the
        // parent's h as their (deferred) priority key.
        let live: Vec<(usize, i32)> = popped
            .iter()
            .zip(hs.iter())
            .filter_map(|(&ni, h)| h.map(|h| (ni, h)))
            .collect();
        let cand_chunks: Vec<Vec<(usize, usize, State, StateKey, i32)>> =
            par::par_map(&live, threads, |&(ni, ph)| {
                let st = &nodes[ni].state;
                let mut v = Vec::new();
                for oi in 0..task.n_ops {
                    if task.op_applicable(oi, st) {
                        let ns = task.apply(oi, st);
                        if let Some(cf) = cost_fluent {
                            if ns.fdef[cf] && ns.fv[cf] >= cost_bound {
                                continue; // cost already >= bound: cannot beat incumbent
                            }
                        }
                        let k = task.state_key_with_cost(&ns, cost_fluent);
                        v.push((ni, oi, ns, k, ph));
                    }
                }
                v
            });

        // SERIAL: dedup + insert (deterministic order, independent of threads).
        for chunk in cand_chunks {
            for (pi, oi, s, k, ph) in chunk {
                if visited.insert(k) {
                    let g = nodes[pi].g + 1;
                    let idx = nodes.len();
                    nodes.push(Node {
                        state: s,
                        father: pi,
                        op: oi,
                        g,
                    });
                    heap.push(Reverse((cfg.w_g * g as i64 + cfg.w_h * ph as i64, idx)));
                }
            }
        }
    }

    PlanResult::Unsolvable {
        evaluated,
        capped: false,
    }
}

fn reconstruct(nodes: &[Node], mut ni: usize) -> Vec<usize> {
    let mut ops = Vec::new();
    while nodes[ni].father != usize::MAX {
        ops.push(nodes[ni].op);
        ni = nodes[ni].father;
    }
    ops.reverse();
    ops
}

/// Whole-task search (start = initial state, goal = task goal) with tunable
/// weighted-best-first parameters.
pub fn search(task: &PackedTask, threads: usize, cfg: SearchCfg) -> PlanResult {
    search_from(
        task,
        &task.initial(),
        &task.goal_pos,
        &task.goal_num,
        None,
        f64::INFINITY,
        threads,
        cfg,
    )
}

/// Outcome of [`plan`]: the op sequence (if solved), states evaluated, and
/// whether EHC gave up and best-first took over.
pub struct PlanOutcome {
    pub ops: Option<Vec<usize>>,
    pub evaluated: usize,
    pub ehc_fell_back: bool,
}

/// Plan the whole task. With `ehc_first`, run enforced hill-climbing (fast on
/// most problems) and fall back to weighted best-first if it gets stuck;
/// otherwise run best-first directly. EHC plans are valid but not length-optimal
/// — this matches the FF/Metric-FF default and is the main speed lever.
pub fn plan(task: &PackedTask, threads: usize, cfg: SearchCfg, ehc_first: bool) -> PlanOutcome {
    if ehc_first {
        if let Some((ops, evaluated)) = ehc(task) {
            return PlanOutcome {
                ops: Some(ops),
                evaluated,
                ehc_fell_back: false,
            };
        }
    }
    let (ops, evaluated) = match search(task, threads, cfg) {
        PlanResult::Plan { ops, evaluated, .. } => (Some(ops), evaluated),
        PlanResult::Unsolvable { evaluated, .. } => (None, evaluated),
    };
    PlanOutcome {
        ops,
        evaluated,
        ehc_fell_back: ehc_first,
    }
}

/// Enforced hill-climbing toward the task goal. From the current state, run a
/// breadth-first lookahead restricted to HELPFUL actions until a strictly
/// lower-h state is found, then jump to it and repeat. Returns the plan + states
/// evaluated, or None if it gets stuck / hits a dead end (caller falls back to
/// best-first, which is complete). Single-threaded and deterministic.
fn ehc(task: &PackedTask) -> Option<(Vec<usize>, usize)> {
    let init = task.initial();
    let mut sc = Scratch::new(task);
    let (mut cur_h, _) = relaxed_helpful(
        task,
        &mut sc,
        &init.bits,
        &init.fv,
        &init.fdef,
        &task.goal_pos,
        &task.goal_num,
    )?;
    let mut evaluated = 1usize;
    if task.goal_met(&init) {
        return Some((Vec::new(), evaluated));
    }
    // Total work budget: if EHC hasn't solved it within this many evaluations it
    // is likely stuck, so bail and leave the time budget to the complete
    // best-first fallback (which often solves these much faster from scratch).
    const TOTAL_CAP: usize = 30_000;
    let mut current = init;
    let mut plan: Vec<usize> = Vec::new();
    loop {
        match bfs_improve(task, &mut sc, &current, cur_h, &mut evaluated) {
            Some((ops, next, next_h)) => {
                plan.extend(ops);
                current = next;
                cur_h = next_h;
                if task.goal_met(&current) {
                    return Some((plan, evaluated));
                }
                if evaluated > TOTAL_CAP {
                    return None; // taking too long — hand off to best-first
                }
            }
            None => return None, // stuck — let best-first take over
        }
    }
}

/// Breadth-first search from `start`, expanding each node with ITS helpful
/// actions, until a state with `h < h_start` is found. Returns (path, state, h).
fn bfs_improve(
    task: &PackedTask,
    sc: &mut Scratch,
    start: &State,
    h_start: i32,
    evaluated: &mut usize,
) -> Option<(Vec<usize>, State, i32)> {
    // Fail FAST: if a helpful-restricted lookahead can't improve h within this
    // many expansions it is almost certainly on a plateau EHC won't escape, so
    // bail and let the complete best-first fallback use the time budget. Kept
    // small because per-evaluation cost is high on big numeric tasks — a large
    // cap made EHC burn ~20s before falling back.
    const BFS_CAP: usize = 5_000;
    struct N {
        state: State,
        father: usize,
        op: usize,
    }
    let (_, root_helpful) = relaxed_helpful(
        task,
        sc,
        &start.bits,
        &start.fv,
        &start.fdef,
        &task.goal_pos,
        &task.goal_num,
    )?;
    let mut nodes = vec![N {
        state: start.clone(),
        father: usize::MAX,
        op: usize::MAX,
    }];
    let mut visited: FxHashSet<StateKey> = FxHashSet::default();
    visited.insert(task.state_key(start));
    let mut queue: VecDeque<(usize, Vec<u32>)> = VecDeque::new();
    queue.push_back((0, root_helpful));
    let mut expanded = 0usize;

    while let Some((ni, helpful)) = queue.pop_front() {
        for &oi in &helpful {
            let oi = oi as usize;
            if !task.op_applicable(oi, &nodes[ni].state) {
                continue;
            }
            let ns = task.apply(oi, &nodes[ni].state);
            if !visited.insert(task.state_key(&ns)) {
                continue;
            }
            *evaluated += 1;
            let (h_ns, helpful_ns) = match relaxed_helpful(
                task,
                sc,
                &ns.bits,
                &ns.fv,
                &ns.fdef,
                &task.goal_pos,
                &task.goal_num,
            ) {
                Some(x) => x,
                None => continue, // dead-end successor
            };
            let idx = nodes.len();
            nodes.push(N {
                state: ns.clone(),
                father: ni,
                op: oi,
            });
            if h_ns < h_start {
                let mut ops = Vec::new();
                let mut c = idx;
                while nodes[c].father != usize::MAX {
                    ops.push(nodes[c].op);
                    c = nodes[c].father;
                }
                ops.reverse();
                return Some((ops, ns, h_ns));
            }
            expanded += 1;
            if expanded > BFS_CAP {
                return None;
            }
            queue.push_back((idx, helpful_ns));
        }
    }
    None
}

/// Subplanner API: return the op sequence achieving `(goal_pos, goal_num)` from
/// `start`, or None if unsolvable. This is what `sgp` calls per partition.
pub fn solve_subgoal(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    threads: usize,
    cfg: SearchCfg,
) -> Option<Vec<usize>> {
    match search_from(
        task,
        start,
        goal_pos,
        goal_num,
        None,
        f64::INFINITY,
        threads,
        cfg,
    ) {
        PlanResult::Plan { ops, .. } => Some(ops),
        PlanResult::Unsolvable { .. } => None,
    }
}

/// Subplanner with a monotone COST upper bound on `cost_fluent`: returns a plan
/// reaching the goal whose final cost is < `bound`, or None if none exists under
/// the bound. The anytime branch-and-bound metric optimizer (sgp) calls this
/// with a tightening bound.
pub fn solve_subgoal_bounded(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    cost_fluent: usize,
    bound: f64,
    threads: usize,
) -> (Option<Vec<usize>>, bool) {
    match search_from(
        task,
        start,
        goal_pos,
        goal_num,
        Some(cost_fluent),
        bound,
        threads,
        SearchCfg::default(),
    ) {
        PlanResult::Plan { ops, .. } => (Some(ops), false),
        PlanResult::Unsolvable { capped, .. } => (None, capped),
    }
}
