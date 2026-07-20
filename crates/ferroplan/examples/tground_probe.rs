//! Temporal grounding probe: snap-compile + ground an instance, print task
//! dimensions and a fact-space histogram by predicate head — the attribution
//! eyes for temporal memory work (which predicate owns a blown-up fact space).
//! Usage: tground_probe <domain> <problem> [top_n]
use std::collections::HashMap;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let top_n: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(12);
    let dom = std::fs::read_to_string(&a[1]).unwrap();
    let prob = std::fs::read_to_string(&a[2]).unwrap();
    let d = ferroplan::parser::parse_domain(&dom).unwrap();
    let p = ferroplan::parser::parse_problem(&prob).unwrap();
    let c = ferroplan::temporal::compile(&d, &p);
    let t0 = std::time::Instant::now();
    let task = match ferroplan::ground::ground_stratified(&c.domain, &c.problem, 1) {
        ferroplan::ground::Outcome::Task(t) => t,
        _ => panic!("ground"),
    };
    let ground_ms = t0.elapsed().as_millis();
    println!(
        "facts {}  words {}  ops {}  fluents {}  ground {}ms",
        task.n_facts,
        task.words,
        task.n_ops,
        task.fv0.len(),
        ground_ms
    );
    let mut by_head: HashMap<&str, usize> = HashMap::new();
    for name in &task.fact_names {
        let head = name
            .trim_start_matches('(')
            .split_whitespace()
            .next()
            .unwrap_or("?");
        *by_head.entry(head).or_default() += 1;
    }
    let mut rows: Vec<(&str, usize)> = by_head.into_iter().collect();
    rows.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for (head, n) in rows.into_iter().take(top_n) {
        println!("{n:>10}  {head}");
    }
}
