fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dom = std::fs::read_to_string(&a[1]).unwrap();
    let prob = std::fs::read_to_string(&a[2]).unwrap();
    eprintln!("parsing domain...");
    let d = ferroplan::parser::parse_domain(&dom).unwrap();
    eprintln!(
        "domain ok: {} actions, types={:?}",
        d.actions.len(),
        d.types
    );
    eprintln!("parsing problem...");
    let p = ferroplan::parser::parse_problem(&prob).unwrap();
    eprintln!(
        "problem ok: {} objects, {} init atoms",
        p.objects.len(),
        p.init_atoms.len()
    );
    let objs = ferroplan::ground::objects_by_type(&d, &p);
    let mut sizes: Vec<(String, usize)> = objs.iter().map(|(t, o)| (t.clone(), o.len())).collect();
    sizes.sort();
    eprintln!("objects_by_type: {:?}", sizes);
    for act in &d.actions {
        eprintln!("  action {} params={:?}", act.name, act.params);
    }
    let (d, p) = ferroplan::derived::compile(&d, &p).unwrap();
    eprintln!("derived ok");
    let t = std::time::Instant::now();
    let task = ferroplan::ground::ground_task(&d, &p, 4).unwrap();
    eprintln!(
        "ground: {:?} -> {} ops, {} facts",
        t.elapsed(),
        task.n_ops,
        task.n_facts
    );
}
