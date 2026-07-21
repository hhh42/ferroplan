//! The population shape (0.13 Phase 2) — a bazaar full of NPCs, ONE world.
//!
//! 0.12 answered "one agent thinking in one world" (ground once, replan
//! many). This example answers the population: `Session::fork` clones a
//! mind over the SAME grounded world — the grounded payload (operator
//! columns, names, indexes) shares behind `Arc`, each fork carries only
//! its own facts/fluents/goal — so N minds cost ONE grounding plus N
//! state views. Before 0.13, every bazaar NPC re-paid the whole world
//! load: ~2 s of grounding through a ~500 MB transient peak, retaining
//! ~40 MB — twelve times over. A memory leak with personality.
//!
//! Run: cargo run --release -p ferroplan --example many_minds
use ferroplan::{Options, Session};
use std::time::Instant;

const DOM: &str = include_str!("../../../benchmarks/bench/bazaar.pddl");
const PRB: &str = include_str!("../../../benchmarks/bench/bazaar-redistribution.pddl");

/// Resident set size in MB (Linux; 0.0 elsewhere) — coarse, but the story
/// here is GB vs MB, not KB precision.
fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| {
                l.split_whitespace()
                    .nth(1)
                    .and_then(|kb| kb.parse::<f64>().ok())
            })
        })
        .map(|kb| kb / 1024.0)
        .unwrap_or(0.0)
}

fn main() -> Result<(), String> {
    let base = rss_mb();

    // ONE grounding: the world loads once, however many minds live in it.
    let t = Instant::now();
    let world = Session::new(DOM, PRB, &Options::default())?;
    let ground_s = t.elapsed().as_secs_f64();
    let after_ground = rss_mb();
    println!(
        "world load (ground once): {ground_s:.1} s, +{:.0} MB RSS \
         (~{:.0} MB shared payload retained)",
        after_ground - base,
        world.world_bytes() as f64 / 1e6
    );

    // TWELVE minds: each fork shares the grounded payload and carries only
    // its own state view. Each NPC wants a different item — one world,
    // twelve desires.
    let t = Instant::now();
    let mut npcs: Vec<Session> = (0..12).map(|_| world.fork()).collect();
    let fork_s = t.elapsed().as_secs_f64();
    for (k, npc) in npcs.iter_mut().enumerate() {
        // items 1..=36 circulate (37..39 are unheld — wanting one is an
        // honest set_goal error, exactly as the session promises).
        let want = (k * 7) % 36 + 1;
        npc.set_goal(&format!("(has h{} item{})", k + 1, want))?;
    }
    let after_forks = rss_mb();
    println!(
        "12 forks + 12 retargets: {:.1} ms, +{:.1} MB RSS total \
         (~{:.0} KB private state per mind)",
        fork_s * 1e3,
        after_forks - after_ground,
        npcs[0].mind_bytes() as f64 / 1e3
    );

    // Every mind thinks from the same world toward its own goal.
    let t = Instant::now();
    let mut solved = 0;
    for npc in &npcs {
        let think = npc.replan_budgeted(200_000, Some(256));
        assert!(think.solved, "a depth-1 trade must solve");
        solved += think.plan.as_ref().map(|p| p.length).unwrap_or(0);
    }
    println!(
        "12 thinks: {:.2} s total, {solved} trade steps planned",
        t.elapsed().as_secs_f64()
    );

    // The pre-0.13 way, for contrast: mind #13 pays a WHOLE grounding —
    // wall time AND its own copy of the shared payload.
    let t = Instant::now();
    let independent = Session::new(DOM, PRB, &Options::default())?;
    let solo_s = t.elapsed().as_secs_f64();
    println!(
        "one INDEPENDENT session (the old way): {solo_s:.1} s and its own \
         ~{:.0} MB payload — what each of the 12 minds used to cost",
        independent.world_bytes() as f64 / 1e6
    );
    Ok(())
}
