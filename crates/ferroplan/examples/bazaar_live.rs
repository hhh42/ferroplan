//! The living bazaar (0.14 Phases 1+2) — N minds, ONE world, driven
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
//! CONFLICT, counted once (the dead plan drops at break time).
//! Budgeted thinks + fixed order = the whole simulation replays
//! byte-identical at any thread count.
//!
//! Phase 2 adds CONTENTION POLICIES, all loop-side — the engine only
//! provides the mask:
//! - `Naive`: plan against the current state, ignore rivals' intents.
//! - `Claims`: before thinking, mask away trades that would TAKE an
//!   item a rival's active plan still claims (its remaining steps'
//!   receives). A mind that cannot plan under claims WAITS (claims
//!   release as rivals act) instead of burning toward dormancy.
//! - `ClaimsFollowing`: claims + broken plans rethink through
//!   `replan_following` (keep the surviving prefix, search the tail).
//!
//! Run: cargo run --release -p ferroplan --example bazaar_thinks \
//!        > benchmarks/bazaar-thinks.md
//!      cargo run --release -p ferroplan --example bazaar_live \
//!        >> benchmarks/bazaar-thinks.md
use ferroplan::{Options, Plan, Session};
use std::collections::HashSet;
use std::time::Instant;

const DOM: &str = include_str!("../../../benchmarks/bench/bazaar-chain-domain.pddl");
const PRB: &str = include_str!("../../../benchmarks/bench/bazaar-chain.pddl");
const PRB_X2M: &str = include_str!("../../../benchmarks/bench/bazaar-chain-x2m.pddl");

const THINK_EVALS: usize = 20_000;
const THINK_MB: usize = 128;
/// Consecutive failed CLAIM-FREE rethinks before a mind gives up (its goal
/// has usually become genuinely unreachable — a rival holds the rung and
/// the one-way want-edges cannot bring it back). Claim-masked failures WAIT
/// instead: the claim releases as the rival's plan drains.
const DORMANT_AFTER: usize = 3;
const TICK_CAP: usize = 200;

#[derive(Clone, Copy, PartialEq)]
enum Policy {
    Naive,
    Claims,
    ClaimsFollowing,
}

struct Mind {
    name: &'static str,
    me: String,
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
    waits: usize,
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

/// One replayable simulation event: (tick, mind, kind, detail) — the feed
/// the browser demo's canned-trace page animates (`--trace`).
type Event = (usize, String, String, String);

/// Drive one row: the named minds toward their goals in a shared world.
/// With `trace`, every think/follow/trade/conflict/verdict lands in the
/// event feed.
fn run_row(
    label: &str,
    prb: &str,
    goals: &[(&'static str, &str)],
    policy: Policy,
    mut trace: Option<&mut Vec<Event>>,
) -> Result<(), String> {
    let world = Session::new(DOM, prb, &Options::default())?;
    let mut minds: Vec<Mind> = Vec::new();
    for &(name, goal) in goals {
        let mut s = world.fork();
        let me = format!("TRADE {} ", name.to_ascii_uppercase());
        let mep = me.clone();
        s.restrict_ops(move |d| d.starts_with(&mep));
        s.set_goal(goal)?;
        minds.push(Mind {
            name,
            me,
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
            waits: 0,
        });
    }

    let t0 = Instant::now();
    let mut ticks = 0;
    let mut quiet_ticks = 0;
    for tick in 1..=TICK_CAP {
        ticks = tick;
        let mut anyone_moved = false;
        for mi in 0..minds.len() {
            // A pure STATE test — a zero-budget think would answer "could I
            // still plan," and a near-done mind must not confuse the two.
            if minds[mi].s.goal_met() && !minds[mi].done {
                minds[mi].done = true;
                if let Some(t) = trace.as_deref_mut() {
                    t.push((
                        tick,
                        minds[mi].name.into(),
                        "met".into(),
                        "goal achieved".into(),
                    ));
                }
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
                if let Some(t) = trace.as_deref_mut() {
                    t.push((
                        tick,
                        minds[mi].name.into(),
                        "follow".into(),
                        "plan holds — free suffix replay, zero search".into(),
                    ));
                }
            } else {
                // A break counts ONCE: the dead plan is dropped here, so the
                // retry thinks of a struggling mind don't re-count it. Serial
                // loop: this suffix validated when the mind last acted; only
                // rivals moved since. Exact attribution.
                let cursor = minds[mi].cursor;
                let old_plan = minds[mi].plan.take();
                let old_suffix = old_plan.as_ref().map(|p| {
                    minds[mi].conflicts += 1;
                    disp_steps(p, cursor)
                });
                if old_suffix.is_some() {
                    if let Some(t) = trace.as_deref_mut() {
                        t.push((
                            tick,
                            minds[mi].name.into(),
                            "conflict".into(),
                            "a rival's trade broke the plan".into(),
                        ));
                    }
                }
                // Claims: every item a rival's ACTIVE plan still intends to
                // receive. Masked BEFORE the think; empty under Naive.
                let claimed: HashSet<String> = if policy == Policy::Naive {
                    HashSet::new()
                } else {
                    minds
                        .iter()
                        .enumerate()
                        .filter(|&(j, m)| j != mi && !m.done)
                        .flat_map(|(_, m)| {
                            m.plan
                                .iter()
                                .flat_map(|p| p.steps[m.cursor..].iter().map(|s| s.args[3].clone()))
                        })
                        .collect()
                };
                let waiting_on_claims = !claimed.is_empty();
                {
                    let me = minds[mi].me.clone();
                    minds[mi].s.restrict_ops(move |d| {
                        d.starts_with(&me)
                            && d.split_whitespace()
                                .nth(4)
                                .map(|y| !claimed.contains(y))
                                .unwrap_or(true)
                    });
                }
                let think = match (&old_plan, policy) {
                    (Some(p), Policy::ClaimsFollowing) => {
                        minds[mi]
                            .s
                            .replan_following(p, cursor, THINK_EVALS, Some(THINK_MB))
                    }
                    _ => minds[mi].s.replan_budgeted(THINK_EVALS, Some(THINK_MB)),
                };
                minds[mi].thinks += 1;
                minds[mi].evals += think.statistics.evaluated_states;
                match think.plan {
                    Some(new) if think.solved => {
                        if let Some(old) = &old_suffix {
                            minds[mi].churn += edit_distance(old, &disp_steps(&new, 0));
                        }
                        if let Some(t) = trace.as_deref_mut() {
                            t.push((
                                tick,
                                minds[mi].name.into(),
                                "think".into(),
                                format!(
                                    "planned {} trades in {} evals",
                                    new.length, think.statistics.evaluated_states
                                ),
                            ));
                        }
                        minds[mi].plan = Some(new);
                        minds[mi].cursor = 0;
                        minds[mi].failed_thinks = 0;
                    }
                    _ => {
                        if waiting_on_claims {
                            // The blocked exchange may free up as the rival's
                            // plan drains — wait, don't march to dormancy.
                            minds[mi].waits += 1;
                            if let Some(t) = trace.as_deref_mut() {
                                t.push((
                                    tick,
                                    minds[mi].name.into(),
                                    "wait".into(),
                                    "blocked by a rival's claimed exchange — waiting".into(),
                                ));
                            }
                        } else {
                            minds[mi].failed_thinks += 1;
                            if let Some(t) = trace.as_deref_mut() {
                                let d = if minds[mi].failed_thinks >= DORMANT_AFTER {
                                    "no plan exists — giving up honestly"
                                } else {
                                    "no plan found within budget"
                                };
                                t.push((tick, minds[mi].name.into(), "stuck".into(), d.into()));
                            }
                        }
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
            if let Some(t) = trace.as_deref_mut() {
                t.push((
                    tick,
                    minds[mi].name.into(),
                    "trade".into(),
                    format!(
                        "gives {} to {}, takes {}",
                        x.to_lowercase(),
                        b.to_lowercase(),
                        y.to_lowercase()
                    ),
                ));
            }
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
        // A quiet tick is not quiescence yet: a waiting mind's claims empty
        // as rivals drain, and its next thinks then fail CLAIM-FREE toward
        // an honest give-up. A few quiet rounds let that resolve.
        quiet_ticks = if anyone_moved { 0 } else { quiet_ticks + 1 };
        if all_settled || quiet_ticks > DORMANT_AFTER {
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
    // Trace mode: the event feed is the product; no markdown.
    if let Some(t) = trace {
        for m in &minds {
            t.push((
                ticks,
                m.name.into(),
                "verdict".into(),
                format!(
                    "{} — {} follows, {} conflicts, {} thinks, {} evals",
                    outcome(m),
                    m.follows,
                    m.conflicts,
                    m.thinks,
                    m.evals
                ),
            ));
        }
        return Ok(());
    }
    println!();
    println!(
        "**{label}** — quiescent after {ticks} ticks, {:.1} ms wall: \
         {met}/{} goals met (state-verified), {} gave up or stalled.",
        ms,
        minds.len(),
        minds.len() - met
    );
    println!();
    println!(
        "| mind | goal | outcome | free follows | conflicts | thinks | evals | churn | waits |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    for (m, &(_, goal)) in minds.iter().zip(goals) {
        println!(
            "| {} | `{}` | {} | {} | {} | {} | {} | {} | {} |",
            m.name,
            goal,
            outcome(m),
            m.follows,
            m.conflicts,
            m.thinks,
            m.evals,
            m.churn,
            m.waits
        );
    }
    Ok(())
}

fn main() -> Result<(), String> {
    // --trace: emit the canned deterministic event feed for the browser
    // demo's replay page instead of the markdown scoreboard section.
    if std::env::args().any(|a| a == "--trace") {
        let x2m: &[(&str, &str)] = &[("a0", "(has a0 itemA8)"), ("a1", "(has a1 itemB8)")];
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let mut rows = Vec::new();
        for (label, policy) in [("naive", Policy::Naive), ("claims", Policy::Claims)] {
            let mut ev: Vec<Event> = Vec::new();
            run_row(label, PRB_X2M, x2m, policy, Some(&mut ev))?;
            let items: Vec<String> = ev
                .iter()
                .map(|(t, m, k, d)| format!("[{t},\"{}\",\"{}\",\"{}\"]", esc(m), esc(k), esc(d)))
                .collect();
            rows.push(format!("\"{label}\": [{}]", items.join(",")));
        }
        println!("{{{}}}", rows.join(","));
        return Ok(());
    }
    println!();
    println!("## The live loop (0.14 Phases 1+2): N minds, one world, measured");
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
    println!("last one. A mind gives up after {DORMANT_AFTER} consecutive failed CLAIM-FREE");
    println!("thinks; claim-masked failures WAIT instead (the claim releases as");
    println!("the rival's plan drains).");

    // ---- Phase 1 rows: the solo-chain fixture, naive policy ----

    // Overlapping trade-up ranges: v1 climbs 1→5, v3 climbs 3→7, v5 climbs
    // 5→9, v7 climbs 7→11 — neighbors share rungs AND raid each other's
    // starting stock. In a one-way want-edge economy these four goals are
    // NOT jointly satisfiable — this row measures mutual destruction.
    run_row(
        "Overlapping goals, naive (zero-sum by construction)",
        PRB,
        &[
            ("v1", "(has v1 item5)"),
            ("v3", "(has v3 item7)"),
            ("v5", "(has v5 item9)"),
            ("v7", "(has v7 item11)"),
        ],
        Policy::Naive,
        None,
    )?;

    // The same zero-sum set under claims: prevention cannot make an
    // unsatisfiable set satisfiable — the honest question is whether claims
    // pick DETERMINISTIC WINNERS instead of mutual destruction.
    run_row(
        "Overlapping goals, claims (zero-sum: do winners survive?)",
        PRB,
        &[
            ("v1", "(has v1 item5)"),
            ("v3", "(has v3 item7)"),
            ("v5", "(has v5 item9)"),
            ("v7", "(has v7 item11)"),
        ],
        Policy::Claims,
        None,
    )?;

    // Disjoint ranges through non-mind vendors only: the control row — the
    // same loop should show ZERO conflicts and pure follow-through.
    run_row(
        "Disjoint goals (the control)",
        PRB,
        &[
            ("v1", "(has v1 item3)"),
            ("v4", "(has v4 item6)"),
            ("v7", "(has v7 item9)"),
            ("v10", "(has v10 item11)"),
        ],
        Policy::Naive,
        None,
    )?;

    // ---- Phase 2 rows: the two-mind crossed-chain fixture ----
    // JOINTLY satisfiable (each mind can stay in its lane), yet contended:
    // every vendor stocks both chains and will hand a B-rung to an A-offer,
    // so a naive mind can raid the other's lane and strand it. This is the
    // fixture where prevention has something real to prevent.
    let x2m: &[(&str, &str)] = &[("a0", "(has a0 itemA8)"), ("a1", "(has a1 itemB8)")];
    run_row(
        "Crossed chains x2m, naive",
        PRB_X2M,
        x2m,
        Policy::Naive,
        None,
    )?;
    run_row(
        "Crossed chains x2m, claims",
        PRB_X2M,
        x2m,
        Policy::Claims,
        None,
    )?;
    run_row(
        "Crossed chains x2m, claims + follow-biased rethinks",
        PRB_X2M,
        x2m,
        Policy::ClaimsFollowing,
        None,
    )?;

    println!();
    println!("The contention cost is the difference between the rows; whatever");
    println!("it says, it ships.");
    Ok(())
}
