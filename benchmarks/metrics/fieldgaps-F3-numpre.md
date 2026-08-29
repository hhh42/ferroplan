# F3 §1 — `FF_NUMPRE_TEMPORAL`: the numeric-precondition charge on temporal groundings (2026-08-29)

Spec: `docs/field-gaps-execution-0.26.md` F3 §1. Gate: opened by Sitting C
(`fieldgaps-C-metrictime.md`, "numeric-accumulation relaxation blindness …
outside the closed temporal h-accounting ledger").

## Build

One armed line, `ground.rs` packed-task constructor:
`charge_pre_num: !stratified || std::env::var("FF_NUMPRE_TEMPORAL").is_ok()`.
Unset is bit-identical (short-circuit). `FF_NO_NUMPRE` stays the deep restore
(`heuristic.rs` gate). Stale comments in `packed.rs` / `heuristic.rs` updated;
the `FF_H_ENDGATE` / `FF_TRPG` co-fire is declared untested in source.

## Referee — the 2006 metric-time constituency, armed, 30 s, solo

Rows: `benchmarks/air26-probes/numpre-temporal/{pathways,tpp,rovers}-metric-time.jsonl`
(engine `3f18a8beca1fc5b0`, `FF_NUMPRE_TEMPORAL=1 FF_TIME_LIMIT=30`, perl alarm 50).

| board | banked (unset) | armed | movement |
|---|---|---|---|
| pathways-metric-time | 0/30 | **1/30** | i1: unsolved at 26.98 s → solved 1.52 s, 12 steps, makespan 2.0 |
| tpp-metric-time | 3/40 | 3/40 | i1–i3 both ways |
| rovers-metric-time | 5/40 | 5/40 | i1, i2, i4, i7, i8 both ways |

The RED fixture, pathways i2, does not convert: armed, its first pass
evaluates 1.3 M states in 7.5 s (unset: 11.8 k in 55 ms) and the ladder still
ends at the grounding checkpoint with no verdict — the charge gives the pass a
gradient to spend, not one that reaches the goal. i3–i30 the same.

Solo spot reads before the probe (same binary): pathways i1 unset 26.98 s
unsolved / armed 1.52 s solved; tpp i4 solved both ways (8.04 s unset, 9.58 s
armed); rovers i3 unsolved both ways.

**Contention note.** Timberborn (Steam) started at 10:46, six minutes into the
probe (~150 % CPU for the rest of it). pathways i1–i12 ran clean; tpp and
rovers are contended reads. They match the banked coverage row for row; the
one row that moved under contention is tpp i4 (solo-solved both ways at
8–10 s, timed out at 30 s in the probe) — a near-wall row that is wall-flaky,
not flag-sensitive.

## The mandatory quality rider — FIRED

Village workshop (`benchmarks/village/workshop.pddl`), plan length:

| arm | steps |
|---|---|
| unset | 25 (carve plan: forge chisel, carve two decoys, sell) |
| `FF_NUMPRE_TEMPORAL=1` | **47** (chisel-sale plan, never carves) |
| … + `FF_NUMPRE_NODAMP=1` | 47 |
| … + `FF_NUMPRE_NOSKIP=1` | 47 |
| … + `FF_NUMPRE_NOSUM=1` | 47 |
| … + `FF_NO_NUMPRE_CHAIN=1` | 47 |
| … + `FF_NUMPRE_DEPTH=0` | 47 |

The 0.21 negative recurs exactly, and none of the 0.22/0.24 damping and chain
halves touch it: on this task the re-route is the charge's SHAPE on temporal
snap tasks, not the a1 over-count the damping corrected.

## Verdict

Hatch kept, **opt-in**, priced at +1 (pathways i1). No board arms it; the
workshop re-route rules out default-on on temporal groundings until a charge
that does not fire it exists. `tests/numpre_temporal.rs` pins the unset carve
plan (25 steps, forge before carve) and the armed re-route (longer than
unset) — a disappearing re-route re-opens the referee, it does not promote.

Open thread for Sitting C's ledger: the pathways i2+ failure survives a
working gradient (1.3 M evals in the first pass) — the mechanism is the
charge's reach, consistent with the AIBR reading (gap-denominated estimates
non-flat by construction) being the next build, not more charge tuning.
