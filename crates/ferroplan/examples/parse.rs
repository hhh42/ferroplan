//! Syntax-check a PDDL file and print a structure summary — no grounding or solving.
//!
//! ```text
//! cargo run --example parse -- domain.pddl
//! cargo run --example parse -- problem.pddl
//! ```
//!
//! Handy as a fast "is this valid PDDL?" check while authoring (the same thing the
//! `parse` MCP tool exposes to an agent). Pass `--json` to print the raw
//! [`ferroplan::ParseReport`] instead of the human summary.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let path = match args.iter().find(|a| !a.starts_with("--")) {
        Some(p) => p,
        None => {
            eprintln!("usage: parse [--json] <file.pddl>");
            return ExitCode::FAILURE;
        }
    };
    let src = std::fs::read_to_string(path).expect("read pddl");

    let report = ferroplan::parse(&src);

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        return if report.ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    if !report.ok {
        eprintln!(
            "✗ parse error in {}: {}",
            report.kind.as_deref().unwrap_or("pddl"),
            report.error.unwrap_or_default()
        );
        return ExitCode::FAILURE;
    }

    let kind = report.kind.as_deref().unwrap_or("?");
    let name = report.name.as_deref().unwrap_or("?");
    println!("✓ valid {kind}: {name}");
    if !report.requirements.is_empty() {
        println!("  requirements: {}", report.requirements.join(" "));
    }
    if let Some(d) = report.domain {
        println!(
            "  {} types · {} predicates · {} functions · {} actions · {} durative · {} derived",
            d.types.len(),
            d.predicates.len(),
            d.functions.len(),
            d.actions.len(),
            d.durative_actions.len(),
            d.derived,
        );
        if !d.actions.is_empty() {
            println!("  actions: {}", d.actions.join(", "));
        }
    }
    if let Some(p) = report.problem {
        println!(
            "  domain {} · {} objects · {} init facts · {} fluents · {} TILs · goal={} · metric={}",
            p.domain,
            p.objects,
            p.init_facts,
            p.init_fluents,
            p.timed_initial_literals,
            p.has_goal,
            p.has_metric,
        );
    }
    ExitCode::SUCCESS
}
