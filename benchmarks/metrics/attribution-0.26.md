# 0.26 Phase 0 — the proof-gap sitting

Executed 2026-08-27 on the promoted cut binary (`ff 0.25.0`,
`target/release/ff`, not rebuilt), box quiet (load 1.66 at sitting
start, one planner process at a time, no builds). Receipts:
`benchmarks/metrics/probes-0.26/proof-gap/`. The sitting ran in two
legs — the 07:58–08:12 board-replica leg (interrupted) and this
completion leg — all receipts in the one directory. NO code was
changed. The temporal delete-relaxation ledger was not touched: pot 3
is a CEGAR arming question, not temporal h accounting.

**Verdict up front: the centerpiece is NOT refused.** All three pots
yield a named mechanism with a priced band. One cross-cutting finding
falls out for free and is recorded at the bottom: the opt sweeps'
jobs-2 contention is systematically booking in-budget proofs as
node-cap rows.

---

## Pot 1 — onlycraft-opt 2/20 vs its own 20/20 satisficing row

**Mechanism: an admissibility ceiling on the numeric goal bound —
not a node-cap problem and not search order.** The node-cap notes on
the board are the proximate symptom only.

What was run and what it says:

- The board record (`benchmarks/air25/ipc2026-numeric.jsonl`,
  onlycraft-opt rows): i1 PROVEN cost 5 (154 expansions, 0.01 s),
  i2 PROVEN cost 13 (112,348 expansions, 0.26 s), i3–i20 all
  node-capped or wall-killed. The satisficing rows solve all 20 in
  ≤2.74 s, plan lengths 6/17/25/35/43/… (+8–10 per instance).
- The ceiling's name is in the root gate
  (`proof-gap/onlycraft-i3-board.err`): **h^max 0 vs LM-cut 0, num
  root 6**. The classical heuristics are stone blind (all goal
  distance is numeric) and the numfold goal-margin bound (1 goal
  margin, 17 floored ops) reaches 6 — against a true cost around
  19–21 (sat UB 25 for i3; i2's sat-vs-proven ratio is 17→13).
- Memory does not buy it. The 300 s wall run
  (`onlycraft-i3-w300.{json,err,time}`) exhausted the x2 node-cap
  refill at 81.7 s with 276.5 s of wall left — memory binds before
  wall. Forcing the cap to 40M (refilled to 80M):
  `onlycraft-i3-cap40m.{json,err,time}` — 50,283,640 expansions,
  102,336,054 evaluated, 7.4 GB peak RSS, 305.7 s, **still no
  certificate**. That is ~2.4–3.8× the board's node budget for
  nothing.
- The growth rate is measured, not guessed: i1→i2 is 154→112,348
  expansions over +8 cost units ≈ **2.3× per cost unit**.
  Extrapolated to i3 (cost ≈ 20): ~10^8 expansions — exactly where
  the 102M-evaluated no-cert receipt sits.

**Priced band:** with the 2.3×/unit scaling, an admissible numeric
bound reaching within ~3 of true cost prices i3 at ~1–3M expansions —
comfortably inside the 60 s / 8 GiB board budget. But the instance
ladder adds +8–10 cost units per instance, i.e. ~2000× expansions per
step: **+1–2 instances (i3, possibly i4) of the 18 missing**, and no
admissible-h improvement short of near-exact converts the row
wholesale. An honest small band. The lever, if Phase 1 takes it, is
the numfold goal bound's collapse of accumulated-resource distance
(the RED fixture is i3: provable-after means certified at cost ~20).

## Pot 2 — barman-opt 0/14 and parking-opt 0/20

**Mechanism: wall-shape near-misses under the sweep's conditions —
the proofs exist at 0.9–3.4× the 60 s budget, and the front of both
rows is being erased by the jobs-2 contention cliff in the
wall-fraction ladder.** Not an admissibility wall on either domain.

barman (ipc-2014 barman-sequential-optimal):

- Board rows (`benchmarks/air25/ipc2014-opt.jsonl`): all 14
  node-capped at 57.7–58.5 s, 1.0–4.6M expansions, jobs 2 threads 1.
- Solo t1 board-replica at 60 s (`proof-gap/barman-i1-board.*`,
  first leg): LM-cut probe inconclusive → h^max resumes → node cap at
  6,460,152 expansions at 56.9 s with <10% wall left (no refill).
- **At a 300 s wall the same search PROVES i1: cost 49, 6,949,349
  expansions, h^max+orbits, in 52.3 s at t1 and 50.7 s at t6 —
  identical expansion counts, threads irrelevant**
  (`barman-i1-t1-w300.*`, `barman-i1-w300.*`). i2 proves at 55.2 s,
  same cost, 7.5M expansions (`barman-i2-w150.*`). Sat UB for i1 is
  50 (`barman-i1-sat.json`) — the proof lands at 49, so h^max+orbits
  carries a cost-49 certificate at ~7M expansions despite the LM-cut
  root of 9.
- So barman i1/i2 need ~52–55 SOLO seconds ≈ 0.9× the board wall.
  Under the 60 s wall the ladder's slicing plus the node cap tripping
  in the last 3 s books it inconclusive; under jobs-2 contention the
  whole track is pushed past the wall.

parking (ipc-2014 parking-sequential-optimal):

- i3 proves in ~22 s under EVERY solo condition tried: default
  (`parking-i3-board.*`, t1, first leg — 59,031 expansions), t1 with
  no mem env (`parking-i3-t1-nomem.*`), the board's exact
  `FF_MEM_BUDGET_GB=8` (`parking-i3-memgb8.*` — 21.9 s, cost 17), and
  no-sprint (`parking-i3-nosprint.*`, 47.2 s). The banked entries
  baseline agrees (`benchmarks/air25-entries/parking-opt-i3.json`,
  cost 17 at 23.2 s). **The board still booked i3 as node-cap at
  256,527 expansions** (`air25/ipc2014-opt.jsonl` row 3) — the only
  remaining delta is the jobs-2 sibling: contention stretches the
  LM-cut resume past its wall-fraction slice and the h^max fallback
  burns the rest. The mem-budget hypothesis was tested and is dead.
- i1 PROVES cost 18 at 88.2 s, 285,800 expansions, LM-cut
  (`parking-i1-w300.*`). i2 fails the default ladder even at 180 s
  (probe slice trips, h^max resume drowns: `parking-i2-w180.*`) but
  PROVES cost 18 at 203 s with `FF_NO_HMAX_SPRINT=1`
  (`parking-i2-nosprint-w240.*`, 72,404 expansions) — the ladder's
  sprint is holding wall the LM-cut resume needed. LM-cut root 12 vs
  costs 17–18: the heuristic is fine; LM-cut label-pass throughput
  (~20–25 K/s at 4,332 grounded actions) is the whole price. Sat UBs:
  i1 22, i2 22, i4 23 (`parking-i{1,2,4}-sat.json`).

**Priced band:** at the 60 s board, **+1 (parking i3) from
de-contending the opt sweeps alone** (jobs 1 — zero engine change).
At a 300 s wall: **barman +2 proven in hand (i1, i2) and plausibly
+4–6** (i3 shares i1/i2's 4.5M-expansion board profile; i4–i6's 2.5M
profile is nearer the line), **parking +2–3** (i1 and i3 proven in
hand; i2 with the ladder mis-slice fixed — the sprint ceding to the
LM-cut resume is the one code-shaped lever these receipts name).
Deeper instances (parking i5+ at 128K→27K expansions per 60 s,
h^max-path) stay out of any near band.

## Pot 3 — the CEGAR-seeding question

**Answer: proof-shaped horizons ARE separable — in-run, per-rung,
and the engine already narrates the separator.** Not ex ante: no
pre-solve signal distinguishes the SAT rung. But the poison announces
itself at the first STN refutation, and refutations occur ONLY on
SAT-shaped rungs.

Fresh receipts, this sitting:

- `tms-i2-plain.{json,err,time}`: horizons 1, 2, 4, 8 **proven UNSAT
  with zero STN refutations**, then h16 — STN refutation #1 and the
  solve lands at 0.42 s.
- `tms-i2-fwd.{json,err,time}` (`FF_SAT_BRANCH=fwd`): the same h16
  floods — **244 refutations**, the 60 s wall burns, no solve. The
  0.25 poison (recorded as 247) reproduces on the promoted binary.
- `storage-i1-plain.{json,err,time}`: 60 s, **zero `[sat]` lines** —
  the wing never runs at board budget; the ladder eats the wall,
  exactly as the 0.25 exhaustion-arm note said.
- The 0.25 receipts already established the other half: storage-t's
  h1–32 are pure UNSAT proofs (re-verified under the sound pairing
  guard, roadmap-0.25.md Phase 3), and the profiling read's own
  words — "mc's 200k-conflict grind has zero refutations, storage's
  wall is deep-proof conflict counts."

So the separable arming is: **arm `fwd` seeding per rung; on the
first STN refutation, disarm and restart the rung unseeded.**
Pure-proof stacks (storage h1–32, TMS h1–8) never trigger the
tripwire and keep the full gradient; the SAT rung pays one refutation
detection (~1 s on TMS) and then solves unseeded in 0.4 s.

**Priced band: the measured 1.8× on the deep UNSAT-proof stack
(storage-t h1–32, 6.8 s → 3.7 s, 0.25 receipt) retained with the
SAT-side poison fenced.** Priced honestly: at the 60 s board the wing
is starved (the zero-`[sat]`-lines receipt), so this converts board
rows only where the wing gets wall (300 s tracks, or behind the
conditional ladder-reserve 0.25 declined to take blanket). As a
proof-stack speedup it is real, measured, and now armable.

---

## Cross-cutting finding (recorded for the cut ledger)

The pot-2 receipts convict the sweep conditions, not just the engine:
**parking i3 and the barman i1-class prove inside or within a whisker
of the 60 s budget solo, and the jobs-2 opt boards booked every one
of them as node-cap rows.** The wall-fraction ladder turns a 2×
throughput loss into a 0/1 proof loss — a cliff, not a slope. The
opt boards should sweep at jobs 1 (or the ladder should slice by
CPU-time, not wall). This is a standing-corrections-shaped item in
the 0.26 Phase 2 mould: coverage moves (+1 minimum) from runner
conditions alone.

## Exit clauses

- Pot 1: named (numeric-bound admissibility ceiling), banded (+1–2).
  Not refused.
- Pot 2: named (wall-shape near-miss + contention cliff; ladder
  mis-slice on parking i2), banded (+1 at 60 s from conditions alone;
  +4–9 across both domains at 300 s). Not refused.
- Pot 3: named (STN-refutation-gated separability), banded (1.8×
  proof-side, poison fenced). Not refused.
- The centerpiece proceeds to Phase 1 pricing on these three
  mechanisms. The refusal clause was not needed.
