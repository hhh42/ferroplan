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
use crate::search;

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
    /// PDDL2.1 durative actions via decision-epoch temporal search.
    Temporal,
    /// Sequential portfolio over complementary classical configurations
    /// under one shared eval budget (ferroplan-roadmap.md Phase 6).
    /// Classical-search only: temporal and preference/metric problems keep
    /// their own machinery (this mode falls back to it, like `auto`).
    Portfolio,
}

/// Which search strategy to use within a mode.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Search {
    /// Let the engine choose: enforced hill-climbing, then weighted best-first if
    /// EHC gets stuck (the FF/Metric-FF default — fast on most problems).
    #[default]
    Auto,
    /// Enforced hill-climbing (+ helpful actions), falling back to best-first if
    /// it finds no improving state (kept complete).
    Ehc,
    /// Weighted best-first over the whole task (complete; ignores helpful actions).
    BestFirst,
    /// EHC first, fall back to best-first if it gets stuck (same as `auto`).
    EhcThenBestFirst,
}

fn default_weight_g() -> f64 {
    1.0
}
fn default_weight_h() -> f64 {
    5.0
}
fn default_true() -> bool {
    true
}

/// Solver options — the single, library-first configuration surface. Every knob
/// is settable from code and round-trips through JSON (`serde`); the CLI derives
/// the same flags. Unspecified JSON fields fall back to these defaults.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Options {
    /// Planning mode (`auto` routes by problem features).
    #[serde(default)]
    pub mode: Mode,
    /// Search strategy within the mode.
    #[serde(default)]
    pub search: Search,
    /// Restrict expansion to helpful actions (used by EHC; no effect on plain
    /// best-first yet).
    #[serde(default = "default_true")]
    pub helpful_actions: bool,
    /// Best-first `g` (path-length) weight.
    #[serde(default = "default_weight_g")]
    pub weight_g: f64,
    /// Best-first `h` (heuristic) weight. Default `1·g + 5·h`.
    #[serde(default = "default_weight_h")]
    pub weight_h: f64,
    /// Worker threads; `0` = auto (`min(cores, 6)` or `FFDP_THREADS`).
    #[serde(default)]
    pub threads: usize,
    /// Cap on evaluated states; `None` = engine default.
    #[serde(default)]
    pub max_evaluated: Option<usize>,
    /// PDDL3: optimize the metric (`true`) vs. return a satisficing plan over the
    /// hard goals only (`false`).
    #[serde(default = "default_true")]
    pub optimize: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            mode: Mode::Auto,
            search: Search::Auto,
            helpful_actions: true,
            weight_g: default_weight_g(),
            weight_h: default_weight_h(),
            threads: 0,
            max_evaluated: None,
            optimize: true,
        }
    }
}

impl Options {
    fn search_cfg(&self) -> crate::search::SearchCfg {
        crate::search::SearchCfg::from_weights(self.weight_g, self.weight_h, self.max_evaluated)
    }
}

/// One grounded action in the plan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Step {
    pub index: usize,
    pub action: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Temporal mode: the action's dispatch time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<f64>,
    /// Temporal mode: the durative action's duration (absent for instantaneous).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
}

/// A found plan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Plan {
    pub steps: Vec<Step>,
    pub length: usize,
    /// PDDL3 metric value (cost), when a metric was optimized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<f64>,
    /// Temporal mode: total plan makespan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub makespan: Option<f64>,
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

/// One contract in a [`Decomposition`]: a sub-goal small enough for the temporal
/// search to solve whole, the sub-plan that achieves it, and where that sub-plan sits
/// in the stitched global timeline.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Contract {
    pub index: usize,
    /// The sub-goal this contract discharges, rendered for inspection
    /// (e.g. `(order o1), (order o2)` or `coin >= 15`).
    pub goal: String,
    /// The contract's sub-plan, timed relative to its own start.
    pub steps: Vec<Step>,
    /// Sub-plan makespan.
    pub makespan: f64,
    /// Offset of this contract's sub-plan in the stitched whole-goal timeline.
    pub offset: f64,
}

/// The inspectable result of decomposing a temporal goal into solvable contracts:
/// the ordered contracts, the stitched whole-goal plan, and whether the goal had to
/// fall back to a single monolithic solve (un-splittable, or the split didn't
/// validate — then there is exactly one contract: the whole goal).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Decomposition {
    pub solved: bool,
    pub contracts: Vec<Contract>,
    /// The stitched, validated whole-goal plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<Plan>,
    /// True when the goal couldn't be split — `contracts` is then the single whole goal.
    pub monolithic: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// A "just parse" report: validate PDDL **syntax** and summarize the structure
/// *without* grounding or solving. Auto-detects domain vs problem. `ok` is `false`
/// with `error` set on a parse failure. Serde-serializable — for editor tooling, an
/// LLM authoring loop that wants fast syntax feedback, or the MCP `parse` tool. For
/// the full typed AST, use [`crate::parser::parse_domain`] / `parse_problem`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParseReport {
    pub ok: bool,
    /// `"domain"` or `"problem"` (best-effort classification, set even on error).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<String>,
    /// Parse error (with line number) when `ok` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<DomainSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<ProblemSummary>,
}

/// Structure summary of a parsed domain (signatures rendered as `name argtypes…`).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DomainSummary {
    pub types: Vec<String>,
    pub predicates: Vec<String>,
    pub functions: Vec<String>,
    pub actions: Vec<String>,
    pub durative_actions: Vec<String>,
    pub derived: usize,
}

/// Structure summary of a parsed problem.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProblemSummary {
    /// The domain name this problem references.
    pub domain: String,
    pub objects: usize,
    pub init_facts: usize,
    pub init_fluents: usize,
    pub timed_initial_literals: usize,
    pub has_goal: bool,
    pub has_metric: bool,
}

/// Parse a PDDL source string (auto-detecting domain vs problem) into a structured
/// [`ParseReport`] — syntax validation + a structure summary, no grounding or solving.
pub fn parse(src: &str) -> ParseReport {
    // Same content-routing heuristic the visualizer uses: whichever of `(problem` /
    // `(domain` appears first wins.
    let up = src.to_ascii_uppercase();
    let is_problem = match (up.find("(PROBLEM"), up.find("(DOMAIN")) {
        (Some(p), Some(d)) => p < d,
        (Some(_), None) => true,
        _ => false,
    };
    if is_problem {
        match parser::parse_problem(src) {
            Ok(p) => ParseReport {
                ok: true,
                kind: Some("problem".into()),
                name: Some(p.name.to_lowercase()),
                requirements: Vec::new(), // problem-file requirements are over-read
                error: None,
                domain: None,
                problem: Some(ProblemSummary {
                    domain: p.domain_name.to_lowercase(),
                    objects: p.objects.len(),
                    init_facts: p.init_atoms.len(),
                    init_fluents: p.init_fluents.len(),
                    timed_initial_literals: p.til.len(),
                    has_goal: !matches!(p.goal, crate::types::Formula::True),
                    has_metric: p.metric.is_some(),
                }),
            },
            Err(e) => parse_err("problem", e),
        }
    } else {
        match parser::parse_domain(src) {
            Ok(d) => ParseReport {
                ok: true,
                kind: Some("domain".into()),
                name: Some(d.name.to_lowercase()),
                requirements: d
                    .requirements
                    .iter()
                    .map(|r| format!(":{}", r.trim_start_matches(':').to_lowercase()))
                    .collect(),
                error: None,
                domain: Some(DomainSummary {
                    types: d.types.iter().map(|t| t.to_lowercase()).collect(),
                    predicates: d.predicates.iter().map(|(n, a)| render_sig(n, a)).collect(),
                    functions: d.functions.iter().map(|(n, a)| render_sig(n, a)).collect(),
                    actions: d.actions.iter().map(|a| a.name.to_lowercase()).collect(),
                    durative_actions: d
                        .durative_actions
                        .iter()
                        .map(|a| a.name.to_lowercase())
                        .collect(),
                    derived: d.derived.len(),
                }),
                problem: None,
            },
            Err(e) => parse_err("domain", e),
        }
    }
}

/// `name argtype1 argtype2 …` (just `name` for a 0-arity predicate/function).
fn render_sig(name: &str, arg_types: &[String]) -> String {
    if arg_types.is_empty() {
        name.to_lowercase()
    } else {
        let args = arg_types
            .iter()
            .map(|t| t.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        format!("{} {}", name.to_lowercase(), args)
    }
}

fn parse_err(kind: &str, e: crate::types::ParseError) -> ParseReport {
    ParseReport {
        ok: false,
        kind: Some(kind.to_string()),
        name: None,
        requirements: Vec::new(),
        error: Some(e.to_string()),
        domain: None,
        problem: None,
    }
}

/// Re-exported so callers can name the PDDL3 metric type if needed.
pub type Metric = f64;

/// Errors that prevent producing a [`Solution`].
#[derive(thiserror::Error, Debug)]
pub enum SolveError {
    #[error("domain parse error: {0}")]
    DomainParse(crate::types::ParseError),
    #[error("problem parse error: {0}")]
    ProblemParse(crate::types::ParseError),
    #[error("{kind} {pred} uses an unknown or empty type {ty}")]
    EmptyType {
        kind: String,
        pred: String,
        ty: String,
    },
    #[error("derived predicate error: {0}")]
    Derived(String),
    #[error("unsupported feature: {0}")]
    Unsupported(String),
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

pub(crate) fn steps_of(
    task: &PackedTask,
    ops: &[usize],
    synthetic: Option<&HashSet<String>>,
) -> Vec<Step> {
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
            time: None,
            duration: None,
        });
        idx += 1;
    }
    steps
}

/// Strip the synthetic `TRAJ-END` step from a converted step list (0.8 END
/// construction), applied IFF the constraint gate compiled. Indices are
/// re-derived so they stay contiguous over real actions.
fn strip_end_steps(steps: Vec<Step>, constrained: bool) -> Vec<Step> {
    if !constrained {
        return steps;
    }
    steps
        .into_iter()
        .filter(|s| s.action != crate::constraints::END_ACTION)
        .enumerate()
        .map(|(i, mut s)| {
            s.index = i;
            s
        })
        .collect()
}

/// Convert a temporal plan's timed steps to API [`Step`]s (action head + args + time
/// + duration). Shared by the temporal solve and the decomposer.
pub(crate) fn timed_steps(tp: &crate::temporal::TimedPlan) -> Vec<Step> {
    tp.steps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut it = s.action.split_whitespace();
            Step {
                index: i,
                action: it.next().unwrap_or("").to_string(),
                args: it.map(|x| x.to_string()).collect(),
                time: Some(s.time),
                duration: s.duration,
            }
        })
        .collect()
}

pub(crate) fn stats(task: &PackedTask, evaluated: usize, threads: usize) -> Statistics {
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
            makespan: None,
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
///
/// **Temporal domains, v0.3.0+:** on a failed default-tier search, this retries at
/// the `Full` demand tier and then the goal decomposer before giving up (see
/// [`crate::temporal::solve`]) — an instance that used to fail fast can now take
/// substantially longer to return `solved: false`. Set `FF_NO_ESCALATE` (or
/// [`crate::features::set_escalate_override`]`(false)` in-process) to restore the
/// single-pass pre-0.3.0 behavior.
pub fn solve(domain_src: &str, problem_src: &str, opts: &Options) -> Result<Solution, SolveError> {
    let domain = parser::parse_domain(domain_src).map_err(SolveError::DomainParse)?;
    let problem = parser::parse_problem(problem_src).map_err(SolveError::ProblemParse)?;
    // Compile `:derived` axioms away (static rules -> init facts) before routing.
    let (domain, problem) =
        crate::derived::compile(&domain, &problem).map_err(SolveError::Derived)?;
    // 0.7: hard untimed trajectory constraints compile into monitor automata;
    // everything else gets a NAMED rejection (see constraints::gate).
    // `constrained` records that the gate compiled — the flag that tells
    // reporting to strip the synthetic TRAJ-END step (0.8 END construction);
    // it is never set on the constraint-free byte-identical path.
    let (domain, problem, constrained) = match crate::constraints::gate(&domain, &problem) {
        Ok(Some((d, p))) => (d, p, true),
        Ok(None) => (domain, problem, false),
        Err(reason) => return Err(SolveError::Unsupported(reason)),
    };
    let threads = if opts.threads == 0 {
        crate::par::num_threads()
    } else {
        opts.threads
    };

    let mode = match opts.mode {
        Mode::Auto => {
            if crate::temporal::is_temporal(&domain) {
                Mode::Temporal
            } else if pddl3::has_preferences(&problem) {
                Mode::Pddl3
            } else {
                Mode::Ff
            }
        }
        m => m,
    };

    // Portfolio is a classical-search feature: problems the portfolio's
    // members cannot represent keep their own machinery, exactly like auto.
    let mode = if mode == Mode::Portfolio
        && (crate::temporal::is_temporal(&domain) || pddl3::has_preferences(&problem))
    {
        if crate::temporal::is_temporal(&domain) {
            Mode::Temporal
        } else {
            Mode::Pddl3
        }
    } else {
        mode
    };

    match mode {
        Mode::Temporal => solve_temporal(&domain, &problem, threads),
        Mode::Pddl3 => solve_pddl3(&domain, &problem, opts, threads, constrained),
        _ => solve_classic(
            &domain,
            &problem,
            opts,
            threads,
            mode,
            Vec::new(),
            constrained,
        ),
    }
}

/// Decompose a temporal goal into solvable contracts, solve and stitch them, and
/// return the inspectable [`Decomposition`]. This always runs the partition-and-
/// resolve decomposer (independent of the `FF_TDECOMP` flag): a goal too big for the
/// one-shot temporal search is split into ordered sub-contracts — each solved whole
/// and verified — then stitched into one validated plan. A goal that can't be split
/// (or whose split doesn't validate) falls back to a single monolithic contract.
pub fn decompose(
    domain_src: &str,
    problem_src: &str,
    opts: &Options,
) -> Result<Decomposition, SolveError> {
    let domain = parser::parse_domain(domain_src).map_err(SolveError::DomainParse)?;
    let problem = parser::parse_problem(problem_src).map_err(SolveError::ProblemParse)?;
    let (domain, problem) =
        crate::derived::compile(&domain, &problem).map_err(SolveError::Derived)?;
    // 0.7 gate: decompose targets temporal goals, where trajectory
    // constraints stay rejected (Phase 3) — the gate names that. A CLASSICAL
    // constrained input still passes through (falling back to one contract),
    // so `constrained` drives the TRAJ-END step strip below.
    let (domain, problem, constrained) = match crate::constraints::gate(&domain, &problem) {
        Ok(Some((d, p))) => (d, p, true),
        Ok(None) => (domain, problem, false),
        Err(reason) => return Err(SolveError::Unsupported(reason)),
    };
    let threads = if opts.threads == 0 {
        crate::par::num_threads()
    } else {
        opts.threads
    };

    let mut notes = Vec::new();
    if !crate::temporal::is_temporal(&domain) {
        notes.push(
            "decomposition targets temporal (durative-action) goals; this domain has none".into(),
        );
    }

    match crate::tresolve::decompose(&domain, &problem, threads) {
        Some(d) => {
            let contracts = d
                .contracts
                .iter()
                .enumerate()
                .map(|(i, cr)| Contract {
                    index: i,
                    goal: cr.goal.clone(),
                    steps: strip_end_steps(timed_steps(&cr.plan), constrained),
                    makespan: cr.plan.makespan,
                    offset: cr.offset,
                })
                .collect();
            let steps = strip_end_steps(timed_steps(&d.plan), constrained);
            if d.monolithic {
                notes.push(
                    "goal could not be split into independent contracts; solved monolithically"
                        .into(),
                );
            }
            Ok(Decomposition {
                solved: true,
                contracts,
                plan: Some(Plan {
                    length: steps.len(),
                    steps,
                    metric: None,
                    makespan: Some(d.plan.makespan),
                }),
                monolithic: d.monolithic,
                notes,
            })
        }
        None => {
            notes.push("no plan found (decomposed or monolithic)".into());
            Ok(Decomposition {
                solved: false,
                contracts: Vec::new(),
                plan: None,
                monolithic: false,
                notes,
            })
        }
    }
}

fn solve_temporal(
    domain: &crate::types::Domain,
    problem: &crate::types::Problem,
    threads: usize,
) -> Result<Solution, SolveError> {
    // FF_TDECOMP routes through the partition-and-resolve decomposer (Phase B), the
    // same gate as the text path (run_planner); the default is `temporal::solve` —
    // the monolithic search plus its on-failure escalation ladder.
    let result = if crate::features::tdecomp() {
        crate::tresolve::solve(domain, problem, threads)
    } else {
        crate::temporal::solve(domain, problem, threads)
    };
    match result {
        Some(tp) => {
            let steps = timed_steps(&tp);
            Ok(Solution {
                solved: true,
                mode: Mode::Temporal,
                plan: Some(Plan {
                    length: steps.len(),
                    steps,
                    metric: None,
                    makespan: Some(tp.makespan),
                }),
                statistics: Statistics {
                    threads,
                    ..Default::default()
                },
                notes: Vec::new(),
            })
        }
        None => Ok(unsolved(
            Mode::Temporal,
            Statistics {
                threads,
                ..Default::default()
            },
            Vec::new(),
        )),
    }
}

fn solve_classic(
    domain: &crate::types::Domain,
    problem: &crate::types::Problem,
    opts: &Options,
    threads: usize,
    mode: Mode,
    extra_notes: Vec<String>,
    // The constraint gate compiled: strip the synthetic TRAJ-END step
    // from the reported plan (0.8 END construction).
    strip_end: bool,
) -> Result<Solution, SolveError> {
    let mut notes = extra_notes;
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
                notes,
            ))
        }
    };

    let (ops, evaluated) = if mode == Mode::Portfolio {
        let o = crate::portfolio::solve(&task, threads, opts.search_cfg());
        if let Some(w) = o.winner {
            notes.push(format!("portfolio: solved by member `{w}`"));
        }
        (o.ops, o.evaluated)
    } else if mode == Mode::Partition {
        let groups = crate::invariants::synthesize(domain, &task);
        match resolve::solve(&task, threads, opts.search_cfg(), &groups) {
            Solved::Plan(ops, _) => (Some(ops), 0),
            Solved::Unsolvable => (None, 0),
        }
    } else {
        let ehc_first = opts.search != Search::BestFirst;
        let o = search::plan(&task, threads, opts.search_cfg(), ehc_first);
        if o.ehc_fell_back && o.ops.is_some() {
            notes.push("EHC found no improving state; used weighted best-first".into());
        }
        (o.ops, o.evaluated)
    };

    match ops {
        Some(mut ops) => {
            // IPC6 `:action-costs`: report the metric's real value and run the
            // anytime cost-improvement sweep (0.9 Phase 2). The first plan
            // above is untouched machinery — only this polish pass is new.
            let mut metric = None;
            let mut sweep_evals = 0;
            if let Some(cf) =
                crate::costs::metric_fluent(problem).and_then(|disp| task.fluent_id(&disp))
            {
                match crate::costs::plan_cost(&task, cf, &ops) {
                    Some(c0) if opts.optimize => {
                        let r = crate::costs::improve(
                            &task,
                            cf,
                            ops,
                            c0,
                            threads,
                            opts.search_cfg(),
                            evaluated,
                        );
                        ops = r.ops;
                        metric = Some(r.cost);
                        sweep_evals = r.evaluated;
                        if r.improved {
                            notes.push(format!(
                                "anytime cost sweep improved plan cost {} -> {}",
                                c0, r.cost
                            ));
                        }
                        if r.proven {
                            notes.push("plan cost proven optimal".into());
                        }
                    }
                    Some(c0) => {
                        metric = Some(c0);
                        notes.push("cost metric reported, not optimized (--satisfice)".into());
                    }
                    None => notes
                        .push("metric fluent undefined at plan end; metric not reported".into()),
                }
            } else if problem.metric.is_none() && opts.optimize {
                // Metric-FREE problem: plan LENGTH is the quality measure.
                // Iterated-weight anytime (0.9 Phase 3 remainder) — bounded
                // re-searches at decreasing w_h keep the shortest plan;
                // FF_LEN_SWEEP_EVALS=0 restores first-found byte-identically.
                let len0 = ops.len();
                let (better, evals, improved) =
                    crate::costs::improve_length(&task, ops, threads, opts.search_cfg(), evaluated);
                ops = better;
                sweep_evals = evals;
                if improved {
                    notes.push(format!(
                        "iterated-weight sweep shortened plan {} -> {} steps",
                        len0,
                        ops.len()
                    ));
                }
            }
            if strip_end {
                crate::constraints::strip_end(&task, &mut ops);
            }
            let steps = steps_of(&task, &ops, None);
            Ok(Solution {
                solved: true,
                mode,
                plan: Some(Plan {
                    length: steps.len(),
                    steps,
                    metric,
                    makespan: None,
                }),
                statistics: stats(&task, evaluated + sweep_evals, threads),
                notes,
            })
        }
        None => Ok(unsolved(mode, stats(&task, evaluated, threads), notes)),
    }
}

fn solve_pddl3(
    domain: &crate::types::Domain,
    problem: &crate::types::Problem,
    opts: &Options,
    threads: usize,
    // The constraint gate compiled: strip the synthetic TRAJ-END step
    // from the reported plan (0.8 END construction).
    strip_end: bool,
) -> Result<Solution, SolveError> {
    // caller opted out of metric optimization -> satisficing plan (hard goals).
    if !opts.optimize {
        let note = "PDDL3 metric not optimized (optimize = false); satisficing plan".to_string();
        return solve_classic(
            domain,
            problem,
            opts,
            threads,
            Mode::Pddl3,
            vec![note],
            strip_end,
        );
    }

    let mut c = pddl3::compile(domain, problem);
    if strip_end {
        // TRAJ-END is a real action to the P3 machinery (it plans before the
        // freeze) but a synthetic step to every reporting surface.
        c.synthetic
            .insert(crate::constraints::END_ACTION.to_string());
    }

    // metric outside the supported class -> satisficing plan over the hard goals
    if let Some(reason) = c.unsupported.clone() {
        let note = format!(
            "PDDL3 metric not optimized ({}); returning a satisficing plan",
            reason
        );
        return solve_classic(
            domain,
            problem,
            opts,
            threads,
            Mode::Pddl3,
            vec![note],
            strip_end,
        );
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
    match pddl3::metric_optimize(&task, cf, &forgos, &groups, c.folded_metric, threads) {
        Some(r) => {
            let mut notes = Vec::new();
            if c.warn_other {
                notes.push(
                    "metric has terms beyond is-violated/total-cost; optimized the supported part"
                        .into(),
                );
            }
            if c.maximized {
                notes.push(
                    "maximize metric normalized to minimize; reported metric is the original \
                     (maximized) value"
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
                    metric: Some(c.display_metric(r.cost)),
                    makespan: None,
                }),
                statistics: stats(&task, 0, threads),
                notes,
            })
        }
        None => Ok(unsolved(Mode::Pddl3, stats(&task, 0, threads), Vec::new())),
    }
}
