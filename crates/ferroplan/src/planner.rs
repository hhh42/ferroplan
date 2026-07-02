// planner orchestration (ported from sgp) — a data-parallel SGPlan-style partition-and-resolve meta-planner that
// reuses `ffdp` as its modified-Metric-FF subplanner.
//
// `run_planner` parses + grounds via `ffdp`, partitions the goal, solves
// subproblems in parallel with `crate::solve_subgoal`, resolves cross-subplan
// conflicts by merge-on-stuck (monolithic `ffdp` fallback), and renders the
// plan in classic-FF (default) or IPC (`ipc=true`) format. Same exit-code and
// message contract as `ffdp`/`metricff`, so it is a drop-in and shares the
// differential test harness.

use crate::ground::{ground, Outcome};
use crate::resolve::Solved;
use crate::{pddl3, report, resolve};

pub fn run_planner(
    domain_src: &str,
    problem_src: &str,
    opts: &crate::Options,
    ipc: bool,
) -> (String, i32) {
    let threads = if opts.threads == 0 {
        crate::par::num_threads()
    } else {
        opts.threads
    }
    .max(1);
    let cfg =
        crate::search::SearchCfg::from_weights(opts.weight_g, opts.weight_h, opts.max_evaluated);
    let mut out = String::new();

    out.push_str("\nff: parsing domain file\n");
    let domain = match crate::parser::parse_domain(domain_src) {
        Ok(d) => d,
        Err(e) => {
            out.push_str(&format!("\nff: parse error in domain file: {}\n", e));
            return (out, 1);
        }
    };
    out.push_str(&format!("domain '{}' defined\n ... done.\n", domain.name));

    out.push_str("ff: parsing problem file\n");
    let problem = match crate::parser::parse_problem(problem_src) {
        Ok(p) => p,
        Err(e) => {
            out.push_str(&format!("\nff: parse error in problem file: {}\n", e));
            return (out, 1);
        }
    };
    out.push_str(&format!("problem '{}' defined\n ... done.\n", problem.name));

    // Compile `:derived` axioms away (static rules -> init facts) before routing.
    let (domain, problem) = match crate::derived::compile(&domain, &problem) {
        Ok(dp) => dp,
        Err(e) => {
            out.push_str(&format!("\nff: {}\n", e));
            return (out, 1);
        }
    };

    // PDDL2.1 temporal: durative actions -> decision-epoch search, IPC plan format.
    // FF_TDECOMP routes through the partition-and-resolve decomposer (Phase B) FIRST;
    // the default is `temporal::solve` — the monolithic search plus its on-failure
    // escalation ladder (Full tier, then decomposer; `FF_NO_ESCALATE` restores the
    // single-rung search).
    if crate::temporal::is_temporal(&domain) {
        let solved = if crate::features::tdecomp() {
            crate::tresolve::solve(&domain, &problem, threads)
        } else {
            crate::temporal::solve(&domain, &problem, threads)
        };
        match solved {
            Some(tp) => {
                out.push_str("\nff: found legal plan as follows\n");
                out.push_str(&tp.to_ipc());
                out.push_str(&format!("\nplan makespan: {:.3}\n", tp.makespan));
                return (out, 0);
            }
            None => {
                out.push_str("\n\nno temporal plan found.\n\n");
                return (out, 1);
            }
        }
    }

    // PDDL3.0: soft-goal preferences / metric -> compile + anytime B&B optimize.
    if pddl3::is_pddl3(&problem) {
        let code = plan_pddl3(
            &mut out,
            &domain,
            &problem,
            opts.optimize,
            threads,
            cfg,
            ipc,
        );
        return (out, code);
    }

    let task = match ground(&domain, &problem, threads) {
        Outcome::EmptyType { kind, pred, ty } => {
            out.push_str(&format!(
                "\n\n{} {} is declared to use unknown or empty type {}\n",
                kind, pred, ty
            ));
            return (out, 1);
        }
        Outcome::GoalTrue => {
            out.push_str("\n\nff: goal can be simplified to TRUE. The empty plan solves it\n\n");
            return (out, 1);
        }
        Outcome::GoalFalse => {
            out.push_str("\n\nff: goal can be simplified to FALSE. No plan will solve it\n\n");
            return (out, 1);
        }
        Outcome::GoalUndefinedFluent => {
            out.push_str(
                "\n\nff: goal accesses a fluent that will never have a defined value. Problem unsolvable.\n\n",
            );
            return (out, 1);
        }
        Outcome::Task(t) => t,
    };

    out.push_str(&report::preamble(threads));
    let groups = crate::invariants::synthesize(&domain, &task);
    match resolve::solve(&task, threads, cfg, &groups) {
        Solved::Plan(ops, stats) => {
            if ipc {
                out.push_str(&report::ipc_plan(&task, &ops, None));
            } else {
                out.push('\n');
                out.push_str(&report::ff_plan(&task, &ops));
                out.push('\n');
            }
            out.push_str(&report::timing(&stats, threads));
            (out, 0)
        }
        Solved::Unsolvable => {
            out.push_str("\n\nbest first search space empty! problem proven unsolvable.\n\n");
            (out, 0)
        }
    }
}

/// PDDL3 path: compile soft goals away, ground the augmented problem, and
/// anytime branch-and-bound minimize the metric. Appends to `out`, returns exit.
#[allow(clippy::too_many_arguments)]
fn plan_pddl3(
    out: &mut String,
    domain: &crate::types::Domain,
    problem: &crate::types::Problem,
    optimize: bool,
    threads: usize,
    cfg: crate::search::SearchCfg,
    ipc: bool,
) -> i32 {
    // caller opted out of metric optimization -> satisficing plan over hard goals.
    if !optimize {
        return satisficing_fallback(
            out,
            domain,
            problem,
            threads,
            cfg,
            ipc,
            "optimize disabled (--satisfice)",
        );
    }

    let c = pddl3::compile(domain, problem);

    // metric outside the supported class -> don't silently optimize the wrong
    // objective; return a satisficing plan for the HARD goals + a clear note.
    if let Some(reason) = c.unsupported.clone() {
        return satisficing_fallback(out, domain, problem, threads, cfg, ipc, &reason);
    }

    let task = match ground(&c.domain, &c.problem, threads) {
        Outcome::EmptyType { kind, pred, ty } => {
            out.push_str(&format!(
                "\n\n{} {} is declared to use unknown or empty type {}\n",
                kind, pred, ty
            ));
            return 1;
        }
        Outcome::GoalTrue => {
            out.push_str("\n\nff: goal can be simplified to TRUE. The empty plan solves it\n\n");
            return 1;
        }
        Outcome::GoalFalse => {
            out.push_str("\n\nff: goal can be simplified to FALSE. No plan will solve it\n\n");
            return 1;
        }
        Outcome::GoalUndefinedFluent => {
            out.push_str(
                "\n\nff: goal accesses a fluent that will never have a defined value. Problem unsolvable.\n\n",
            );
            return 1;
        }
        Outcome::Task(t) => t,
    };

    out.push_str(&report::preamble(threads));
    let cf = task
        .fluent_id(pddl3::COST_DISP)
        .expect("compile injects total-cost");
    let forgos: Vec<(usize, f64)> = c
        .forgos
        .iter()
        .filter_map(|(name, w)| {
            task.op_display
                .iter()
                .position(|d| d == name)
                .map(|oi| (oi, *w))
        })
        .collect();
    // Mutex groups feed the resource-aware guidance (renewable counter resources).
    let groups = crate::invariants::synthesize(&c.domain, &task);
    match pddl3::metric_optimize(&task, cf, &forgos, &groups, threads) {
        Some(r) => {
            let mut note = String::new();
            if c.warn_other {
                note.push_str(" metric has terms beyond is-violated/total-cost; optimized the supported part.");
            }
            if !r.proven {
                note.push_str(" search bound hit; value is best-found, not proven optimal.");
            }
            render_plan(
                out,
                &task,
                &r.ops,
                Some(r.cost),
                threads,
                &c,
                r.iterations,
                ipc,
                &note,
            );
            0
        }
        None => {
            out.push_str("\n\nbest first search space empty! problem proven unsolvable.\n\n");
            0
        }
    }
}

/// Satisficing fallback for unsupported metrics: solve the HARD goals only,
/// emit the plan with an explicit "metric not optimized" note (never claims a
/// metric it did not optimize).
#[allow(clippy::too_many_arguments)]
fn satisficing_fallback(
    out: &mut String,
    domain: &crate::types::Domain,
    problem: &crate::types::Problem,
    threads: usize,
    cfg: crate::search::SearchCfg,
    ipc: bool,
    reason: &str,
) -> i32 {
    let task = match ground(domain, problem, threads) {
        Outcome::Task(t) => t,
        Outcome::GoalTrue => {
            out.push_str("\n\nff: goal can be simplified to TRUE. The empty plan solves it\n\n");
            return 1;
        }
        Outcome::GoalFalse => {
            out.push_str("\n\nff: goal can be simplified to FALSE. No plan will solve it\n\n");
            return 1;
        }
        _ => {
            out.push_str("\n\nbest first search space empty! problem proven unsolvable.\n\n");
            return 0;
        }
    };
    out.push_str(&report::preamble(threads));
    let note = format!(
        " PDDL3 metric NOT optimized ({}); returning a satisficing plan.",
        reason
    );
    let groups = crate::invariants::synthesize(domain, &task);
    match resolve::solve(&task, threads, cfg, &groups) {
        Solved::Plan(ops, stats) => {
            if ipc {
                out.push_str(&report::ipc_plan(&task, &ops, None));
                out.push_str(&format!(";{}\n", note));
            } else {
                out.push('\n');
                out.push_str(&report::ff_plan(&task, &ops));
                out.push('\n');
                out.push_str(&report::timing(&stats, threads));
                out.push_str(&format!("note:{}\n", note));
            }
            0
        }
        Solved::Unsolvable => {
            out.push_str("\n\nbest first search space empty! problem proven unsolvable.\n\n");
            0
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_plan(
    out: &mut String,
    task: &crate::packed::PackedTask,
    ops: &[usize],
    cost: Option<f64>,
    threads: usize,
    c: &pddl3::Compiled,
    iterations: usize,
    ipc: bool,
    note: &str,
) {
    // strip the artificial Keyder-Geffner bookkeeping actions by their explicit
    // synthetic-name set (robust vs a "P3" display-prefix that could collide
    // with a real domain action).
    let display: Vec<usize> = ops
        .iter()
        .copied()
        .filter(|&oi| {
            let name = task.op_display[oi].split_whitespace().next().unwrap_or("");
            !c.synthetic.contains(name)
        })
        .collect();
    let ops = &display[..];
    if ipc {
        out.push_str(&report::ipc_plan(task, ops, cost));
        if !note.is_empty() {
            out.push_str(&format!(";{}\n", note));
        }
    } else {
        out.push('\n');
        out.push_str(&report::ff_plan(task, ops));
        out.push('\n');
        out.push_str(&report::metric_footer(
            cost.unwrap_or(0.0),
            iterations,
            c.n_prefs,
            threads,
            c.warn_other,
        ));
        if !note.is_empty() {
            out.push_str(&format!("note:{}\n", note));
        }
    }
}
/// Plain FF best-first over the whole task (no partitioning) — the engine mode.
pub fn run_ff(domain_src: &str, problem_src: &str, opts: &crate::Options) -> (String, i32) {
    let threads = if opts.threads == 0 {
        crate::par::num_threads()
    } else {
        opts.threads
    }
    .max(1);
    let cfg =
        crate::search::SearchCfg::from_weights(opts.weight_g, opts.weight_h, opts.max_evaluated);
    let mut out = String::new();
    out.push_str("\nff: parsing domain file\n");
    let domain = match crate::parser::parse_domain(domain_src) {
        Ok(d) => d,
        Err(e) => {
            out.push_str(&format!("\nff: parse error in domain file: {}\n", e));
            return (out, 1);
        }
    };
    out.push_str(&format!("domain '{}' defined\n ... done.\n", domain.name));
    out.push_str("ff: parsing problem file\n");
    let problem = match crate::parser::parse_problem(problem_src) {
        Ok(p) => p,
        Err(e) => {
            out.push_str(&format!("\nff: parse error in problem file: {}\n", e));
            return (out, 1);
        }
    };
    out.push_str(&format!("problem '{}' defined\n ... done.\n", problem.name));
    match ground(&domain, &problem, threads) {
        Outcome::EmptyType { kind, pred, ty } => {
            out.push_str(&format!(
                "\n\n{} {} is declared to use unknown or empty type {}\n",
                kind, pred, ty
            ));
            (out, 1)
        }
        Outcome::GoalTrue => {
            out.push_str("\n\nff: goal can be simplified to TRUE. The empty plan solves it\n\n");
            (out, 1)
        }
        Outcome::GoalFalse => {
            out.push_str("\n\nff: goal can be simplified to FALSE. No plan will solve it\n\n");
            (out, 1)
        }
        Outcome::GoalUndefinedFluent => {
            out.push_str("\n\nff: goal accesses a fluent that will never have a defined value. Problem unsolvable.\n\n");
            (out, 1)
        }
        Outcome::Task(task) => {
            let o =
                crate::search::plan(&task, threads, cfg, opts.search != crate::Search::BestFirst);
            let result = match o.ops {
                Some(ops) => crate::search::PlanResult::Plan {
                    ops,
                    advance: Vec::new(),
                    evaluated: o.evaluated,
                    max_g: 0,
                },
                None => crate::search::PlanResult::Unsolvable {
                    evaluated: o.evaluated,
                    capped: false,
                },
            };
            let (body, code) = crate::output::render(&task, &result, threads);
            out.push_str(&body);
            (out, code)
        }
    }
}
