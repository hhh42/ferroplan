# Grounding scale — 0.22 Phase 7 receipts

**Provenance.** Solo probes run 2026-08-07 against the Phase 7 build
(0.22-dev tree, post-Wave-1 base `7496d8e`), `nice -n 15`, one probe at
a time, foreground. Corpus: `benchmarks/.ipc-corpus`; VAL per
`FERROPLAN_VAL`. **Box caveat:** shared build box — wall times are
solo-run but not sweep-grade; the gates below carry generous
pre-registered bars for exactly that reason.

The three levers, all landed behind hatches:

- **MCV join ordering** (`FF_MCV_THRESHOLD` 1e6 per action,
  `FF_NO_MCV_JOIN` hatch): greedy bound-connected most-constrained
  variable order inside `for_each_binding`, survivors SORTED BACK to
  declaration row-major — RawOp stream and fact-intern order
  byte-identical by construction. The golden battery
  (tests/mcv_ground.rs: gripper, tpp-metric i1, sokoban-mini with
  `FF_MCV_THRESHOLD=1` forced) pins it.
- **Threshold-routed fixpoint** (`FF_FIXPOINT_THRESHOLD` 1e8,
  `FF_NO_FIXPOINT_GROUND` hatch): plain solve entries whose
  post-restriction typed product exceeds the bar route through the
  0.12 reached-restricted fixpoint enumeration, MCV active inside.
  `benchmarks/ground-audit.py` asserts the route is vacuous on every
  currently-solved row (agricola i1, 6.3e7, is the near-threshold
  negative control) and that the named gates sit above their bars.
- **Factored goal-check compilation** (`FF_GOAL_FACTOR_THRESHOLD`
  65536, `FF_NO_GOAL_FACTOR` hatch): or-goal products compile as a
  chained per-item check (sum of disjuncts, not product) under a
  PLAN-MODE freeze — sound by the freeze argument, complete by
  construction (tests/goal_factor.rs, RED 262,144 REACH ops → GREEN
  36).

## Pre-registered gates (solo, niced)

| gate | bar | RED (pre-P7, on record) | GREEN (this build) |
|---|---|---|---|
| 2048 i8 grounds | <1 s | 67–74 s of a 60 s budget in binding enumeration (0.22 scoping); honest budget exit since Wave 1 | TBD |
| 2048 i8 search residual at 60 s | report | never reached search | TBD |
| organic-synthesis i01 grounds | <30 s | 0.21 Phase 8 gate missed; fixpoint receipt: memory flat, time the wall | TBD |
| organic-synthesis i11 grounds | <30 s | same | TBD |
| caldera i4 grounds | <10 s | 2,292/2,292 stack samples in binding recursion | TBD |
| sokoban-t 2008 i21 grounds + residual | report | alarm-killed >150 s PRE-SEARCH (sitting table) | TBD |
| sokoban-t 2011 i8 grounds + residual | report | alarm-killed >150 s PRE-SEARCH (sitting table) | TBD |
| block-grouping i3 | report | honest 7.8 s budget exit, unsolved (Wave 1) | TBD |
| block-grouping i13/i20 | report | fast mem-caps (RSS watchdog kills pre-Wave-1) | TBD |
| agricola i1 negative control | unchanged | 246,879 ops on the plain path | TBD |

## Audit

TBD — `python3 benchmarks/ground-audit.py` output summary.
