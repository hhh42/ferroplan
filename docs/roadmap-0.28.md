# ferroplan 0.28 roadmap — the rung 0.27 skipped

Scoped 2026-09-06, while the 0.27 cut sweep was still running. **This cycle
does not merge to `main` until 0.27.0 is published**: the cut is measuring
`ff 0.27.0 [86302e06d81b]`, and an engine change on `main` would re-identify
the binary the sweep is halfway through. All 0.28 work lives on
`engine-0.28` in its own worktree (`/Users/harold/ferroplan-0.28`, its own
`target/`) so a build here cannot touch the binary the sweep spawns.

The headline is not a new mechanism. It is a lever this project already
built, already measured, and wired into three of its four search rungs.

---

## The finding this cycle is built on

0.27 added the anchored successor generator (`PackedTask::applicable_ops`,
`packed.rs`): expansion walks the state's true facts through an index
instead of testing every grounded operator. It was wired into the
classical rung (`search.rs:1038`), the LAMA rung (`lama.rs:343`) and the
novelty rungs (`novelty.rs:317/467/784`).

It was **not** wired into the temporal rung. `temporal.rs:3291` is still

    (0..task.n_ops).filter(|&oi| allow(oi)).collect()

and `grep -c applicable_ops crates/ferroplan/src/temporal.rs` returns 0.

That omission ran a controlled experiment nobody designed. Between
`benchmarks/air26/` and `benchmarks/air27/` there is exactly one engine
commit (`git log 677eafc..HEAD -- crates/ferroplan/src` → `69cb96a`), and
on the boards the 0.27 sweep has finished:

| board | 0.26 | 0.27 | delta | rung wired |
|---|---:|---:|---:|---|
| ipc2014-sat | 161 | 173 | **+12** | yes |
| ipc2014-agile | 158 | 172 | **+14** | yes (board still owed rows) |
| ipc67-netben | 268 | 270 | +2 | yes |
| ipc2026-opt | 22 | 23 | +1 | yes |
| **ipc2014-tempo** | **78** | **78** | **±0** | **no** |

Two things follow, and the second one matters as much as the first.

1. **The lever has an in-house price.** The identical change bought +12 and
   +14 where it was wired. Its temporal constituency is real: 83 of the
   122 unsolved rows on `ipc2014-tempo` carry the note "temporal ladder
   stopped at the wall" — storage-t 18/20, floor-tile-t 18/19, rtam 14/16,
   driver-log 10/19.
2. **It rules out the instrument as the explanation for 0.27's gains.**
   The packed scheduler applied to every board equally. If the movement
   were width, `ipc2014-tempo` would have moved too. It did not.

---

## Phase 0 — the entry gate (blocking, no cycle work until it closes)

1. **cut27 reaches a terminal state.** Five of thirty-two boards are
   `.done`; no 0.28 row claim is refereeable against a partial baseline.
2. **The two-binary differential.** Same box, solo, alternating, three
   reps, a canary reading between pairs: `ff 0.26.0 [03a17198744b]` against
   `ff 0.27.0 [86302e06d81b]` over every instance 0.26 solved that 0.27
   did not. Pre-registered fork, written before the numbers land:
   - both fail now → the box changed, not the engine;
   - 0.26 solves and 0.27 fails → `69cb96a`'s byte-identity claim is false
     in the field, **Lane A does not open**, and the cycle's first job is
     that defect;
   - both solve → conditions, and every packed 0.27 row is suspect.
   The differential doubles as Lane A's referee, so it is run once and
   used twice.
3. **Real width on the row.** Every raw still stamps the manifest's
   `jobs = 2` whether it was measured 2-wide (air25), 1-wide (air26) or
   10-wide (air27). Until the measured concurrency is on the row, no
   reader can separate a +N from packing noise. Known prerequisite: the
   runner passes the NOMINAL batch width, fixed outside the worker scope,
   while the width policy collapses the real one constantly.
4. **The cut27 postmortem**, as Phase 4 of the 0.27 roadmap pre-registered
   it: passes, wall-clock hours, and the box's condition, whatever they say.

### Phase 0 — status, 2026-09-16

1. **CLOSED.** All 32 cut27 boards are `.done`; 0.27.0 was promoted and
   published 2026-09-10 (5,122/8,444).
2. **Run 2026-09-16, same box, both binaries alternating, 3 reps.** The
   lost set, read from the crucible database: 22 instances where 0.26.0
   [03a17198744b] banked a solve in the cut26 sweep and 0.27.0
   [86302e06d81b] banked a miss in cut27. Neither original binary survives
   (`target/` was rebuilt), so the pair is `v0.26.0` rebuilt [8fe896b4fd53]
   against `v0.27.0` rebuilt. 16 of the 22 were solved by 0.26 at ≥ 58 s
   of a 60 s wall. Results are recorded below the postmortem.

   **CLOSED 2026-09-16: the fork reads "not the engine".** Each cell
   counts solves out of 3 repeats. The canary read 0.49–1.14 s (median
   0.55) across 66 readings. The box was shared with the v0.26.0 backfill.
   In repeat 1, three tetris-mco runs peaked near 8 GB each and pushed it
   into swap; that is the repeat where everything failed.

   | outcome | instances | 0.26 | 0.27 |
   |---|---|---:|---:|
   | both solve | tetris-mco t2 i16, t4 i15, t4 i16, t8 i16; hydropower-opt i14; elevator-t i10; elevator-t-strips i28; pathways-complex i11 | 18/24 | 18/24 |
   | 0.27 solves, 0.26 does not | parking-agile i10; parking-mco-t8 i1, i9, i17; recharging-robots i14 | 1/15 | 12/15 |
   | both fail | spider i17; folding-agile-300s i2; recharging-robots i4; storage-qual i12, i18; scanalyzer i18, strips i28; parking-mco-t4 i19 | 0/24 | 0/24 |
   | 0.26 once, 0.27 never | scanalyzer-strips i29 | 1/3 | 0/3 |

   Not one instance fits "0.26 solves and 0.27 fails". The single 0.26-only
   solve is 1 of 3 at the wall. `69cb96a`'s byte-identity claim holds in the
   field, and the 22 cut27 misses are conditions (8 failures shared by
   both binaries, 8 instances that both solve on a quieter box) plus five
   rows where 0.27 is the stronger binary. The Lane A arm is moot, since
   that lane is already a recorded negative.

   **The whole-set answer, 2026-09-18.** The v0.26.0 backfill of cut27
   finished — 32/32 boards, 765 banked in 9 passes, no row owed on either
   side — so `crucible compare` now answers the question like for like on
   one box, one referee, one binary identity per side
   (`benchmarks/cut27-compare-0.26-vs-0.27.txt`):

   **0.26.0 [8fe896b4fd53] 5,007 · 0.27.0 [86302e06d81b] 5,122 · net
   +115.** 0.27 is ahead on 22 boards, level on 6, behind on 4:
   ipc5-qual-pref −5, ipc5-complex-pref −2, ipc67-temporal −1,
   ipc2023-agile-300s −1.

   Per instance, 15 rows went 0.26-solved → 0.27-unsolved (board nets hide
   the ones 0.27 won back). **Twelve of the 15 were 0.26 solves at ≥ 57 s
   of a 60 s wall**, i.e. wall-edge rows, and the three that were not are
   elevator-temporal-strips i28 (46.5 s), tidybot-mco-t2 i20 (42.6 s) and
   the two 300 s agile rows at 298 s. The differential above re-ran
   elevator-t-strips i28 three times against both binaries and both solved
   every time. The published +134 stands; the honest like-for-like figure
   for the cycle is **+115**, and the difference is the 0.26 baseline
   re-measured on today's box rather than on cut26's.

   > **SUPERSEDED 2026-09-19 — see "THE INSTRUMENT" below.** "One binary
   > identity per side" was necessary and not sufficient: the two sides
   > were not given the same NUMBER of attempts. 0.26.0 re-ran 27.0 % of
   > its failures, 0.27.0 re-ran 68.7 % of its, neither ever re-ran a
   > solve, and the banked delta decomposes as first-attempt −18 plus a
   > rescue difference of +133. Under equal-N the same runs read −41
   > [−80, +1]. The +115 is not like-for-like; it is max-over-unequal-N.
3. **BUILT** (`crucible-r2` fd4ea3f). A row's `neighbours` is now the PEAK
   number of planners that ran beside it over its lifetime, not
   `width − 1` fixed before the workers started. It goes to the
   database, where the referee reads it, and onto the raw row as
   `neighbours`. The resume identity keeps `jobs`, so no banked row is
   re-owed. The running resident still uses the old binary until it is
   restarted.
4. **The cut27 postmortem.** The pre-registration was one pass plus its
   solo tail, within 36 h. **It took 141 h of wall-clock** (2026-09-04
   22:20 → 09-10 19:28) across nine sweep sessions; the last one finished
   in 2 passes. 12,800 runs banked 8,444 rows, and 2,486 instances took
   more than one run (at most 18). Of the 134.6 planner-hours, **63.4
   (47 %) went to runs that were not banked**:

   | verdict (not banked) | runs | planner-h |
   |---|---:|---:|
   | demoted (the E-core defect, fixed 09-07) | 1,996 | 33.1 |
   | thermal | 878 | 13.9 |
   | starved | 682 | 4.2 |
   | packed | 259 | 4.3 |
   | contended | 307 | 4.2 |
   | suspect / recheck / swap / uncovered | 234 | 3.8 |

   The box's condition: in ordinary use throughout. Worst incident: the
   09-07 disk cleanup deleted `target/` and the sweep sat dead for 11 h.
   The E-core demotion alone cost more than the whole 36 h budget. Mean
   nominal neighbours on the record: 2.4, max 9. That number is the old
   stamp, which is what item 3 fixes.

## Lane A (headline) — the temporal successor rung

**Measure first.** A per-expansion split (candidate scan / h / apply) on
driver-log-t i5 (61,092 ops, 1.0–1.1k evals/s), rtam i16 (13,176 ops,
~4.5k/s), satellite i5 (17,676 ops, 5.5k/s) and storage-time i15 (3,744
ops), on a quiet box.

**Pre-registered kill:** if the candidate scan is under ~10 % of the
per-evaluation wall on all four, the lever is a **recorded negative that
day** and Lane A ends there. Nobody has read the scan's share of a
*temporal* evaluation; `temporal-attribution-0.23.md` measured a different
term (h-build per eval), and 0.27 already recorded that one as a negative.

**Band, labelled as interpolation:** +2 to +8 on `ipc2014-tempo`,
concentrated on storage-t and rtam. NOT the +10–30 the 0.27 speed lane
pre-registered for itself.

**Carve-out:** the orbit/symmetry block records a generator class before
the applicability test, so the substitution is not byte-identical under
`FF_ORBIT_GEN=1` — that arm keeps the full scan, and it is already a
recorded negative ("match-cellar lost 9 instances to it").

### Lane A — RECORDED NEGATIVE, 2026-09-11. The lane ends here.

The pre-registered kill fired. Candidate-scan SELF time, sampled on a box
with one competing core (Steam), `FF_NO_TSUCC=1` so the full scan is the
thing being measured, `sample` at ~1 kHz:

| instance | ops | scan share | heuristic share |
|---|---|---|---|
| driver-log-t i5 | 61,092 | **0.01 %** | 99.6 % |
| rtam i16 | 13,176 | **7.70 %** | ~97 % |
| satellite i5 | 17,676 | **0.09 %** | 98.7 % |
| storage-t i15 | 3,744 | **0.72 %** | 96.9 % |

Under ~10 % on all four, which is the condition this lane wrote down in
advance for ending itself. The 7.70 % on rtam is the loose end and it is
loose in the safe direction: that sample carries more than one thread, so
its denominator is understated and the true share is lower.

**Why it is not close.** 61,092 ops was chosen as the case where an
O(n_ops) scan should hurt most, and it is the instance where the scan is
LEAST visible — 0.01 %. Every sample that is not the scan is the relaxed
planning graph: `push_node → eval_node → relaxed_helpful →
relaxed_to_inner → build_rpg`, 16,384 of 16,441 samples on driver-log-t.
The scan does not even appear as a frame; it inlines into
`temporal_search` and shows only as self time.

**What this confirms rather than discovers.** 0.27 recorded the same shape
for the classical path: "what is left on these boards is the relaxation
floor itself, ~1.7-2 ms per evaluation at 60-80k ops, proportional to the
effects the fired ops carry. Moving it means firing fewer ops (relevance)
or evaluating fewer states, not a faster scan." Nobody had read that
share for a TEMPORAL evaluation, which is why the measurement was worth
its afternoon. It reads the same.

**The code stays.** `98a74e1` wires the anchored generator into the
temporal rung and is byte-identical by construction (candidates sorted
back into the scan's order), so it is neither a win nor a risk: it
removes a 0.01-7.7 % term and costs nothing. `FF_NO_TSUCC=1` keeps the
full scan for exactly this kind of measurement. What is withdrawn is the
CLAIM -- the +2 to +8 band on `ipc2014-tempo` is not pursued and no
differential is run for it, because a term that small cannot pay for one.

**Cost of the lane:** one afternoon, four samples, no sweep.

## Lane A' (the replacement headline) — the node cap is a model, and the model may be wrong

Lane A died on its own kill criterion the same afternoon it was measured,
so this takes the headline. It comes from the 0.27 cut rather than from a
guess.

**The observation.** 937 rows across the seven optimality boards banked
UNSOLVED with the note `inconclusive: node cap reached after N
expansions`. They did not run out of time; they ran out of node budget.
And they did it while barely touching the 6 GB the boards declare:

| board | capped rows | avg GB actually used |
|---|---|---|
| ipc-opt-2008-11 | 253 | 1.74 |
| ipc2023-numeric-opt | 179 | 3.36 |
| ipc2014-opt | 174 | 1.38 |
| ipc2018-opt | 117 | **0.93** |
| ipc2026-opt-full | 96 | 4.38 |
| ipc2023-opt | 81 | **0.80** |
| ipc2026-opt | 37 | 4.00 |

`ipc2018-opt` stops having used 15 % of its declared budget. The cap is
derived from `node_bytes_target` over a MODEL of per-insertion cost --
deliberately, because the count has to be serial and thread-count
independent, so it can never be RSS. That design is right. The question
nobody has asked is whether the model's bytes-per-node is ACCURATE, and
four boards stopping under 2 GB of a 6 GB budget is what makes it worth
asking.

**The receipt.** `openstacks-sequential-optimal-strips/16`, traced at the
0.27 cut: 0.26 proved it in 53.29 s using 3.82 GB and 4,250,212
expansions; 0.27 capped at 3,798,074 expansions with 3.05 GB -- 89 % of
the way to a proof it had the memory to finish. The cut-time re-check
proved it again on a quiet box. One row, but it is the mechanism in
miniature: the cap, not the clock, and not the engine.

**Measure first.** Modelled bytes-per-node against measured, on the four
boards with the widest gap (2018-opt, 2023-opt, 2014-opt,
opt-2008-11). The measurement is RSS growth over nodes inserted across a
run, compared with what `per_node_model_bytes` predicted for the same
task. No engine change to take the measurement.

**Pre-registered kill:** if the model is within 25 % of measured
bytes-per-node on all four, the caps are honest memory exhaustion, this
lane is a **recorded negative that day**, and the proof tracks' ceiling is
real rather than self-inflicted. Raising a cap that is already accurate
would buy swap, not proofs -- and the referee already owes 50 rows of
this sweep to swap.

**Band, labelled as interpolation:** +15 to +60 across the seven proof
boards, concentrated where the gap is widest. NOT the 937 -- most of
those instances are genuinely too large, and the openstacks receipt was
at 89 % precisely because it was close. Rows recovered here are
certified optima, which is why a smaller band than Lane A's is worth
more: on a proof track, coverage IS proof rate.

**Why it is the right headline for this cycle.** The optimality boards
are the weakest coverage in the table (20-38 %). Every other lane this
cycle aims at boards already above 60 %. And the 0.27 engine work has
now twice been told the same thing by measurement -- the relaxation floor
is the term, not the scan -- so a cycle that keeps pushing per-evaluation
speed is a cycle arguing with its own instrument.

### Lane A' — RECORDED NEGATIVE, 2026-09-16. The premise was a mislabel.

Nothing was measured against the cap, because the cap was not what stopped
these rows. `api.rs` built its node-cap note from `inconclusive()`, the same
constructor the clock trip uses, and never read `clock_tripped`, so **every
optimal row the 60 s wall stopped was banked as "node cap reached"**. The
air27 times had already said so. On the four target boards, 545 of the 625
"capped" rows ran ≥ 57 s and none ran under 50 s; across all seven boards
14 of 937 ran under 50 s, and none under 40 s. The early exits are the
teardown reserve (stored bytes / 4e8 s), not the cap.

**The probe.** 26 of the 937 rows (4 per board, evenly spaced; 2 more
could not be located), re-run with the fixed note (`engine-0.28` 5c94647),
`FF_WALL_DEBUG` for the cap and its per-node model, and
`/usr/bin/time -l` for the peak footprint, with the box shared with the
backfill:

- **26 of 26 stopped at the wall. 0 hit the cap. 0 raised it.**
- Headroom was large everywhere: agricola-opt i1 expanded 234 k nodes
  against a 13.8 M cap (250 MB); barman-2014-opt i1 5.0 M against 18.6 M
  (2.0 GB).
- Where memory was heavy, nothing suggests the model over-charges.
  fo-farmland i19's whole process peaked at 4.89 GB before reaching its
  cap, which is MORE than the 3.87 GB the model charges for a full cap
  (9.76 M × 396 B). factory-robot i14 peaked at 3.96 GB against 3.87 GB
  modelled. The footprint includes the task tables and the open list, so
  this is not a per-node measurement, but the direction is clear. A higher
  cap on the numeric boards would buy swap, not proofs.

The openstacks-16 receipt reads the same way: 0.27 was 89 % through a
proof at the wall, not at the cap. **The ceiling on the proof tracks is the
clock**, so a proof-track lever has to be a cheaper expansion or a better
bound, not a larger cap.

**What stays:** the note fix (5c94647). From the next sweep on, a wall stop
reads `inconclusive: wall reached after N expansions`. The 937 air27 rows
keep their old text, so read them as wall stops.

## Lane B — two cheap claims at the existing 60 s wall

- **The tpp complex-preferences parse.** All 20 rows of
  `tpp-preferences-complex` die `engine-exit-1` at ~0.26 s — the spawn
  floor — in both air25-entries and air26. The domain writes a legal PDDL3
  `preference` inside a durative `:condition`; the parser refuses any
  conjunct head that is not `at`/`over`. Band +3 to +6, sized from this
  engine's own twin (`tpp-metric-time` solves i1–i8 of 40 at the same
  budget on the byte-identical hard goal). **Guaranteed regardless of
  band:** 20 rows leave `engine-reject/error` for a real verdict class —
  the shape the 0.26 mem-cap fix already shipped once. **Open question the
  build must answer first:** if the preference is parsed and then dropped
  downstream, the plans are legal but the METRIC is wrong, which is worse
  than failing loudly. A number the record cannot defend is worse than a
  missing number.
  **BUILT 2026-09-16** (`engine-0.28` 9aabf88). The parser takes
  `(preference name (at start phi))` inside a durative `:condition`, and the
  search drops it. The open question is answered in code: `score_soft`
  binds each durative step's condition preferences and counts one
  violated instance per application (at start before the start happening,
  at end before the end, over all across every state inside the
  interval). The fixtures pin all three timespecs and the forced metric.
  **The board, 20 instances at 60 s, one pass, box shared with the
  backfill: 8 of 20 solve** (i1–i4 in 16–36 s; i5–i8 in 58.6–59.1 s, which
  is on the wall and will not all survive a sweep). The other 12 stop at
  the wall, and none dies engine-exit any more. Honest band: **+4 solid, up
  to +8**, against the +3 to +6 the lane was sized at. No solved plan
  violates p-drive, so the new count reads 0 on every banked row. The
  fixtures, not the board, are what prove the count.
- **The barman optimal gate, no-code arm first.** The 300 s probe proves
  cost 49 at 6,949,349 expansions in 50.4 s of user time; the 60 s board
  rows node-cap at 6.4M expansions in 57 s — short of a certificate the
  same engine completes. Every knob is already an env var. Measure both
  arms solo, then **measure the loss side nobody has measured**: sweep the
  four optimal boards at the changed margin and count the rows lost where
  the gate currently fires inside the old band. Ships only as a changed
  default with an `FF_NO_*` restore. Band +1 to +3, **and a net negative
  is possible.**

## Lane C — two probes, both expected negative

- **Floor-tile's dead-end test.** Specified with a verbatim numeric exit
  clause since 0.26 and never run; `grep FF_DEADEND crates/` returns
  nothing after three cycles. Re-anchor its counters (0.27 moved them) and
  re-take the row set against the 0.27 binary before a line is written.
- **A classical `best_h` trace.** There is no h-descent trace on the
  classical path at all — the 138-row AIBR constituency named in the
  0.26 dossier has never been measured for flat h. One print site fixes
  that, and it is what makes the next cycle's numeric-h question askable.

## Lane D — the per-call budget (unscoped, consumer-driven; SHIPPED IN 0.27.1)

Not on the cycle plan. A consumer embedding ferroplan as a library filed a
concrete ask with a measurement attached, and the measurement is why it
jumped the queue: `solve` was blocking and uninterruptible, so a host that
abandoned the work leaked that thread for the life of the process. Their
own instrumentation recorded 213 solves completed, then 2 abandoned, after
which the pool was dead — 0 completed, 4 abandoned.

Neither existing budget could bound one call. `max_evaluated` caps
evaluated STATES and grounding runs before the first state exists;
`FF_TIME_LIMIT` is armed once per PROCESS from the first solve, so in a
long-lived host it either bounds nothing or eventually refuses
everything.

**Shipped as 0.27.1, not here.** Two fields on `Options`, both `None` by
default and inert unless set: `wall_ms` (armed at the top of `solve`, so
it bounds parsing and grounding too) and `should_continue` (an
`Arc<AtomicBool>` polled wherever the wall is). A stop returns
`solved: false` with a note naming which budget bound and where, never
the word "unsolvable".

It went out as a patch release on the 0.27 line rather than waiting for
this cycle **because it is additive API with no engine change** — so it
needed no sweep, and holding it would have kept a blocked consumer
waiting on Lane A's measurement for no reason. The two commits were
cherry-picked; the rest of this branch stayed behind until it has been
measured. That is the precedent worth keeping: API that cannot move a
number does not need a cut behind it.

Held per call in a thread-local armed by an RAII guard, so concurrent
solves carry their own and a spent budget cannot leak into the next call
on its thread. `FF_NO_RUNG_WALLCAP` does not disable it — that hatch
governs the env wall, and a budget passed in code outranks an environment
variable.

**Recorded, not acted on:** the rung ladder rations a wall across its
rungs, so a budget is not "time until I stop". A 30-block instance
solving in 34 ms unbudgeted is stopped by a 120 ms wall and solved under a
200 ms one — and `FF_TIME_LIMIT=0.12` fails the same instance while `0.2`
solves it, so this is the env wall's behaviour too, not the new path's.
Teaching the ladder to skip its slicing under small budgets would be a
behaviour change with no measurement behind it.

### Lane C — the classical `best_h` trace, BUILT 2026-09-16

`FF_HTRACE=1` prints `htrace: best_h H at N evaluated (T s)` on stderr each
time the classical weighted-best-first search improves its best h. That is
the site `advance` already records; the trace adds the evaluation count and
the clock. Off by default and silent. The trace has not been read on the
AIBR constituency yet; that read is the probe.

## The SGPlan question, recorded rather than scoped

The operator's standing goal is to beat SGPlan5, and the record's ledger
is the place that fight is scored: metric-time 64/200 vs 151/200,
complex-pref 26/108 vs 105/108, qualitative 46/100 vs 100/100,
propositional 188/220 like-for-like vs 218/220, constraints 20/80 vs
47/80 — against `time`, which this engine now **leads** (88/130 vs 80/130).

Two facts change how that gap should be read, and both are already
measured:

- **On the instances both solve, ferroplan wins on quality**, often by a
  wide margin (qualitative rovers: 68 vs 88, 26.1 vs 43.4, 37.6 vs 88,
  556 vs 674 — lower is better), and the ESPC penalty loop is why
  (`FF_NO_ESPC=1` degrades the same instances to 23/24/29/39/66/65/126/370
  against 19/23/17/16/21/22/66/87). The gap is **coverage**, not quality
  and not a missing mechanism.
- **SGPlan5's numbers were measured at IPC-5's 30-minute limit**; ours are
  at 60 seconds. The published gap therefore overstates the deficit by an
  unknown amount.

### The parity probe, RUN 2026-09-18/19. The gap is 76 rows, not 125.

Both boards at IPC-5's own 1800 s limit, once, as a MEASUREMENT: no board
bought, no table changed. Only instances UNSOLVED at the 60 s wall were
re-run (a 60 s solve is a 1800 s solve), 128 of them, packed for coverage
with timing dirty by construction. Raws:
`benchmarks/probes-0.28/sgplan-parity-1800s.txt` and the two jsonl beside
it. Engine: `engine-0.28` at e42c30c, so the complex board includes the
Lane B tpp fix.

| board | 60 s | 1800 s | total | SGPlan5 | gap at 60 s | gap at 1800 s |
|---|---:|---:|---:|---:|---:|---:|
| ipc5-qual-pref | 51 | **76** | 100 | 100 | 49 | **24** |
| ipc5-complex-pref | 29 | **53** | 108 | 105 | 76 | **52** |

**Half the qualitative gap and a third of the complex one were the wall.**
49 new solves for 128 instance-runs. By domain: rovers +9, storage +7,
tpp +5, openstacks +2, trucks +2 (qualitative); pathways +14, tpp +8,
pipesworld +1, storage +1 (complex).

**The finding that matters is not the count, it is the SHAPE.** 14 of the
49 landed within 300 s and three within 60 s — rovers-qual i5 in 40 s, on
an instance the 60 s board misses. Those are not instances that needed 30
minutes of search. The rung ladder rations one wall across its rungs
(Lane D recorded the same behaviour from the other side: a 34 ms instance
fails under a 120 ms budget and solves under 200 ms), so at 60 s the
ladder never reaches the rung that solves them. **A cheaper lever than any
preferences wing is the ladder's slicing policy**, and it is now measured
rather than asserted.

**A defect this probe found, worth its own lane.** The 21 instances that
banked `mem-cap` at 60 s were re-run one and two at a time. Under a
declared `FF_MEM_BUDGET_GB=6` each grew to **~528 GB of virtual address
space with 20 MB resident** while system swap reached **44 GB** — 28
minutes at 0.1 % CPU, thrashing rather than searching, until the external
alarm. One of them (storage-complex i2) still solved, at 1803 s, so the
class is not hopeless; but the internal node cap plainly does not hold
this class to its declared budget at a long wall. At 60 s the runner's RSS
watchdog hides it. 0.22's sailing-wind receipt describes the same
pathology and the cap was supposed to have fixed it.

**What the probe does NOT license.** 76/100 and 53/108 are 1800 s numbers
against SGPlan5's 1800 s numbers; the published table stays at 60 s. The
remaining 76 rows across both boards are a real capability gap, and a
preferences wing would still be pricing itself against that, not against
the 125 the old reading implied.

### CORRECTION, 2026-09-19: the probe ran six threads, the boards run one

`probe.sh` invoked `ff` with no `--threads` flag, so the probe took the
auto thread count — `min(cores, 6)`, and `statistics.threads: 6` on a
re-run confirms it. Every published row on both boards is `threads: 1`
(all 208 of them, `benchmarks/ipc5-qual-pref.jsonl` and
`ipc5-complex-pref.jsonl`). The probe bought ~30× wall AND 6× cores at
once, so its rows are not like-for-like with the 60 s board, and not with
SGPlan5's single-core 1800 s results either.

The three fastest new solves, re-run SINGLE-THREADED at the same 1800 s
wall, same binary, same box:

| witness | probe, 6 threads | re-run, 1 thread | 60 s board |
|---|---:|---:|---|
| tpp-preferences-complex i1 | 40 s | 82 s | unsolved |
| tpp-preferences-complex i2 | 56 s | 115 s | unsolved |
| rovers-preferences-qualitative i5 | 40 s | 457 s | unsolved |

**WITHDRAWN: "three within 60 s", and the slicing lever it motivated.**
Single-threaded, the three need 82 s, 115 s and 457 s. None is reachable
inside the board's 60 s wall under ANY slicing policy, because the WORK
exceeds the wall — there is no rung left to give it to. The ratios share
no constant (2.05×, 2.05×, 11.4×), so the probe's remaining times cannot
be rescaled to a single-threaded estimate: the 49-solve table is a
6-thread table, and the honest single-threaded gap is somewhere between
76 and 125 rows, unmeasured.

**Also withdrawn: the mechanism.** These two boards never enter the
sliced ladder at all. `FF_WALL_DEBUG=1` on a qualitative-preferences
solve prints NOTHING (`mode: pddl3`) — the preference path runs
`pddl3::metric_optimize`'s restart ladder over `PROFILES` against a flat
2,000,000-eval `pref_eval_budget()`, which is wall-blind. Whatever the
60 s wall costs these boards, "the rung ladder rations one wall across
its rungs" is not a description of it.

**What survives.** 49 of 128 previously-unsolved instances do solve given
1800 s and six cores — true, as SIX-THREAD numbers, and worth what that
is worth. The mem-cap defect below is untouched: it was diagnosed from
address space and swap, not from timing. What is gone is the
like-for-like reading, and the cheap lever it implied.

### The 49, reclassified — 8 of them were never a wall result

Zero box time: join the probe's 128 rows against the committed 60 s raws
on `(variant, instance)` and read what the 60 s row actually SAID.

| the 60 s row said | count | what the 1800 s solve means |
|---|---:|---|
| plain timeout | 38 | genuine extra compute (wall AND cores) |
| `engine-exit-1` | **8** | the 60 s row was a CRASH, not a timeout |
| `mem-cap` | 3 | the memory class, re-run 1- and 2-wide |

The 8 `engine-exit-1` rows are tpp — **the Lane B parse fix, already
banked in this cycle**. Their 60 s rows record `rc=1` because the engine
died on the parse; the probe re-ran them with the fixed engine and scored
the conversion a second time, as a wall result. The probe's own write-up
says "the tpp contributions only exist because of the Lane B parse fix"
and then counts them anyway. Deduct them: the wall-and-cores conversion
is 41 rows, not 49, and on ipc5-complex-pref it is 16, not 24.

**Owed before any SGPlan claim, and cheap: one parity probe.** Run the
qualitative and complex boards at 1800 s, once, as a MEASUREMENT and not a
tier — it answers whether the fight is 54 rows or 5. The standing anti-pot
against new tiers is about *published* tiers; this buys no board and
changes no table. Until that number exists, an SGPlan wing is unpriced,
and this project has three cycles of evidence that bands taken from other
planners' published results deliver +1.

### The timing read, 2026-09-20. SGPlan5's median solve is 0.54 s.

Everything above argues about the WALL. `benchmarks/IPC5-results.tgz` has
held the answer the whole time: every SGPlan5 `.soln` carries a `; Time`
header, its own CPU seconds on 2006 hardware. Joined per instance against
the 60 s boards (`benchmarks/probes-0.28/sgplan-timing/join.py`), like-for-
like variants only:

| | |
|---|---:|
| SGPlan5 solves across the seven IPC-5 boards | 860 |
| ... finished inside 1 s / 10 s / 60 s | 480 (55 %) / 668 (77 %) / 769 (89 %) |
| ... median | **0.54 s** |
| rows SGPlan5 solves and this engine does not | 335 |
| ... that took SGPlan5 under 60 s / under 1 s | **283 / 143** |

**The "their 1800 s against our 60 s" reading explains at most 52 rows.**
`pathways-metric-time`: this engine 0/30 at 60 s, SGPlan5 30/30 with a
median of 0.22 s and a maximum of 1.1 s. `tpp-metric-time-constraints`:
0/30 against a median of 0.01 s. A planner that answers in a hundredth of a
second is not out-searching anything. It is not searching.

**Also corrected: the quality claim above.** "On the instances both solve,
ferroplan wins on quality" is true of the rovers rows it quotes and of
storage; across the boards it is a split, and on simple-preferences it is
the other way round (SGPlan5 better on 70 of 118, ferroplan on 29, by
SGPlan5's self-reported `MetricValue`; qualitative 26–22 to ferroplan;
complex 20–8 to SGPlan5). The margins are small where this engine loses
(tpp 129 vs 105) and large where it wins (storage 25 vs 87), but "coverage,
not quality" was half right.

**What the unsolved rows were actually doing** (`why.py`, the 60 s raws'
own notes column): stopping at the wall inside the decision-epoch ladder
(~117), exhausting the temporal ladder's budgets with wall to spare (33),
`mem-cap` three to ten seconds in (29), the Lane B parse crash (20), and a
plain 60 s timeout inside the PDDL3 optimizer (~120). Three probes, each
threads 1 / 60 s / single attempt / VAL-judged against the ORIGINAL task /
counted only under 58 s (`satprobe.py`, `tprobe.py`, `tally.txt`):

| lever | what the probe ran | rows converted |
|---|---|---:|
| the PDDL3 route has no first plan | `ff --satisfice` on the rows the optimizer timed out on | **60** (qual 51 → 97 of 100; simple 119 → 130 of 130) |
| the temporal ladder has no sequential rung | durative → one classical action, plan, lay out end to end (`compress.py`) | **121** (complex-pref 29 → 82; metric-time 44 → 89; time 53 → 76) |
| the preference tier overruns its own wall | (found while probing) a banked, valid plan returned at 21.94 s of a 20 s wall | folded into the two above |

Six boards, **319 → 500 against SGPlan5's 612**, from levers that are
architecture rather than search. One shape under all three: **a valid plan
was in hand, or milliseconds away, and the route had no way to return it.**
SGPlan5 is "feasible first, better second" all the way down. This engine
was "best or nothing", with the wall deciding which.

**What is left after them is real engine work, and it is numeric.** The
compressed `rovers-metric-time` runs ~210 evaluations a second against
>5,000 for its same-size propositional twin (`heuristic.rs::build_rpg`
widens intervals layer by layer -- a hypothesis, not a profile). And
`tpp-metric-time` i9 is 20 facts and 264 ops on which the classical ladder
spends 5,000,000 evaluations without a plan, while `--mode partition`
evaluates zero states on a numeric goal -- the one place SGPlan's per-goal
partitioning is doing the work its name claims. Worth noticing on the way
past: the decision-epoch search's demand tiers guide numeric accumulation
BETTER than the classical relaxation does (a mini-pathways task the
compressed search fails on for 32 s solves there in 40 ms). That is ~112
rows and the next cycle's question; the propositional board (231 vs 248)
is untouched by any of this.

## Lanes S, I and T — feasible first (BUILT 2026-09-20)

Built in cost order on `engine-0.28`. Fixtures first where a fixture can be
RED at unit scale; where the defect is a scale phenomenon, the fixture pins
the mechanism and the RED receipt is a named board row.

### Lane S — a banked plan is reported inside the wall (`ac99b41`)

`temporal::solve`'s preference tiers already banked coverage first. They
then chased quality against the SAME wall, the ladder under the chase opened
further rungs (each re-grounding) after the wall had expired, and the caller
grounded the original pair a second time to score whatever came back --
12.7 s on `pathways-complex` i20, all of it past the deadline. The runner
kills at the wall.

- `search::tighten_deadline`: a scoped deadline riding the per-call budget
  every checkpoint already joins. `reserve_for_report` holds back 3 % of the
  wall's TOTAL (0.5–3 s) plus `ops / 1e5` s for closing and dropping a big
  task. (The first cut read the REMAINING wall; after a 30 s grounding that
  is 0.9 s, and `storage-qualitative` i20 exited at 60.2.)
- `score_soft` splits into `SoftScorer::prepare` (the grounding) and
  `::score` (a replay): the scorer is built BEFORE the chase, under the same
  tightened deadline, so the winning plan is one replay from its report. A
  banked plan whose scorer cannot be built in time is reported UNSCORED,
  with a note -- a plan without a metric is a solve; a plan never printed is
  not.
- `solve_ladder` refuses the Full tier and the decomposer rung once the wall
  is spent.
- `tests/pref_chase_wall.rs`: the `tsearch_wall` ring with its unreachable
  goal moved into a preference, plus a K³ ballast action so grounding costs
  real time. **v0.27.1: reported at 6.523 s of a 6 s wall. Now inside it.**

### Lane I — incumbent zero (`5bfa26e`)

The compiled preference task prices every preference into the goal and
rides a monitor block on every op, so its FIRST plan is far dearer than the
hard goals' plan. `rovers-qualitative` i9: 70k evaluations at ~3,600/s and a
whole 20 s wall without reaching the hard goal once; the classical ladder
reaches it in 133 evaluations and 30 ms. `metric_optimize` returned `None`
and the route said "unsolved" (the text path said "proven unsolvable").

- `pddl3::hard_goal_seed` plans the ORIGINAL pair and lifts the plan into
  the compiled task by display name (precondition-preference variants share
  a name and are mutually exclusive; the trajectory gate's `TRAJ-END` is a
  real op to both). `close_seed` closes it with the phase tail.
- `metric_optimize_seeded` uses it as a **floor, not a bound**: the B&B runs
  exactly as it did -- the `ipc5_*_metric_no_regression` and
  `reported_equals_verified` locks pass unchanged -- and the seed comes back
  only when nothing cheaper was found, with a note saying so.
  `FF_PREF_SEED_BOUND=1` also opens the B&B with it; `FF_PREF_NO_SEED=1` is
  the 0.27 restore.
- **The seed ladder gets the whole wall.** Handed 80 % of it, EHC's
  proportional slice is 12 s and `rovers-qualitative` i20 -- which EHC solves
  at 14.6 s -- is handed down the ladder and lost. This is Lane W's
  pathology met from the other side, and the second receipt for it.
- A plan in hand still has to cross the wire, and the optimizer had four
  ways to miss: the best-first loop read the clock once per 256-eval batch
  (two seconds at the 8 ms an evaluation costs on a compiled task; and on
  wide states EXPANSION is the slow half -- one batch, 2.3 s); the selection
  DFS's node cap is wall-blind and every node scans all the preferences
  (~15 s on `storage-qualitative` i17's 23k instances, for a result the
  caller discards as "capped"); and the B&B loops, the selection probes and
  the legacy fallback kept opening one-evaluation sweeps after the wall.
  All four now read an armed deadline. A tripped batch is a capped return
  that never looks at its values, so a run that ends inside the wall is
  evaluation-for-evaluation what it was.
- `tests/pref_seed.rs` pins the lift through both places the op sets differ,
  and the floor's exact price. The RED receipts are board rows:

  | row | wall | 0.27.1 | now (process exit) |
  |---|---:|---|---|
  | rovers-qual i9 | 20 s | unsolved | solved, 19.88 s |
  | rovers-qual i20 | 60 s | unsolved | solved, 58.16 s |
  | storage-qual i20 | 60 s | unsolved | solved, 57.95 s, metric 5737 < floor 6046 |
  | rovers-qual i1 | 20 s | metric 68.039 | 68.039 |

### Lane T — the compression rung (`tcompress.rs`)

Every durative action becomes one instantaneous action (condition = start ∧
over-all ∧ end; effect = start then end, a token added at start and deleted
at end netting to its delete), the classical ladder plans it, and a
left-shift over the ops' read/write sets puts the plan back on the clock.
`temporal::validate` judges the result against the original pair before
anyone sees it. Declines required concurrency (the existing detector), timed
initial literals and trajectory constraints.

- **It banks; the decision-epoch ladder then runs as a bounded quality
  chase, and the smaller makespan is returned.** The left-shift is coarse
  (whole-op interference) and the ladder's plans are much better where it
  has one -- `openstacks-time` i5: 343 against the rung's 897 -- so a row the
  ladder already solved keeps its plan, byte for byte, and a row it never
  solved has one.
- **The bet is bounded in both regimes.** Under a wall, a quarter of what is
  left (`FF_TCOMPRESS_WALL_FRAC`); unwalled, 2,000 evaluations per classical
  rung -- the first cut had no unwalled bound and spent 32 s failing on a
  task the ladder below solves in 40 ms. A ladder that then finds nothing
  leaves the rung a second, unbounded attempt.
- **The validator had to be fixed twice to be usable here.** It fired ends
  before starts inside one epoch, which put a ZERO-DURATION step's end ahead
  of its own start: every plan through pathways' dur-0 `choose` /
  `initialize` was refused (`temporal::epoch_rank`; the search and VAL both
  order the pair start-then-end). And it grounds the full snap task it
  replays on, which for `pipesworld-metric-time` is the very blow-up the
  board records as `mem-cap` -- i20's plan is found in 4 s and its validation
  was still grounding 96 s later. The rung validates at the PLAN's size: the
  domain specialised to one parameterless durative action per ground step.
  `tests/tcompress.rs` cross-checks that against the full validator.
- An engine-semantics note, found by the fixture and NOT changed: the snap
  compile asks for the over-all invariant in the START snap's precondition,
  which is stricter than PDDL2.1's open interval -- a self-established
  over-all lock is unsolvable here by rule. The rung follows the engine's
  rule (it only relaxes self-established AT-END conditions).
- `tests/tcompress.rs`: a held token nets to its release; independent work
  overlaps and interference does not; zero-duration steps validate; the
  three declines; and, `#[ignore]`d, `pathways-metric-time` p01 -- an 18-step
  plan the ladder cannot find (its dur-0 happenings pile up at t = 0 and the
  search permutes them to its node caps), the whole domain 0/30 on the 0.27
  board.

### The board sit

`benchmarks/probes-0.28/lanes-sit/run.sh`: the six IPC-5 boards these lanes
were built for, and the two temporal boards they were NOT built for
(`tempo-sat`, `tempo-sat-2014` -- the regression read: the rung touches every
temporal task without required concurrency). One attempt per row, 2-wide,
threads 1, 60 s, against the PUBLISHED boards, which are best-of-N -- so a
single attempt is the conservative side of every delta, and a row the new
engine loses is re-run on the old binary under the same conditions before it
is called a loss. **Estimator declared in advance: first attempt.**

RESULTS PENDING -- the sit was launched after the lanes were committed.

## THE INSTRUMENT — a published row is best-of-N, and N was not controlled

Found while trying to reproduce ONE row. It outranks every lane below it,
because every lane below it is scored on this instrument.

**The reproduction failure.** `ipc5-metric-time`'s
`pipesworld-metric-time` i7/i8/i9 are published SOLVED (29.46 s len 9,
29.03 s len 11, 42.69 s len 36, `val: true` — VAL checked the plans). On
today's box, at the board's own 60 s wall, through the board's own
harness (`ipc67.py --track metric-time-2006`), they do not solve under
FIVE arms: engine-0.28 1-wide, engine-0.28 2-wide, v0.27.1 1-wide,
v0.26.0 1-wide, and by hand. v0.26.0 is the binary that solved all three
on 2026-09-11. Not the engine, not the width, not the corpus (pinned
2017 checkout, untouched since July), not `FF_MEM_BUDGET_GB`.

**The cause, from `~/.crucible/db/crucible.db`.** The crucible re-ran
them until they solved, and the board kept the attempt that did:

```
i7 @ 0.27.0   attempt 1 unsolved, 2 unsolved, 3 SOLVED   -> banked
i8 @ 0.27.0   attempt 1 unsolved, 2 unsolved, 3 SOLVED   -> banked
i9 @ 0.27.0   attempts 1-4 unsolved, attempt 5 SOLVED    -> banked (dirty)
```

My single-shot runs reproduce attempts 1–4 faithfully. Nothing is wrong
with them.

**The scale.** Across the record — keyed by BUILD, not by version string,
because three distinct 0.26.0 builds sit in this db and keying on `ver`
splices their attempts into one sequence — **543 cells are solved only
because a retry rescued them**, and 324 of those winning attempts are
`timing_quality: dirty`. Where one build both solved and failed the same
instance (590 cells), the record banks the solve **590 times out of 590**.
N per cell runs 1–18.

**Why the retries are one-directional.** A re-run is triggered by failure
and never by success, so extra attempts can only ADD solves:

```
                   attempt 1 SOLVED        attempt 1 FAILED       rescued
v0.26.0       4845 cells, re-run  0.0%   3599 cells, re-run 27.0%    162
ff 0.27.0     4827 cells, re-run  0.0%   3617 cells, re-run 68.7%    295
```

The retry rule is sound and exists for a measured reason (cut27's false
timeouts under swap thrash; the SUSPECT rule). Two things about it are
not: a SOLVE under the same thrash is never re-tested (0.0 %, both
engines), and the two engines' failures were re-rolled at rates that
differ by 2.5×.

**What it does to this cycle's headline.** `benchmarks/attempts-estimator.py`,
over the 8,444 instances both builds ran (receipt:
`benchmarks/metrics/attempts-estimator-cut27.txt`):

| estimator | v0.26.0 | ff 0.27.0 | delta | 95 % CI |
|---|---:|---:|---:|---|
| banked — what the boards publish | 5007 | 5122 | **+115** | [+93, +139] |
| solved on ANY attempt | 5007 | 5122 | +115 | [+93, +139] |
| FIRST attempt only | 4845 | 4827 | −18 | [−54, +22] |
| **equal-N** (max over first min(N_A,N_B)) | 4931 | 4890 | **−41** | [−80, +1] |
| per-run (expected coverage of ONE run) | 4918.6 | 4949.5 | +31 | [+4, +59] |

And the delta decomposes exactly:

```
first-attempt -18  +  rescue difference (295 - 162) +133  =  +115
```

**The whole of +115 is the difference in how many failures each engine was
allowed to retry.** On the one instrument both engines were given equally
— their first attempt — 0.27.0 is 18 rows BEHIND, inside the noise.

**This is not a claim that 0.27 regressed**, and the per-board table says
why. Under equal-N the 2011/2014 families hold their gains
(`ipc2014-sat` +17, `ipc2014-mco-t4` +15, `ipc7-mco-t2` +9,
`ipc67-results` +9) while others reverse hard (`ipc5-prop` +5 → −43,
`ipc67-temporal` −1 → −25, `ipc2018-sat` +4 → −22). Some of the engine
work is real. The AGGREGATE is what the instrument cannot support.

**A trap for whoever reads this next.** `timing_quality` is not a
severity: clean runs solve LESS often than dirty ones (v0.26.0 28.2 % vs
64.1 %), because clean re-runs were targeted at the hard owed rows.
`clean-only` is therefore a biased subset, not a cleaner measurement, and
the `+76` it reports is worth nothing.

**Owed, and it blocks the lanes.** (1) Adopt one estimator, declared in
advance — equal-N or per-run, not banked — and re-read cut27 under it;
the db has every attempt, so this is re-analysis, not box time. (2) Re-test
a sample of BANKED SOLVES, which nothing in the loop currently does, to
price the ratchet's other half. (3) Publish attempts and timing_quality
beside coverage, so a row says how many tries it took. (4) Only then
re-read anything resting on the delta, this cycle's SGPlan arithmetic
included.

**Deliberately NOT changed here.** `README.md` ("61 % coverage
(5,122/8,444) … +134 over 0.26.0") and `CHANGELOG.md` state the banked
coverage of a SHIPPED release. The coverage figure is what the boards
record and stays true as that; the COMPARATIVE claims (+134, +115) are the
ones this section withdraws, and rewriting a published release's notes is
the operator's call, not a correction to make quietly inside a cycle doc.
`benchmarks/cut27-compare-0.26-vs-0.27.txt` is left exactly as the tool
wrote it, for the same reason a receipt is not edited.

## Lane W (candidate, NOT scoped) — declaring a wall can make the engine worse

Found while chasing the withdrawn slicing lever, and kept because it
reproduces in under a second on the CLASSICAL path, where the rung ladder
actually runs. `blocks(30)` — the `tests/call_budget.rs` fixture, 30
blocks in a single tower, `--threads 1`, release:

| declared wall | solved | spent |
|---|---|---:|
| *none* | yes | 0.19 s |
| 0.12 s | **no** | 0.13 s |
| 0.20 s | **no** | 0.25 s |
| 0.50 s | **no** | 0.59 s |
| 1.00 s | yes | 0.18 s |

A 0.5 s wall is 2.6× the unbudgeted solve time and still fails: telling
the engine it has half a second is strictly worse than telling it
nothing. `FF_WALL_DEBUG` names the mechanism outright —

```
wall: EHC slice exhausted (3678 evals in 0.12s), handing down the ladder
wall: novelty-light slice exhausted (4096 pops in 0.05s)
wall: LAMA slice extended to 0.13s ... extended to 0.37s (recency)
wall: novelty skipped (remaining Some(0.0) unaffordable at rung entry)
wall: best-first checkpoint expired at 1 evals (capped return)
```

— against `wall: solved by EHC` at 1.0 s and unbudgeted. EHC needs ~0.19 s
and `FF_EHC_WALL_FRAC` (0.25) hands it 0.12 s.

**The existing hatch is the zero-code arm.** `FF_NO_EHC_WALLCAP=1` converts
every failing wall above AND finishes INSIDE it (0.11 / 0.16 / 0.18 s) —
it is not trading the wall away. `FF_NO_RUNG_WALLCAP=1` also "solves" but
overruns 3× (0.88–1.39 s), so it is not a candidate policy, only the
permanent RED record it already is.

**Why this is not a free +N, and why the lane is unscoped.** The
discrimination is the whole problem, and this repo has already lost one
round of it. `ladder_wall.rs`'s `capped` leg exists because on the
`laddertax` shape EHC-direct eats ~2× the wall and the SLICE is what
converts the row; blocks(30) is the opposite shape at the same rung.
LAMA got a progress-conditional extension for exactly this reason (0.22
Phase 5A a1: hiking-2014 i6's 43 s of steady progress vs tetris i4's 400k
evals of none), and the 0.24 attempt to sharpen recency into ARRIVAL died
because nurikabe and spider's constants CROSS. EHC never got either
treatment. Its own progress signal ("no improving state") is
arrival-shaped by construction, which is a reason to think it separates
where LAMA's did not — and no reason at all to claim it does.

Owed before any claim: the two fixtures side by side (blocks(30) green,
`laddertax` still green), then a board A/B. Constituency on the boards is
UNMEASURED; nothing here licenses a number.

## Anti-pots — priced at zero, standing

Everything 0.27 listed carries forward. Added this cycle:

- **A preferences/ESPC wing without the parity probe.** The mechanism is
  already in the tree (`partition.rs`, `espc.rs`, `resolve.rs`), already
  default-on, and already beating SGPlan5 on quality. What is missing is a
  price, and one 1800 s probe is the whole cost of getting it.
- **Any Lane A claim from a board diff.** The 0.27 staged diff reads −22
  on `ipc5-prop` and −11 on `ipc5-time` on boards that still owe rows;
  board diffs on unfinished boards are noise. Named instances, same box,
  old binary, or nothing.
- **Widening the packed scheduler on the strength of 0.27's gains.** The
  gains are the engine (see the table above). The packing calibration's
  own numbers — 4-wide +72.8 % median wall inflation — stand.
