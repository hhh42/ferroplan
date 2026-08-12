# Temporal attribution sitting — 0.23 Phase 6, the blind spot

**Provenance.** Sitting run 2026-08-10 against `target/release/ff`
reporting **0.22.0** (0.23-dev `main` @ `0576f1a`, wave-1 integrated).
Solo sequential probes: `nice -n 15`, board env (`FF_TIME_LIMIT=60`,
`FF_MEM_BUDGET_GB=6`, `--json --threads 1`), `FF_NO_ESCALATE=1`,
`FF_RES_DEBUG=1`, `FF_TEVAL_BUDGET` ladder **15k/60k/240k**, external
kill at 65 s. Idle% recorded per run: 76–90% on most runs; the
exceptions (satellite i5@240k at 51%, i16@60k at 68%, two
parc-printer rows at 56–74%) carry eval-denominated reads only.
**Box caveat** stands: wall times are ceilings, never signals; every
read below is eval-count-denominated.
Protocol notes earned this sitting: (1) the eval budget is consumed
by pass 1 — passes 2–4 open with budget 0 and read `best_h
2147483647`, so all numbers below are single-pass, per the 0.22 rule;
(2) the temporal path does **not** check `FF_TIME_LIMIT` mid-pass —
satellite i9@240k hit its eval cap at 86.6 s wall (the 65 s watchdog
killed the wrapper; the orphaned pass finished and the eval-
denominated read is intact) — a 240k rung on a ≥25k-op grounding is
not a board-shaped run, and is marked where it happened.

**Sitting close: complete.** Every planned rung landed. The 0.22
sitting's `PENDING` satellite row is CLOSED and driver-log's
provisional ladder is FINISHED — both below. Probe script and
per-run raws: `probe.sh`, 34-run matrix, scratchpad of this sitting.

Classes: **PLATEAU** (flat best_h), **GRIND** (descending best_h,
throughput/dedup), **BLOCKED** (b_blocked share growing), **MEM**,
**SCALE**, **MIXED** — the 0.21 basket discipline, 0.22 vocabulary.

## Per-family attribution

| family (unsolved mass) | instances probed | class | key numbers | 0.24 lever implicated |
|---|---|---|---|---|
| transport-t 2008 (26: {3–10, 13–30}; 4/30 held) | i3 @15k/60k/240k; i13 @15k/60k; i23 @15k | **PLATEAU** | i3: best_h **12 → 12 → 10** across 16× budget, dedup 13.1% → 16.8% → 19.8%; i13: 32 → 32 flat, dedup 0; i23: 50 @15k — and its pruned/masked passes exit at the root (goal-masked relaxation reads S₀ dead; only the unmasked pass searches — a fuel-visibility signature, recorded not diagnosed); dead_end **0**, b_blocked **0**, doomed **0** on every completed pass; ~29k evals/s, RSS ≤458 MB @240k; fv 48–79 (fuel fluents) | **TRPG-lite re-probe first** (in-tree this cycle; a fuel-blind flat-h plateau with zero dead ends is the clear-chain class); if flat post-TRPG, **window/deadline propagation** (numeric resource windows — model-train's named lever; this family is its fuel-chain cousin). Field LB ≥12/30 (LPG-td), so ~8 of the 26 are field-winnable mass |
| parc-printer-t (25: 12×2008 {12–20, 25, 29, 30} + 13×2011 {4–10, 15–20}) | 2008 i12 @15k/60k/240k, i20 @15k, i30 @15k/60k; 2011 i4 @15k/60k, i15 @15k | **PLATEAU** | i12: best_h **14 flat at all three rungs** on a 550-op grounding, dedup 0.0%, dead_end 0; i30: 20 flat 15k→60k with dead_end **23.8% → 8.7%** (front-loaded dead-end mining, h still never moves); i20: 126, 2011 i4: 72 flat 15k→60k, i15: 132; b_blocked **0** everywhere; RSS ≤150 MB | **None-known — field-SAT territory, now double-receipted:** Phase 4's orbit scan found ZERO candidate groups (sheet profiles distinguish every sheet), and the field file has ITSAT 20/20 where OPTIC/TFD score 0/20 (LPG-td 7/20 on 2011). Joins storage-t in the style-mates cluster; no heuristic-forward lever is pitched against machinery we do not have |
| rtam-2014 (17: {3–8, 10–20}; 3/20 held, solved plans run 438–567 steps) | i3 @15k/60k/240k; i10, i16 @15k | **GRIND** (decelerating) | i3: best_h **552 → 528 → 498** (−54 over 16× budget; descent rate 0.53 → 0.19 per 1k evals), dedup steady 11.3–12.1%; i10: 663 @15k (10,086 ops); i16: 717 @15k (13,176 ops); dead_end **0**, b_blocked **0**; throughput ~15k evals/s (i3, 3,332 ops) falling to ~4.5k/s at i16's op ramp — the 60 s wall affords ~0.3–1M evals and goal contact at the measured descent rate is millions away | **Throughput class, not h-structure** — h descends the whole ladder on a family whose solved plans run 450–567 steps; the lever is per-eval cost at the 10–13k-op ramp (temporal h-build per eval), the turn-and-open contrast class. No structural lever implicated; dedup ~12% says canonical-agenda work already lands here |
| satellite-2014 (12: {5–10, 15–20}; the 0.22 PENDING row — **CLOSED**) | i5 @15k/60k/240k; i9 @15k/60k/240k; i16 @15k/60k | **MIXED** (grind + hard plateau in one family) | i5: best_h **124 → 48 → 24** — halving per 4× budget, extrapolates to goal contact ~1–2M evals vs the ~300k a wall affords at 5.5k evals/s (60k rung, 88% idle; 17,676 ops); i9: best_h **253 flat at all three rungs** (232k evals @240k, dedup ≤3.3%) — a hard plateau two instances up the same subseries; i16: 163 flat 15k→60k (42,596 ops, 30 s for the 60k rung — wall ceiling noted, idle 68%); dead_end **0**, b_blocked **0** everywhere; RSS 806 MB @240k (i5) | Split lever, honestly: **i5-class rows are throughput** (the descent is real; a faster eval loop converts the low end of {5–10}); **i9/i16-class rows are h-structure** — flat best_h with zero dead ends and zero blocking is the clear-chain signature, TRPG-lite's constituency. Field says the family is winnable (LPG-td 20/20 re-run, field-average ≈64%) — this is the sitting's best conversion candidate either way |
| driver-log-2014 (19: all but i2; the 0.22 provisional — **ladder FINISHED**) | i1 @60k/240k (15k on 0.22 record); i5 @15k/60k; i13 @15k/60k | **MIXED** (plateau floor + size-priced grind) | i1: best_h **12 flat at 15k/60k/240k** — the 0.22 provisional PLATEAU is now ladder-complete; dedup 0.0%, dead_end 0, b_blocked 0 at every rung; i5: **181 → 106** descending 15k→60k but at **1.0–1.1k evals/s** on a 61,092-op, 512-word grounding — the 60k rung consumed 55.9 s of a 60 s wall solo, and the 240k rung is wall-infeasible (≥220 s), recorded, not guessed; i13: **84 → 78** (46,656 ops, 39 s for 60k); RSS **1.55 GB at 60k evals** (i5) — the 6 GB board budget prices out ~240k evals at ramp sizes; grounding is NOT the wall (passes start <1.5 s in) | Two named: the **i1-class plateau is TRPG-lite constituency** (clear-chain, per the 0.22 call — now with the full ladder behind it); the ramp is **per-eval cost + node economy at 45–61k ops / 393–512 words** (SCALE-shaped: h descends wherever evals are affordable). Matches the organizers' field note verbatim: "hard by SIZE, not concurrency" (LPG-td 14/20, field-average ≈12%) |

## What the numbers rule out

- **Window-blocking is still not the mechanism anywhere we can
  measure:** b_blocked = **0** on all 34 completed probes across five
  families. Two sittings, zero BLOCKED members.
- **Agenda-dooming is absent:** doomed = 0 everywhere — none of these
  families has TMS's start-spam shape.
- **Dedup is not the wall:** the highest share on any probe is
  transport i3's 19.8% @240k (vs turn-and-open's 26% contrast row);
  parc-printer and driver-log sit at 0.0% at every rung.
- **Grounding is not the wall in this basket** — every family reaches
  search in <1.5 s (contrast: sokoban-t, which Phase 6's wall +
  re-referee owns). Driver-log/satellite ramps pay per-eval and
  per-node, not per-binding.
- **MEM is a co-factor only at the driver-log ramp** (1.55 GB @60k
  evals on i5; 979 MB–1.2 GB on i13) — everywhere else ≤806 MB at
  240k. No probe died on memory.

## Feed into 0.24's lever choice (no engine lever taken here)

The blind spot decodes into the same two territories the 0.22
sitting mapped, plus one honest zero: **h-structure plateaus with
zero dead ends** (transport-t 26, parc-printer's numbers minus its
field verdict, satellite's i9/i16 half, driver-log's i1 floor) —
TRPG-lite's constituency, so its 0.23 receipts adjudicate most of
this mass and the 0.24 pitch should be read against them;
**throughput/SCALE grinds** (rtam 17, satellite's i5 half,
driver-log's ramp) where best_h visibly descends and the wall is
per-eval cost at 10–61k grounded ops — an eval-loop/node-economy
lever, not a heuristic one; and **parc-printer-t as the named
zero** — double-receipted field-SAT territory (ITSAT 20/20, orbit
scan zero groups), priced against style-mates like storage-t, no
lever promised. Satellite is the cheapest board win on the table:
half its unsolved mass is a measured, converging grind.
