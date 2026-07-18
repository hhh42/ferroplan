# STATUS — living source of truth for the IPC6/IPC7 roadmap

Per `ferroplan-roadmap.md`: updated at the end of every phase. Where this
file and the code disagree, the code wins and this file gets fixed.

Last update: **Phase 0 complete** (v0.8.0 + roadmap work).

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
| 2 — action costs | **next** | costs parse as numerics but metric is ignored in `ff` mode (see baseline) |
| 3 — LAMA-style config | not started | landmarks greenfield; helpful actions exist (EHC only) |
| 4 — net-benefit | close | IPC6 netben metric = existing PDDL3 class under `maximize (- C …)`; today auto→pddl3 returns empty plan, metric unreported |
| 5 — prefs × costs | substrate done (0.6–0.8) | composition once Phase 2 lands |
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

## Honest baseline (pre-Phase-2), vendored costs subset

`run.py --timeout 10 --only costs`, single thread: **35/54 solved**, all
solved plans VAL-valid, **metric reported: none** (costs ignored; plans
are shortest-length, not cheapest). Frontier (all instances timeout at
10s): parking11, tidybot11, barman11 (+scanalyzer08 p04, visitall11
p03/p04, nomystery11 p02, openstacks08 — see results table). These are
exactly the domains Phase 3's landmark/preferred-operator machinery
targets.

Net-benefit baseline: auto→pddl3 returns the legal empty plan with no
metric on elevators08 netben p01 — Phase 4's starting point.

## Open game-design questions (shape, not gate)

Turn-based vs real-time; genuine concurrency (more temporal investment?);
per-tick planning budget (drives `Options` wall-clock budget +
`Session` extensions). Record answers here when known.
