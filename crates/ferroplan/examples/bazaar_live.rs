//! The living bazaar (0.14 Phase 1) — N minds, ONE world, driven
//! end-to-end. Emits the "live loop" section of
//! `benchmarks/bazaar-thinks.md`.
//!
//! Everything 0.12/0.13 shipped, finally composed: each mind is a
//! `fork` of one grounded world, `restrict_ops`-scoped to its OWN
//! trades (a rival's moves reach it as `set_fact` drift, never as plan
//! steps), following its plan while `plan_still_valid` says so and
//! spending a bounded think only when the world breaks it. The tick
//! loop is SERIAL with a fixed mind order, so attribution is exact: a
//! mind's suffix validated when it last acted, therefore any break
//! found at its next turn was caused by rival trades in between — a
//! CONFLICT, counted as such. Budgeted thinks + fixed order = the whole
//! simulation replays byte-identical at any thread count.
//!
//! Two rows, same world: OVERLAPPING goals (trade-up ranges sharing
//! rungs — contention by construction) and DISJOINT goals (the
//! control). Whatever the loop shows, it ships.
//!
//! Run: cargo run --release -p ferroplan --example bazaar_thinks \
//!        > benchmarks/bazaar-thinks.md
//!      cargo run --release -p ferroplan --example bazaar_live \
//!        >> benchmarks/bazaar-thinks.md
use ferroplan::{Options, Plan, Session};
use std::time::Instant;

const DOM: &str = include_str!("../../../benchmarks/bench/bazaar-chain-domain.pddl");
const PRB: &str = include_str!("../../../benchmarks/bench/bazaar-chain.pddl");

const THINK_EVALS: usize = 20_000;
const THINK_MB: usize = 128;
/// Consecutive failed rethinks before a mind gives up (its goal has
/// usually become genuinely unreachable — a rival holds the rung and the
/// one-way want-edges cannot bring it back).
const DORMANT_AFTER: usize = 3;
const TICK_CAP: usize = 200;

struct Mind {
    name: &'static str,
    s: Session,
    plan: Option<Plan>,
    cursor: usize,
    done: bool,
    failed_thinks: usize,
    // metrics
    follows: usize,
    conflicts: usize,
    thinks: usize,
    evals: usize,
    churn: usize,
}

fn edit_distance(a: &[String], b: &[String]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, sa) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, sb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(sa != sb);
            cur.push(sub.min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

fn disp_steps(p: &Plan, from: usize) -> Vec<String> {
    p.steps[from..]
        .iter()
        .map(|s| format!("{} {}", s.action, s.args.join(" ")))
        .collect()
}

/// Drive one row: the named minds toward their goals in a shared world.
fn run_row(label: &str, goals: &[(&'static str, &str)]) -> Result<(), String> {
    let world = Session::new(DOM, PRB, &Options::default())?;
    let mut minds: Vec<Mind> = Vec::new();
    for &(name, goal) in goals {
        let mut s = world.fork();
        let me = format!("TRADE {} ", name.to_ascii_uppercase());
        s.restrict_ops(|d| d.starts_with(&me));
        s.set_goal(goal)?;
        minds.push(Mind {
            name,
            s,
            plan: None,
            cursor: 0,
            done: false,
            failed_thinks: 0,
            follows: 0,
            conflicts: 0,
            thinks: 0,
            evals: 0,
            churn: 0,
        });
    }

    let t0 = Instant::now();
    let mut ticks = 0;
    for tick in 1..=TICK_CAP {
        ticks = tick;
        let mut anyone_moved = false;
        for mi in 0..minds.len() {
            // A pure STATE test — a zero-budget think would answer "could I
            // still plan," and a near-done mind must not confuse the two.
            if minds[mi].s.goal_met() {
                minds[mi].done = true;
            }
            if minds[mi].done || minds[mi].failed_thinks >= DORMANT_AFTER {
                continue;
            }
            // Follow if the remaining suffix still executes and ends in the
            // goal (free — a replay, not a think).
            let valid = minds[mi]
                .plan
                .as_ref()
                .is_some_and(|p| minds[mi].s.plan_still_valid(p, minds[mi].cursor));
            if valid {
                minds[mi].follows += 1;
            } else {
                // A break counts ONCE: the dead plan is dropped here, so the
                // retry thinks of a struggling mind don't re-count it. Serial
                // loop: this suffix validated when the mind last acted; only
                // rivals moved since. Exact attribution.
                let old_suffix = minds[mi].plan.take().map(|p| {
                    minds[mi].conflicts += 1;
                    disp_steps(&p, minds[mi].cursor)
                });
                let think = minds[mi].s.replan_budgeted(THINK_EVALS, Some(THINK_MB));
                minds[mi].thinks += 1;
                minds[mi].evals += think.statistics.evaluated_states;
                match think.plan {
                    Some(new) if think.solved => {
                        if let Some(old) = &old_suffix {
                            minds[mi].churn += edit_distance(old, &disp_steps(&new, 0));
                        }
                        minds[mi].plan = Some(new);
                        minds[mi].cursor = 0;
                        minds[mi].failed_thinks = 0;
                    }
                    _ => {
                        minds[mi].failed_thinks += 1;
                        continue;
                    }
                }
            }
            // Execute the next step of the (now valid) plan: mutate the ONE
            // world by mirroring the trade into every mind's view.
            let step = {
                let p = minds[mi].plan.as_ref().unwrap();
                if minds[mi].cursor >= p.steps.len() {
                    continue; // plan drained; the state probe rules next turn
                }
                p.steps[minds[mi].cursor].clone()
            };
            let (a, b, x, y) = (&step.args[0], &step.args[1], &step.args[2], &step.args[3]);
            for m in minds.iter_mut() {
                m.s.set_fact(&format!("(has {a} {x})"), false)?;
                m.s.set_fact(&format!("(has {b} {y})"), false)?;
                m.s.set_fact(&format!("(has {a} {y})"), true)?;
                m.s.set_fact(&format!("(has {b} {x})"), true)?;
            }
            minds[mi].cursor += 1;
            anyone_moved = true;
        }
        let all_settled = minds
            .iter()
            .all(|m| m.done || m.failed_thinks >= DORMANT_AFTER);
        if all_settled || !anyone_moved {
            break;
        }
    }
    let ms = t0.elapsed().as_secs_f64() * 1e3;

    let met = minds.iter().filter(|m| m.done).count();
    let outcome = |m: &Mind| {
        if m.done {
            "MET"
        } else if m.failed_thinks >= DORMANT_AFTER {
            "gave up"
        } else {
            "stalled"
        }
    };
    println!();
    println!(
        "**{label}** — quiescent after {ticks} ticks, {:.1} ms wall: \
         {met}/{} goals met (state-verified), {} gave up or stalled.",
        ms,
        minds.len(),
        minds.len() - met
    );
    println!();
    println!("| mind | goal | outcome | free follows | conflicts | thinks | evals | churn |");
    println!("|---|---|---|---|---|---|---|---|");
    for (m, &(_, goal)) in minds.iter().zip(goals) {
        println!(
            "| {} | `{}` | {} | {} | {} | {} | {} | {} |",
            m.name,
            goal,
            outcome(m),
            m.follows,
            m.conflicts,
            m.thinks,
            m.evals,
            m.churn
        );
    }
    Ok(())
}

fn main() -> Result<(), String> {
    println!();
    println!("## The live loop (0.14 Phase 1): N minds, one world, measured");
    println!();
    println!("Generated by `cargo run --release -p ferroplan --example bazaar_live`.");
    println!("Serial tick loop over the wants-gated bazaar: each mind is an");
    println!("actor-scoped fork (`restrict_ops` — it plans only its OWN trades),");
    println!("follows its plan while the free suffix replay holds, and spends a");
    println!(
        "bounded think ({} evals / {} MB) only when the world broke it.",
        THINK_EVALS, THINK_MB
    );
    println!("Serial order makes conflict attribution EXACT: a break found at a");
    println!("mind's turn can only have been caused by rival trades since its");
    println!("last one. A mind gives up after {DORMANT_AFTER} consecutive failed thinks");
    println!("(one-way want-edges: a lost rung usually cannot come back).");

    // Overlapping trade-up ranges: v1 climbs 1→5, v3 climbs 3→7, v5 climbs
    // 5→9, v7 climbs 7→11 — neighbors share rungs AND raid each other's
    // starting stock (v3's route trades WITH mind v5, etc.).
    run_row(
        "Overlapping goals (contention by construction)",
        &[
            ("v1", "(has v1 item5)"),
            ("v3", "(has v3 item7)"),
            ("v5", "(has v5 item9)"),
            ("v7", "(has v7 item11)"),
        ],
    )?;

    // Disjoint ranges through non-mind vendors only: the control row — the
    // same loop should show ZERO conflicts and pure follow-through.
    run_row(
        "Disjoint goals (the control)",
        &[
            ("v1", "(has v1 item3)"),
            ("v4", "(has v4 item6)"),
            ("v7", "(has v7 item9)"),
            ("v10", "(has v10 item11)"),
        ],
    )?;

    println!();
    println!("The contention cost is the difference between the rows; whatever");
    println!("it says, it ships (Phase 2 works on making it livable).");
    Ok(())
}
