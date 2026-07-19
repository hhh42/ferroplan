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
| 6 — portfolio | **shipped, acceptance settled: NOT met as stated** | `portfolio.rs`: 4 complementary members, doubling eval slices, deterministic; winner named in notes; un-capped exhaustion settles unsolvability. Full-corpus verdict (580 inst, 60 s t1): "better somewhere" now DEMONSTRATED (no-mystery11 p10 + woodworking08 p29 solve only under the portfolio; sokoban 5 / floor-tile 3 cheaper common solves) but "at least as good overall" FAILS — 416/580 vs the default's 427/580; all 13 lost instances sit at 27–56 s default solve times (sokoban ×7, visit-all ×4, barman p19, elevator11 p12): the doubling-slice restart tax prices out exactly the barely-in-budget instances. Stays opt-in; recorded next idea: budget-aware scheduling (default member runs to its natural end before diversification spends anything) |
| 7 — optimal | not started | optional |
| 8 — temporal | shipped (0.5–0.8); **first corpus recon done** | tempo-sat corpus (630 inst, 30 s tier): **326/630**. Sweeps: crew-planning 50/50, openstacks-strips/numeric 80/80, parking11 19/20, woodworking 28/30. Three measured wall classes: (1) `?duration` in expressions unparsed — model-train 0/30 is a pure PDDL2.1 feature gap; (2) memory blowups — big elevator/openstacks-ADL temporal instances hit 7–10 GB fast (OOM-killed under the 3-job run; needs a memory-bounded route); (3) search walls — turn-and-open, temporal-machine-shop, storage11, sokoban11, floor-tile11 all-timeouts (required-concurrency-shaped domains among them). Caveat: temporal plans internally validated only (runner writes untimestamped plans, VAL skipped) |

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

## Full-corpus scoreboard (potassco checkout, `ipc67.py`, 60 s / 1 thread / 3 parallel jobs)

First run over the WHOLE IPC-2008/2011 seq-sat corpus (580 instances,
24 variants), 2026-07-18: **427/580 solved, every plan VAL-validated**
(`benchmarks/ipc67-results.md`; portfolio comparison in
`ipc67-portfolio.md`, per-instance diffs via `ipc67-diff.py`).

- **Clean sweeps:** cyber-security 30/30, elevator08 30/30,
  openstacks08-strips 30/30, parc-printer 50/50, peg-solitaire 50/50,
  **barman11 20/20** (the Phase 3 LAMA rung's domain, now at full scale).
- **The measured frontier, in order of leverage:**
  1. **transport11 0/20** — search-bound, NOT grounding: p01 grounds to
     1 052 facts / 21 136 actions in <1 s but evaluates ~520 states/s
     single-threaded (~30 k states in the budget). Eval throughput ×
     heuristic guidance is the wall; transport08's solved instances are
     ~100× smaller tasks.
  2. **openstacks08-ADL 6/30** vs the STRIPS twin's 30/30 — the ADL
     compilation path leaves coverage on the table.
  3. **floor-tile11 5/20, visit-all11 8/20** — the known
     quality/anytime domains; sokoban 21/30 + 11/20 nearby.

## Game-design answers (recorded 2026-07-18; shape, not gate)

The three open questions are ANSWERED:

- **Real-time**, with episodic planning: agents can "stop and think" —
  a think-pause produces a plan, then the agent FOLLOWS that long plan
  in real time. So the engine's model is plan-on-demand at think
  moments (a latency budget per think, potentially generous), NOT
  per-tick replanning — `Session`'s ground-once/replan-many shape fits;
  the `Options` budget work should express a per-think wall/eval budget
  rather than tick slicing.
- **Economy is mostly BARTER — any item to any item; money exists but
  is just another item.** No special-casing of currency in domains:
  trades are generic item-for-item exchange actions. Implication: the
  item×item action space makes grounding scale the binding constraint —
  exactly what the join-style static pruning and shared-block work
  already serve; keep that path fast.
- **Genuine concurrency EXISTS** (multiple agents act simultaneously),
  so the temporal investment stays justified — durative actions and the
  decision-epoch machinery are load-bearing for the game, and the
  carried temporal phases (constraints on the temporal path, temporal
  selection) remain live rather than archival.

All four tracks ran: seq-sat 427/580 (60 s), net-benefit **223/270**
(60 s, all VAL-valid; crew-planning 10/30 and the transport/woodworking
numeric variants are the tails), tempo-sat 326/630 (30 s recon).

## Next-cycle agenda (measured, in leverage order)

1. **transport11 eval throughput × guidance** — ANSWERED 2026-07-19
   (p01, 21,136 ops / 1,052 facts; FF_RES_DEBUG phase attribution now in
   search_from/relaxed_to):
   - **Attribution:** h is 86% of best-first wall at t1 (12.8 s of
     14.9 s per 20k evals); within h, build_rpg is ~96%. A counter-based
     build measured EQUIVALENT (identical 20,126 evals, 12.85 s vs
     12.83 s) — nearly every op fires in every build, so ~614 µs/eval
     worker time is the delete-relaxation FLOOR, not scan waste.
     Reverted; negative recorded in the build_rpg doc comment.
   - **Throughput half is healthy:** best-first at t4 does 300,190
     evals in 62.5 s (4,800 evals/s) vs 1,350 at t1 — 3.6× on 4 cores,
     per-eval worker time unchanged (no bandwidth wall). The old
     process-level numbers (~520–580 evals/s t1) understate the engine:
     the single-threaded EHC prefix and ladder stages dilute them.
   - **Measurement question answered: t4 does NOT flip transport11.**
     p01 (the smallest instance) at t4 with a 240 s wall: no solve —
     the ladder explored ~600k+ states (LAMA's full 400k-cap rung plus
     several hundred k best-first) and h^FF never converged. The
     1-thread scoreboard methodology is not hiding transport coverage;
     **guidance, not throughput, is the binding term** — transport11
     moves only with a better gradient (richer landmarks / domain
     structure), which folds into the quality/guidance items below.
2. **Temporal memory bound** — big temporal instances allocate 7–10 GB
   in seconds (elevator-08-t p22: 7.4 GB in 30 s). A memory-bounded
   temporal route turns OOM deaths into honest timeouts and likely
   recovers coverage; also required for game embedding (a think-budget
   must bound memory too).
3. **`?duration` in expressions** (PDDL2.1 duration-dependent effects/
   constraints) — model-train 0/30 is pure parser; unlocks a whole
   variant.
4. **openstacks08-ADL seq-sat gap** — 6/30 vs the STRIPS twin's 30/30,
   while netben-ADL scores 29/30: the ADL machinery is fine, the
   seq-sat variant's structure (goal shape / compilation) is the miss.
5. **Quality/anytime for floor-tile/visit-all** (5/20, 8/20) — the
   within-one-search length-anytime idea recorded at the Phase 3 close.
6. **Portfolio budget-aware scheduling** — from the settled Phase 6
   verdict: default member to its natural end before diversification.
7. **Temporal search walls** — turn-and-open / temporal-machine-shop /
   storage11 (all-timeout): check required-concurrency completeness of
   the decision-epoch scheme before assuming it's scale.
8. Runner polish: temporal VAL (timestamped plan output), a
   memory cap per job to keep parallel runs honest.
