//! Use ferroplan as a library and serialize the structured result to JSON —
//! the shape a web service or another tool would consume.
//!
//! ```text
//! cargo run --example json_api
//! ```

use ferroplan::{solve, Mode, Options};

const DOMAIN: &str = "(define (domain gripper)
 (:requirements :strips :typing)
 (:types room ball gripper)
 (:predicates (at-robby ?r - room) (at ?b - ball ?r - room)
              (free ?g - gripper) (carry ?b - ball ?g - gripper))
 (:action move :parameters (?from ?to - room)
   :precondition (at-robby ?from) :effect (and (at-robby ?to) (not (at-robby ?from))))
 (:action pick :parameters (?b - ball ?r - room ?g - gripper)
   :precondition (and (at ?b ?r) (at-robby ?r) (free ?g))
   :effect (and (carry ?b ?g) (not (at ?b ?r)) (not (free ?g))))
 (:action drop :parameters (?b - ball ?r - room ?g - gripper)
   :precondition (and (carry ?b ?g) (at-robby ?r))
   :effect (and (at ?b ?r) (free ?g) (not (carry ?b ?g)))))";

const PROBLEM: &str = "(define (problem g1) (:domain gripper)
 (:objects rooma roomb - room  ball1 ball2 - ball  left right - gripper)
 (:init (at-robby rooma) (free left) (free right)
        (at ball1 rooma) (at ball2 rooma))
 (:goal (and (at ball1 roomb) (at ball2 roomb))))";

fn main() {
    let opts = Options {
        mode: Mode::Auto,
        threads: 0,
    };
    let solution = solve(DOMAIN, PROBLEM, &opts).expect("solve");

    // the whole Solution is serde-serializable
    let json = serde_json::to_string_pretty(&solution).unwrap();
    println!("{json}");

    // ...and it round-trips back into typed structures
    let back: ferroplan::Solution = serde_json::from_str(&json).unwrap();
    assert_eq!(back.solved, solution.solved);
}
