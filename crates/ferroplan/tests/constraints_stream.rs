//! STREAMING CONSTRAINT SIMPLIFICATION (0.29 Lane 4,
//! `constraints::expand_simplified`): the static simplifier runs on each
//! member as the expansion produces it, so a statically accepted member is
//! never stored. The result must be byte-identical to expanding everything
//! and simplifying afterwards -- that is the whole contract, and it is what
//! keeps every consumer of the expanded set (the monitor compile, the
//! scorer, the verifier) reading the same thing.
//!
//! The board shape: storage-qualitative i20 expands 2.3 million preference
//! instances, 98 % of them statically accepted, and materialising every one
//! before dropping it took the process past 6 GB before any plan existed.

use ferroplan::constraints;
use ferroplan::parser::{parse_domain, parse_problem};

/// N crates, N depots; `(preference p (forall (?c ?d) (sometime (or (fixed ?d) (in ?c ?d)))))`
/// where `fixed` is static and true of most depots -- most instances are
/// accepted at production and only the live ones are kept.
fn storage_like(n: usize) -> (String, String) {
    let crates: String = (0..n).map(|i| format!(" c{i}")).collect();
    let depots: String = (0..n).map(|i| format!(" d{i}")).collect();
    // every depot but the last two is `fixed`
    let fixed: String = (0..n.saturating_sub(2))
        .map(|i| format!(" (fixed d{i})"))
        .collect();
    (
        "(define (domain st)
          (:requirements :strips :typing :constraints :preferences)
          (:types crate depot)
          (:predicates (fixed ?d - depot) (in ?c - crate ?d - depot) (free ?c - crate))
          (:action put :parameters (?c - crate ?d - depot)
            :precondition (free ?c) :effect (and (in ?c ?d) (not (free ?c)))))"
            .into(),
        format!(
            "(define (problem p) (:domain st) (:objects{crates} - crate{depots} - depot)
             (:init{fixed}{})
             (:constraints (and
               (forall (?c - crate ?d - depot)
                 (preference near (sometime (fixed ?d))))
               (forall (?c - crate) (always (or (free ?c) (in ?c d0))))))
             (:goal (and{})))",
            (0..n).map(|i| format!(" (free c{i})")).collect::<String>(),
            (0..n)
                .map(|i| format!(" (in c{i} d{})", n - 1))
                .collect::<String>()
        ),
    )
}

#[test]
fn simplifying_as_it_expands_is_byte_identical_to_simplifying_afterwards() {
    let (d, p) = storage_like(6);
    let d = parse_domain(&d).unwrap();
    let p = parse_problem(&p).unwrap();
    let mut whole = constraints::expand(&d, &p).unwrap();
    let streamed = constraints::expand_simplified(&d, &p).unwrap();
    // The plain expansion keeps every instance's every member: 6 x 6.
    assert_eq!(whole.soft.len(), 36);
    assert_eq!(whole.soft.iter().map(|(_, ms)| ms.len()).sum::<usize>(), 36);
    // The streamed one keeps the same instances (same names, same order)
    // and only the members the simplifier does not accept: `(fixed d)` is
    // static and true of four depots, so those 24 members are gone; the two
    // depots that are not fixed leave a statically FALSE `sometime`, which is
    // never dropped -- the monitors must price it.
    assert_eq!(
        streamed.soft.len(),
        36,
        "an instance is never dropped, only its members"
    );
    assert!(streamed.soft.iter().all(|(n, _)| n == "NEAR"));
    assert_eq!(
        streamed.soft.iter().map(|(_, ms)| ms.len()).sum::<usize>(),
        12
    );
    // The hard `always` is not static (free/in move): kept whole.
    assert_eq!(streamed.hard.len(), 6);
    // THE CONTRACT: byte for byte what the post-hoc pass produces.
    constraints::simplify_static(&mut whole, &d, &p);
    assert_eq!(format!("{:?}", streamed.soft), format!("{:?}", whole.soft));
    assert_eq!(format!("{:?}", streamed.hard), format!("{:?}", whole.hard));
}

/// `FF_PREF_NO_STATIC=1` keeps everything, in both forms.
#[test]
fn the_hatch_keeps_every_member() {
    if std::env::var("STREAM_CHILD").is_ok() {
        let (d, p) = storage_like(4);
        let d = parse_domain(&d).unwrap();
        let p = parse_problem(&p).unwrap();
        let streamed = constraints::expand_simplified(&d, &p).unwrap();
        println!(
            "CHILD-MEMBERS:{}",
            streamed.soft.iter().map(|(_, ms)| ms.len()).sum::<usize>()
        );
        return;
    }
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "the_hatch_keeps_every_member", "--nocapture"])
        .env("STREAM_CHILD", "1")
        .env("FF_PREF_NO_STATIC", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("CHILD-MEMBERS:16"), "{stdout}");
}
