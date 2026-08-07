# Temporal attribution sitting — 0.22 Phase 8 rider

**Provenance.** Sitting run 2026-08-07 against `target/release/ff`
reporting **0.21.0** (0.22-dev tree, post-`012b7ac`). Solo probes:
`nice -n 15`, `FF_TIME_LIMIT=30`, `FF_RES_DEBUG=1`, `FF_NO_ESCALATE=1`,
`FF_TEVAL_BUDGET` ladder 15k/60k/240k. **Box caveat:** three build
agents share this box — wall times are ceilings, never signals; every
read below is eval-count-denominated (user time cited only where the
phase split needs it). `FF_NO_ESCALATE` is part of the probe protocol,
not an engine opinion: escalation re-arms `FF_TEVAL_BUDGET` per retry
and the goal-decomposer pass ignores it entirely, so single-pass is
the only eval-denominated read (receipt: floor-tile 2011 i2 smoke ran
>180 s under a 15k budget with escalation on, 160 ms without).

**Sitting close:** the 15k rung landed for floor-tile-t (both eras)
and sokoban-t before the sitting closed; the 60k/240k rungs and the
driver-log/satellite probes did not complete — those rows are marked
`PENDING` rather than guessed. The probe matrix and per-run raws are
reproducible from the protocol above (script: `probe.sh`, 39-run
matrix, scratchpad of this sitting).

Classes: **PLATEAU** (flat best_h), **GRIND** (descending best_h,
throughput/dedup), **BLOCKED** (b_blocked share growing), **MEM**,
**MIXED** — per the 0.21 basket discipline.

## Per-family attribution

| family (unsolved mass) | instances probed | class | key numbers | 0.23 lever implicated |
|---|---|---|---|---|
| floor-tile-t (35: 16×2011 + 19×2014) | 2011 i2, i6; 2014 i2, i12 @15k; seed 2011 i2 @120k on record | **PLATEAU** | best_h 16/38/24/47 at 15k; seed: best_h 8 flat at 120k, dead_end 2,269; dedup **0.0%** everywhere, b_blocked **0**, doomed **0**, dead_end 4.4–8.9% of evals; RSS 12–16 MB; identical signature both eras | **Clear-chain/serialization** (relaxation blind to paint-self-trap ordering; halving best_h costs ~8× evals with zero dedup — not throughput); symmetry-engine constituency secondary (2 interchangeable robots) |
| sokoban-t (34: 18×2008 + 16×2011) | 2008 i2, i21; 2011 i8 @15k | **MIXED** (grounding wall, then plateau) | i21/i8: alarm-killed >150 s solo **pre-search**, zero search output; i2: ~47 s user pre-search of 54 s total — 8 s stack sample lands in `ground::for_each_binding::rec` + String-key SipHash + malloc churn; push carries **3 location params**, 317–446 objects → typed product ~1e7–1e8; under the wall i2 shows best_h 78 at 15k, dead_end 9.0%, dedup 0, b_blocked 0 | **Phase 7 MCV join ordering** (this cycle) is the first-order unblock — same class as 2048/caldera, and this is the receipt that it pays a temporal board; residual after Phase 7: plateau, clear-chain candidate — re-probe post-MCV |
| driver-log (19×2014) | 2014 i1 @15k (i5 in flight at close; ladder incomplete) | **PLATEAU** (provisional, 1 instance, 1 rung) | i1: best_h 12, dead_end **0**, dedup 0.0%, b_blocked 0, doomed 0 at 15k — h flat with no dead-ends at all, on a 1/20 board floor; 6,040 ops (i1) vs 61,092 ops (i5): 10× op growth two instances in | clear-chain/TRPG-lite constituency on the i1 signature; **finish the ladder** (i5/i13, 60k/240k) before 0.23 freezes — the op-count ramp says grounding must also be watched at i13+ |
| satellite (12×2014) | queued, not reached at close | **PENDING** | 8/20 board, unsolved = {5–10, 15–20}; no eval-denominated read taken | none-known-yet — finish the ladder before 0.23 scopes it |
| storage-t (40: 20×2011 + 20×2014, zero block) | none this sitting (per rider: reuse, don't re-run) | **NONE-KNOWN** | 0/40 three releases deep; 0.20 named it in the deep required-concurrency cluster; no mechanism-precise probe on any record | none — stays honestly unattributed; the IPC-2011 field-results vendoring rider bounds whether the ceiling is shared |
| model-train (30×2008, zero block) | on-record 0.22 scoping (anti-pots) | **PLATEAU** (accounting-exhausted) | numeric charge armed on temporal groundings re-levels plateau 6 → 13 and stays flat at board scale: **683,555 evals, no plan** | structural only: **window/deadline propagation** (numeric resource windows); the h-accounting lever class is exhausted |
| temporal-machine-shop (40, zero block) | on-record 0.22 scoping (anti-pots) | **PLATEAU** (start-spam floor, accounting-exhausted) | pairwise agenda-doom kills **92%** of candidates at birth; best_h re-levels 110 → 180 and stays flat across 4× budget | **symmetry-engine temporal constituency** (goal-paired piece symmetry, the 0.13 fence — already named by Phase 6 for 0.23) |
| turn-and-open (seed row, on record) | 2014 i2 (0.22 scoping) | **GRIND** (dedup-heavy) | **26% duplicates** — the one measured throughput/dedup constituent in the temporal mass | throughput/dedup territory, not h-structure — the contrast class that keeps PLATEAU calls honest |

## What the numbers rule out

- **Window-blocking is not the mechanism on the probed mass:**
  b_blocked = 0 on every completed probe (floor-tile ×4, sokoban i2);
  the BLOCKED class is empty where we could measure.
- **Throughput/dedup is not floor-tile's or sokoban's problem:**
  duplicate share 0.0% on all five completed probes (contrast:
  turn-and-open's 26%). A faster eval loop moves nothing here.
- **MEM is nowhere at probe scale:** 12–16 MB RSS at 15k evals.
- **Sokoban-t re-attribution is the sitting's hard finding:** the
  family has been carried as temporal-search mass, but 2 of 3 probed
  instances never reached search in 150 s solo (grounding), and the
  third spent ~87% of its user time there. Phase 7's gates should
  re-referee sokoban-t before 0.23 buys it any temporal-search lever.

## Feed into 0.23's lever choice (no engine lever taken here)

The measured mass splits: grounding (sokoban-t bulk → Phase 7, this
cycle), clear-chain-shaped plateau (floor-tile-t 35 + sokoban
residual), accounting-exhausted plateaus with named structural levers
(model-train → window/deadline propagation; TMS → symmetry temporal
constituency), one dedup grind (turn-and-open), and an honestly
unattributed zero-block member (storage-t). This supports 0.23's
pre-scoped order — goal-isomorphism symmetry first (TMS + floor-tile
secondary constituency), TRPG-lite second (the clear-chain plateaus
are its constituency) — with the driver-log/satellite ladder to be
finished before that choice is frozen.
