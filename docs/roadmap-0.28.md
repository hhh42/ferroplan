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

**Owed before any SGPlan claim, and cheap: one parity probe.** Run the
qualitative and complex boards at 1800 s, once, as a MEASUREMENT and not a
tier — it answers whether the fight is 54 rows or 5. The standing anti-pot
against new tiers is about *published* tiers; this buys no board and
changes no table. Until that number exists, an SGPlan wing is unpriced,
and this project has three cycles of evidence that bands taken from other
planners' published results deliver +1.

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
