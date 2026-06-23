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
| `selfcheck.py` | runs the new `ff --validate` over real durative domains (validator demo + regression guard) |
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
- **techtree** — a realistic RPG crafting tech-tree (17 recipes, 26 resources): multi-input
  recipes with quantities (`house = 2 frame + 1 cutstone + 1 window`, `frame = 2 plank`, …),
  shared intermediates (plank feeds frame/tool/cart/sword), and a couple of distractor
  recipes off the goal path. Knob: `N` settlements. This is the case that needs the
  data-table **arity-family** (`craft1/craft2/craft3`) and tests where its grounding cost
  bites versus forall-numeric's one-action generality.

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

# the realistic tech-tree (slower; N>=2 hits the monolithic-search border), then
# reassemble the full RESULTS.md from all the saved metrics (no re-solving)
python3 benchmarks/encoding-ab/run_experiment.py --contents techtree --max-evaluated 300000 --timeout 150
python3 benchmarks/encoding-ab/run_experiment.py --report-only --contents chain converge techtree

# validate a plan under ferroplan's OWN semantics (auto-detects classical vs temporal)
ff -o domain.pddl -f problem.pddl --mode temporal > plan.txt
ff -o domain.pddl -f problem.pddl --validate plan.txt          # -> Plan valid / Plan invalid: ...
python3 benchmarks/encoding-ab/selfcheck.py                    # validator over real durative domains

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

**2b. On the realistic tech-tree (multi-input + quantities) the same picture holds, and the
data-table's feared grounding blow-up does NOT materialise.** All three solve N=1 with
*identical* `evaluated_states` (187 261) and plan length (25); wall-clock is **specific 17 s ·
data-table 14 s · forall 136 s**. So the data-table **arity-family** (`craft1/craft2/craft3`)
is as fast as hand-written specific even with three-input recipes — ferroplan's grounder
constrains the `R^arity` candidate enumeration via the static `(recipeN …)` facts rather than
materialising it — while forall is ~8× slower per node. N≥2 is unsolvable monolithically by
all three (the realistic shared-intermediate tree hits the `BORDERS.md` border — which is
exactly why the production system decomposes a build into contracts rather than solving it whole).

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

## Built-in validator (`ff --validate`)

External VAL is built for textbook PDDL2.1 and actively disagrees with ferroplan on
numeric-durative plans (it rejects two durative actions touching `(workers)` at once as a
mutex, and demands ε-separation VAL-strictly), so it fails perfectly good ferroplan plans —
even the shipped `examples/rpg` one. So ferroplan now has its **own** validator: `ff -o D
-f P --validate plan.txt` replays a plan under the engine's *own* `apply`/`op_applicable`/
`goal_met` semantics (reusing `verify::verify` for classical and `temporal::validate` for
durative; library entry `ferroplan::plan::validate_plan`). It auto-detects classical
(`step N: …`) vs temporal (`t: (…) [d]`) and prints `Plan valid` / `Plan invalid: <reason>`.

Two things it bought us immediately (`selfcheck.py`, unit tests in `crates/ferroplan/src/plan.rs`):

1. **It accepts the resource-parallel plans VAL wrongly rejects.** The `rpg` 3-worker plan
   with `build-house` ε-corrected to t=8.002 validates here but still fails VAL (VAL's
   concurrent-`(workers)` mutex) — the original motivation, demonstrated.
2. **It caught a real ferroplan bug on its first run.** `selfcheck.py` flags 2 of 5 real
   durative plans (`rpg/3workers` at t=8.001, `rpg-world/woodlot` at t=10.001): the temporal
   printer **under-separates ε** at a same-timestamp produce-at-end / consume-at-start
   boundary (a saw produces planks at its `at end`, the next action consumes them at its
   `at start` at the *same* printed time). Nudging the consumer by +0.001 makes the plan
   validate. The fix belongs in the temporal ε-separation / printer (`temporal.rs`
   `epsilon_separate`); it is logged here, not yet applied.

### Notes / caveats

- **VAL is advisory only for the durative domains** (see above): it rejects ferroplan's
  resource-parallel temporal plans on a strict concurrent-numeric mutex, so the durative
  comparison uses ferroplan's own coverage/length/makespan (deterministic) and the new
  built-in validator, not VAL.
- **The shipped `Prohibited` problem (`planner/bin/simple_problem.pddl`) is stale** — its
  goal simplifies to FALSE. `asis/prohibited-consume.pddl` is a minimal working instance
  of the same (unmodified) domain.
