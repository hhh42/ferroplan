# Encoding A/B/C benchmark — does a *generic* PDDL encoding search faster than an *action-specific* one?

The same crafting world can be modeled two ways. This benchmark answers, with
controlled measurements, **which one lets ferroplan search faster** — and where the
crossover is.

- **action-specific** — one `:action` per recipe (`craft_0`, `craft_1`, …). The
  "chop / mine / smelt" style: every verb is its own schema. (cf. the durative
  `examples/rpg`, `examples/village`, `examples/rpg-world`.)
- **generic / data-driven** — push the per-verb variation into data so a single
  action covers every recipe. The "use-item over constants" style (cf. the original
  `Prohibited` domain's `consume` action over `(consumable ?type ?verb)`). Two flavors:
  - **data-table** — `(:action craft ?rec ?in ?out)` gated by a static `(recipe ?rec
    ?in ?out)` table; recipes/resources are `:constants`. Ground actions stay thin
    (1 input → 1 output).
  - **forall-numeric** — `(:action craft ?rec)` that quantifies over *all* resources
    via per-recipe `(need ?rec ?res)` / `(make ?rec ?res)` quantity functions. One
    fat ADL operator; handles multi-input recipes natively.

The trick to a *fair* test: all three encodings model the **same content** and run in
the **same planner mode**, so the only variable is the encoding. (Comparing the
domains that already exist in the repo is confounded — they differ in content *and*
mode; `asis.py` runs that confounded comparison separately as a raw data point.)

## Layout

| file | what |
|---|---|
| `gen.py` | emits the matched domains+problems for any `{encoding}×{mode}×{content}` |
| `run_experiment.py` | solves every corpus, writes `metrics/*.json` + `RESULTS.md` |
| `asis.py` | Part A: the *confounded* as-is comparison of the existing domains |
| `proto/` | tiny hand-verified prototypes of all 6 domains (the "run it before you trust it" set) |
| `asis/prohibited-consume.pddl` | a working instance of the existing `Prohibited` domain (the shipped problem is stale) |
| `corpora/` | generated (gitignored); rebuilt by `gen.py emit-corpora` |
| `metrics/` | generated metric JSONs (perf.py schema; `perf.py compare`-able) |
| `RESULTS.md` | generated tables + the head-to-head verdict |

## Content models

- **chain** — `r0 → r1 → … → rK` (single-input recipes). Knobs: `K` chain length,
  `N` goal quantity. Linear/accumulative work — the FF heuristic keeps a clean gradient.
- **converge** — a balanced binary assembly tree of depth `D` (two-input recipes); the
  goal item needs ≥2 sub-assemblies that each need sub-chains. This is the case where
  the delete-relaxed heuristic "goes flat" (≥2 contributions converging onto one goal
  quantity — see `../../examples/BORDERS.md`), so it is where encodings diverge most.

## Metrics

Read from `ff … --json --threads 1`:

- **`evaluated_states`** — node expansions; the headline search-efficiency number.
  Only populated on the **classical FF path** (instantaneous actions). The temporal
  path reports 0, so durative runs compare **makespan** instead.
- **plan length** — solution quality / equivalence check (all encodings must return the
  same length on the same problem, since they model identical content).
- **coverage** — solved within the node cap / timeout.
- **grounded_actions / grounded_facts** — exposes the data-table's grounding cost (its
  `craft` schema enumerates `K·R²` (chain) or `R³` (converge) candidate groundings,
  then prunes to the `K` with a true `(recipe …)` fact).
- **wall-clock ms** — machine-dependent; profiling only, never the verdict.

`--threads 1` keeps `evaluated_states` and timings deterministic.

## Reproduce

```sh
cd ferroplan
cargo build --release -p ferroplan-cli        # -> target/release/ff

# the controlled experiment (writes metrics/*.json and RESULTS.md)
python3 benchmarks/encoding-ab/run_experiment.py --contents chain converge

# the confounded as-is comparison (Part A)
python3 benchmarks/encoding-ab/asis.py

# one-offs: emit a single domain/problem to stdout
python3 benchmarks/encoding-ab/gen.py domain  --encoding forall --mode inst --content converge --depth 3
python3 benchmarks/encoding-ab/gen.py problem --encoding forall --mode inst --content converge --depth 3 --qty 2

# pairwise diff any two metric files with the existing harness
python3 benchmarks/perf.py compare benchmarks/encoding-ab/metrics/converge-specific-inst.json \
                                   benchmarks/encoding-ab/metrics/converge-forall-inst.json
```

## Findings

Full tables in `RESULTS.md` (committed run: `ff @ dbb9bb9`, threads=1, cap=2M nodes,
45s timeout). Plan length is identical across all three encodings on every solved
instance — they really do model the same content, so the comparison is fair.

**1. The data-table generic encoding is search-free — it ties the hand-written
action-specific encoding exactly.** Identical `evaluated_states` on *every* instance,
chain and converge, with identical coverage and plan length:

| | chain (inst) total_eval | converge (inst) total_eval |
|---|---|---|
| specific | 8523 | 51465 |
| data-table | 8523 (+0.0%) | 51465 (+0.0%) |
| forall | 8523 (+0.0%) | 51668 (**+16.5% geomean**) |

So "generic vs action-specific" is **not** a search tradeoff *if you use the data-table
style*: collapsing N action schemas into one `craft ?rec ?in ?out` + a `(recipe …)` data
table costs nothing in node expansions. Its only cost is **grounding** — the `craft`
schema enumerates `K·R²` (chain) / `R³` (converge) candidate groundings before the static
`(recipe …)` precondition prunes them to `K`. At these scales that is single-digit-to-low
milliseconds (geomean_ms +69% chain / +184% converge — still ms), but it compounds badly
under temporal mode: `data-table-temporal` on converge takes 11.7 s at d3·n2 and times out
across d4, where `specific-temporal` is far cheaper.

**2. The forall-numeric encoding is the one that creates "a very different, harder search
domain"** — exactly the user's intuition, but only in two specific ways:
- **Search penalty that grows with convergence.** On linear chains it's identical to the
  others (the heuristic gradient is unchanged), but on the convergent tree it expands more
  states: +33% (d2·n2), **+65% (d2·n4)**, +19% (d3·n2), +25% (d3·n4). This is the
  delete-relaxed heuristic "going flat" where ≥2 contributions converge (`BORDERS.md`); the
  fat `forall`-over-resources operator perturbs the relaxed plan more than thin operators do.
- **Per-state CPU cost.** Even where node counts tie (chains), evaluating a `forall`-over-all-
  resources operator is ~15–20× slower in wall-clock: chain k32·n4 = 611 ms vs 30 ms
  (specific) / 40 ms (data-table). This drops its temporal coverage (chain 10/12 vs 11/12).

**3. Temporal makespan is a wash / noisy.** ferroplan's temporal search is satisficing
(first plan, not makespan-optimal), so makespans aren't directly comparable; `forall` even
gets *shorter* makespans on converge where it solves (d3·n1 = 6 vs 14) because its atomic
fat craft exposes more parallelism. The robust temporal signal is **coverage**, which
falls for all three past d3 (temporal search is limited on big monolithic instances).

**4. As-is (Part A, confounded) — `metrics/asis.txt`.** The generic `Prohibited` domain
solves instantaneously (ev=6, len=5 via `claim → pick-up → consume×3`); the action-specific
durative `rpg`/`village`/`rpg-world` solve with makespans 7–25. Different content *and* mode,
so not comparable — a raw data point only.

### Verdict

- **Want generic *and* fast?** Use the **data-table** style. It is the maintainability win
  (one schema + a data table instead of one schema per recipe) with **zero** search penalty
  versus hand-written specific actions — pay only a small, bounded grounding cost.
- **Avoid forall-numeric for satisficing speed.** It is the most expressive (native multi-input
  recipes and quantities) but it is the encoding that genuinely makes the search harder as soon
  as the recipe graph branches, and it is CPU-heavy per node. Reach for it only when you need
  its expressiveness or its tighter temporal makespans.
- **Action-specific** remains the search baseline; data-table matches it. The interesting
  divergence is forall-numeric × convergent content, and grounding cost × scale/temporal.

### Notes / caveats

- **VAL validation of the durative domains is unreliable here.** ferroplan's temporal
  output uses a shared numeric resource and prints integer timestamps; VAL's strict
  PDDL2.1 semantics reject concurrent numeric updates (mutex) and the exact-boundary
  start-after-end, so even the shipped `examples/rpg` plan fails VAL. The durative
  comparison therefore uses ferroplan's own coverage/length/makespan (deterministic),
  with VAL as advisory only.
- **The shipped `Prohibited` problem (`planner/bin/simple_problem.pddl`) is stale** — its
  goal simplifies to FALSE. `asis/prohibited-consume.pddl` is a minimal working instance
  of the same (unmodified) domain.
