//! The factored goal-check compilation (0.22 Phase 7, the lever Wave 1's
//! decode handed over): a goal whose disjunctive items DNF-MULTIPLY
//! (block-grouping i3: 21 coordinate-Eq or-items, 4 disjuncts each, 4^21
//! conjuncts) used to compile Metric-FF style into one REACH-GOAL op per
//! WHOLE-GOAL disjunct — hopeless past a few items. Above
//! `FF_GOAL_FACTOR_THRESHOLD` (default 65536) the goal now compiles
//! FACTORED: one GOAL-ITEM-j fact per multi-disjunct item, one REACH op
//! per ITEM DISJUNCT (sum, not product), chained j-1 -> j, with a
//! PLAN-MODE fact that every real op requires and every REACH op deletes.
//! Soundness is the freeze argument: once the first REACH fires no real
//! op can, so every item is checked against the SAME final real state;
//! completeness is by construction (any goal state extends with one
//! REACH per item). Below the threshold the path is byte-identical;
//! `FF_NO_GOAL_FACTOR=1` restores the product compilation wholesale.
//!
//! ONE test fn on purpose: FF_* are process-global env knobs and this
//! file's scenarios flip them in-process.

use ferroplan::ground::{self, Outcome};
use ferroplan::parser::{parse_domain, parse_problem};

/// K goal items in the block-grouping shape: each
/// `(or (not (= (xi) (yi))) (not (= (yi) (xi))))` expands to 4 singleton
/// DNF conjuncts, so the whole-goal DNF is 4^K. Item 0 reads FALSE at
/// init (x0 = y0 = 0) and one BUMP fixes it; every other item is already
/// true — so the task is trivially solvable and the plan is exactly one
/// real step, whatever the goal compilation.
fn orchain(k: usize) -> (String, String) {
    let mut fns = String::new();
    let mut init = String::new();
    let mut goal = String::new();
    for i in 0..k {
        fns.push_str(&format!(" (x{i}) (y{i})"));
        let yv = if i == 0 { 0 } else { 1 };
        init.push_str(&format!(" (= (x{i}) 0) (= (y{i}) {yv})"));
        goal.push_str(&format!(
            " (or (not (= (x{i}) (y{i}))) (not (= (y{i}) (x{i}))))"
        ));
    }
    (
        format!(
            "(define (domain orchain) (:requirements :numeric-fluents)
               (:functions{fns})
               (:action bump :parameters () :effect (increase (x0) 1)))"
        ),
        format!("(define (problem oc) (:domain orchain) (:init{init}) (:goal (and{goal})))"),
    )
}

fn reach_ops(dom: &str, prb: &str) -> (usize, usize) {
    let d = parse_domain(dom).expect("domain parses");
    let p = parse_problem(prb).expect("problem parses");
    match ground::ground(&d, &p, 1) {
        Outcome::Task(t) => {
            let n_reach = (0..t.n_ops)
                .filter(|&oi| t.op_display[oi].starts_with("REACH-GOAL"))
                .count();
            (t.n_ops, n_reach)
        }
        _ => panic!("orchain must ground to a task"),
    }
}

fn solve(dom: &str, prb: &str) -> ferroplan::Solution {
    let opts = ferroplan::Options {
        threads: 1,
        ..Default::default()
    };
    ferroplan::solve(dom, prb, &opts).expect("solve runs")
}

#[test]
fn or_goal_product_beyond_threshold_compiles_factored() {
    for h in ["FF_NO_GOAL_FACTOR", "FF_GOAL_FACTOR_THRESHOLD"] {
        std::env::remove_var(h);
    }

    // --- THE PIN (RED before the lever): K=9 is 4^9 = 262,144 whole-goal
    // disjuncts. Factored: 9 chained items x 4 disjuncts = 36 REACH ops.
    let (dom, prb) = orchain(9);
    let (_, n_reach) = reach_ops(&dom, &prb);
    assert!(
        n_reach < 100,
        "an or-goal product beyond the threshold must compile factored \
         (sum of item disjuncts, ~36), not multiplied ({n_reach} REACH ops)"
    );

    // The factored task still SOLVES and the reported plan carries only
    // the real step (REACH ops are synthetic bookkeeping, stripped like
    // the classic REACH-GOAL closer). Soundness oracle, per item: replay
    // the STRIPPED plan on the factored task and check the ORIGINAL goal
    // semantics directly — every pair must read xi != yi in the final
    // real state. (plan::validate_plan is NOT the oracle here: replaying
    // a stripped plan against a product-compiled validator task cannot
    // reach its GOAL-REACHED artifact — a pre-existing hole shared with
    // the classic REACH-GOAL compilation, probed on the 0.21 baseline.)
    let sol = solve(&dom, &prb);
    assert!(sol.solved, "factored orchain must solve: {:?}", sol.notes);
    let plan = sol.plan.expect("plan");
    assert_eq!(plan.length, 1, "one real step; REACH ops stripped");
    assert_eq!(plan.steps[0].action, "BUMP");
    {
        let d = parse_domain(&dom).unwrap();
        let p = parse_problem(&prb).unwrap();
        let t = match ground::ground(&d, &p, 1) {
            Outcome::Task(t) => t,
            _ => panic!("factored orchain must ground"),
        };
        let bump = (0..t.n_ops)
            .find(|&oi| t.op_display[oi] == "BUMP")
            .expect("BUMP grounded");
        let s = t.apply(bump, &t.initial());
        for i in 0..9 {
            let xf = t.fluent_id(&format!("(X{i})")).expect("x kept");
            let yf = t.fluent_id(&format!("(Y{i})")).expect("y kept");
            assert!(
                (s.fv[xf] - s.fv[yf]).abs() > 1e-9,
                "item {i}: original goal must hold in the final real state"
            );
        }
    }

    // --- the hatch restores the product compilation wholesale ---------
    std::env::set_var("FF_NO_GOAL_FACTOR", "1");
    let (_, n_reach_hatched) = reach_ops(&dom, &prb);
    std::env::remove_var("FF_NO_GOAL_FACTOR");
    assert_eq!(
        n_reach_hatched,
        1 << 18,
        "FF_NO_GOAL_FACTOR must restore the 4^9 product compilation"
    );

    // --- below the threshold: byte-identical to the standing machinery -
    // K=2 is 16 disjuncts, far under 65536: same REACH-GOAL product ops,
    // same plan, same eval count, lever on or off.
    let (dom2, prb2) = orchain(2);
    let (n_ops_on, reach_on) = reach_ops(&dom2, &prb2);
    let sol_on = solve(&dom2, &prb2);
    std::env::set_var("FF_NO_GOAL_FACTOR", "1");
    let (n_ops_off, reach_off) = reach_ops(&dom2, &prb2);
    let sol_off = solve(&dom2, &prb2);
    std::env::remove_var("FF_NO_GOAL_FACTOR");
    assert_eq!((n_ops_on, reach_on), (n_ops_off, reach_off));
    assert_eq!(reach_on, 16, "below threshold keeps the product compile");
    assert!(sol_on.solved && sol_off.solved);
    let steps = |s: &ferroplan::Solution| {
        s.plan
            .as_ref()
            .unwrap()
            .steps
            .iter()
            .map(|st| (st.action.clone(), st.args.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(steps(&sol_on), steps(&sol_off), "plan must not move");
    assert_eq!(
        sol_on.statistics.evaluated_states, sol_off.statistics.evaluated_states,
        "eval count must not move below the threshold"
    );

    // --- a custom threshold routes the SAME small goal through the
    // factored compile and it still solves to the same stripped plan
    // (the sound-by-construction half exercised at toy scale) — and the
    // cross-compilation oracle agrees: replaying the stripped plan on
    // the PRODUCT-compiled task leaves some whole-goal REACH-GOAL op
    // applicable, i.e. the original disjunctive goal holds.
    std::env::set_var("FF_GOAL_FACTOR_THRESHOLD", "8");
    let (_, reach_forced) = reach_ops(&dom2, &prb2);
    let sol_forced = solve(&dom2, &prb2);
    std::env::remove_var("FF_GOAL_FACTOR_THRESHOLD");
    assert_eq!(reach_forced, 8, "2 items x 4 disjuncts, factored");
    assert!(sol_forced.solved);
    assert_eq!(steps(&sol_forced), steps(&sol_on));
    {
        let d = parse_domain(&dom2).unwrap();
        let p = parse_problem(&prb2).unwrap();
        std::env::set_var("FF_NO_GOAL_FACTOR", "1");
        let t = match ground::ground(&d, &p, 1) {
            Outcome::Task(t) => t,
            _ => panic!("product orchain must ground"),
        };
        std::env::remove_var("FF_NO_GOAL_FACTOR");
        let bump = (0..t.n_ops)
            .find(|&oi| t.op_display[oi] == "BUMP")
            .expect("BUMP grounded");
        let s = t.apply(bump, &t.initial());
        assert!(
            (0..t.n_ops)
                .any(|oi| t.op_display[oi].starts_with("REACH-GOAL") && t.op_applicable(oi, &s)),
            "the stripped plan must satisfy the original whole-goal disjunction"
        );
    }
}
