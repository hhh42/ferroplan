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

criterion_group!(benches, bench);
criterion_main!(benches);
