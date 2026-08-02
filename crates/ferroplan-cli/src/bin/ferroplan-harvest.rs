use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use ferroplan_cli::harvest::{
    collect_with_gh, compile_pack, load_observation_pack, load_receipt, receipt_exit_code,
    replay_pack, save_observation_pack, ObservationWindow,
};

#[derive(Parser, Debug)]
#[command(
    name = "ferroplan-harvest",
    version,
    about = "Evidence-bounded GitHub work harvester and PPDDL corpus compiler"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Collect a read-only, date-bounded GitHub observation pack through `gh api`.
    Collect {
        #[arg(long = "repo", required = true)]
        repositories: Vec<String>,
        #[arg(long)]
        since: String,
        #[arg(long)]
        until: String,
        #[arg(long, default_value = "America/Los_Angeles")]
        timezone: String,
        #[arg(long, default_value_t = 10)]
        max_pages: usize,
        #[arg(long)]
        output: PathBuf,
    },
    /// Admit observed work and compile a generated PPDDL corpus plus receipt.
    Compile {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
    },
    /// Collect and compile in one read-only-observation / local-construction run.
    Run {
        #[arg(long = "repo", required = true)]
        repositories: Vec<String>,
        #[arg(long)]
        since: String,
        #[arg(long)]
        until: String,
        #[arg(long, default_value = "America/Los_Angeles")]
        timezone: String,
        #[arg(long, default_value_t = 10)]
        max_pages: usize,
        #[arg(long)]
        output_dir: PathBuf,
    },
    /// Recompile an exact observation pack and compare generated artifact identities.
    Replay {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        receipt: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Command::Collect {
            repositories,
            since,
            until,
            timezone,
            max_pages,
            output,
        } => {
            let pack = collect_with_gh(
                &repositories,
                ObservationWindow {
                    start_utc: since,
                    end_exclusive_utc: until,
                    timezone,
                },
                max_pages,
            )?;
            save_observation_pack(&output, &pack)?;
            println!("{}", serde_json::to_string_pretty(&pack)?);
            if pack.transport_failures.is_empty() {
                0
            } else {
                1
            }
        }
        Command::Compile { input, output_dir } => {
            let pack = load_observation_pack(&input)?;
            let receipt = compile_pack(&pack, &output_dir)?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
            receipt_exit_code(&receipt)
        }
        Command::Run {
            repositories,
            since,
            until,
            timezone,
            max_pages,
            output_dir,
        } => {
            let pack = collect_with_gh(
                &repositories,
                ObservationWindow {
                    start_utc: since,
                    end_exclusive_utc: until,
                    timezone,
                },
                max_pages,
            )?;
            save_observation_pack(&output_dir.join("observation-pack.json"), &pack)?;
            let receipt = compile_pack(&pack, &output_dir)?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
            receipt_exit_code(&receipt)
        }
        Command::Replay {
            input,
            receipt,
            output_dir,
        } => {
            let pack = load_observation_pack(&input)?;
            let expected = load_receipt(&receipt)?;
            let replayed = replay_pack(&pack, &expected, &output_dir)?;
            println!("{}", serde_json::to_string_pretty(&replayed)?);
            receipt_exit_code(&replayed)
        }
    };
    std::process::exit(exit_code);
}
