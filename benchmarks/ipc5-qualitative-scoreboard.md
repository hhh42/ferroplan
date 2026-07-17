# IPC-5 (2006) qualitative-preferences scoreboard — ferroplan (self-scored)

Vendored suite: `benchmarks/ipc/qualpref/{openstacks,rovers,storage,tpp,trucks}`
— the IPC-5 *qualitative-preferences* track (there is no qualitative pathways;
the track ran five domains). These instances add PDDL3 `(:constraints ...)`
trajectory PREFERENCES — `always`, `sometime`, `at-most-once`,
`sometime-before`, all `(preference name ...)`-wrapped, all untimed — on top
of soft goals. The metric is each problem's `(:metric minimize …)` over
violated preferences (goal AND constraint preferences share the one
`(is-violated name)` namespace); **lower is better**.

Run one: `ff -o qualpref/<domain>/domain.pddl -f qualpref/<domain>/pNN.pddl`
(the constraint gate lowers each constraint preference to monitor automata +
a goal-side preference, then the PDDL3 metric optimizer prices it — see
`docs/roadmap-0.7.md` Phase 2).

## Reference status (honest gap)

**This board is self-scored.** The official IPC-5 qualitative results
(`IPC5-results.tgz`, the source of the simple-preferences board's SGPlan5
numbers) are unreachable from this container (host not in the network
allowlist), and the SGPlan6 Docker advisory path (`compare.py --cat
qualpref`) needs a Docker daemon this environment doesn't run. Both paths
work from a normal dev machine; when either is run, graft the reference
columns in and recompute W/T/L. Until then this ledger records ferroplan's
own defaults-only numbers and their oracle status — *scoring honestly is the
gate; leading is not* (`docs/roadmap-0.7.md`).

Two facts anchor the numbers even without a reference row:

- **reported == verified, exactly, on every oracle-checked plan.** The
  independent verifier replays the plan over the ORIGINAL problem, folds
  every constraint preference's semantics over the trajectory (never the
  compiled monitors), grounds all inner quantifiers, and recomputes the
  metric. Checked exactly: all five p01s (`tests/ipc5_qual_metric.rs`,
  asserted in CI's heavy tier) plus spot checks on storage p03 (60), storage
  p05 (47, under the documented env), openstacks p05 (122.5), tpp p08 (246),
  trucks p05 (0), rovers p08 (888) — every one equal.
- **Metrics agree at every thread count wherever both complete** (t1 ≡ t8 on
  all 33 instances with both runs inside budget; the largest instances need
  a longer wall budget at 1 thread — budget-bound, never divergent).

## ferroplan, p01–p08 (metric; wall seconds at 8 threads)

| domain | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| openstacks | 66 | 68.6 | 77.8 | 89.2 | 122.5 | 121¹ | 283¹ | 617.7¹ |
| rovers | 86.65 | 33.44 | 58.77 | 42.34 | 317.35² | 66.64² | 99.91 | 888 |
| storage | 0 | 10 | 60 | 78 | 47³ | 90³⁴ | —⁵ | —⁵ |
| tpp | 24 | 42 | 60 | 78 | 156 | 186 | 216 | 246 |
| trucks | 0 | 10 | 1 | 0 | 0 | 5⁴ | —⁶ | —⁶ |
| *t8 secs* | | | | | | | | |
| openstacks | 7.7 | 7.5 | 69 | 65 | 48 | 295 | 354 | 351 |
| rovers | 7.0 | 4.4 | 6.8 | 6.9 | 149 | 165 | 69 | 146 |
| storage | 0.0 | 6.9 | 8.7 | 16 | 186 | 313 | — | — |
| tpp | 4.2 | 6.2 | 7.6 | 8.7 | 15 | 23 | 21 | 25 |
| trucks | 1.2 | 6.8 | 6.8 | 4.8 | 16 | 373 | — | — |

¹ openstacks p06–p08 complete at BOTH thread counts inside a 600 s budget
(equal metrics t1/t8); they exceed the sweep's default 300 s.
² rovers p05/p06 at 1 thread need ~350–400 s (equal metrics).
³ storage p05–p06 run `FF_NO_ESPC=1` (see finding 2 below); p05 completes
at both thread counts (t1: 503 s).
⁴ t1 exceeds the 600 s budget on storage p06 and trucks p06 (t8 metric
shown; budget-bound, not divergent).
⁵ storage p07/p08: no metric — see finding 2 (memory).
⁶ trucks p07/p08: no metric — both thread counts exceed 600 s. The trucks
tail was already the hardest simple-preferences draw (0.6 Phase-4 record:
shared-timeline scheduling out of selection's reach); the qualitative
variants add `sometime-before` ordering constraints on top. The 0.7
Phase-4 gate (temporal selection) is the recorded lever.

## Coverage

**36 of 40 instances produce a plan and a metric** (32 on pure defaults
within 300 s at 8 threads; +3 openstacks within 600 s; +2 storage under the
documented `FF_NO_ESPC=1` env; trucks p06 within 600 s). Every gap has a
named reason: storage p07/p08 exceed 15 GB during grounding (1,147 and more
surviving monitors × the ground action set — finding 2), trucks p07/p08
exceed the 600 s search budget. All 40 parse, gate, and compile with no
rejection.

## The two scaling findings this suite forced (both recorded, one fixed)

1. **Quadratic forall-preferences OOM'd grounding** — storage's `p6A`
   (`forall (?c1 ?c2 - crate ?s1 ?s2 - storearea) (always (imply ...))`)
   expands to thousands of instances, each a monitor with a `When`
   transition on every action; p03+ killed a 15 GB container. FIXED as a
   default: constraint-side static simplification (`constraints.rs`,
   `simplify_static`) drops statically-accepted instances before
   compilation — p05 drops 10,693 of 11,136 — the same `peval_static` move
   that made the simple-preferences storage instances tractable in 0.5.
   `FF_PREF_NO_STATIC=1` restores the blind expansion.
2. **Wide-monitor states break two memory budgets on the storage tail.**
   Even after the drop, the survivors are the genuinely-coupled instances
   (incompatible × connected pairs), and each surviving monitor adds facts
   to every packed state and a `When` transition to every action. Two
   distinct failure points, both exit-137 in a 15 GB container:
   - **p05/p06 (443+ survivors): the ESPC monolithic pass.** One penalized
     tightening-B&B pass exceeds memory before its deterministic eval
     budget bites (a reduced `FF_ESPC_EVAL_BUDGET` does not help — the
     frontier blows inside a single pass). `FF_NO_ESPC=1` completes (p05:
     47, p06: 90) because the closure optimizer's bounded loop replaces
     the tightening pass — that env is recorded per-row above, house
     convention. Memory-bounding the ESPC pass on wide-state tasks is 0.8
     work.
   - **p07/p08 (1,147+ survivors): grounding itself.** The monitor ×
     ground-action product exceeds memory before any search starts; no
     env knob reaches it. The recorded lever is the same one Phase 1
     recorded for the goal-DNF blow-up: a shared END-action construction
     that takes monitors off the per-action transition path
     (`docs/roadmap-0.7.md`).

## Provenance

- Binary: release build at the Phase-2 head (constraint gate + soft
  lowering + constraint-side static simplification + quantifier
  grounding), frozen before the sweep.
- Runs: one per (instance, thread count) ∈ {1, 8} at 300 s defaults;
  every timeout/failure row re-run sequentially on an idle box at 600 s
  (and storage p05–p08 under the documented env). Container wall clock,
  advisory — the metrics, not the times, are the locked quantity; heavy
  locks live in `tests/ipc5_qual_metric.rs`.
- Instances: potassco mirror `ipc-2006/domains/<d>-preferences-qualitative/`
  (`instances/instance-N.pddl` → `pNN.pddl`), see
  `benchmarks/ATTRIBUTION.md`.
