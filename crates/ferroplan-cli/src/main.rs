//! `ff` — the ferroplan command-line interface.
//!
//! Drop-in for Metric-FF's `ff -o domain.pddl -f problem.pddl` (classic text
//! output), plus `--json` for a structured [`ferroplan::Solution`] and
//! `--json-request` for a self-contained `{domain, problem, options}` job.

use std::io::Read;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use ferroplan::{Mode, Options, Search};
use serde::Deserialize;

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

    /// Cap on evaluated states (default: engine default).
    #[arg(long, value_name = "N")]
    max_evaluated: Option<usize>,

    /// PDDL3: return a satisficing plan over hard goals instead of optimizing.
    #[arg(long)]
    satisfice: bool,

    /// Worker threads (0 = auto).
    #[arg(long, default_value_t = 0)]
    threads: usize,

    /// IPC time-stamped plan format (classic text mode only).
    #[arg(long)]
    ipc: bool,

    /// Validate a plan FILE against the domain/problem under ferroplan's own
    /// semantics instead of solving. Auto-detects classical vs temporal.
    #[arg(long, value_name = "FILE")]
    validate: Option<PathBuf>,
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
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ModeArg {
    Auto,
    Ff,
    Partition,
    Pddl3,
    Temporal,
}

impl From<ModeArg> for Mode {
    fn from(m: ModeArg) -> Self {
        match m {
            ModeArg::Auto => Mode::Auto,
            ModeArg::Ff => Mode::Ff,
            ModeArg::Partition => Mode::Partition,
            ModeArg::Pddl3 => Mode::Pddl3,
            ModeArg::Temporal => Mode::Temporal,
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

fn read_source(path: &str) -> Result<String> {
    if path == "-" {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        Ok(s)
    } else {
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path))
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // (1) JSON job request: self-contained {domain, problem, options} -> Solution JSON
    if let Some(req_path) = &cli.json_request {
        let raw = read_source(req_path)?;
        let req: JobRequest = serde_json::from_str(&raw).context("parsing JSON job request")?;
        let sol = ferroplan::solve(&req.domain, &req.problem, &req.options)?;
        println!("{}", serde_json::to_string_pretty(&sol)?);
        std::process::exit(if sol.solved { 0 } else { 1 });
    }

    // (2) file-based: -o / -f
    let (domain, problem) = match (&cli.domain, &cli.problem) {
        (Some(d), Some(p)) => (
            std::fs::read_to_string(d).with_context(|| format!("reading {}", d.display()))?,
            std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?,
        ),
        _ => bail!("need both -o <domain> and -f <problem> (or --json-request <file>)"),
    };

    // (2a) validate a supplied plan instead of solving
    if let Some(plan_path) = &cli.validate {
        let plan_src = std::fs::read_to_string(plan_path)
            .with_context(|| format!("reading {}", plan_path.display()))?;
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
