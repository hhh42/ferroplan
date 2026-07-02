//! Ground once, replan many — measure where [`ferroplan::Session`] pays.
//!
//! Run: cargo run --release -p ferroplan --example replan
//!
//! The contrast below is the point: a small per-tick contract (`errand`) is
//! grounding-dominated, so skipping the re-parse/re-ground wins big; a deep
//! planning problem (`township`) is search-dominated, so a session is merely
//! break-even. Size game/agent contracts like `errand` (the decomposer exists
//! precisely to keep them that small).
use std::time::Instant;

fn probe(name: &str, dpath: &str, ppath: &str) -> Result<(), String> {
    let d = std::fs::read_to_string(dpath).map_err(|e| e.to_string())?;
    let p = std::fs::read_to_string(ppath).map_err(|e| e.to_string())?;
    let opts = ferroplan::Options::default();

    let t = Instant::now();
    let session = ferroplan::Session::new(&d, &p, &opts)?;
    let ground = t.elapsed();

    const TICKS: u32 = 50;
    let t = Instant::now();
    for _ in 0..TICKS {
        assert!(session.replan().solved, "{name} must solve");
    }
    let per_replan = t.elapsed() / TICKS;

    let t = Instant::now();
    for _ in 0..TICKS {
        ferroplan::solve(&d, &p, &opts).map_err(|e| e.to_string())?;
    }
    let per_solve = t.elapsed() / TICKS;

    println!(
        "{name:22} ground-once {ground:>9.1?}   replan/tick {per_replan:>9.1?}   solve()/tick {per_solve:>9.1?}   speedup {:.1}x",
        per_solve.as_secs_f64() / per_replan.as_secs_f64()
    );
    Ok(())
}

fn main() -> Result<(), String> {
    probe(
        "villagers/errand",
        "examples/villagers/domain.pddl",
        "examples/villagers/errand.pddl",
    )?;
    probe(
        "villagers/township",
        "examples/villagers/domain.pddl",
        "examples/villagers/township.pddl",
    )?;
    Ok(())
}
