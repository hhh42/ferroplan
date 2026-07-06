//! Split a too-big temporal goal into ordered contracts — [`ferroplan::decompose`].
//!
//! Run: cargo run --release -p ferroplan --example decompose
//!
//! `order-8` is an 8-part conjunctive order in the rpg-world economy. Solved as one
//! monolithic goal it needs the on-failure escalation ladder (~minutes); handed to
//! the decomposer it is split into ordered sub-contracts — each solved and verified
//! whole — and stitched into one validated plan in a fraction of the time. This is
//! the "LLM authors the domain, the planner decomposes the goal" bet, made
//! inspectable: you get each contract's sub-goal and sub-plan back as data.
fn main() -> Result<(), String> {
    let domain = std::fs::read_to_string("examples/rpg-world/domain.pddl")
        .map_err(|e| format!("domain: {e}"))?;
    let problem = std::fs::read_to_string("examples/rpg-world/hard/order-8.pddl")
        .map_err(|e| format!("problem: {e}"))?;

    let opts = ferroplan::Options::default();
    let d = ferroplan::decompose(&domain, &problem, &opts).map_err(|e| e.to_string())?;

    if !d.solved {
        println!("unsolved. notes: {:?}", d.notes);
        return Ok(());
    }

    if d.monolithic {
        println!("goal could not be split — solved as one contract.");
    } else {
        println!("split into {} ordered contracts:\n", d.contracts.len());
        for c in &d.contracts {
            println!(
                "  #{:<2} {:<28} {:>2} steps, makespan {:>6.1} @ offset {:>6.1}",
                c.index,
                c.goal,
                c.steps.len(),
                c.makespan,
                c.offset,
            );
        }
    }

    if let Some(plan) = &d.plan {
        println!(
            "\nstitched plan: {} steps, makespan {:?}",
            plan.steps.len(),
            plan.makespan
        );
    }
    Ok(())
}
