//! Output formatting. The plan block, the unsolvable/trivial messages, and the
//! exit codes match Metric-FF (and metricff) so this binary is a drop-in: the
//! planner crate's regex and the differential harness consume it unchanged. The
//! search-configuration / progress lines honestly reflect the parallel engine.

use crate::packed::PackedTask;
use crate::search::PlanResult;

fn plan_block(task: &PackedTask, ops: &[usize]) -> String {
    let mut s = String::from("\nff: found legal plan as follows\n\nstep ");
    let mut i = 0;
    for &oi in ops {
        // skip the synthetic disjunctive-goal closer (not a real domain action)
        if task.op_display[oi] == "REACH-GOAL" {
            continue;
        }
        s.push_str(&format!("{:4}: {}", i, task.op_display[oi]));
        s.push_str("\n     ");
        i += 1;
    }
    s
}

fn timing(task: &PackedTask, evaluated: usize, max_g: usize, threads: usize) -> String {
    format!(
        "\ntime spent:    0.00 seconds instantiating {} easy, {} hard action templates\n\
         \x20              0.00 seconds reachability analysis, yielding {} facts and {} actions\n\
         \x20              0.00 seconds creating final representation with {} relevant facts, {} relevant fluents\n\
         \x20              0.00 seconds computing LNF\n\
         \x20              0.00 seconds building connectivity graph\n\
         \x20              0.00 seconds searching ({} threads), evaluating {} states, to a max depth of {}\n\
         \x20              0.00 seconds total time\n\n",
        task.n_easy,
        task.n_hard,
        task.n_reach_facts,
        task.n_reach_actions,
        task.n_reach_facts,
        task.n_relevant_fluents,
        threads,
        evaluated,
        max_g,
    )
}

pub fn preamble(threads: usize) -> String {
    format!(
        "\n\nno metric specified. plan length assumed.\n\n\
         checking for cyclic := effects --- OK.\n\n\
         ff: search configuration is parallel best-first on 1*g(s) + 5*h(s) [{} threads]\n",
        threads
    )
}

pub fn render(task: &PackedTask, result: &PlanResult, threads: usize) -> (String, i32) {
    let mut out = preamble(threads);
    match result {
        PlanResult::Plan {
            ops,
            advance,
            evaluated,
            max_g,
        } => {
            if !ops.is_empty() {
                out.push('\n');
                for (i, d) in advance.iter().enumerate() {
                    if i == 0 {
                        out.push_str(&format!("advancing to distance: {:4}\n", d));
                    } else {
                        out.push_str(&format!("                       {:4}\n", d));
                    }
                }
            }
            out.push_str(&plan_block(task, ops));
            out.push('\n');
            out.push_str(&timing(task, *evaluated, *max_g, threads));
            (out, 0)
        }
        PlanResult::Unsolvable { evaluated, capped } => {
            // "proven unsolvable" only on genuine open-list exhaustion; a
            // capped search says so (0.21 Phase 3 honesty rider). Exit code
            // stays 0 either way — the Metric-FF clean-run convention.
            if *capped {
                out.push_str(
                    "\n\nsearch cap reached! no plan found within budget (search space NOT exhausted).\n\n",
                );
            } else {
                out.push_str("\n\nbest first search space empty! problem proven unsolvable.\n\n");
            }
            out.push_str(&timing(task, *evaluated, 0, threads));
            (out, 0)
        }
    }
}

#[cfg(test)]
mod tests {
    //! The capped/proven wording split (0.21 Phase 3 honesty rider): a
    //! budget-limited failure must never read as a completeness proof.

    use super::*;
    use crate::search::PlanResult;

    fn tiny_task() -> PackedTask {
        let d = crate::parser::parse_domain(
            "(define (domain t) (:predicates (p) (q))
               (:action a :precondition (p) :effect (q)))",
        )
        .unwrap();
        let p = crate::parser::parse_problem(
            "(define (problem t1) (:domain t) (:init (p)) (:goal (q)))",
        )
        .unwrap();
        crate::ground::ground_task(&d, &p, 1).unwrap()
    }

    #[test]
    fn capped_never_claims_a_proof() {
        let task = tiny_task();
        let unsolved = |capped| PlanResult::Unsolvable {
            evaluated: 7,
            capped,
        };
        let (capped_out, code) = render(&task, &unsolved(true), 1);
        assert_eq!(code, 0, "a capped run is still a clean exit");
        assert!(capped_out.contains("search cap reached"));
        assert!(!capped_out.contains("proven unsolvable"));
        let (proven_out, _) = render(&task, &unsolved(false), 1);
        assert!(proven_out.contains("proven unsolvable"));
    }
}
