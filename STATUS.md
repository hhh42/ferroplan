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
2. **Temporal memory bound** — SHIPPED 2026-07-19, root cause found and
   fixed; one successor lever remains:
   - **Root cause was the GROUNDER, not the search:** Phase C interns
     atoms from every raw candidate op; reachability then prunes the ops
     but leaves their fact ids behind, so `words` — every State bitset
     and visited key — was sized by the RAW atom space. Temporal snap
     compilation makes it catastrophic: elevator-08-t p22's 5-ary
     BOARD-END/LEAVE-END enumerate RUNNING-* over the full typed space,
     minting 2.35M facts (287 KB/state, 8 GB RSS, killed at any budget)
     while ~7k facts are live. **Fact-space compaction** (monotone
     renumber after reachability, `FF_NO_FACT_COMPACT` hatch) packs only
     reached/referenced/goal facts: p22 → 6,969 facts (words 36,736 →
     109). Classical tasks are bit-identical (their raw references
     survive); equivalence verified on/off across STRIPS/numeric/costs/
     ADL/temporal — identical plans and eval counts. Plus the
     byte-aware temporal node cap (`temporal_node_cap`, classical model
     + agenda/key extras, static dims, `FF_TEMPORAL_NODE_CAP` hatch)
     replacing the byte-blind 400k count.
   - **Measured recovery:** elevator-08-t-strips 19/30 → **21/30** at
     the baseline 30 s; p22–p25 (8 GB kills at ANY budget before) now
     solve in 38/54/62/80 s; elevator-11-t p01 solves (was 0/20), p02
     solves solo at 120 s.
   - **Stratified Phase B grounding — SHIPPED same day:** END actions
     (gated on producer-known RUNNING-* tokens: no init atoms, all
     adders in stratum 1) ground join-restricted to the atoms the
     STARTs actually produce, through the existing static-literal
     pruning machinery. Post-reachability op set identical; raw order
     preserved by splicing; temporal-path only (fact-id first-reference
     order can shift, and the classical fixtures pin today's ids);
     `FF_NO_STRAT_GROUND=1` for A/B. p22: solve 38 s → ~26 s (inside
     the 30 s budget), transient 8.0 → 4.1 GB, same 97-step plan.
     **Coverage at the baseline 30 s: elevator-08-t-strips 19/30 →
     22/30; elevator-11-t 0/20 → 3/20.**
   - **Residual, recorded:** stratum-1 START enumeration (BOARD-START's
     passenger×floor×count cross survives static pruning) still costs
     ~4 GB transient and most of the grounding wall on the biggest
     instances — the elevator-11 tail needs either
     reachability-interleaved grounding (a much bigger project) or
     just bigger budgets (p22-class instances all solve at 120 s).
3. **`?duration` in expressions** — SHIPPED 2026-07-19, three layers
   (parser pseudo-fluent → snap-compile substitution, exact at START,
   end-side only when the duration reads no assigned fluent, else the
   action is SKIPPED — never compiled wrong → state-dependent durations:
   grounded `NExpr` side table, resolved per expansion / at the plan
   step's source state / at the validator's start happening). Proven by
   the `durexpr` fixture (duration must resolve 3 against the start
   state, not init's 1; suite 141→142/0). **model-train outcome,
   honest:** the variant now parses, grounds (772 ops, i1) and
   searches — but stays 0 solved at 30 s: `avg_helpful 0.0` (relaxed
   helpful ops are empty on this domain), all four passes exhaust the
   node budget. The wall moved from the parser to temporal guidance —
   folds into item 7.
4. **openstacks08-ADL seq-sat gap** — SHIPPED 2026-07-19: the miss was
   a 2^k DNF explosion, not the ADL machinery. `(forall ?o (imply
   (includes ?o ?p) (started ?o)))` expanded BOTH branches of every
   imply, so each non-included order doubled the conjunct count (i5:
   45,166 redundant ops; i7: 15 GB RSS mid-grounding, dead). `to_dnf`
   now resolves fully-bound never-added literals against init and
   absorbs True disjuncts (`FF_NO_DNF_STATIC` hatch). The sound folding
   is asymmetric — dropping a conjunct needs only never-added; folding
   a literal away needs never-added AND never-deleted (the constraints
   suite caught the first cut folding away delete-only TRAJ-PLANNING —
   7 fixtures red, `del_preds` guard restored them). i5 → 220 ops, i7 →
   480 ops. **Coverage: seq-sat-ADL 6/30 → 30/30 (60 s); the same fix
   swept the temporal twins — temporal-ADL 6/30 → 30/30, temporal-ADL-
   numeric 7/30 → 30/30 (30 s), +71 instances total.** Classical
   STRIPS/costs paths bit-identical on/off (gripper, blocks, barman,
   woodworking: same plans, same eval counts); suite 142/0.
5. **Quality/anytime for floor-tile/visit-all** — MEASURED NEGATIVE
   2026-07-19 (0.10 Phase 3), shipped OPT-IN (`FF_LEN_ANYTIME=1`). The
   within-one-search drain (same open list, live g-bound, ≤2× eval
   ceiling — no restart, unlike 0.9's improve_length) was implemented
   in both open-list searches (best-first + the LAMA rung; EHC is
   structurally out — visit-all's plans come from EHC, floor-tile's
   from LAMA). At the 60 s budget: ZERO length gains on the two
   motivating domains and 9 instances of coverage LOST to the doubled
   wall (sokoban −7, floor-tile −1, visit-all −1) against sokoban's 4
   shorter plans (−234 steps). Default OFF; the honest quality lever
   for these domains is guidance, not post-solve polish.
6. **Portfolio budget-aware scheduling** — SHIPPED 2026-07-19: phase A
   gives the ladder the FULL eval pool (coverage ≥ default BY
   CONSTRUCTION); diversification doubles only over what an early
   internal wall left behind. `FF_PORTFOLIO_SLICED=1` restores pure
   doubling. Measured on all seven delta variants at the baseline
   60 s: every old loss recovered exactly to default (sokoban08 17→21,
   sokoban11 8→11, visit-all 4→8, barman 19→20, elevator11 10→11) and
   the no-mystery diversification win KEPT (15 vs default's 14); the
   sole trade is woodworking08's old +1 (ladder consumes the full pool
   there → default's 29). Extrapolated corpus ~428 ≥ default 427 ≥ old
   portfolio 416 — the Phase 6 acceptance now honestly met.
7. **Temporal search walls** — ANSWERED 2026-07-19 (0.10 Phase 2). The
   completeness question closes: a minimal turn-and-open repro (now a
   suite test) proved same-epoch chaining handles the
   start-inside-an-interval pattern — NO semantics gap. The real
   amplifier was the visited key: ABSOLUTE agenda times made every
   retimed permutation a fresh state. TIL-free tasks are
   shift-invariant, so the key now uses pending-end DELTAS
   (`FF_TEMPORAL_ABS_KEY=1` restores). **Measured at the 30 s
   baseline: sokoban08-t 7→10/30, sokoban11-t 0→2/20, floor-tile11-t
   0→3/20, all VAL-validated; turn-and-open 0→1/20 at 60 s (i1 solves
   in ~25 s solo — the 30 s/3-job methodology clips it). Sentinels
   unchanged (crew 50/50, pegsol 46/50, match-cellar 6/20;
   elevator-strips 21 vs 22 is borderline-p22 contention variance,
   VAL-green).** The two unmoved walls are classified honestly:
   storage11 explored 3M nodes with a LIVE 2.2M heap (no exhaustion →
   no semantics gap) but `avg_helpful → 0` far from init — a pure
   h^FF-guidance wall, same family as transport11/model-train; TMS
   drowns in genuine-concurrency interleavings (avg 47 pending ends
   per node, 15k ops). Both fold into the guidance agenda.
   *0.13 Phase 5 addendum*: the TMS diagnosis is now mechanism-precise.
   Agenda-level symmetry reduction shipped (canonical pending-interval
   order + redundant identical-interval skip, `FF_NO_TSYMM=1` reverts)
   and eliminated the re-fire/re-bake copies (avg agenda 47→40), but
   the wall STANDS: the residual blowup is goal-paired PIECE-SUBSET
   state symmetry — interchangeable pieces distinguished only by which
   `(baked-structure p q)` pair they serve, so every subset-assignment
   of "which identical piece is baking" is a distinct visited state.
   Collapsing it needs goal-respecting object-symmetry orbits (a real
   symmetry-breaking engine), filed with the research fence, not the
   agenda.
8. **Runner polish** — SHIPPED 2026-07-19 (0.10 Phase 1): ipc67.py
   VALs tempo-sat plans (timestamped rendering, `-t` at ff's 0.001 ε,
   auto-finds the get-val.sh build) and caps each job's address space
   (`--mem-gb`, default RAM/jobs — a spike kills ITS job with a
   `mem-cap` note instead of the OOM killer executing siblings). The
   new validation immediately caught a real bug: same-instant numeric
   write-write (two `board`s on one `(passengers l)`) passed the
   fact-only mutex test — `epsilon_separate` now counts numeric
   footprints (write-write + write-read, incl. conditional targets and
   value reads) and its cap rose 600→2000 happenings.
   elevator-numeric val 1/3 → 3/3; crew 20/20, all sweeps val-green.
