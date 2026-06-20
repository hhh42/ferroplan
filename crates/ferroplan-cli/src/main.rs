//! `ff` — the ferroplan command-line interface.
//!
//! Drop-in for Metric-FF's `ff -o domain.pddl -f problem.pddl` (classic text
//! output), plus `--json` for a structured [`ferroplan::Solution`] and
//! `--json-request` for a self-contained `{domain, problem, options}` job.

use std::io::Read;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use ferroplan::{Mode, Options};
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(name = "ff", version, about = "ferroplan — a fast, data-parallel PDDL planner")]
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

    /// Planning strategy.
    #[arg(long, value_enum, default_value_t = ModeArg::Auto)]
    mode: ModeArg,

    /// Worker threads (0 = auto).
    #[arg(long, default_value_t = 0)]
    threads: usize,

    /// IPC time-stamped plan format (classic text mode only).
    #[arg(long)]
    ipc: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ModeArg {
    Auto,
    Ff,
    Partition,
    Pddl3,
}

impl From<ModeArg> for Mode {
    fn from(m: ModeArg) -> Self {
        match m {
            ModeArg::Auto => Mode::Auto,
            ModeArg::Ff => Mode::Ff,
            ModeArg::Partition => Mode::Partition,
            ModeArg::Pddl3 => Mode::Pddl3,
        }
    }
}

#[derive(Deserialize)]
struct JobRequest {
    /// PDDL domain source text.
    domain: String,
    /// PDDL problem source text.
    problem: String,
    #[serde(default)]
    options: Option<JobOptions>,
}

#[derive(Deserialize, Default)]
struct JobOptions {
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    threads: Option<usize>,
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
        let opts = Options {
            mode: req
                .options
                .as_ref()
                .and_then(|o| o.mode.as_deref())
                .map(parse_mode)
                .transpose()?
                .unwrap_or(Mode::Auto),
            threads: req.options.and_then(|o| o.threads).unwrap_or(0),
        };
        let sol = ferroplan::solve(&req.domain, &req.problem, &opts)?;
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

    if cli.json {
        let opts = Options { mode: cli.mode.into(), threads: cli.threads };
        let sol = ferroplan::solve(&domain, &problem, &opts)?;
        println!("{}", serde_json::to_string_pretty(&sol)?);
        std::process::exit(if sol.solved { 0 } else { 1 });
    }

    // classic text output (drop-in)
    let threads = cli.threads;
    let (text, code) = match cli.mode {
        ModeArg::Ff => ferroplan::run_ff(&domain, &problem, max1(threads)),
        _ => ferroplan::run_planner(&domain, &problem, max1(threads), cli.ipc),
    };
    print!("{}", text);
    std::process::exit(code);
}

fn max1(t: usize) -> usize {
    if t == 0 {
        ferroplan::par::num_threads()
    } else {
        t
    }
}

fn parse_mode(s: &str) -> Result<Mode> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "auto" => Mode::Auto,
        "ff" => Mode::Ff,
        "partition" => Mode::Partition,
        "pddl3" => Mode::Pddl3,
        other => bail!("unknown mode `{}` (auto|ff|partition|pddl3)", other),
    })
}
