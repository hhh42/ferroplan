//! The smart, serde-serializable public API.
//!
//! [`solve`] grounds and plans, returning a typed [`Solution`] (plan as
//! structured [`Step`]s, statistics, optional PDDL3 metric) instead of text.
//! Everything here is `serde`-serializable, so it round-trips to/from JSON.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::ground::{ground, Outcome};
use crate::packed::PackedTask;
use crate::parser;
use crate::pddl3;
use crate::resolve::{self, Solved};
use crate::search::{self, PlanResult};

/// Which planning strategy to use.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// PDDL3 metric mode if the problem has preferences/metric, else classic FF.
    #[default]
    Auto,
    /// Classic delete-relaxation FF best-first over the whole task.
    Ff,
    /// SGPlan-style partition-and-resolve.
    Partition,
    /// PDDL3 soft-goal preferences + anytime branch-and-bound metric optimization.
    Pddl3,
}

/// Solver options.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Options {
    pub mode: Mode,
    /// Worker threads; `0` = auto (`min(cores, 6)` or `FFDP_THREADS`).
    pub threads: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            mode: Mode::Auto,
            threads: 0,
        }
    }
}

/// One grounded action in the plan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Step {
    pub index: usize,
    pub action: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

/// A found plan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Plan {
    pub steps: Vec<Step>,
    pub length: usize,
    /// PDDL3 metric value (cost), when a metric was optimized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<f64>,
}

/// Grounding/search statistics.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Statistics {
    pub grounded_facts: usize,
    pub grounded_actions: usize,
    pub evaluated_states: usize,
    pub threads: usize,
}

/// The result of a solve.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Solution {
    pub solved: bool,
    pub mode: Mode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<Plan>,
    pub statistics: Statistics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// Re-exported so callers can name the PDDL3 metric type if needed.
pub type Metric = f64;

/// Errors that prevent producing a [`Solution`].
#[derive(thiserror::Error, Debug)]
pub enum SolveError {
    #[error("domain parse error: {0}")]
    DomainParse(String),
    #[error("problem parse error: {0}")]
    ProblemParse(String),
    #[error("{kind} {pred} uses an unknown or empty type {ty}")]
    EmptyType {
        kind: String,
        pred: String,
        ty: String,
    },
}

enum Grounded {
    Task(Box<PackedTask>),
    /// goal already true — the empty plan solves it
    Trivial,
    /// goal provably false / references an undefined fluent
    Unsolvable,
}

fn do_ground(
    domain: &crate::types::Domain,
    problem: &crate::types::Problem,
    threads: usize,
) -> Result<Grounded, SolveError> {
    match ground(domain, problem, threads) {
        Outcome::Task(t) => Ok(Grounded::Task(Box::new(t))),
        Outcome::GoalTrue => Ok(Grounded::Trivial),
        Outcome::GoalFalse | Outcome::GoalUndefinedFluent => Ok(Grounded::Unsolvable),
        Outcome::EmptyType { kind, pred, ty } => Err(SolveError::EmptyType {
            kind: kind.to_string(),
            pred,
            ty,
        }),
    }
}

fn steps_of(task: &PackedTask, ops: &[usize], synthetic: Option<&HashSet<String>>) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut idx = 0;
    for &oi in ops {
        let disp = &task.op_display[oi];
        let mut it = disp.split_whitespace();
        let action = it.next().unwrap_or("").to_string();
        // strip the artificial goal-closer + PDDL3 bookkeeping actions
        if action == "REACH-GOAL" || synthetic.is_some_and(|s| s.contains(&action)) {
            continue;
        }
        steps.push(Step {
            index: idx,
            action,
            args: it.map(|s| s.to_string()).collect(),
        });
        idx += 1;
    }
    steps
}

fn stats(task: &PackedTask, evaluated: usize, threads: usize) -> Statistics {
    Statistics {
        grounded_facts: task.n_reach_facts,
        grounded_actions: task.n_reach_actions,
        evaluated_states: evaluated,
        threads,
    }
}

fn trivial(mode: Mode, threads: usize) -> Solution {
    Solution {
        solved: true,
        mode,
        plan: Some(Plan {
            steps: Vec::new(),
            length: 0,
            metric: None,
        }),
        statistics: Statistics {
            threads,
            ..Default::default()
        },
        notes: vec!["goal already satisfied; the empty plan solves it".into()],
    }
}

fn unsolved(mode: Mode, stats: Statistics, notes: Vec<String>) -> Solution {
    Solution {
        solved: false,
        mode,
        plan: None,
        statistics: stats,
        notes,
    }
}

/// Ground and plan, returning a structured [`Solution`].
pub fn solve(domain_src: &str, problem_src: &str, opts: &Options) -> Result<Solution, SolveError> {
    let domain = parser::parse_domain(domain_src).map_err(SolveError::DomainParse)?;
    let problem = parser::parse_problem(problem_src).map_err(SolveError::ProblemParse)?;
    let threads = if opts.threads == 0 {
        crate::par::num_threads()
    } else {
        opts.threads
    };

    let mode = match opts.mode {
        Mode::Auto => {
            if pddl3::has_preferences(&problem) {
                Mode::Pddl3
            } else {
                Mode::Ff
            }
        }
        m => m,
    };

    if mode == Mode::Pddl3 {
        return solve_pddl3(&domain, &problem, threads);
    }
    solve_classic(&domain, &problem, threads, mode, Vec::new())
}

fn solve_classic(
    domain: &crate::types::Domain,
    problem: &crate::types::Problem,
    threads: usize,
    mode: Mode,
    extra_notes: Vec<String>,
) -> Result<Solution, SolveError> {
    let task = match do_ground(domain, problem, threads)? {
        Grounded::Task(t) => t,
        Grounded::Trivial => return Ok(trivial(mode, threads)),
        Grounded::Unsolvable => {
            return Ok(unsolved(
                mode,
                Statistics {
                    threads,
                    ..Default::default()
                },
                extra_notes,
            ))
        }
    };

    let (ops, evaluated) = if mode == Mode::Partition {
        match resolve::solve(&task, threads) {
            Solved::Plan(ops, _) => (Some(ops), 0),
            Solved::Unsolvable => (None, 0),
        }
    } else {
        match search::search(&task, threads) {
            PlanResult::Plan { ops, evaluated, .. } => (Some(ops), evaluated),
            PlanResult::Unsolvable { evaluated, .. } => (None, evaluated),
        }
    };

    match ops {
        Some(ops) => {
            let steps = steps_of(&task, &ops, None);
            Ok(Solution {
                solved: true,
                mode,
                plan: Some(Plan {
                    length: steps.len(),
                    steps,
                    metric: None,
                }),
                statistics: stats(&task, evaluated, threads),
                notes: extra_notes,
            })
        }
        None => Ok(unsolved(
            mode,
            stats(&task, evaluated, threads),
            extra_notes,
        )),
    }
}

fn solve_pddl3(
    domain: &crate::types::Domain,
    problem: &crate::types::Problem,
    threads: usize,
) -> Result<Solution, SolveError> {
    let c = pddl3::compile(domain, problem);

    // metric outside the supported class -> satisficing plan over the hard goals
    if let Some(reason) = c.unsupported.clone() {
        let note = format!(
            "PDDL3 metric not optimized ({}); returning a satisficing plan",
            reason
        );
        return solve_classic(domain, problem, threads, Mode::Pddl3, vec![note]);
    }

    let task = match do_ground(&c.domain, &c.problem, threads)? {
        Grounded::Task(t) => t,
        Grounded::Trivial => return Ok(trivial(Mode::Pddl3, threads)),
        Grounded::Unsolvable => {
            return Ok(unsolved(
                Mode::Pddl3,
                Statistics {
                    threads,
                    ..Default::default()
                },
                Vec::new(),
            ))
        }
    };

    let cf = task
        .fluent_id(pddl3::COST_DISP)
        .expect("compile() always injects the total-cost fluent");

    match pddl3::metric_optimize(&task, cf, threads) {
        Some(r) => {
            let mut notes = Vec::new();
            if c.warn_other {
                notes.push(
                    "metric has terms beyond is-violated/total-cost; optimized the supported part"
                        .into(),
                );
            }
            if !r.proven {
                notes.push("search bound hit; metric is best-found, not proven optimal".into());
            }
            let steps = steps_of(&task, &r.ops, Some(&c.synthetic));
            Ok(Solution {
                solved: true,
                mode: Mode::Pddl3,
                plan: Some(Plan {
                    length: steps.len(),
                    steps,
                    metric: Some(r.cost),
                }),
                statistics: stats(&task, 0, threads),
                notes,
            })
        }
        None => Ok(unsolved(Mode::Pddl3, stats(&task, 0, threads), Vec::new())),
    }
}
