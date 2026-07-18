# STATUS — living source of truth for the IPC6/IPC7 roadmap

Per `ferroplan-roadmap.md`: updated at the end of every phase. Where this
file and the code disagree, the code wins and this file gets fixed.

Last update: **Phase 4 complete** (net-benefit; Phases 0, 2, 4 done —
Phase 3 next).

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
| 3 — LAMA-style config | **next** | landmarks greenfield; helpful actions exist (EHC only) |
| 4 — net-benefit | **done** | maximize normalized onto minimize B&B (`metric_konst` reporting transform); `cost_monotone` accepts static nonneg expressions; netben subset 16/16, VAL-valid, net benefit reported |
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

## Costs-subset scoreboard (vendored, `run.py --timeout 10 --only costs`)

- **Pre-Phase-2 (0.8.0):** 35/54 solved, all VAL-valid, no metric
  reported anywhere (costs ignored; shortest-length plans).
- **Post-Phase-2:** 32/54 solved at the same 10s budget, all VAL-valid,
  **cost metric reported on every cost domain** (elevators08 p01: 54 vs
  the 100 a cost-blind plan replays to). The 3-instance dip is the
  polish tax at a tight budget: the sweep spends up to ~2× the solve's
  evals improving cost (woodworking08 p04 now needs >10s; solves with
  margin at the 30s default). Quality-for-time is the intended IPC6
  trade; the sweep is budget-bounded and `FF_COST_SWEEP_EVALS=0`
  restores pure-coverage behavior.
- Frontier (all instances timeout at 10s): parking11, tidybot11,
  barman11, floortile11 (+scanalyzer08 p04, visitall11 p03/p04,
  nomystery11) — exactly the domains Phase 3's landmark /
  preferred-operator machinery targets.

Net-benefit (post-Phase-4): `run.py --timeout 30 --only netben` —
**16/16 solved, all VAL-valid, net benefit reported everywhere**
(elevators08 33/60/21/73; crew08 2100/1988/2160/2042 of ceiling 3335;
was: empty plans, no metric).

## Open game-design questions (shape, not gate)

Turn-based vs real-time; genuine concurrency (more temporal investment?);
per-tick planning budget (drives `Options` wall-clock budget +
`Session` extensions). Record answers here when known.
