//! Print a grounded op's pre/add/del rows by display substring (debug probe).
//! Usage: op_probe <domain> <problem> <substr>...
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dom = std::fs::read_to_string(&a[1]).unwrap();
    let prob = std::fs::read_to_string(&a[2]).unwrap();
    let d = ferroplan::parser::parse_domain(&dom).unwrap();
    let p = ferroplan::parser::parse_problem(&prob).unwrap();
    let c = ferroplan::temporal::compile(&d, &p);
    let task = ferroplan::ground::ground_task(&c.domain, &c.problem, 1).expect("ground");
    for pat in &a[3..] {
        for oi in 0..task.n_ops {
            if task.op_display[oi].contains(pat.as_str()) {
                let names = |s: &[u32]| {
                    s.iter()
                        .map(|&f| task.fact_names[f as usize].clone())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!(
                    "{}\n  pre: {}\n  add: {}\n  del: {}\n  cond-effs: {}",
                    task.op_display[oi],
                    names(task.pre_pos.slice(oi)),
                    names(task.add.slice(oi)),
                    names(task.del.slice(oi)),
                    task.n_cond_effs(oi)
                );
            }
        }
    }
}
