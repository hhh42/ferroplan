//! The village hire story (0.17 Phase 4, rung 3): HIRING IS A GOAL
//! CONTRACT, not a domain action. The game loop hires a worker by giving
//! that worker's Session a goal; the planner plans one mind's labor; a
//! re-hire is `set_goal` again on the same mind. This example
//! demonstrates the whole mechanism on the village domain — two workers,
//! one shared world, each scoped to their OWN actions (`restrict_ops`,
//! the bazaar pattern) — without a tick simulation: the live-loop
//! village rides with the village page (severed to the next cycle,
//! docs/roadmap-0.17.md Phase 5 note).
//!
//!   cargo run --release -p ferroplan --example village

use ferroplan::session::Session;
use ferroplan::{Options, ThinkBudget, ThinkVerdict};

const DOM: &str = include_str!("../../../benchmarks/village/domain.pddl");
const PRB: &str = include_str!("../../../benchmarks/village/pair.pddl");

// A hire think is a real think: the communal village exhibits the
// start-credit plateau in miniature (h^FF pays for each GATHER start on
// firing, so gather-spam floods the pruned pass — the same mechanism as
// the TMS wall, docs/roadmap-0.15.md, now with a game-shaped witness).
// 1M evals rides past it; the h-surgery fence is the real fix.
//
// 0.24 Phase 5 re-measure knob: `THINK_EVALS=<n>` overrides the budget so
// the pair contract can be re-priced per engine (the 0.21 read: 1M sails,
// 200k dies — and the 200k death is a CAPPED verdict, which is why `hire`
// rides the budget-stamped think below instead of calling a starved
// contract "unplannable". The 0.24 re-measure receipts ride the phase's
// commit record; the tick-loop numbers live in benchmarks/village-live.md).
const THINK_EVALS: usize = 1_000_000;
const THINK_MB: usize = 512;

fn think_evals() -> usize {
    std::env::var("THINK_EVALS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(THINK_EVALS)
}

fn hire(name: &str, s: &mut Session, contract: &str) -> Result<usize, String> {
    s.set_goal(contract)?;
    // The budget-stamped think (0.24 Phase 5): the verdict tells the game
    // loop the TRUTH about a failed hire — a capped think is a budget
    // problem, not an impossible contract.
    let think = s.think(&ThinkBudget {
        max_evaluated: Some(think_evals()),
        memory_mb: Some(THINK_MB),
        ..Default::default()
    });
    let plan = match (&think.solution.plan, think.solution.solved) {
        (Some(p), true) => p.clone(),
        _ => {
            return Err(match think.verdict {
                ThinkVerdict::Capped => format!(
                    "{name}: contract {contract} CAPPED at {} evals / {} ms — \
                     not proven unreachable; raise THINK_EVALS",
                    think.spent_evals, think.spent_ms
                ),
                _ => format!("{name}: contract {contract} unplannable (search exhausted)"),
            })
        }
    };
    println!(
        "hired {name} on `{contract}` -> {} steps ({} evals, {} ms):",
        plan.steps.len(),
        think.spent_evals,
        think.spent_ms
    );
    for st in &plan.steps {
        println!(
            "   {:>7.3}  {} {}",
            st.time.unwrap_or(0.0),
            st.action,
            st.args.join(" ")
        );
    }
    Ok(plan.steps.len())
}

fn main() -> Result<(), String> {
    let world = Session::new(DOM, PRB, &Options::default())?;

    // Each worker's mind: a fork of the ONE grounded world, restricted to
    // ops that mention that worker — it plans only its own labor.
    let mut bob = world.fork();
    bob.restrict_ops(|d| d.contains(" BOB"));
    let mut mara = world.fork();
    mara.restrict_ops(|d| d.contains(" MARA"));

    // The loop hires: two contracts, two independent plans over the ONE
    // communal economy (per-worker tills are a catalog evolution for the
    // game project; the mechanism is identical).
    let a = hire("bob", &mut bob, "(>= (stock stool) 1)")?;
    let b = hire("mara", &mut mara, "(>= (stock decoy) 1)")?;

    // A RE-hire is just another contract on the same mind.
    let c = hire("bob (re-hired)", &mut bob, "(>= (stock plank) 3)")?;

    println!("\nsummary: bob {a} + {c} steps across two contracts, mara {b} steps — one world, zero new machinery");
    Ok(())
}
