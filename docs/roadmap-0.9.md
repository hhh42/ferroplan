# 0.9 roadmap record — the IPC6/IPC7 arc opens

Working record for the 0.9 cycle, per `ferroplan-roadmap.md` (the
top-level IPC6/IPC7 + RPG roadmap). Phase numbering follows that
document; `STATUS.md` is the living summary.

## Shipped this cycle

### Phase 0 — measurement scaffolding

- Vendored IPC-2008/2011 subsets: `benchmarks/ipc/costs/` (14
  sequential-satisficing action-costs domains, ≤4 instances each,
  per-instance `pNN-domain.pddl` supported) and `benchmarks/ipc/netben/`
  (4 net-benefit domains).
- External **VAL** validation of every solved plan in `run.py`
  (`FERROPLAN_VAL` / PATH; exit 1 on any failure); `get-val.sh` builds
  it, `get-ipc.sh` + `ipc67.py` cover full-corpus scoreboard runs.
- `STATUS.md` — audited capabilities and phase states.

### Phase 2 — `:action-costs` on the classical path (`costs.rs`)

`(:metric minimize <fluent>)` detected; the plan's cost REPLAYED (never
estimated) and reported; a bounded anytime B&B improvement sweep —
ordered by accumulated cost, guided by the cost-augmented relaxed plan
(`relaxed_costed` = selected-op cost + length) — trades length for
cost after the untouched EHC/best-first machinery finds its first
plan. Proportionate budget (`FF_COST_SWEEP_EVALS` override; 0
disables); `--satisfice` reports without sweeping; maximize/compound
shapes never silently claimed; uncapped exhaustion ⇒ proven optimal.
Recorded: elevators08 p01 cost 100 → 54.

### Phase 3 core — the LAMA rung (`landmarks.rs`, `lama.rs`)

Fact landmarks by first-achiever backchaining (sound, O(n_facts)
memory); path-dependent landmark count (per-node accepted bitset) as a
second signal beside deferred FF h; preferred-operator boosting via a
dual open list. Runs bounded between EHC failure and the complete
weighted fallback — can only add coverage (`FF_NO_LAMA=1` removes it;
explicit `--search bfs` never enters). Recorded: **barman11 p01 solves
for the first time at any tested budget**; parking11/floortile11 p01
drop from >130 s / >10 s to seconds.

### Phase 4 — net-benefit (`pddl3.rs` normalization)

`maximize` metrics normalize onto the minimize B&B (extraction at
scale −1; affine constant carried in `Compiled::metric_konst`, mapped
back at reporting via `display_metric`). `cost_monotone` accepts
provably non-negative STATIC cost expressions (sums/products/quotients
of non-negative constants and static fluents) instead of constants
only. The empty plan stays a legal candidate (oversubscription
semantics). Recorded: netben subset **16/16 solved, all VAL-valid, net
benefit reported everywhere** (was: empty plans, no metric). Text-path
routing for pure-cost problems now matches the library API (classical
`costs.rs` path).

### Phase 5 — composition (`tests/costs_prefs.rs`)

Action costs + preferences share ONE metric evaluation with no double
counting — the satisfy-vs-forgo decision flips exactly at the weight
boundary — and a hard `always` monitor stays enforced while the
combined metric is optimized. IPC5 pref/qualitative baselines
re-verified green throughout (19 heavy guards).

## Scoreboard (vendored costs subset, 30 s, VAL-validated)

| | 0.8.0 baseline | this cycle |
|---|---|---|
| coverage | 35/54 | **46/54** |
| barman11 | 0/4 | **4/4** (~4.5 s each) |
| cost metric reported | never | every cost domain |
| external validation | none | every solved plan |

Full per-instance table: `benchmarks/ipc-results.md` (whole vendored
corpus 110/166 at 30 s; netben 16/16). Remaining frontier: tidybot11
(all 4, even at 240 s — grounding/search scale), floortile11 p03/p04,
parking11 p03/p04.

### Post-cycle: the grounder frontier (tidybot11 0/4 → 4/4)

The "grounding/search scale" attribution was measured and turned out to
be two separate grounder walls, neither search:

1. **A type-cycle hang.** tidybot's domain legally redeclares the
   built-in root type (`(:types ... object ...)`); the parser recorded
   the self-edge `OBJECT → OBJECT` and every parent-chain walk
   (`objects_by_type`, derived's `is_a`) spun forever at 3 MB RSS —
   the planner never reached grounding at any budget. The parser now
   skips self-edges and rejects genuinely cyclic `(:types ...)` by
   name; both walks are hop-bounded as defense for programmatically
   built domains.
2. **The cartesian binding product.** With the hang gone, grounding
   took 91.6 s: 9-parameter actions over grid statics
   (`sum-x`/`sum-y`/`leftof`) enumerate ~10^8 bindings the post-filter
   then rejects. `for_each_binding` now checks each static literal at
   the first level where its variables are bound, pruning whole
   subtrees (join-style grounding) — 91.6 s → 2.8 s, and the surviving
   binding ORDER is unchanged by construction, so the grounded task is
   byte-identical (full suite green, unchanged).

Measured: tidybot11 **4/4** (p01 11 s / 72 steps, p02 124 s, p03 6 s,
p04 6 s; every plan oracle-replayed to goal), costs subset 46/54 →
**49/54** at 30 s (p02 sits in the 240 s tier). floortile11 p03/p04 and
parking11 p03/p04 remain — now provably SEARCH-bound, the next lever's
target (iterated-weight anytime / portfolio, per the scope cuts above).

## Deliberate scope cuts (why, not just what)

- **Iterated-weight anytime for UNIT-cost quality** (rest of Phase 3):
  cost domains already improve via the Phase 2 sweep; pure-length
  domains (visitall) keep first-found quality. Next cycle.
- **LAMA's lazy heuristic evaluation**: ferroplan's batch-parallel
  evaluation answers the same throughput question differently; measured
  batching beats porting the trick blind.
- **Landmark orderings / needed-again**: v1 counts monotone acceptance
  only — sound, and already decisive on the frontier domains.
- **Text-path partition mode** (`run_planner` default) lacks the LAMA
  rung inside `resolve::solve`'s subgoal machinery — library/JSON path
  is the measured surface; unify next cycle.
