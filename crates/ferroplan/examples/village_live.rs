//! The LIVING village (0.18 Phase 2): the tick-loop economy over
//! `benchmarks/village/` — the thing rung 3 (examples/village.rs)
//! deliberately did not build. One authoritative world `Session`; workers
//! are HIRED by goal contract and re-hired when they finish; each tick the
//! loop elapses world time, dispatches due plan steps as in-flight
//! durative starts (`apply_start`), and lets interval ends fire from
//! `elapse` itself (the 0.14 machinery). A mid-run DISRUPTION (stock
//! theft) breaks a plan and forces a rethink.
//!
//! v1 is the SIGHTED village (the bazaar's naive/claims tier, not fog):
//! the world is authoritative and fully visible, so a think is simply a
//! fresh fork of the world, restricted to the worker's own labor, with
//! the worker's contract as goal. Belief bookkeeping arrives with the fog
//! tier and the live page (Phase 3).
//!
//!   cargo run --release -p ferroplan --example village_live

use ferroplan::session::Session;
use ferroplan::{Options, Plan};

const DOM: &str = include_str!("../../../benchmarks/village/domain.pddl");
const PRB: &str = include_str!("../../../benchmarks/village/pair.pddl");

const THINK_EVALS: usize = 1_000_000;
const THINK_MB: usize = 512;
const MAX_TICKS: usize = 120;
const THEFT_TICK: usize = 17;

struct Worker {
    name: &'static str,
    filter: &'static str,
    contracts: Vec<&'static str>,
    contract: usize,
    plan: Option<Plan>,
    cursor: usize,
    clock: f64,
    thinks: usize,
    steps_applied: usize,
    rethinks_on_drift: usize,
    done_at: Option<usize>,
}

fn think(world: &Session, w: &Worker) -> (Option<Plan>, usize) {
    let mut mind = world.fork();
    let f = w.filter;
    mind.restrict_ops(move |d| d.contains(f));
    if mind.set_goal(w.contracts[w.contract]).is_err() {
        return (None, 0);
    }
    let t = mind.replan_budgeted(THINK_EVALS, Some(THINK_MB));
    let evals = t.statistics.evaluated_states;
    match (t.plan, t.solved) {
        (Some(p), true) => (Some(p), evals),
        _ => (None, evals),
    }
}

fn goal_met(world: &Session, w: &Worker) -> bool {
    let mut probe = world.fork();
    probe.set_goal(w.contracts[w.contract]).is_ok() && probe.goal_met()
}

fn main() -> Result<(), String> {
    let mut world = Session::new(DOM, PRB, &Options::default())?;
    let mut workers = vec![
        Worker {
            name: "bob",
            filter: " BOB",
            contracts: vec!["(>= (stock stool) 1)", "(>= (money) 16)"],
            contract: 0,
            plan: None,
            cursor: 0,
            clock: 0.0,
            thinks: 0,
            steps_applied: 0,
            rethinks_on_drift: 0,
            done_at: None,
        },
        Worker {
            name: "mara",
            filter: " MARA",
            contracts: vec!["(>= (stock decoy) 1)"],
            contract: 0,
            plan: None,
            cursor: 0,
            clock: 0.0,
            thinks: 0,
            steps_applied: 0,
            rethinks_on_drift: 0,
            done_at: None,
        },
    ];
    let mut events: Vec<(usize, String)> = Vec::new();
    let mut evals_total = 0usize;
    let t0 = std::time::Instant::now();

    for tick in 0..MAX_TICKS {
        // World time moves; in-flight interval ends fire here.
        let fired = world.elapse(1.0)?;
        for f in &fired {
            events.push((tick, format!("world: {f}")));
        }
        // The disruption: two planks vanish from the communal stock.
        if tick == THEFT_TICK {
            let cur = world.fluent("(stock plank)").unwrap_or(0.0);
            world.set_fluent("(stock plank)", (cur - 2.0).max(0.0))?;
            events.push((
                tick,
                format!("THEFT: planks {cur} -> {}", (cur - 2.0).max(0.0)),
            ));
        }

        for w in workers.iter_mut() {
            if w.done_at.is_some() {
                continue;
            }
            if goal_met(&world, w) {
                events.push((
                    tick,
                    format!("{}: contract `{}` DONE", w.name, w.contracts[w.contract]),
                ));
                if w.contract + 1 < w.contracts.len() {
                    w.contract += 1;
                    w.plan = None;
                    w.cursor = 0;
                    events.push((
                        tick,
                        format!("{}: RE-HIRED on `{}`", w.name, w.contracts[w.contract]),
                    ));
                } else {
                    w.done_at = Some(tick);
                    continue;
                }
            }
            // Follow if the suffix still executes on the CURRENT world —
            // validity is judged against the WORKER'S CONTRACT, so the
            // probe fork carries their goal (the world session's own goal
            // is just the problem file's and belongs to nobody).
            let valid = w.plan.as_ref().is_some_and(|p| {
                let mut probe = world.fork();
                probe.set_goal(w.contracts[w.contract]).is_ok()
                    && probe.plan_still_valid(p, w.cursor)
            });
            if !valid {
                if w.plan.is_some() {
                    w.rethinks_on_drift += 1;
                    events.push((tick, format!("{}: plan broken, rethinking", w.name)));
                }
                let (p, evals) = think(&world, w);
                w.thinks += 1;
                evals_total += evals;
                w.cursor = 0;
                w.clock = 0.0;
                if p.is_none() {
                    events.push((tick, format!("{}: contract unplannable this tick", w.name)));
                }
                w.plan = p;
            }
            // Dispatch every step whose relative time has arrived.
            if let Some(p) = w.plan.clone() {
                while w.cursor < p.steps.len() {
                    let st = &p.steps[w.cursor];
                    let due = st.time.unwrap_or(0.0) <= w.clock + 1e-9;
                    if !due {
                        break;
                    }
                    let disp = if st.args.is_empty() {
                        format!("({})", st.action)
                    } else {
                        format!("({} {})", st.action, st.args.join(" "))
                    };
                    match world.apply_start(&disp) {
                        Ok(()) => {
                            w.steps_applied += 1;
                            w.cursor += 1;
                        }
                        Err(_) => break, // not applicable yet — retry next tick
                    }
                }
            }
            w.clock += 1.0;
        }
        if workers.iter().all(|w| w.done_at.is_some()) {
            break;
        }
    }
    let wall = t0.elapsed().as_secs_f64();

    println!("# The living village — tick-loop scoreboard\n");
    println!("Generated by `cargo run --release -p ferroplan --example village_live`.");
    println!("Two workers, one communal world; hires and re-hires are goal");
    println!("contracts; theft at tick {THEFT_TICK} breaks plans mid-flight.\n");
    println!("## Events\n");
    for (t, e) in &events {
        println!("- tick {t:>3}: {e}");
    }
    println!("\n## Summary\n");
    println!(
        "| worker | contracts done | thinks | steps applied | drift rethinks | done at tick |"
    );
    println!("|---|---|---|---|---|---|");
    for w in &workers {
        println!(
            "| {} | {}/{} | {} | {} | {} | {} |",
            w.name,
            w.contract + usize::from(w.done_at.is_some()),
            w.contracts.len(),
            w.thinks,
            w.steps_applied,
            w.rethinks_on_drift,
            w.done_at.map(|t| t.to_string()).unwrap_or("—".into())
        );
    }
    println!("\ntotal think evals {evals_total}, wall {wall:.2}s for the whole run");
    Ok(())
}
