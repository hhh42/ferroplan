//! Minimal: solve a PDDL problem from a path pair and print the plan.
//!
//! ```text
//! cargo run --example solve -- domain.pddl problem.pddl
//! ```

use std::process::ExitCode;

use ferroplan::{solve, Options};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: solve <domain.pddl> <problem.pddl>");
        return ExitCode::FAILURE;
    }
    let domain = std::fs::read_to_string(&args[1]).expect("read domain");
    let problem = std::fs::read_to_string(&args[2]).expect("read problem");

    match solve(&domain, &problem, &Options::default()) {
        Ok(sol) if sol.solved => {
            let plan = sol.plan.unwrap();
            println!("solved in {} steps ({:?} mode):", plan.length, sol.mode);
            for step in &plan.steps {
                println!("  {} {}", step.action, step.args.join(" "));
            }
            if let Some(m) = plan.metric {
                println!("metric: {m}");
            }
            ExitCode::SUCCESS
        }
        Ok(_) => {
            println!("no plan found");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
