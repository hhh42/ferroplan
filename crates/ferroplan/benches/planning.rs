//! End-to-end planning micro-benchmarks (parse + ground + search) over a few
//! vendored IPC problems. Run with `cargo bench`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ferroplan::{solve, Mode, Options};

fn read(rel: &str) -> String {
    let path = format!(
        "{}/../../benchmarks/ipc/{}",
        env!("CARGO_MANIFEST_DIR"),
        rel
    );
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {path}"))
}

fn read_bench(rel: &str) -> String {
    let path = format!(
        "{}/../../benchmarks/bench/{}",
        env!("CARGO_MANIFEST_DIR"),
        rel
    );
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {path}"))
}

/// Large, scale-sensitive instances so size-dependent wins (grounding,
/// per-eval heuristic cost) are measurable — small IPC problems are too fast to
/// resolve them. gripper is untyped, which stresses grounding hardest.
fn bench_large(c: &mut Criterion) {
    let dom = read("strips/gripper/domain.pddl");
    let cases = [
        ("gripper_ground_400", "gripper-ground.pddl"), // grounding-dominated
        ("gripper_search_50", "gripper-search.pddl"),  // search-dominated
    ];
    let mut g = c.benchmark_group("solve_large");
    g.sample_size(30);
    for (name, p) in cases {
        let prob = read_bench(p);
        let opts = Options {
            mode: Mode::Ff,
            threads: 1,
            ..Default::default()
        };
        g.bench_function(name, |b| {
            b.iter(|| solve(black_box(&dom), black_box(&prob), &opts).unwrap())
        });
    }
    g.finish();
}

fn bench(c: &mut Criterion) {
    let cases = [
        (
            "gripper",
            "strips/gripper/domain.pddl",
            "strips/gripper/p02.pddl",
            Mode::Ff,
        ),
        (
            "blocks",
            "strips/blocks/domain.pddl",
            "strips/blocks/p02.pddl",
            Mode::Ff,
        ),
        (
            "rovers_numeric",
            "numeric/rovers/domain.pddl",
            "numeric/rovers/p01.pddl",
            Mode::Ff,
        ),
    ];
    let mut g = c.benchmark_group("solve");
    for (name, d, p, mode) in cases {
        let dom = read(d);
        let prob = read(p);
        let opts = Options {
            mode,
            threads: 1,
            ..Default::default()
        };
        g.bench_function(name, |b| {
            b.iter(|| solve(black_box(&dom), black_box(&prob), &opts).unwrap())
        });
    }
    g.finish();
}

criterion_group!(benches, bench, bench_large);
criterion_main!(benches);
