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
  metric. Checked exactly: all five p01s plus storage p05 on pure defaults
  (`tests/ipc5_qual_metric.rs`, asserted in CI's heavy tier) plus spot
  checks on storage p03 (60), openstacks p05 (122.5), tpp p08 (246),
  trucks p05 (0), rovers p08 (888), and — new in 0.8 — storage p07 (200)
  and p08 (261) via `examples/verify_plan.rs` — every one equal.
- **Metrics agree at every thread count wherever both complete** (t1 ≡ t8 on
  all 34 instances with both runs inside budget — of the 36 with a metric,
  only storage p06 and trucks p06 lack a completed t1 run; the largest
  instances need a longer wall budget at 1 thread — budget-bound, never
  divergent).

## ferroplan, p01–p08 (metric; wall seconds at 8 threads)

| domain | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| openstacks | 66 | 68.6 | 77.8 | 89.2 | 122.5 | 121¹ | 283¹ | 617.7¹ |
| rovers | 86.65 | 33.44 | 58.77 | 42.34 | 317.35² | 66.64² | 99.91 | 888 |
| storage | 0 | 10 | 60 | 78 | 47³ | 90³⁴ | 200³⁵ | 261³⁵ |
| tpp | 24 | 42 | 60 | 78 | 156 | 186 | 216 | 246 |
| trucks | 0 | 10 | 1 | 0 | 0 | 5⁴ | —⁶ | —⁶ |
| *t8 secs* | | | | | | | | |
| openstacks | 7.7 | 7.5 | 69 | 65 | 48 | 295 | 354 | 351 |
| rovers | 7.0 | 4.4 | 6.8 | 6.9 | 149 | 165 | 69 | 146 |
| storage | 0.0 | 6.9 | 8.7 | 16 | 186 | 313 | ⁵ | ⁵ |
| tpp | 4.2 | 6.2 | 7.6 | 8.7 | 15 | 23 | 21 | 25 |
| trucks | 1.2 | 6.8 | 6.8 | 4.8 | 16 | 373 | — | — |

¹ openstacks p07/p08 exceed the sweep's default 300 s but complete at BOTH
thread counts inside a 600 s budget (equal metrics t1/t8); p06 completes at
t8 just under the default budget (294 s) and needs ~530 s at t1.
² rovers p05/p06 at 1 thread need ~350–400 s (equal metrics).
³ storage p05–p08 ran `FF_NO_ESPC=1` in the 0.7 sweep; **since 0.8 the
same metrics reproduce on PURE DEFAULTS** (ESPC no longer engages on
monitor artifacts — finding 2 below). p05 completes at both thread counts
(0.7 sweep t1: 503 s).
⁴ t1 exceeds the 600 s budget on storage p06 and trucks p06 (t8 metric
shown; budget-bound, not divergent).
⁵ storage p07/p08: **first covered in 0.8** — the 0.7 sweep had no metric
(15 GB grounding OOM; finding 2). Measured on the 0.8 box (4 cores,
defaults, within the 600 s budget): p07 metric 200 (grounds in 313 ms at
109 MB peak), p08 metric 261 (676 ms / 174 MB); both spot-checked
reported == verified exact. Wall times are from the 4-core box and are
not comparable to this table's 8-thread column; t1 re-runs are
budget-bound there, never divergent.
⁶ trucks p07/p08: no metric — both thread counts exceed 600 s. The trucks
tail was already the hardest simple-preferences draw (0.6 Phase-4 record:
shared-timeline scheduling out of selection's reach); the qualitative
variants add `sometime-before` ordering constraints on top. The 0.7
Phase-4 gate (temporal selection, carried to 0.9) is the recorded lever.

## Coverage

**38 of 40 instances produce a plan and a metric on pure defaults**
(since 0.8): 33 within 300 s at 8 threads, +2 (openstacks p07/p08) within
600 s, +1 (trucks p06) within 600 s, +2 (storage p07/p08, first covered
in 0.8) within 600 s on the 0.8 measurement box. Every remaining gap has
a named reason: trucks p07/p08 exceed the 600 s search budget. All 40
parse, gate, and compile with no rejection. (The 0.7 ledger read 36/40
with storage p05/p06 under a documented `FF_NO_ESPC=1` env and p07/p08
uncovered — both walls fell in 0.8; see the findings.)

## The two scaling findings this suite forced (recorded 0.7; retired 0.8)

1. **Quadratic forall-preferences OOM'd grounding** — storage's
   crate²×storearea² always-preference (`forall (?c1 ?c2 - crate ?s1 ?s2 -
   storearea) (always (imply ...))`, named `p6A` in p03 and `p8A` in p05)
   expands to thousands of instances, each a monitor with a `When`
   transition on every action; p03+ killed a 15 GB container. FIXED as a
   default in 0.7: constraint-side static simplification (`constraints.rs`,
   `simplify_static`) drops statically-accepted instances before
   compilation — p05 drops 10,693 of 11,136 — the same `peval_static` move
   that made the simple-preferences storage instances tractable in 0.5.
   `FF_PREF_NO_STATIC=1` restores the blind expansion.
2. **Wide-monitor states broke two memory budgets on the storage tail —
   both retired in 0.8** (`docs/roadmap-0.8.md` Phases 2–3). As recorded
   in 0.7, the survivors of the static drop each added facts to every
   packed state and a `When` transition to every action, producing two
   distinct exit-137s in a 15 GB container:
   - **p05/p06 (443+ survivors): the ESPC monolithic pass.** One penalized
     tightening-B&B pass exceeded memory before its deterministic eval
     budget bit. Root cause found in 0.8: ESPC's deadline-pair detection
     was pairing MONITOR ARTIFACTS (every action conditionally adds
     monitor bits that appear in the priced preferences' collect
     preconditions), engaging the pass on tasks with no real once-only
     achievement structure. Since 0.8 the shared monitor block is not
     scanned for deliverables, these tasks take the closure optimizer on
     pure defaults — p05: 47, p06: 90, the exact `FF_NO_ESPC=1` metrics —
     and `FF_ESPC_TRAJ_PAIRS=1` restores the old pairing. A deterministic
     search node cap (8 GiB byte model, `FF_SEARCH_NODE_CAP`) now also
     backstops any wide-state pass.
   - **p07/p08 (1,147+ survivors): grounding itself.** The monitor ×
     ground-action product exceeded memory before any search started.
     Retired in 0.8 by the shared monitor block: the transition block is
     byte-identical across every ground op, so it is ground ONCE and
     shared (`Domain.monitors` + per-op bits; `FF_NO_COND_SHARE=1`
     restores per-op copies) — p07 grounds in 313 ms at 109 MB peak, p08
     in 676 ms at 174 MB, and both produce first-ever metrics
     (200 / 261, reported == verified exact).

## Provenance

- Binary: p01–p08 columns from the 0.7 Phase-2-head sweep (release,
  frozen); the 0.8 additions (storage p05–p08 defaults confirmation and
  the first p07/p08 rows) from the 0.8 Phase-3 head on a 4-core / 15 GB
  box — metrics identical where both measured, walls not comparable.
- Runs: 0.7 sweep — one per (instance, thread count) ∈ {1, 8} at 300 s
  defaults; every timeout/failure row re-run sequentially on an idle box
  at 600 s (storage p05–p08 then under the documented env, since 0.8 on
  pure defaults). Container wall clock, advisory — the metrics, not the
  times, are the locked quantity; heavy locks live in
  `tests/ipc5_qual_metric.rs`.
- Instances: potassco mirror `ipc-2006/domains/<d>-preferences-qualitative/`
  (`instances/instance-N.pddl` → `pNN.pddl`), see
  `benchmarks/ATTRIBUTION.md`.
