# IPC-5 (2006) qualitative-preferences scoreboard — ferroplan vs the field

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

## Reference status: GRAFTED from the official archive

The reference gap this board carried for three cycles is CLOSED
(2026-07-24). The official `IPC5-results.tgz` is now **vendored at
`benchmarks/IPC5-results.tgz`** — retrieved by hand from the old
Brescia site's live redirect after the Wayback Machine proved to have
captured only a 301 for the file, never its bytes. Reference metrics
below are read from the archive's per-instance `; MetricValue`
headers (`RESULTS/<planner>/<domain>/QualitativePreferences/pNN.soln`).
The parser is cross-validated against the simple-preferences board:
it reproduces every committed SGPlan5 row there EXACTLY.

The qualitative field was **SGPlan5** (the track winner; full 20/20
coverage in all five domains), **HPlan-P** (70/100), **MIPS-XXL**
(16/100), and **MIPS-BDD** (16/100). YochanPS did not enter this
track. Ferroplan's own numbers remain defaults-only, verified as
before (reported == verified on every oracle-checked plan).

## The field, p01–p08 (grafted; lower is better; **bold** = ferroplan ≤ SGPlan5)

| openstacks | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 | track cov. |
|---|---|---|---|---|---|---|---|---|---|
| ferroplan (0.16) | **66** | 68.6 | 77.8 | 89.2 | **122.5** | 121 | **283** | **617.7** | 8/8 run |
| SGPlan5 | 70 | 62.4 | 77 | 82.4 | 123.5 | 116.5 | 300 | 619.2 | 20/20 |
| HPlan-P | 76 | 71.2 | 88.8 | 94.2 | 147.5 | 144.5 | 294 | 618.5 | 18/20 |
| MIPS-XXL | 14 | 11.6 | — | — | — | — | — | — | 2/20 |
| MIPS-BDD | 68 | 66 | — | — | — | — | — | — | 2/20 |

| rovers | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 | track cov. |
|---|---|---|---|---|---|---|---|---|---|
| ferroplan (0.16) | **68.04** | **32.67** | **29.19** | **26.06** | 238.66 | **37.39** | **37.64** | **556** | 8/8 run |
| SGPlan5 | 88.08 | 40.44 | 39.31 | 43.43 | 236.32 | 75.43 | 87.96 | 674 | 20/20 |
| HPlan-P | 111.63 | 40.44 | 29.19 | 40.17 | 160.97 | 82.76 | 107.41 | 620 | 14/20 |
| MIPS-XXL | — | — | — | — | — | — | — | — | 0/20 |

| storage | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 | track cov. |
|---|---|---|---|---|---|---|---|---|---|
| ferroplan (0.16) | **0** | **1** | **2** | **5** | **47** | **90** | 200 | 261 | 8/8 run |
| SGPlan5 | 8 | 13 | 26 | 39 | 104 | 160 | 183 | 251 | 20/20 |
| HPlan-P | 0 | 1 | 17 | 36 | 78 | 149 | 240 | 337 | 14/20 |
| MIPS-XXL | 0 | 1 | 10 | 44 | — | — | — | — | 4/20 |
| MIPS-BDD | 0 | 1 | 2 | 15 | — | — | — | — | 4/20 |

| tpp | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 | track cov. |
|---|---|---|---|---|---|---|---|---|---|
| ferroplan (0.16) | **13** | **10** | **26** | **29** | **23** | **41** | 57 | **93** | 8/8 run |
| SGPlan5 | 13 | 12 | 32 | 32 | 27 | 64 | 49 | 126 | 20/20 |
| HPlan-P | 13 | 10 | 27 | 31 | 53 | 59 | 86 | 142 | 20/20 |
| MIPS-XXL | — | 33 | 52 | 73 | 199 | 229 | 273 | 317 | 9/20 |
| MIPS-BDD | 13 | 10 | 33 | 67 | 156 | 186 | 216 | 246 | 9/20 |

| trucks | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 | track cov. |
|---|---|---|---|---|---|---|---|---|---|
| ferroplan (0.16) | **0** | **1** | **0** | 2 | **0** | 4 | — | — | 6/8 run |
| SGPlan5 | 0 | 2 | 0 | 0 | 0 | 3 | 3 | 7 | 20/20 |
| HPlan-P | 0 | 1 | 5 | — | 13 | — | — | — | 4/20 |
| MIPS-XXL | 0 | — | — | — | — | — | — | — | 1/20 |
| MIPS-BDD | 1 | — | — | — | — | — | — | — | 1/20 |

## W/T/L vs SGPlan5 (the winner), p01–p08

Re-measured 2026-07-25 with the current engine (the first graft,
computed against the stale 0.8-era ledger, read 12W/3T/23L — that
verdict measured a planner seven cycles old and is superseded):

| domain | W | T | L | note |
|---|---|---|---|---|
| openstacks | 4 | 0 | 4 | dead even with the winner |
| rovers | 7 | 0 | 1 | ahead of the winner (p05 the lone loss, 238.7 vs 236.3) |
| storage | 6 | 0 | 2 | ahead of the winner (p07/p08 the tail losses) |
| tpp | 6 | 1 | 1 | ahead of the winner (p07 the lone loss, 57 vs 49) |
| trucks | 1 | 3 | 2 | plus p07/p08 ferroplan no-run (coverage gap) |
| **total** | **24** | **4** | **10** | + 2 no-runs |

The honest sentences: **ferroplan, on today's defaults, beats the
IPC-5 qualitative-preferences winner 24–10 across the 38 comparable
instances, winning three domains outright (rovers, storage, tpp),
splitting openstacks, and trailing only on trucks** — while beating
HPlan-P/MIPS-XXL/MIPS-BDD broadly on coverage and quality. A
correction, on the record: the first graft, scored against the stale
ledger, attributed a tpp rout to an all-forgo plateau (the stale row
coincided exactly with MIPS-BDD's) — that DIAGNOSIS described the
0.7/0.8-era engine faithfully, but the machinery that retired it
(the 0.5.1 barrier default and the 0.6 selection layer maturing
through 0.10's DNF static resolution) had already shipped; the
board simply had never been re-measured. What remains is small and
named: tpp p07 (57 vs 49), trucks p04/p06 quality (2 vs 0, 4 vs 3),
and the trucks p07/p08 600 s no-runs (⁶ below).

Two facts anchor the numbers even without a reference row:

- **reported == verified, exactly, on every oracle-checked plan.** The
  independent verifier replays the plan over the ORIGINAL problem, folds
  every constraint preference's semantics over the trajectory (never the
  compiled monitors), grounds all inner quantifiers, and recomputes the
  metric. `tests/ipc5_qual_metric.rs` asserts reported == verified on
  all five p01s in CI's heavy tier (value-independent, so engine
  improvements keep it green; the p01 regression CEILINGS in the same
  file are re-locked to the current metrics), and the 0.7/0.8 spot
  checks via `examples/verify_plan.rs` (storage p03/p05/p07/p08,
  openstacks p05, tpp p08, trucks p05, rovers p08) all verified exact
  against the plans of their day.
- **Metrics agree at every thread count wherever both complete** (t1 ≡ t8 on
  all 34 instances with both runs inside budget — of the 36 with a metric,
  only storage p06 and trucks p06 lack a completed t1 run; the largest
  instances need a longer wall budget at 1 thread — budget-bound, never
  divergent).

## ferroplan, p01–p08 (metric; wall seconds, 4-core box, pure defaults)

Re-measured 2026-07-25 (the 0.16 standings cycle) with the 0.15.0
binary — defaults only, 600 s cap per instance, one sweep. **The
previous ledger was the 0.7/0.8-era engine and had gone badly stale
on three domains**: seven cycles of engine work (the 0.10
precondition-DNF static resolution is the likely prime mover on the
`imply`/`exists`-heavy preference compilations, with the richer-h
and optimizer maturation behind it — per-cycle attribution not
reconstructed; this is a standings ledger, not a bisect) had
silently taken rovers 86.65→68.04 (p01) … 888→556 (p08), storage
p02–p04 10/60/78→**1/2/5**, and tpp — the board's recorded rout —
24/42/60/78/156/186/216/246 → **13/10/26/29/23/41/57/93**. The
0.8-era numbers survive in this file's git history.

| domain | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| openstacks | 66 | 68.6 | 77.8 | 89.2 | 122.5 | 121 | 283 | 617.7 |
| rovers | 68.04 | 32.67 | 29.19 | 26.06 | 238.66 | 37.39 | 37.64 | 556 |
| storage | 0 | 1 | 2 | 5 | 47 | 90 | 200 | 261 |
| tpp | 13 | 10 | 26 | 29 | 23 | 41 | 57 | 93 |
| trucks | 0 | 1 | 0 | 2 | 0 | 4 | —⁶ | —⁶ |
| *secs* | | | | | | | | |
| openstacks | 5.8 | 5.8 | 68 | 62 | 43 | 245 | 313 | 277 |
| rovers | 29 | 23 | 37 | 38 | 125 | 52 | 65 | 130 |
| storage | 0.0 | 0.1 | 19 | 24 | 126 | 178 | 322 | 328 |
| tpp | 0.1 | 10 | 17 | 19 | 48 | 57 | 68 | 90 |
| trucks | 0.1 | 19 | 15 | 42 | 61 | 422 | — | — |

⁶ trucks p07/p08: still no metric — the 600 s cap is exceeded. The
trucks tail was already the hardest simple-preferences draw (0.6
Phase-4 record: shared-timeline scheduling out of selection's
reach); the qualitative variants add `sometime-before` ordering
constraints on top. The 0.7
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
