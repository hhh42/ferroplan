# Profiling & performance tracking

Two complementary tools: a **metrics harness** to track whether changes improve or
regress the planner (over time, across machines), and **sampling profilers** to
find the hotspots to optimize.

## 1. Track improvement/regression — `benchmarks/perf.py`

The harness records **deterministic** metrics that compare cleanly across runs and
machines, so progress is real and not measurement noise:

| metric | meaning | deterministic? |
|---|---|---|
| coverage | problems solved | ✅ |
| `evaluated` | states expanded by the search | ✅ — the primary search-efficiency metric |
| `length` / `metric` | plan length / PDDL3 cost | ✅ — solution quality |
| `ms` | wall-clock | ❌ machine/load-dependent — profiling only, never the verdict |

```sh
cargo build --release

# measure the current corpus -> a metrics file
FF=target/release/ff python3 benchmarks/perf.py run \
    --corpus benchmarks/ipc --label "what I changed" --out /tmp/now.json

# compare against the committed baseline (exits non-zero on a regression)
python3 benchmarks/perf.py compare benchmarks/metrics/baseline.json /tmp/now.json
```

`compare` flags, per problem and in aggregate: coverage lost/gained, more/fewer
states evaluated, worse/better plans. The verdict ignores `ms` (noisy). Point
`--corpus` at the larger external IPC set (see `COMPARING.md`) for a stronger
signal than the small vendored subset.

### The committed baseline

`benchmarks/metrics/baseline.json` is the reference point. The optimization loop:

1. `perf.py run --out /tmp/before.json` (or just use the committed baseline).
2. Make a change.
3. `perf.py compare benchmarks/metrics/baseline.json /tmp/after.json` — confirm
   fewer evaluated states / better coverage and **no regression**.
4. When an improvement lands, **refresh the baseline** (`perf.py run --out
   benchmarks/metrics/baseline.json`, commit it) so the gain is locked in and
   future work is measured against the new bar. Commit baselines alongside the
   change that moved them — the file's git history *is* the performance log.

## 2. Find hotspots — samply (recommended on macOS)

A profiling build is release-optimized **with debug symbols**:

```sh
cargo build --profile profiling -p ferroplan-cli      # -> target/profiling/ff

# sample a run that takes long enough to profile (>~1s); opens the Firefox Profiler
samply record -- target/profiling/ff \
    -o <domain.pddl> -f <hard-problem.pddl> --threads 1
```

Pick a workload with real work — a large numeric/ADL instance or a PDDL3 metric
problem (`--mode pddl3`); trivial problems finish before the sampler gets samples.
Use `--threads 1` so the profile reflects single-core hotspots, not scheduling.

`samply record --save-only -o profile.json.gz -- …` saves a profile without opening
a browser (for CI/headless capture).

### Turnkey text hotspots — `benchmarks/profile.py`

For a quick, headless, no-browser hotspot list (builds the profiling binary, runs
samply `--save-only`, symbolicates the hottest addresses with `atos`):

```sh
python3 benchmarks/profile.py <domain.pddl> <problem.pddl> -- --mode pddl3
# -> "NN.N%  function" lines, hottest self-time first
```

> First measured run (openstacks p01, `--mode pddl3`) was *grounding-bound*:
> ~15% string formatting + ~6% `HashMap::insert` (interning) + float formatting,
> with the search heuristic <2% — i.e. building per-op/fact display strings and
> interning during grounding dominate object-heavy instances. Profile a
> search-heavy instance (many `evaluated` states) to see the search hotspots
> instead.

### Linux / flamegraph alternative

```sh
cargo install flamegraph
cargo flamegraph --profile profiling --bin ff -- -o domain.pddl -f problem.pddl
```

## 3. Micro-benchmarks — criterion baselines

`crates/ferroplan/benches/planning.rs` times parse+ground+search on a few problems
with statistical rigor. Use named baselines to catch micro-regressions:

```sh
cargo bench -- --save-baseline main      # record a reference
# …change code…
cargo bench -- --baseline main           # report % change vs the reference
```

CI already gates `cargo bench --no-run` so the benchmarks never bit-rot.

## What to optimize, in order

`evaluated` states dominate runtime, so the highest-leverage wins reduce them
(better heuristic guidance, helpful-action quality, dead-end detection) — and they
show up deterministically in `perf.py compare`. Only after that does per-evaluation
cost (the samply hotspots: relaxed-plan construction, state hashing, apply) matter.
Always confirm a wall-time win with `perf.py compare` so you know it's a real
algorithmic gain, not noise.
