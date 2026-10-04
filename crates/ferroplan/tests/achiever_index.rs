//! The achiever index on a monitored task (0.29 Lane 4). The shared monitor
//! block's adds belong to EVERY monitored op, and `add_by_fact` used to carry
//! one entry per op per monitor add -- ops x monitors wide, hundreds of
//! millions of `u32` on storage-complex, the `mem-cap` class there. The
//! index now holds each op's OWN adds, and `PackedTask::achievers` merges
//! in every monitored op for a fact the block adds: the same set an op scan
//! gives, each op once, ascending, and a table whose size is the own adds'.

use ferroplan::constraints;
use ferroplan::ground;
use ferroplan::parser::{parse_domain, parse_problem};
use std::collections::BTreeSet;

const DOM: &str = "(define (domain lamp)
  (:requirements :strips :constraints)
  (:predicates (off) (on) (lamp) (a) (b))
  (:action turn-on :parameters () :precondition (off) :effect (and (on) (not (off))))
  (:action turn-off :parameters () :precondition (on) :effect (and (off) (not (on))))
  (:action light :parameters () :precondition (on) :effect (lamp))
  (:action toggle :parameters () :precondition (b) :effect (and (a) (not (b)))))";

const PRB: &str = "(define (problem p) (:domain lamp)
  (:init (off) (b))
  (:goal (and (lamp) (a)))
  (:constraints (and (sometime (a)) (at-most-once (on)) (sometime-before (lamp) (a)))))";

#[test]
fn achievers_are_the_op_scan_and_the_table_is_the_own_adds() {
    let d = parse_domain(DOM).expect("domain");
    let p = parse_problem(PRB).expect("problem");
    let (d2, p2) = constraints::gate(&d, &p)
        .expect("the constraints compile")
        .expect("there are constraints, so a compiled pair");
    let task = ground::ground_task(&d2, &p2, 1).expect("grounds");
    assert!(
        task.monitored.iter().any(|&m| m),
        "the fixture must carry a monitor block"
    );
    assert!(
        !task.shared_cond.is_empty(),
        "the monitor transitions are the shared block"
    );
    let mut own_adds = 0usize;
    for f in 0..task.n_facts {
        // The oracle: every op whose own adds, own conditional adds, or
        // (when monitored) the shared block's adds include `f`.
        let mut oracle: BTreeSet<u32> = BTreeSet::new();
        for oi in 0..task.n_ops {
            let own = task.add.slice(oi).contains(&(f as u32))
                || task
                    .cond
                    .slice(oi)
                    .iter()
                    .any(|ce| ce.add.contains(&(f as u32)));
            let shared = task.monitored[oi]
                && task
                    .shared_cond
                    .iter()
                    .any(|ce| ce.add.contains(&(f as u32)));
            if own || shared {
                oracle.insert(oi as u32);
            }
        }
        let got: Vec<u32> = task.achievers(f).collect();
        let as_set: BTreeSet<u32> = got.iter().copied().collect();
        assert_eq!(as_set, oracle, "fact {f}: {:?}", task.fact_names.get(f));
        assert_eq!(got.len(), as_set.len(), "fact {f}: each op once");
        assert!(got.windows(2).all(|w| w[0] < w[1]), "fact {f}: ascending");
        own_adds += task.add_by_fact.slice(f).len();
    }
    // The table holds the own adds and nothing per monitor: every row entry
    // is an op whose OWN effects add the fact.
    for f in 0..task.n_facts {
        for &oi in task.add_by_fact.slice(f) {
            let oi = oi as usize;
            assert!(
                task.add.slice(oi).contains(&(f as u32))
                    || task
                        .cond
                        .slice(oi)
                        .iter()
                        .any(|ce| ce.add.contains(&(f as u32))),
                "fact {f}: op {oi} is in the table without an own add"
            );
        }
    }
    assert_eq!(task.add_by_fact.flat.len(), own_adds);
    // ... and the shared facts really route to every monitored op.
    let shared_fact = (0..task.n_facts)
        .find(|&f| task.shared_add[f])
        .expect("the block adds something");
    let monitored: Vec<u32> = (0..task.n_ops)
        .filter(|&oi| task.monitored[oi])
        .map(|oi| oi as u32)
        .collect();
    for oi in &monitored {
        assert!(task.achievers(shared_fact).any(|a| a == *oi));
    }
    assert_eq!(&task.monitored_ops[..], &monitored[..]);
}

/// A task without constraints has no block: the walk IS the table's row.
#[test]
fn without_a_monitor_block_the_walk_is_the_table() {
    let d = parse_domain(DOM).expect("domain");
    let p = parse_problem(
        "(define (problem q) (:domain lamp) (:init (off) (b)) (:goal (and (lamp) (a))))",
    )
    .expect("problem");
    let task = ground::ground_task(&d, &p, 1).expect("grounds");
    assert!(task.monitored_ops.is_empty());
    for f in 0..task.n_facts {
        let walk: Vec<u32> = task.achievers(f).collect();
        assert_eq!(walk, task.add_by_fact.slice(f).to_vec(), "fact {f}");
    }
}
