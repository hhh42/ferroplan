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

### Post-cycle: the frontier closes (costs 54/54 at 240 s, library path)

The "search-bound" re-attribution above was itself a measurement
artifact: those 240 s runs used the TEXT path, where the LAMA rung never
runs (the recorded scope cut). On the LIBRARY path — the `--json`
surface `run.py` measures — the whole remaining frontier solves inside
the budget: **floortile11 p03 42 s, p04 40 s; parking11 p03 22 s, p04
24 s**, every plan oracle-replayed to goal. With tidybot11's 4/4 that
makes the vendored costs subset **54/54 at a 240 s library-path budget**
(the quick 30 s / 1-thread tier stays 49/54).

The text-path gap is now HALF closed: `resolve::solve`'s monolithic
case runs the full library ladder (EHC → bounded LAMA rung → complete
weighted best-first), restoring the module's "solvable exactly when the
subplanner is" doctrine for collapsed partitions — but the partition
cascade still solves SUBGOALS without landmark guidance
(`goal_landmarks` is whole-goal), so barman11 p01 still times out at
60 s on the text path against ~4.5 s on the library path. Per-subgoal
landmarks are the recorded next step of the unification.

Second half shipped, same day: `landmarks_for` / `lama::search_subgoal`
generalize the whole-goal forms over a (start, subgoal) pair, the
cascade's subgoal solves became a bounded ladder (100k-eval probe →
subgoal LAMA rung → merge), and the monolithic endpoint keeps the
complete full-budget library ladder — so bounded probes only make
merges happen SOONER while overall solvability is unchanged. Measured:
barman11 p01 text path never-finishes → **57 s** (9 groups, 7 merges);
the residual 57-vs-4.5 s gap is the cascade's per-merge re-solve loop
itself, which is the partition path's identity, not a missing rung.

### Post-cycle: Phase 6 shipped (portfolio), acceptance half-met

`portfolio.rs` + `Mode::Portfolio` / `--mode portfolio`: four
complementary classical members (default ladder; LAMA rung alone;
best-first at w_h 3 and 1) over one shared evaluated-state pool,
doubling restart slices, fixed order — deterministic by construction.
First plan wins (the winner is named in `Solution.notes`); a complete
member's un-capped exhaustion settles unsolvability early; temporal and
preference problems fall back to their own machinery like `auto`.

Measured (costs subset, 30 s, single thread): **49/54 — exactly the
default configuration's coverage AND its unsolved set** (floortile11
p03/p04, parking11 p03/p04, tidybot11 p01). The acceptance criterion's
first half ("at least as good as the best single configuration") is met
with parity; the second half ("better on at least some domains") is not
demonstrable on the vendored subset — this cycle's frontier fixes
removed every curated instance where the default faceplants and a
different member could win. The full-corpus `ipc67.py` run is the
recorded venue for that half; an earlier in-session claim that the
portfolio shifted the tidybot frontier was a baseline error (the text
path was measured against the library path) and is corrected here.

### Post-cycle: the full corpus speaks (seq-sat 427/580; Phase 6 settled)

The recorded venue ran: the whole potassco IPC-2008/2011 seq-sat corpus,
580 instances at 60 s / 1 thread each (3 parallel jobs, every solved
plan VAL-validated). Default configuration: **427/580** — with clean
sweeps on cyber-security (30/30), elevator08 (30/30), parc-printer
(50/50), peg-solitaire (50/50), and barman11 (20/20, the LAMA rung at
full scale). `benchmarks/ipc67-results.md` holds the table.

**The Phase 6 acceptance settles as NOT met as stated, with the
interesting half now true.** Portfolio mode: 416/580. "Better on some
domains" is finally demonstrated — no-mystery11 p10 and woodworking08
p29 solve ONLY under the portfolio, and it finds cheaper plans on 5
sokoban + 3 floor-tile common solves — but overall parity fails: the
13 instances the portfolio loses all have default solve times of
27–56 s (sokoban ×7, visit-all ×4, barman11 p19, elevator11 p12). The
mechanism is structural: doubling restart slices tax precisely the
instances that barely fit the budget. The portfolio stays opt-in;
the recorded next idea is budget-aware scheduling — let the default
member run to its natural EHC/ladder end before diversification
spends anything.

**New frontier, measured (in leverage order):**

1. **transport11 0/20 — search-bound.** Not a grounder wall: p01
   grounds to 1 052 facts / 21 136 actions in under a second, then
   evaluates ~520 states/s single-threaded — ~30 k states inside the
   whole budget. Eval throughput × guidance is the target (the
   batch-parallel eval exists precisely for this; the 1-thread
   methodology also understates the engine here).
2. **openstacks08-ADL 6/30** against the STRIPS twin's 30/30 — the ADL
   compilation path, not the search, is the gap.
3. **floor-tile11 5/20, visit-all11 8/20** — the quality/anytime
   domains, exactly where the closed length-sweep negative pointed.

## Deliberate scope cuts (why, not just what)

- **Iterated-weight anytime for UNIT-cost quality** (rest of Phase 3):
  cost domains already improve via the Phase 2 sweep; pure-length
  domains (visitall) keep first-found quality. Next cycle.
  *Closed post-cycle as a measured negative*: the restart ladder
  (w_h 3/2/1, incumbent length-bound pruning via the new
  `SearchCfg::g_bound`) is sound and deterministic but pays ~1.8%
  (226 → 222 on visitall p01) at ~28x the solve's evals; at the polish
  doctrine's proportionate budgets it moves nothing. Ships opt-in
  (`FF_LEN_SWEEP_EVALS`, default off — byte-identical); the recorded
  next ideas are a within-one-search length-anytime (the cost sweep's
  shape applied to g) or landmark-guided restarts.
- **LAMA's lazy heuristic evaluation**: ferroplan's batch-parallel
  evaluation answers the same throughput question differently; measured
  batching beats porting the trick blind.
- **Landmark orderings / needed-again**: v1 counts monotone acceptance
  only — sound, and already decisive on the frontier domains.
- **Text-path partition mode** (`run_planner` default) lacks the LAMA
  rung inside `resolve::solve`'s subgoal machinery — library/JSON path
  is the measured surface; unify next cycle.
