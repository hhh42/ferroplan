<p align="center">
  <img src="assets/logo.svg" alt="ferroplan" width="360">
</p>

# ferroplan

[![CI](https://github.com/hhh42/ferroplan/actions/workflows/ci.yml/badge.svg)](https://github.com/hhh42/ferroplan/actions/workflows/ci.yml)
[![docs](https://img.shields.io/badge/docs-mdbook-blue)](https://hhh42.github.io/ferroplan)
[![live demo](https://img.shields.io/badge/live_demo-try_in_browser-6c5ce7)](https://hhh42.github.io/ferroplan/demo/index.html)
[![license](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)

A fast, data-parallel **PDDL planner** in Rust — a deterministic planning core for
the age of AI.

**The bet:** an LLM should be the *author and supervisor* of a planner, not its
runtime. The same reason you don't ask a model to add a column of numbers — you have
it emit code that does the arithmetic deterministically, and for free — applies one
level up: don't ask an LLM to *be* the planner for a whole village of agents. Have it
author a PDDL domain that then plans deterministically, cheaply, and inspectably at
scale, and let it only *nudge* that domain at runtime. PDDL is the auditable interface
between your intent, the model's authoring, and a fast solver — and `ferroplan` is
that solver.

**Why PDDL, not prompt-spaghetti:**

- **Cost** — a solved domain plans essentially for free; an LLM call per decision per agent does not.
- **Determinism** — same problem, same plan; you can regression-test it.
- **Inspectability** — you can read a domain and an axiom; you cannot read a model's weights.
- **Scale** — a village of agents each replanning is tractable for a fast solver, not as a wall of LLM calls.

> **[▶ Try it live in your browser](https://hhh42.github.io/ferroplan/demo/index.html)** —
> pick a built-in example or paste your own PDDL; it plans entirely client-side via
> WebAssembly, no install. There's also a
> [browser visualizer + block editor](https://hhh42.github.io/ferroplan/gui/index.html).

`ferroplan` is a from-scratch reimplementation of the FF family of planners with a
data-oriented core (bitset states, structure-of-arrays / CSR operator tables),
**enforced hill-climbing** (EHC) with a best-first fallback, parallel grounding
and parallel heuristic evaluation, plus an SGPlan-style partition-and-resolve mode,
PDDL3 preference/metric optimization, and **PDDL2.1 temporal** planning (durative
actions). It ships both a **library** (with a structured, JSON-serializable API)
and the **`ff`** command-line binary — a drop-in for Metric-FF's
`ff -o domain -f problem`.

On classical and ADL benchmarks it runs within ~1.4× of the heavily-optimized C
Metric-FF (EHC reaches goals in dozens of evaluations, not thousands); numeric
trails and IPC-5 preference quality is competitive-not-winning — see
[Benchmarks](#benchmarks).

> Status: **v0.2.2** — `ferroplan` + `ferroplan-cli` are on [crates.io](https://crates.io/crates/ferroplan). APIs may shift before 1.0.

## Features

- **EHC + best-first** — enforced hill-climbing with helpful actions (the FF
  speed default), falling back to weighted best-first when it stalls. Selectable
  per solve (`--search auto|ehc|best-first|…`).
- **FF heuristic** — delete-relaxation relaxed-plan heuristic over a
  data-oriented task, deferred evaluation, tunable `g`/`h` weights.
- **Data parallelism** — parallel grounding and parallel batch heuristic
  evaluation (`std::thread`); the plan found is identical for any thread count.
- **PDDL coverage** — STRIPS, typing, negative/disjunctive preconditions,
  numeric fluents (Metric-FF style), **ADL** (conditional effects,
  `forall`/`exists`, equality), and **derived predicates / axioms** (`:derived`,
  static/stratified — closed into the initial state via a datalog fixpoint).
- **PDDL3 preferences** — soft goal preferences (incl. `forall`-quantified and
  precondition preferences) compiled away, with anytime branch-and-bound metric
  optimization. *(Exact-optimal on small/medium instances; best-found, flagged,
  on the largest — see [Limitations](#limitations).)*
- **PDDL2.1 temporal** — `:durative-action`s with `at start`/`over all`/`at end`
  conditions & effects, **constant or parameter-dependent durations**, and
  required concurrency, via a decision-epoch forward search; output in the IPC
  temporal plan format (`t: (action) [dur]`) with a makespan.
- **SGPlan-style partitioning** — an optional partition-and-resolve mode.
- **Robust** — a published library shouldn't crash: malformed/pathological PDDL
  (incl. deeply-nested forms) returns a typed error, never a panic.
- **Structured output** — the library returns typed, `serde`-serializable
  results; the CLI emits classic FF text **or** JSON.

## GUI

[`ferroplan-bevy`](crates/ferroplan-bevy) is a Bevy app that visualizes a
domain+problem as a typed graph, animates the plan, and edits both problems and
domains in a Blockly-style block editor (`cargo run -p ferroplan-bevy`).

![ferroplan-bevy visualizing a delivery problem as a typed graph](book/src/images/graph.png)

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

# pick a mode / search strategy
ff -o domain.pddl -f problem.pddl --mode partition
ff -o domain.pddl -f problem.pddl --search best-first --weight-h 3

# temporal (durative actions) — auto-detected; prints the IPC temporal plan
ff -o temporal-domain.pddl -f problem.pddl --mode temporal

# decompose a too-big temporal goal into ordered, individually-solved contracts
# (the "LLM authors, planner decomposes" bet, made inspectable — text or --json)
ff -o temporal-domain.pddl -f problem.pddl --mode temporal --decompose

# self-contained JSON job: {"domain": "...", "problem": "...", "options": {...}}
ff --json-request job.json
```

Run `ff --help` for all flags (`--search`, `--weight-g/--weight-h`,
`--max-evaluated`, `--satisfice`, `--threads`, …).

## Library

```rust
use ferroplan::{solve, Options};

let domain  = std::fs::read_to_string("domain.pddl")?;
let problem = std::fs::read_to_string("problem.pddl")?;

// Syntax-check before solving (no grounding/solving) — fast authoring feedback.
let report = ferroplan::parse(&domain);
assert!(report.ok, "{:?}", report.error);

let solution = ferroplan::solve(&domain, &problem, &Options::default())?;
if let Some(plan) = solution.plan {
    for step in &plan.steps {
        println!("{} {}", step.action, step.args.join(" "));
    }
    println!("metric: {:?}", plan.metric);
}
# Ok::<(), ferroplan::SolveError>(())
```

The public, `serde`-serializable surface: **`solve`** (plan a domain+problem),
**`decompose`** (split a too-big temporal goal into validated contracts),
**`parse`** (syntax-check + summarize PDDL without solving), and
**`plan::validate_plan`** (independently check a plan). See
[`examples/`](crates/ferroplan/examples) for `solve`, `parse`, and `json_api`.

## Configuration

Every solver knob lives on one `Options` struct (library-first, `serde`-
serializable). The CLI flags and JSON job options map to the same fields; omitted
JSON fields fall back to the defaults shown.

```rust
ferroplan::solve(&domain, &problem, &ferroplan::Options {
    mode:            Mode::Auto,        // auto | ff | partition | pddl3 | temporal
    search:          Search::Auto,      // auto | ehc | best-first | ehc-then-best-first
    helpful_actions: true,              // helpful-action pruning (EHC)
    weight_g:        1.0,               // best-first path-length weight
    weight_h:        5.0,               // best-first heuristic weight  (1·g + 5·h)
    threads:         0,                 // 0 = auto
    max_evaluated:   None,              // search node cap
    optimize:        true,              // PDDL3: optimize metric vs. satisfice
    ..Default::default()                // every field is optional
})?;
```

CLI equivalents: `--mode`, `--search`, `--no-helpful`, `--weight-g/--weight-h`,
`--max-evaluated`, `--satisfice`, `--threads`. Via JSON:
`{"domain": "...", "problem": "...", "options": {"search": "best-first"}}`.

## Workspace layout

| crate | what |
|---|---|
| [`ferroplan`](crates/ferroplan) | the library: engine + modes + `solve` / `decompose` API |
| [`ferroplan-cli`](crates/ferroplan-cli) | the `ff` binary (clap + JSON) |
| [`ferroplan-mcp`](crates/ferroplan-mcp) | an MCP server exposing `solve` / `validate` / `decompose` over stdio — so an LLM agent can author PDDL and drive the planner |
| [`ferroplan-bevy`](crates/ferroplan-bevy) | Bevy app: visualize, inspect & animate a domain+problem (`cargo run -p ferroplan-bevy [domain.pddl problem.pddl]`) |

## Examples

[`examples/`](examples) collects worked domains that exercise the full feature set:

- [`rpg-world`](examples/rpg-world) — a ~120-action crafting/economy domain
  (durative actions, numeric resources, renewable capacities, a reachability
  axiom) with a corpus of validated contracts, a flavor-×-scale [`suite/`](examples/rpg-world/suite),
  an adversarial [`hard/`](examples/rpg-world/hard) batch, and an
  [industrial-city](examples/rpg-world/industrial-city) showcase that runs a whole
  metal/stone/wood industry as a pipeline of contracts.
- [`logistics`](examples/logistics) — transshipment: per-location goods, trucks
  with capacity, a train line.
- [`jobshop`](examples/jobshop) — scheduling with machine-exclusion (scales to 100
  concurrent jobs).
- [`BORDERS.md`](examples/BORDERS.md) — a measured map of where one-shot planning
  solves vs. where a goal must be decomposed into contracts. The **`decompose` API /
  `ff --decompose`** acts on that border: it splits a too-big temporal goal into
  ordered, individually-solved contracts and stitches them into one validated plan
  (e.g. `hard/order-8` → 8 named contracts), falling back to a monolithic solve when
  a goal can't be split.

## Benchmarks

Compared against the C **Metric-FF** and **SGPlan6** planners over a subset of
the IPC contest suites. Headline (native Metric-FF, EHC default):

| category | ferroplan solved | speed vs Metric-FF |
|---|---:|---|
| STRIPS | 40/40 | 0.71× (~1.4× slower) |
| ADL | 23/24 | 0.77× (~1.3× slower) |
| numeric | 36/40 | 0.22× |

Full results + the IPC-5 preference scoreboard: [`benchmarks/results.md`](benchmarks/results.md)
(and the [project site](https://hhh42.github.io/ferroplan)). The oracles
are not bundled (GPL / non-commercial licences) — reproduce per
[`benchmarks/COMPARING.md`](benchmarks/COMPARING.md).

**Profiling & perf tracking:** [`PROFILING.md`](PROFILING.md) — a deterministic
metrics harness (`benchmarks/perf.py run`/`compare` against a committed baseline,
so improvement/regression is measurable across machines) plus the samply /
flamegraph / criterion-baseline workflow for finding and tracking hotspots.

## Limitations

- **Numeric** trails Metric-FF: EHC's helpful-action lookahead stalls on some
  numeric domains and falls back to (complete, slower) best-first.
- **IPC-5 preferences**: compiled away + anytime branch-and-bound. Coverage is on
  par with SGPlan6 (≈39/48 on the simple-preferences suite), but on the hardest
  instances the *metric quality* trails SGPlan6's specialised partition-and-penalty
  search (best-found, flagged *not proven optimal*). Closing that gap needs the
  full ESPC penalty-coordination loop — specced in
  [`docs/espc-preferences-spec.md`](docs/espc-preferences-spec.md), not yet built.
- **Temporal**: durative actions with constant or parameter-dependent durations
  and required concurrency are supported, and **plans are VAL-validated** on real
  IPC temporal domains (44/45 produced plans valid — see
  [`benchmarks/temporal-results.md`](benchmarks/temporal-results.md)). Coverage is
  currently search-limited (the decision-epoch search times out on large
  instances). Duration *inequalities* (`(>= ?duration L)` / `(<= ?duration U)` /
  `and` ranges) are supported — the search commits to the shortest feasible
  duration — as are **timed initial literals** (`(at <time> <literal>)` in `:init`).
  Continuous (`#t`) effects are not yet supported.
- **Derived predicates** (`:derived`): static/stratified axioms are supported
  (closed into the initial state); *dynamic* derived predicates (bodies over
  changing facts) are not yet.

## Acknowledgments

This project is built in deep respect for the planners that came before it.

**SGPlan** (SGPlan5 / SGPlan6), by Chih-Wei Hsu and Benjamin W. Wah at the
University of Illinois, has been the standard to beat in this corner of automated
planning for the better part of two decades — the IPC-winning system whose
constraint-partitioning and extended-saddle-point penalty-coordination ideas still
define the state of the art for satisficing planning with preferences and with
temporal/resource constraints. I've followed that line of research for many years,
and to build something that even comes *close* to it on a slice of the benchmarks
is, genuinely, an honor. Enormous credit to that team for the depth, rigor, and
sheer durability of the work — ferroplan is in no small part an attempt to learn
from it, in Rust.

Equal thanks to Jörg Hoffmann's **FF / Metric-FF**, whose relaxed-plan heuristic
and enforced hill-climbing are the backbone of this engine; to the IPC organizers
and domain authors whose benchmarks make progress measurable; and to Derek Long and
Maria Fox's **VAL**, used here to independently validate the temporal plans.

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE),
at your option.
