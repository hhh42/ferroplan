//! Measure mutex-group synthesis coverage on benchmark domains.
//!
//! Usage: `cargo run -p ferroplan --example invariants_coverage -- <domain-dir>...`
//! where each dir has a `domain.pddl` and one or more `pNN.pddl` problems.
//! Prints, per instance: fact count, group count, how many facts the groups
//! cover, coverage %, the largest group size, and the biggest group's predicate.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use ferroplan::ground::{ground, Outcome};
use ferroplan::invariants::synthesize;
use ferroplan::parser::{parse_domain, parse_problem};

fn main() {
    let dirs: Vec<String> = std::env::args().skip(1).collect();
    println!(
        "{:<26} {:>7} {:>7} {:>8} {:>6} {:>6}  {}",
        "instance", "facts", "groups", "covered", "cov%", "maxsz", "biggest-group"
    );
    println!("{}", "-".repeat(92));
    for dir in &dirs {
        run_dir(Path::new(dir));
    }
}

fn run_dir(dir: &Path) {
    let dom_path = dir.join("domain.pddl");
    let Ok(dom_src) = fs::read_to_string(&dom_path) else {
        eprintln!("skip {}: no domain.pddl", dir.display());
        return;
    };
    let domain = match parse_domain(&dom_src) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("skip {}: domain parse: {e}", dir.display());
            return;
        }
    };
    let label = dir.file_name().and_then(|s| s.to_str()).unwrap_or("?");

    let mut probs: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "pddl")
                && p.file_name().and_then(|s| s.to_str()) != Some("domain.pddl")
        })
        .collect();
    probs.sort();

    for prob_path in probs {
        let Ok(prob_src) = fs::read_to_string(&prob_path) else {
            continue;
        };
        let problem = match parse_problem(&prob_src) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let task = match ground(&domain, &problem, 1) {
            Outcome::Task(t) => t,
            _ => continue,
        };
        let groups = synthesize(&domain, &task);

        let covered: HashSet<u32> = groups.iter().flatten().copied().collect();
        let cov_pct = if task.n_facts == 0 {
            0.0
        } else {
            100.0 * covered.len() as f64 / task.n_facts as f64
        };
        let (max_sz, biggest) = groups
            .iter()
            .max_by_key(|g| g.len())
            .map(|g| (g.len(), task.fact_names[g[0] as usize].clone()))
            .unwrap_or((0, "-".into()));

        let pname = prob_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?");
        println!(
            "{:<26} {:>7} {:>7} {:>8} {:>5.0}% {:>6}  {}",
            format!("{label}/{pname}"),
            task.n_facts,
            groups.len(),
            covered.len(),
            cov_pct,
            max_sz,
            truncate(&biggest, 40),
        );
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}
