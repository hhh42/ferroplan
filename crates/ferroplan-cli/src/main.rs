//! `ff` — the ferroplan command-line interface.
//!
//! Drop-in for Metric-FF's `ff -o domain.pddl -f problem.pddl` (classic text
//! output), plus structured JSON, bounded production envelopes, and canonical
//! capability-readiness discovery.

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use ferroplan::{
    Decomposition, Mode, Options, OutcomeClass, ProductionLimits, Search,
};
use serde::Deserialize;

const CLI_HARD_MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
const CLI_MAX_JOB_BYTES: usize = 16 * 1024 * 1024;

/// Human-readable rendering of a [`Decomposition`]: the ordered contracts (each goal
/// + its sub-plan) and the stitched whole-goal plan.
fn render_decomposition(d: &Decomposition) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    if !d.solved {
        let _ = writeln!(s, "No plan found.");
        for n in &d.notes {
            let _ = writeln!(s, "note: {n}");
        }
        return s;
    }
    if d.monolithic {
        let _ = writeln!(
            s,
            "Goal not decomposable — solved as 1 monolithic contract.\n"
        );
    } else {
        let _ = writeln!(s, "Decomposed into {} contracts:\n", d.contracts.len());
    }
    for c in &d.contracts {
        let _ = writeln!(
            s,
            "── contract {} @ offset {:.3}  ⟶  {}",
            c.index, c.offset, c.goal
        );
        for st in &c.steps {
            let args = if st.args.is_empty() {
                String::new()
            } else {
                format!(" {}", st.args.join(" "))
            };
            match (st.time, st.duration) {
                (Some(t), Some(dur)) => {
                    let _ = writeln!(s, "   {:.3}: ({}{}) [{:.3}]", t, st.action, args, dur);
                }
                (Some(t), None) => {
                    let _ = writeln!(s, "   {:.3}: ({}{})", t, st.action, args);
                }
                _ => {
                    let _ = writeln!(s, "   ({}{})", st.action, args);
                }
            }
        }
        let _ = writeln!(s, "   [contract makespan {:.3}]", c.makespan);
    }
    if let Some(plan) = &d.plan {
        let _ = writeln!(
            s,
            "\nStitched plan: {} steps, makespan {:.3}",
            plan.length,
            plan.makespan.unwrap_or(0.0)
        );
    }
    for n in &d.notes {
        let _ = writeln!(s, "note: {n}");
    }
    s
}

#[derive(Parser, Debug)]
#[command(
    name = "ff",
    version,
    about = "ferroplan — a data-parallel PDDL planner"
)]
struct Cli {
    /// Domain file (PDDL).
    #[arg(short = 'o', long = "domain", value_name = "DOMAIN")]
    domain: Option<PathBuf>,

    /// Problem file (PDDL).
    #[arg(short = 'f', long = "problem", value_name = "PROBLEM")]
    problem: Option<PathBuf>,

    /// Read a JSON job `{domain, problem, options}` from FILE (or `-` for stdin).
    #[arg(long, value_name = "FILE")]
    json_request: Option<String>,

    /// Emit a structured JSON solution instead of classic FF text.
    #[arg(long)]
    json: bool,

    /// Emit the canonical capability manifest and its deterministic fingerprint.
    /// This is a contract report, not a self-authored production-admission verdict.
    #[arg(long)]
    readiness: bool,

    /// Execute through the bounded candidate-only production envelope. This
    /// always emits JSON and uses stable production exit classes.
    #[arg(long)]
    production: bool,

    /// Optional request/correlation ID for `--production`.
    #[arg(long, value_name = "ID")]
    request_id: Option<String>,

    /// Production maximum domain bytes.
    #[arg(long, value_name = "BYTES")]
    max_domain_bytes: Option<usize>,

    /// Production maximum problem bytes.
    #[arg(long, value_name = "BYTES")]
    max_problem_bytes: Option<usize>,

    /// Production maximum emitted plan steps.
    #[arg(long, value_name = "N")]
    max_plan_steps: Option<usize>,

    /// Production maximum serialized solution bytes.
    #[arg(long, value_name = "BYTES")]
    max_output_bytes: Option<usize>,

    /// Production maximum worker count.
    #[arg(long, value_name = "N")]
    max_workers: Option<usize>,

    /// Planning mode (`auto` routes by problem features).
    #[arg(long, value_enum, default_value_t = ModeArg::Auto)]
    mode: ModeArg,

    /// Search strategy (applies to ff / library / --json paths).
    #[arg(long, value_enum, default_value_t = SearchArg::Auto)]
    search: SearchArg,

    /// Disable helpful-action pruning (used by EHC).
    #[arg(long = "no-helpful")]
    no_helpful: bool,

    /// Best-first g (path-length) weight.
    #[arg(long, default_value_t = 1.0)]
    weight_g: f64,

    /// Best-first h (heuristic) weight.
    #[arg(long, default_value_t = 5.0)]
    weight_h: f64,

    /// Cap on evaluated states (default: engine default, or production profile).
    #[arg(long, value_name = "N")]
    max_evaluated: Option<usize>,

    /// PDDL3: return a satisficing plan over hard goals instead of optimizing.
    #[arg(long)]
    satisfice: bool,

    /// Worker threads (0 = auto; production mode resolves 0 to one worker).
    #[arg(long, default_value_t = 0)]
    threads: usize,

    /// IPC time-stamped plan format (classic text mode only).
    #[arg(long)]
    ipc: bool,

    /// Validate a plan FILE against the domain/problem under ferroplan's own
    /// semantics instead of solving. Auto-detects classical vs temporal.
    #[arg(long, value_name = "FILE")]
    validate: Option<PathBuf>,

    /// Decompose a (too-big) temporal goal into ordered, solvable contracts and print
    /// the breakdown plus the stitched plan (`--json` for the structured form).
    #[arg(long)]
    decompose: bool,
}

impl Cli {
    fn to_options(&self) -> Options {
        Options {
            mode: self.mode.into(),
            search: self.search.into(),
            helpful_actions: !self.no_helpful,
            weight_g: self.weight_g,
            weight_h: self.weight_h,
            threads: self.threads,
            max_evaluated: self.max_evaluated,
            optimize: !self.satisfice,
        }
    }

    fn production_limits(&self) -> ProductionLimits {
        let defaults = ProductionLimits::default();
        ProductionLimits {
            max_domain_bytes: self.max_domain_bytes.unwrap_or(defaults.max_domain_bytes),
            max_problem_bytes: self.max_problem_bytes.unwrap_or(defaults.max_problem_bytes),
            max_evaluated: self.max_evaluated.unwrap_or(defaults.max_evaluated),
            max_plan_steps: self.max_plan_steps.unwrap_or(defaults.max_plan_steps),
            max_output_bytes: self.max_output_bytes.unwrap_or(defaults.max_output_bytes),
            max_workers: self.max_workers.unwrap_or(defaults.max_workers),
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ModeArg {
    Auto,
    Ff,
    Partition,
    Pddl3,
    Temporal,
    Portfolio,
    /// Sequential-optimal: A* + admissible h^max, proof-or-nothing (0.19).
    Optimal,
}

impl From<ModeArg> for Mode {
    fn from(m: ModeArg) -> Self {
        match m {
            ModeArg::Auto => Mode::Auto,
            ModeArg::Ff => Mode::Ff,
            ModeArg::Portfolio => Mode::Portfolio,
            ModeArg::Partition => Mode::Partition,
            ModeArg::Pddl3 => Mode::Pddl3,
            ModeArg::Temporal => Mode::Temporal,
            ModeArg::Optimal => Mode::Optimal,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum SearchArg {
    Auto,
    Ehc,
    BestFirst,
    EhcThenBestFirst,
}

impl From<SearchArg> for Search {
    fn from(s: SearchArg) -> Self {
        match s {
            SearchArg::Auto => Search::Auto,
            SearchArg::Ehc => Search::Ehc,
            SearchArg::BestFirst => Search::BestFirst,
            SearchArg::EhcThenBestFirst => Search::EhcThenBestFirst,
        }
    }
}

#[derive(Deserialize)]
struct JobRequest {
    /// PDDL domain source text.
    domain: String,
    /// PDDL problem source text.
    problem: String,
    /// Solver options (any subset; omitted fields use defaults).
    #[serde(default)]
    options: Options,
}

fn read_limited(mut reader: impl Read, max_bytes: usize, label: &str) -> Result<String> {
    let limit = u64::try_from(max_bytes)
        .unwrap_or(u64::MAX - 1)
        .saturating_add(1);
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(limit)
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading {label}"))?;
    if bytes.len() > max_bytes {
        bail!("{label} exceeds the {max_bytes}-byte input limit");
    }
    String::from_utf8(bytes).with_context(|| format!("{label} is not valid UTF-8"))
}

fn read_source(path: &str, max_bytes: usize) -> Result<String> {
    if path == "-" {
        let stdin = std::io::stdin();
        read_limited(stdin.lock(), max_bytes, "stdin")
    } else {
        let file = std::fs::File::open(path).with_context(|| format!("opening {path}"))?;
        read_limited(file, max_bytes, path)
    }
}

fn read_path(path: &Path, max_bytes: usize) -> Result<String> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    read_limited(file, max_bytes, &path.display().to_string())
}

fn production_exit(outcome: OutcomeClass, error_code: Option<&str>) -> i32 {
    match outcome {
        OutcomeClass::Solved => 0,
        OutcomeClass::NoPlan => 3,
        OutcomeClass::LimitExceeded => 5,
        OutcomeClass::Refused => match error_code {
            Some("FP_UNSUPPORTED") => 4,
            Some("FP_PARSE" | "FP_MODEL" | "FP_INVALID_REQUEST" | "FP_LIMIT_INPUT") => 2,
            _ => 7,
        },
        OutcomeClass::Failed => 70,
    }
}

fn print_readiness() -> Result<()> {
    let manifest = ferroplan::capability_manifest();
    let fingerprint = manifest.fingerprint()?;
    let report = serde_json::json!({
        "schema_version": "ferroplan.readiness-contract.v1",
        "product_version": env!("CARGO_PKG_VERSION"),
        "manifest_fingerprint": fingerprint,
        "contract_valid": true,
        "admission_state": "declared",
        "admission_notice": "Capability admission is verifier-derived from exact-source evidence; this command does not self-crown the build.",
        "manifest": manifest,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn run_production(
    domain: &str,
    problem: &str,
    options: &Options,
    limits: &ProductionLimits,
    request_id: Option<&str>,
) -> Result<()> {
    let envelope = ferroplan::solve_production(domain, problem, options, limits, request_id);
    let code = production_exit(
        envelope.outcome,
        envelope.error.as_ref().map(|error| error.code.as_str()),
    );
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    std::process::exit(code);
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.readiness {
        return print_readiness();
    }

    let production_limits = cli.production_limits();

    // (1) JSON job request: self-contained {domain, problem, options} -> result JSON
    if let Some(req_path) = &cli.json_request {
        let raw = read_source(req_path, CLI_MAX_JOB_BYTES)?;
        let req: JobRequest = serde_json::from_str(&raw).context("parsing JSON job request")?;
        if cli.production {
            return run_production(
                &req.domain,
                &req.problem,
                &req.options,
                &production_limits,
                cli.request_id.as_deref(),
            );
        }
        if req.domain.len() > CLI_HARD_MAX_INPUT_BYTES
            || req.problem.len() > CLI_HARD_MAX_INPUT_BYTES
        {
            bail!("embedded domain or problem exceeds the CLI hard input limit");
        }
        let sol = ferroplan::solve(&req.domain, &req.problem, &req.options)?;
        println!("{}", serde_json::to_string_pretty(&sol)?);
        std::process::exit(if sol.solved { 0 } else { 1 });
    }

    // (2) file-based: -o / -f
    let domain_limit = if cli.production {
        production_limits.max_domain_bytes
    } else {
        CLI_HARD_MAX_INPUT_BYTES
    };
    let problem_limit = if cli.production {
        production_limits.max_problem_bytes
    } else {
        CLI_HARD_MAX_INPUT_BYTES
    };
    let (domain, problem) = match (&cli.domain, &cli.problem) {
        (Some(d), Some(p)) => (
            read_path(d, domain_limit)?,
            read_path(p, problem_limit)?,
        ),
        _ => bail!(
            "need both -o <domain> and -f <problem> (or --json-request <file>, or --readiness)"
        ),
    };

    if cli.production {
        return run_production(
            &domain,
            &problem,
            &cli.to_options(),
            &production_limits,
            cli.request_id.as_deref(),
        );
    }

    // (2a) validate a supplied plan instead of solving
    if let Some(plan_path) = &cli.validate {
        let plan_src = read_path(plan_path, CLI_HARD_MAX_INPUT_BYTES)?;
        match ferroplan::plan::validate_plan(&domain, &problem, &plan_src) {
            Ok(ferroplan::plan::Validity::Valid) => {
                println!("Plan valid");
                std::process::exit(0);
            }
            Ok(ferroplan::plan::Validity::Invalid(why)) => {
                println!("Plan invalid: {}", why);
                std::process::exit(1);
            }
            Err(e) => bail!("validate: {}", e),
        }
    }

    let opts = cli.to_options();

    // (2b) decompose a temporal goal into contracts instead of a flat solve
    if cli.decompose {
        let d = ferroplan::decompose(&domain, &problem, &opts)?;
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&d)?);
        } else {
            print!("{}", render_decomposition(&d));
        }
        std::process::exit(if d.solved { 0 } else { 1 });
    }

    if cli.json {
        let sol = ferroplan::solve(&domain, &problem, &opts)?;
        println!("{}", serde_json::to_string_pretty(&sol)?);
        std::process::exit(if sol.solved { 0 } else { 1 });
    }

    // classic text output (drop-in)
    let (text, code) = match cli.mode {
        ModeArg::Ff => ferroplan::run_ff(&domain, &problem, &opts),
        _ => ferroplan::run_planner(&domain, &problem, &opts, cli.ipc),
    };
    print!("{}", text);
    std::process::exit(code);
}