//! A MOVE IS NOT SPENDING (0.30 Lane 2b, the transfer rule in
//! `heuristic::consumption_charge`).
//!
//! The charge bills what the relaxed plan decreases of a fluent its
//! preconditions read from below. settlers' `load` decreases the stock at a
//! place and the space on the vehicle by one and increases the cargo by one;
//! `unload` the reverse. Billed as spending, every hop of the chain owed a
//! raiser, and on settlers i5 the charged h read 16 (an eight-op plan of
//! loads and unloads ×2, ×3) where the uncharged plan was two builds; the
//! charged search ran 20 s unsolved at ~2k evaluations/s of fan-out
//! (`benchmarks/probes-0.30/`, the 2026-10-15 traces). Now a decrease whose
//! op increases another RESOURCE -- a fluent some op reads from below -- by
//! the same amount is a move; counters nothing waits on (`labour`,
//! `total-cost`) never pair, so consumption beside a cost is still billed.
//!
//! The fixture is the settlers shape at fixture scale: stone at a quarry, a
//! cart, a site that needs two stones. The charged search must take exactly
//! the evaluations the uncharged one takes -- nothing in this task is spent.
//! One test in this binary: the hatch is a process-global knob.

fn haul() -> (String, String) {
    let dom = "(define (domain haul)
      (:requirements :typing :numeric-fluents :action-costs)
      (:types place vehicle)
      (:predicates (at ?v - vehicle ?p - place) (road ?a - place ?b - place))
      (:functions (stone ?p - place) (cargo ?v - vehicle) (space ?v - vehicle) (labour))
      (:action load :parameters (?v - vehicle ?p - place)
        :precondition (and (at ?v ?p) (>= (stone ?p) 1) (>= (space ?v) 1))
        :effect (and (decrease (stone ?p) 1) (increase (cargo ?v) 1)
                     (decrease (space ?v) 1) (increase (labour) 1)))
      (:action unload :parameters (?v - vehicle ?p - place)
        :precondition (and (at ?v ?p) (>= (cargo ?v) 1))
        :effect (and (increase (stone ?p) 1) (decrease (cargo ?v) 1)
                     (increase (space ?v) 1) (increase (labour) 1)))
      (:action move :parameters (?v - vehicle ?a - place ?b - place)
        :precondition (and (at ?v ?a) (road ?a ?b))
        :effect (and (not (at ?v ?a)) (at ?v ?b) (increase (labour) 1))))"
        .to_string();
    let prb = "(define (problem haul2) (:domain haul)
      (:objects quarry site - place cart - vehicle)
      (:init (at cart quarry) (road quarry site) (road site quarry)
             (= (stone quarry) 3) (= (stone site) 0) (= (cargo cart) 0) (= (space cart) 1)
             (= (labour) 0))
      (:goal (>= (stone site) 2))
      (:metric minimize (labour)))"
        .to_string();
    (dom, prb)
}

fn solve(charged: bool) -> (bool, usize) {
    let (dom, prb) = haul();
    let opts = ferroplan::Options {
        threads: 1,
        max_evaluated: Some(20_000),
        ..Default::default()
    };
    if charged {
        std::env::remove_var("FF_NO_CONSUME");
    } else {
        std::env::set_var("FF_NO_CONSUME", "1");
    }
    let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
    std::env::remove_var("FF_NO_CONSUME");
    (sol.solved, sol.statistics.evaluated_states)
}

#[test]
fn a_move_between_resources_is_not_billed_as_spending() {
    let (solved, uncharged) = solve(false);
    assert!(solved, "the haul is a short plan: {uncharged} evaluations");
    let (solved, charged) = solve(true);
    assert!(
        solved,
        "charged, the haul must still be found: {charged} evaluations"
    );
    assert_eq!(
        charged, uncharged,
        "nothing in this task is spent, so the charge must change nothing: \
         {charged} evaluations charged against {uncharged}"
    );
}
