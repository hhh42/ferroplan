//! Grounding-only probe: gate + ground an instance, print task dimensions
//! and peak RSS. Usage: ground_probe <domain> <problem>
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dom = std::fs::read_to_string(&a[1]).unwrap();
    let prob = std::fs::read_to_string(&a[2]).unwrap();
    let d = ferroplan::parser::parse_domain(&dom).unwrap();
    let p = ferroplan::parser::parse_problem(&prob).unwrap();
    let (d, p) = ferroplan::derived::compile(&d, &p).unwrap();
    let t0 = std::time::Instant::now();
    let (d, p) = match ferroplan::constraints::gate(&d, &p).unwrap() {
        Some(pair) => pair,
        None => (d, p),
    };
    let gate_ms = t0.elapsed().as_millis();
    let c = ferroplan::pddl3::compile(&d, &p);
    let t1 = std::time::Instant::now();
    let task = ferroplan::ground::ground_task(&c.domain, &c.problem, 1).expect("ground");
    let ground_ms = t1.elapsed().as_millis();
    let cond: usize = (0..task.n_ops).map(|oi| task.n_cond_effs(oi)).sum();
    let rss_kb: usize = std::fs::read_to_string("/proc/self/status").unwrap()
        .lines().find(|l| l.starts_with("VmHWM")).unwrap()
        .split_whitespace().nth(1).unwrap().parse().unwrap();
    println!(
        "gate {gate_ms} ms; ground {ground_ms} ms; {} ops, {} facts, shared_cond {}, \
         effective cond effs {}, peak RSS {} MB",
        task.n_ops, task.n_facts, task.shared_cond.len(), cond, rss_kb / 1024
    );
}
