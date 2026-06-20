# ferroplan

A fast, data-parallel **PDDL planner** in Rust.

`ferroplan` is a from-scratch reimplementation of the FF family of planners with a
data-oriented core (bitset states, structure-of-arrays / CSR operator tables),
parallel grounding and parallel heuristic evaluation, plus an SGPlan-style
partition-and-resolve mode and PDDL3 preference/metric optimization. It ships
both a **library** (with a structured, JSON-serializable API) and the **`ff`**
command-line binary — a drop-in for Metric-FF's `ff -o domain -f problem`.

> Status: **v0.1**, not yet on crates.io. APIs may shift before 1.0.

## Features

- **FF heuristic search** — delete-relaxation relaxed-plan heuristic over a
  data-oriented task, weighted best-first (`1·g + 5·h`), deferred evaluation.
- **Data parallelism** — parallel grounding and parallel batch heuristic
  evaluation (`std::thread`); the plan found is identical for any thread count.
- **PDDL coverage** — STRIPS, typing, negative/disjunctive preconditions,
  numeric fluents (Metric-FF style), and **ADL** (conditional effects,
  `forall`/`exists`, equality).
- **PDDL3 preferences** — soft goal preferences (incl. `forall`-quantified and
  precondition preferences) compiled away, with anytime branch-and-bound metric
  optimization. *(Exact-optimal on small/medium instances; best-found, flagged,
  on the largest — see [Limitations](#limitations).)*
- **SGPlan-style partitioning** — an optional partition-and-resolve mode.
- **Structured output** — the library returns typed, `serde`-serializable
  results; the CLI emits classic FF text **or** JSON.

## Install / build

```sh
cargo build --release          # produces target/release/ff
cargo run --release --bin ff -- -o domain.pddl -f problem.pddl
```

## CLI (`ff`)

```sh
# drop-in: classic Metric-FF text output
ff -o domain.pddl -f problem.pddl

# structured JSON solution
ff -o domain.pddl -f problem.pddl --json

# pick a strategy: auto (default) | ff | partition | pddl3
ff -o domain.pddl -f problem.pddl --mode partition

# self-contained JSON job: {"domain": "...", "problem": "...", "options": {...}}
ff --json-request job.json
```

## Library

```rust
use ferroplan::{solve, Options};

let domain  = std::fs::read_to_string("domain.pddl")?;
let problem = std::fs::read_to_string("problem.pddl")?;

let solution = ferroplan::solve(&domain, &problem, &Options::default())?;
if let Some(plan) = solution.plan {
    for step in &plan.steps {
        println!("{} {}", step.action, step.args.join(" "));
    }
    println!("metric: {:?}", plan.metric);
}
# Ok::<(), ferroplan::SolveError>(())
```

See [`examples/`](crates/ferroplan/examples) for `solve` and `json_api`.

## Workspace layout

| crate | what |
|---|---|
| [`ferroplan`](crates/ferroplan) | the library: engine + modes + `solve` API |
| [`ferroplan-cli`](crates/ferroplan-cli) | the `ff` binary (clap + JSON) |

## Benchmarks

`ferroplan` is differentially tested against the C **Metric-FF** and **SGPlan6**
binaries over a curated subset of the IPC contest suites (classical, numeric,
ADL, and IPC-5 simple-preferences). See [`benchmarks/`](benchmarks) and the
[project site](https://haroldhhersey.github.io/ferroplan) for results.

## Limitations

- PDDL3 metric optimization is exact branch-and-bound; on the largest IPC-5
  instances it returns a best-found plan (flagged *not proven optimal*) rather
  than the true optimum within the time bound.
- Temporal/durative actions and derived predicates are not supported.

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE),
at your option.
