//! IPC6 `:action-costs` on the classical path (0.9 roadmap Phase 2).
//!
//! IPC-2008 introduced action costs: a numeric fluent (conventionally
//! `total-cost`) initialized in `:init`, increased by constant amounts in
//! action effects, and minimized by `(:metric minimize (total-cost))`.
//! Ferroplan's numeric machinery already *tracks* such a fluent through
//! grounding and `apply`; what this module adds is making the classical
//! (non-preference) path *reason* about it:
//!
//! 1. [`metric_fluent`] detects the supported metric shape — `minimize` of a
//!    single ground fluent — and names it for [`PackedTask::fluent_id`].
//! 2. [`plan_cost`] replays a plan to read the metric's real final value
//!    (the number the external validator will also compute).
//! 3. [`improve`] runs the anytime cost-improvement sweep: a bounded
//!    branch-and-bound best-first pass under the first plan's cost, ordered
//!    by accumulated cost (`w_c`) and guided by the cost-augmented relaxed
//!    plan ([`crate::heuristic::relaxed_costed`]), keeping the cheapest
//!    incumbent (in-sweep anytime tightening). The first plan itself comes
//!    from the untouched EHC / weighted-best-first machinery — fast, and
//!    bit-identical to pre-cost behavior — so cost support can never regress
//!    the classical baseline; only the polish pass is new.
//!
//! The sweep's eval budget is deliberately bounded (see [`sweep_budget`]):
//! a polish pass must never dwarf the solve that produced the plan.
//! `FF_COST_SWEEP_EVALS` overrides it (`0` disables the sweep).

use crate::packed::PackedTask;
use crate::search::{search_from, PlanResult, SearchCfg};
use crate::types::{Expr, MetricDir, Problem, Term};

/// Floor for the improvement sweep's eval budget: even a plan found almost
/// for free (EHC in a few dozen evals) gets a real polish pass.
const SWEEP_FLOOR: usize = 30_000;
/// The sweep may spend at most this multiple of what finding the first plan
/// cost — the polish stays proportionate to the solve (a hard instance that
/// needed a big best-first fallback must not then double its bill polishing).
const SWEEP_MULT: usize = 2;

/// Detect the supported IPC6 cost-metric shape on a classical problem:
/// `(:metric minimize <ground fluent>)` — returns the fluent's display
/// string (e.g. `"(TOTAL-COST)"`) for [`PackedTask::fluent_id`]. Anything
/// else (maximize, compound expressions, lifted terms) returns None: those
/// metrics are NOT silently optimized — callers report a plan without a
/// metric claim, and the PDDL3 path owns `is-violated` metrics.
pub fn metric_fluent(problem: &Problem) -> Option<String> {
    let (dir, e) = problem.metric.as_ref()?;
    if !matches!(dir, MetricDir::Minimize) {
        return None;
    }
    if let Expr::Fluent(name, args) = e {
        let mut parts = vec![name.to_uppercase()];
        for t in args {
            match t {
                Term::Const(c) => parts.push(c.to_uppercase()),
                Term::Var(_) => return None,
            }
        }
        Some(if parts.len() == 1 {
            format!("({})", parts[0])
        } else {
            format!("({} {})", parts[0], parts[1..].join(" "))
        })
    } else {
        None
    }
}

/// The metric's real value after executing `ops` from the initial state —
/// exact replay through [`PackedTask::apply`], so conditional cost effects
/// and non-constant increases are all honored. None if the fluent ends
/// undefined (no `:init` assignment and nothing wrote it).
pub fn plan_cost(task: &PackedTask, cf: usize, ops: &[usize]) -> Option<f64> {
    let mut s = task.initial();
    for &oi in ops {
        s = task.apply(oi, &s);
    }
    if s.fdef[cf] {
        Some(s.fv[cf])
    } else {
        None
    }
}

/// Outcome of the anytime cost-improvement sweep.
pub struct CostOutcome {
    /// Best plan known (the input plan if the sweep found nothing cheaper).
    pub ops: Vec<usize>,
    /// Its metric value (replayed, not estimated).
    pub cost: f64,
    /// The sweep found a strictly cheaper plan.
    pub improved: bool,
    /// The sweep EXHAUSTED the bounded space without caps: no plan cheaper
    /// than `cost` exists — the value is proven optimal, not just best-found.
    pub proven: bool,
    /// States evaluated by the sweep (for budget accounting / statistics).
    pub evaluated: usize,
}

/// Sweep eval budget: proportionate to the solve (`SWEEP_MULT` × its evals,
/// floored at `SWEEP_FLOOR`), never exceeding what remains of the caller's
/// overall eval budget. `FF_COST_SWEEP_EVALS` overrides (0 disables).
fn sweep_budget(spent: usize, cfg_max: usize) -> usize {
    if let Ok(v) = std::env::var("FF_COST_SWEEP_EVALS") {
        if let Ok(n) = v.trim().parse::<usize>() {
            return n;
        }
    }
    cfg_max
        .saturating_sub(spent)
        .min((SWEEP_MULT * spent).max(SWEEP_FLOOR))
}

/// Anytime cost improvement: bounded B&B best-first under `first_cost`,
/// ordered by accumulated metric cost and guided by the cost-augmented
/// relaxed plan. Deterministic (all knobs are integer/deterministic and the
/// underlying sweep is thread-count independent). Returns the best plan seen
/// — never worse than the input.
pub fn improve(
    task: &PackedTask,
    cf: usize,
    ops: Vec<usize>,
    first_cost: f64,
    threads: usize,
    base: SearchCfg,
    spent: usize,
) -> CostOutcome {
    if first_cost <= 0.0 {
        // Nothing can beat a free plan.
        return CostOutcome {
            ops,
            cost: first_cost,
            improved: false,
            proven: true,
            evaluated: 0,
        };
    }
    let budget = sweep_budget(spent, base.max_eval);
    if budget == 0 {
        return CostOutcome {
            ops,
            cost: first_cost,
            improved: false,
            proven: false,
            evaluated: 0,
        };
    }
    let cfg = SearchCfg {
        max_eval: budget,
        anytime: true,
        ..base.with_cost_weight(1.0).with_cost_h(cf)
    };
    match search_from(
        task,
        &task.initial(),
        &task.goal_pos,
        &task.goal_num,
        Some(cf),
        first_cost,
        threads,
        cfg,
        &[],
        None,
        None,
    ) {
        PlanResult::Plan {
            ops: better,
            evaluated,
            ..
        } => {
            // Replay for the REAL cost — the sweep's bound math is trusted,
            // but the reported number must be the executable plan's value.
            match plan_cost(task, cf, &better) {
                Some(c) if c < first_cost => CostOutcome {
                    ops: better,
                    cost: c,
                    improved: true,
                    proven: false,
                    evaluated,
                },
                _ => CostOutcome {
                    ops,
                    cost: first_cost,
                    improved: false,
                    proven: false,
                    evaluated,
                },
            }
        }
        PlanResult::Unsolvable { evaluated, capped } => CostOutcome {
            ops,
            cost: first_cost,
            // Un-capped exhaustion under the bound = no cheaper plan exists.
            proven: !capped,
            improved: false,
            evaluated,
        },
    }
}

/// Shared text-path hook (run_planner / run_ff): detect the cost metric,
/// replay the plan's cost, run the improvement sweep (unless `optimize` is
/// off), and swap `ops` for the cheaper plan. Returns the final cost and a
/// short annotation for the report, or None when the problem has no
/// supported cost metric (text output then stays byte-identical).
pub fn optimize_text(
    problem: &Problem,
    task: &PackedTask,
    optimize: bool,
    threads: usize,
    cfg: SearchCfg,
    ops: &mut Vec<usize>,
) -> Option<(f64, &'static str)> {
    let disp = metric_fluent(problem)?;
    let cf = task.fluent_id(&disp)?;
    let c0 = plan_cost(task, cf, ops)?;
    if !optimize {
        return Some((c0, " (not optimized: --satisfice)"));
    }
    let r = improve(task, cf, std::mem::take(ops), c0, threads, cfg, 0);
    *ops = r.ops;
    let note = if r.proven {
        " (proven optimal)"
    } else if r.improved {
        " (anytime-improved)"
    } else {
        ""
    };
    Some((r.cost, note))
}
