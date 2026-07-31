//! PPDDL parse/normalization and explicit-MDP policy benchmarks.

use criterion::{criterion_group, criterion_main, Criterion};
use ferroplan::{parse_ppddl, solve_ppddl, ProbabilisticOptions};
use std::hint::black_box;

const DOMAIN: &str = r#"
(define (domain retry)
  (:requirements :strips :negative-preconditions :probabilistic-effects)
  (:predicates (done))
  (:action attempt
    :parameters ()
    :precondition (not (done))
    :effect (probabilistic 0.25 (done))))
"#;

const PROBLEM: &str = r#"
(define (problem retry-p)
  (:domain retry)
  (:init)
  (:goal (done)))
"#;

fn bench_ppddl(c: &mut Criterion) {
    c.bench_function("ppddl_parse_normalize", |b| {
        b.iter(|| parse_ppddl(black_box(DOMAIN), black_box(PROBLEM)))
    });

    let options = ProbabilisticOptions {
        horizon: Some(64),
        threads: 1,
        ..Default::default()
    };
    c.bench_function("ppddl_explicit_mdp_policy", |b| {
        b.iter(|| {
            solve_ppddl(black_box(DOMAIN), black_box(PROBLEM), black_box(&options)).unwrap()
        })
    });
}

criterion_group!(benches, bench_ppddl);
criterion_main!(benches);
