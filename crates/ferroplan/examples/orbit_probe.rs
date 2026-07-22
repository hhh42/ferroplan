//! Orbit probe: snap-compile + ground an instance, run object-symmetry
//! orbit detection (0.14 ext Phase 10), and print what it found — how many
//! orbits, how many interchangeable members each, template sizes, and the
//! theoretical visited-space divisor (∏ k! over orbits). The measurement
//! eyes for the symmetry lever: run this across a corpus BEFORE crediting
//! the reducer with anything.
//! Usage: orbit_probe <domain> <problem>
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dom = std::fs::read_to_string(&a[1]).unwrap();
    let prob = std::fs::read_to_string(&a[2]).unwrap();
    let d = ferroplan::parser::parse_domain(&dom).unwrap();
    let p = ferroplan::parser::parse_problem(&prob).unwrap();
    let c = ferroplan::temporal::compile(&d, &p);
    let task = match ferroplan::ground::ground_stratified(&c.domain, &c.problem, 1) {
        ferroplan::ground::Outcome::Task(t) => t,
        _ => {
            println!("ground failed");
            return;
        }
    };
    let t0 = std::time::Instant::now();
    let om = ferroplan::orbits::detect(&c.domain, &c.problem, &task);
    let detect_us = t0.elapsed().as_micros();
    let Some(om) = om else {
        println!("orbits none  detect {detect_us}us");
        return;
    };
    // ∏ k! saturating — the upper bound on visited-state collapse.
    let mut divisor: f64 = 1.0;
    for o in &om.orbits {
        for i in 2..=o.facts.len() {
            divisor *= i as f64;
        }
    }
    println!(
        "orbits {}  divisor {divisor:.0}  detect {detect_us}us",
        om.orbits.len()
    );
    for (i, o) in om.orbits.iter().enumerate() {
        // Label each member by its first template fact — shows the objects.
        let labels: Vec<&str> = o
            .facts
            .iter()
            .map(|m| {
                m.first()
                    .map(|&f| task.fact_names[f as usize].as_str())
                    .unwrap_or("(no-facts)")
            })
            .collect();
        println!(
            "  orbit {i}: {} members, template {} facts / {} fluents / {} ops",
            o.facts.len(),
            o.facts.first().map(|v| v.len()).unwrap_or(0),
            o.fluent_slots.first().map(|v| v.len()).unwrap_or(0),
            o.ops.first().map(|v| v.len()).unwrap_or(0),
        );
        for l in labels.iter().take(6) {
            println!("    {l}");
        }
        if labels.len() > 6 {
            println!("    ... {} more", labels.len() - 6);
        }
    }
}
