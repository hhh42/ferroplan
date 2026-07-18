# STATUS — living source of truth for the IPC6/IPC7 roadmap

Per `ferroplan-roadmap.md`: updated at the end of every phase. Where this
file and the code disagree, the code wins and this file gets fixed.

Last update: **0.9 cycle — Phases 0, 2, 3 (core), 4, 5 complete**; see
`docs/roadmap-0.9.md` for the cycle record.

## Current capabilities (audited v0.8.0)

- **Representation:** propositional, by design — bitset states over
  SoA/CSR operator tables (`packed.rs`); Helmert-style mutex-group
  synthesis (`invariants.rs`) layered on top, feeding ESPC partitioning.
  There is no SAS+ substrate and none is planned (roadmap Phase 1).
- **Search:** FF-family — EHC with helpful-action lookahead, fallback
  deterministic batch-parallel weighted best-first (`--weight-g/-h`).
  Delete-relaxation FF heuristic. No landmarks in classical search
  (temporal modules have their own deadline machinery).
- **Modes** (`auto` routes by problem features): `ff`
  (classical/numeric/ADL), `pddl3` (preferences/metric, anytime B&B over
  `is-violated` + `total-cost` terms, ESPC penalty loop), `temporal`
  (PDDL2.1 durative, TILs, decision-epoch + scheduler, decomposition
  into contracts), `partition` (SGPlan-style).
- **Embedding:** `Session` = ground once / replan many (`set_fact`,
  `set_fluent`) for classical/numeric/ADL; `solve`/`parse`/`decompose`/
  `validate_plan` library API, serde-serializable.
- **Validation:** internal `--validate` / `plan::validate_plan`;
  external **VAL** wired into the benchmark harness (see below).

## Phase status

| Phase | State | Notes |
|---|---|---|
| 0 — gap audit & scaffolding | **done** | this file; IPC6/7 corpora; VAL in harness |
| 1 — mutex layer | existing, opportunistic | `invariants.rs` shipped in 0.8; exploitation TBD |
| 2 — action costs | **done** | `costs.rs`: replayed metric + anytime cost sweep (`relaxed_costed` guidance); elevators08 p01 100→54; all reported costs VAL-valid |
| 3 — LAMA-style config | **done** | `landmarks.rs` + `lama.rs` rung (now per-subgoal too, in the partition cascade); barman11 p01 solves on both paths. The iterated-weight anytime remainder CLOSED as a measured negative: restart-shaped length improvement pays ~1.8% at ~28x the solve's evals on visitall (proportionate budgets: zero gain) — ships opt-in (`FF_LEN_SWEEP_EVALS`, default off) with the `g_bound` engine lever inert; a within-one-search length-anytime is the recorded next idea. LAMA's lazy eval deliberately skipped — batch-parallel eval is ferroplan's answer |
| 4 — net-benefit | **done** | maximize normalized onto minimize B&B (`metric_konst` reporting transform); `cost_monotone` accepts static nonneg expressions; netben subset 16/16, VAL-valid, net benefit reported |
| 5 — prefs × costs | **done** | `tests/costs_prefs.rs`: one shared metric, satisfy-vs-forgo flips at the weight boundary, `always` monitor enforced under the combined metric |
| 6 — portfolio | seed exists (`auto` routing) | scheduler not started |
| 7 — optimal | not started | optional |
| 8 — temporal | shipped (0.5–0.8) | IPC6/7 temporal benchmarking outstanding |

## Benchmarking & validation scaffolding (Phase 0 deliverables)

- `benchmarks/ipc/costs/` — curated IPC-2008/2011 sequential-satisficing
  subset (14 domains × ≤4 instances, action-costs): elevators08,
  transport08, woodworking08, pegsol08, sokoban08, scanalyzer08,
  parcprinter08\*, openstacks08\* (\*per-instance `pNN-domain.pddl`),
  barman11, floortile11, nomystery11, parking11, tidybot11, visitall11.
- `benchmarks/ipc/netben/` — IPC-2008 net-benefit subset: elevators08,
  pegsol08, openstacks08, crew08.
- `benchmarks/run.py` — now supports per-instance domains, reports
  metric/cost, and **externally validates every solved plan with VAL**
  when available (`$FERROPLAN_VAL` or `Validate` on PATH; exit 1 on any
  VAL failure).
- `benchmarks/ipc67.py` — full-corpus runner over a potassco
  `pddl-instances` checkout (`benchmarks/get-ipc.sh`), per-variant
  coverage/cost/time/VAL summary → `benchmarks/ipc67-results.md`.
- `benchmarks/get-val.sh` — clone + build VAL.
- IPC5 regression guards unchanged (CI heavy step: `espc`,
  `ipc5_pref_metric`).

## Costs-subset scoreboard (vendored, `run.py --timeout 10 --only costs`)

- **Pre-Phase-2 (0.8.0):** 35/54 solved at 10s, all VAL-valid, no
  metric reported anywhere (costs ignored; shortest-length plans).
- **Post-Phase-2 (10s):** 32/54 — metric reported on every cost domain
  (elevators08 p01: 54 vs the cost-blind 100); the dip is the bounded
  polish tax at a tight budget (`FF_COST_SWEEP_EVALS=0` restores it).
- **Post-Phase-3 (30s, clean machine): 46/54**, all VAL-valid, costs
  reported — full table in `benchmarks/ipc-results.md`. **barman11
  goes 0/4 → 4/4** (~4.5 s each, costs 253–261); parking11 and
  floortile11 p01/p02 join the solved set.
- Whole vendored corpus at 30 s: **110/166**, every solved plan
  VAL-validated (costs 46/54, netben 16/16, pref 27/48 and qualpref
  13/40 at this quick budget — the curated scoreboards in
  `benchmarks/ipc5-*.md` use longer budgets, `benchmarks/results.md`
  stays the curated oracle comparison).
- **Post-grounder-frontier fix: costs 49/54 at 30 s** — **tidybot11 goes
  0/4 → 4/4** (11 s / 124 s / 6 s / 6 s at 240 s; three inside the 30 s
  tier). Two walls fell: the parser recorded a self-edge for domains
  that redeclare the built-in `object` root type (every parent-chain
  walk hung — tidybot never reached grounding; cyclic `:types` now
  rejected BY NAME), and the cartesian binding enumeration now prunes
  on static preconditions at first-bound level (join-style; tidybot p01
  grounds 91.6 s → 2.8 s, task byte-identical). All four plans replay
  to goal on the internal oracle; full 124-test suite unchanged.
- **The named frontier is closed**: floortile11 p03/p04 (42 s / 40 s)
  and parking11 p03/p04 (22 s / 24 s) all solve on the LIBRARY path
  (`--json`, the LAMA-rung surface) — the earlier "unsolved at 240 s"
  rows were text-path measurements, where the LAMA rung never ran. The
  vendored costs subset is **54/54 at a 240 s library-path budget**
  (49/54 at the quick 30 s / 1-thread `run.py` tier); every frontier
  plan replays to goal on the internal oracle.
- Text-path unification (partial): `resolve::solve`'s MONOLITHIC case
  now runs the full library ladder (EHC → LAMA → weighted best-first),
  so a collapsed partition solves exactly when the library path does.
  Second half shipped: per-subgoal LAMA (`landmarks_for` /
  `lama::search_subgoal` — landmarks recomputed per (start, subgoal)
  pair) plus BOUNDED subgoal probes (100k evals — a subgoal unsolvable
  in isolation used to burn the full budget proving it before every
  merge). barman11 p01 on the text path: never-finishes -> 57 s
  (library path unchanged at ~4.5 s; the residual gap is the per-merge
  re-solve loop of the cascade itself, recorded).

Net-benefit (post-Phase-4): `run.py --timeout 30 --only netben` —
**16/16 solved, all VAL-valid, net benefit reported everywhere**
(elevators08 33/60/21/73; crew08 2100/1988/2160/2042 of ceiling 3335;
was: empty plans, no metric).

## Open game-design questions (shape, not gate)

Turn-based vs real-time; genuine concurrency (more temporal investment?);
per-tick planning budget (drives `Options` wall-clock budget +
`Session` extensions). Record answers here when known.
