# ferroplan 0.29 roadmap — level on rows; now level on points

Scoped 2026-09-29, the day 0.28.0 was published. Order agreed with the
operator, and it is the order the work runs in: **Phase 0 (the instrument) →
Lane 1 (preference quality, the headline) → Lane 2 (a relaxation that can see
a consumable) → Lane 3 (the constraints board's thrown-away wall) → Lane 4
(the memory class) → Lane W (a declared wall making the ladder worse).** All
0.29 work lives on `engine-0.29` in its own worktree
(`/Users/harold/ferroplan-0.29`); every completed phase pushes to `main` as
well as the branch (`CLAUDE.md`).

The cycle's discipline is unchanged: fixtures first; a measured win or a
recorded negative; every claim through the crucible (`crucible-spec.md` R3
subsets for questions smaller than a release, the 32-board sweep for the
cut); the comparative estimator declared before the sweep runs. This cycle
adds a second declared measure, because the first one has run out of room on
the boards that matter (below).

---

## The finding this cycle is built on

0.28.0 closed the ROW gap to SGPlan5 on the IPC-5 preference boards and left
the POINT gap where it was. On the promoted boards (`probes-0.28/cut28/`):

| track | rows 0.27.0 → 0.28.0 | SGPlan5 rows | IPC quality 0.27.0 → 0.28.0 | SGPlan5 quality |
|---|---|---:|---|---:|
| simple-preferences | 119 → **130** / 130 | 129 | 94.8 → 94.1 | 120.4 |
| qualitative-preferences | 51 → **100** / 100 | 100 | 45.8 → 59.2 | 92.1 |
| complex-preferences | 29 → 80 / 108 | 105 | 20.1 → 53.4 | 99.0 |

The IPC quality score is best-metric / own-metric per cell, summed over the
cells SGPlan5 solved (`cut28/quality.py`; 0 for no plan or no metric). IPC-5
ranked these tracks on it. Two shapes make up the gap, and the row notes name
them cell by cell:

1. **Every simple/qualitative solve ends "search bound hit; best-found"**:
   the optimizer spends its eval budget and reports the best it saw. On
   trucks (qual 5.6 of SGPlan5's 18.5, simple 8.4 of 19.0) and tpp what it
   saw was mostly the incumbent it started from. The seed is a floor, and on
   these the floor was the ceiling.
2. **The empty plan.** 76 of the six boards' 569 solves have no hard goal and
   return "do nothing" — pathways-simple 21 of 30, pathways-complex 28 of 30,
   tpp-qualitative 13 of 20, storage-qual 6 — valid, VAL-accepted, and worth
   zero points on every one of those cells. 0.27.0 carried 27 of these, so
   the cycle that put "feasible first" in the route also doubled them.

Elsewhere the row gap has a shape too. metric-time is 66 of the remaining 121
rows to SGPlan5: rovers 8/40, tpp 8/40, pathways 8/30, every miss "temporal
ladder stopped at the wall" — the relaxation cannot see a consumable being
spent, the residue 0.28 named and did not build. The 2023 and 2026 numeric
boards moved DOWN this cycle (−0.5, −0.6 pts), and four of the five real
losses are numeric (sugar i5, coins i18, line-exchange-snp, sailing-wind-opt
i10). The constraints board is 28/120: tpp-metric-time-constraints 0/30, with
**23 of them "temporal ladder exhausted its budgets with N s of wall left"**
(pipesworld-mt-constraints 14 more) — the ladder's node caps trip and, the
compression rung declining constrained tasks, nothing spends what remains.

And the instrument. The cut fell through a hole the roadmap had already
named once: the referee re-runs a row that FAILED under bad conditions and
never one that SOLVED under them (`judge` opens with `if f.solved { Banked }`).
Ten pathways-simple cells solved during a 52-hour stretch when a stray
simulator held the box 2.4–6.7× slow — the empty plan, unpriced at the wall —
and were banked; simple-preferences quality read 82.0 until 147 rows were
re-opened by hand (`recheck28.py`). The estimator was pre-registered and
did its job (+531 first-attempt against +344 banked, the boards understating
the engine for once), but it lived in a script beside the harness.

---

## The two declared measures (pre-registered, before any lane runs)

1. **Rows**, as 0.28 declared them: the comparative claim is first attempt
   against first attempt over the 8,444, equal-N beside it, banked coverage
   published as the boards' standing number and never as a delta; no delta
   claimed where the estimators disagree in sign.
2. **Points**: the IPC quality score on the three preference boards, over
   the cells SGPlan5 solved, against SGPlan5's `; MetricValue` from the
   vendored archive, computed by a script the crucible pre-flight round-trips
   (Phase 0.2 below). Lane 1 is judged on THIS, not on rows. Its band and its
   kill are written in points.

Both are read on the cut sweep, and on subsets during the cycle only with the
subset's engine hash named beside the number.

---

## Phase 0 — the instrument (blocking; a week; defends every number after it)

0.1 **A solve can be SUSPECT.** In `judge`, a solved row goes through the same
   table a failure does when its outcome could depend on the clock: the
   engine returned it without its number ("NOT priced" / "NOT scored" in the
   notes), or it spent half its budget or more while the box was demoted, the
   canary above the rule's factor, or the window dirty. It is re-run the way
   a failure is, and the LATER row banks. Fixtures: the pathways-simple shape
   (solved, unpriced, canary 3×) is owed; the same row at canary 1.0 banks;
   a 0.3 s solve under any canary banks. Measured: `recheck28.py` over
   cut28's database must select nothing the rule would not have.
   *Pre-registered risk:* more re-runs on slow days; the rule's cost is
   reported as rows re-run per pass beside the sweep's pass count.
0.2 **`compare` prints the estimator, and `standings` prints the points.**
   `crucible compare --a A --b B` reports first-attempt, equal-N, per-run and
   banked with their intervals (the port of `attempts-estimator.py`, which
   then goes away), and the IPC quality score joins `ipc-standings.md`'s
   preference rows as a column. The oracle differential covers both.
0.3 **The five real losses, bisected.** Each of tetris-sat i15, sugar i5,
   coins i18, line-exchange-snp 3_5_90_100, sailing-wind-opt i10 is run solo
   on 0.28.0 with each lane hatch in turn (`FF_NO_TCOMPRESS`,
   `FF_PREF_NO_SEED`, `FF_NO_NEED_DIRS`, `FF_NO_MEM_WALL`, `FF_NO_TSUCC`) and
   on v0.27.1, three reps each. Pre-registered fork: one hatch restores all
   four numeric losses → that lane has a numeric defect and Lane 2 opens on
   it; no hatch restores them → the losses are the relaxation's and Lane 2
   inherits them as fixtures; the box restores them → recorded, not a loss.
0.4 **Small and owed:** a no-progress warning when a sweep has spawned
   nothing for N minutes while it owes rows and is not SUSPENDED (the
   three-hour hang of 09-21); `ru_maxrss` from `wait4` on the row beside the
   sampled `peak_rss`; the crates.io token's age in the release pre-flight
   (the first `publish.sh` of 0.28.0 failed on a token that had expired two
   days before).

Phase 0 ships as a crucible release on `main` before Lane 1 opens; nothing
in it changes a board number except by measuring it again.

---

## Lane 1 (headline) — the preference optimizer's first improving step

**Claim:** the IPC quality score on the three preference boards moves toward
SGPlan5's. **Band:** qualitative 59 → 75 of 92 and simple 94 → 105 of 120
at 60 s, one thread; complex follows the same mechanism but is also memory-
and ladder-bound and is not in the band. **Kill:** a lane that moves rows and
not points is a negative for THIS lane whatever the rows say; a mechanism
that costs a row on any preference board is not shipped as a default.

The mechanism, in the order it is tried (fixtures first for each):

1. **A plan-side local search over the incumbent** — SGPlan5's own
   architecture. From the banked plan (the seed, or the optimizer's
   best-found), try to satisfy one more preference at a time by planning the
   preference's formula as a hard goal from the plan's end state or by
   splicing, keep the result iff the metric improves and the plan validates,
   repeat until the reserve. Anytime, monotone, and it never touches
   coverage: the banked plan is the floor. The fixture is a corridor where the
   optimizer's B&B cannot reach a cheap side trip inside its eval budget and
   the local step can.
2. **The empty plan is a seed, not an answer.** On a problem with no hard
   goal the route currently returns "do nothing" and starts the optimizer on
   it. With (1) in place, the local search is what runs on those 76 cells;
   pathways and tpp-qualitative are where the band is won or lost.
3. **The scorer at plan size.** `SoftScorer::prepare` grounds the full snap
   task to score one temporal plan — five pipesworld-complex cells come back
   "NOT scored" and pathways-complex i20 pays 12.7 s for it. Lane T's
   `validate_at_plan_size` is the pattern; `verify::eval_formula` must first
   read never-grounded atoms from the init rather than as false, with a
   fixture that scores one plan both ways.

Measured through the crucible on `ipc5-simple-pref`, `ipc5-qual-pref`,
`ipc5-complex-pref` as subsets, points and rows both, engine hash beside
every number; then on the cut.

---

## Lane 2 — a relaxation that can see a consumable

**Claim:** metric-time rovers/tpp/pathways (24 of 110 solved, 76 "stopped at
the wall") and the numeric boards move. **Band:** metric-time +15 to +30
rows; 2023/2026 numeric back to 0.27.0's numbers at least. **Kill:** no
movement on rovers-metric-time i10–i20 solo at 60 s after the fixture
passes → recorded negative; the fixture is kept as the RED record.

The named defect (0.28, Lane N's residue): the relaxed planning graph treats
a consumable's un-read lower bound as progress and never learns that the
resource runs out — rovers' best-h reaches 11 and sits there for 900k
evaluations. The mechanism candidates, in order: (a) charge consumption in
the relaxation — a fluent decreased by every achiever of a needed fact
bounds the number of times those achievers can fire; (b) saturate needed
bounds that only ever grow instead of building to the layer cap; (c) the
partition subplanner keeps its Phase-A subplans instead of re-solving every
merge. Phase 0.3's bisection decides whether a numeric defect from 0.28
comes first. Fixtures: the rovers consumable shape at fixture scale, RED on
0.28.0's evaluation count.

---

## Lane 3 — the constraints board throws its wall away

**Claim:** the 37 cells whose note reads "temporal ladder exhausted its
budgets with N s of wall left" (tpp-metric-time-constraints 23 of 30,
pipesworld-metric-time-constraints 14 of 20) get the rest of their wall.
**Band:** +10 to +25 rows on `ipc5-constraints` (28/120 today, SGPlan5 47).
**Kill:** spending the wall does not solve them → recorded, and the note
changes to say what the wall was spent on.

The compression rung's second attempt covers this shape on UNconstrained
tasks (0.28: "nothing else is going to use what is left"); it declines
trajectory constraints, so on the constraints board the ladder's node caps
trip and the process exits with most of a minute unspent. The optimal ladder
already refills its node cap in place when wall remains (`FF_NO_NODECAP_REFILL`
is its hatch); the temporal ladder gets the same, under the memory wall.
`within` / `always-within` are accepted on the temporal path since 0.24 and
are NOT the blocker here; the earlier scoping that said so was wrong, and
this section is its correction.

---

## Lane 4 — the memory class

Two mechanisms, both named in 0.28 and both with a receipt:

- **Streaming constraint expansion.** `constraints::expand` materialises
  every preference instance before dropping the statically true 98 %:
  storage-qualitative i19/i20 pass 6 GB before any plan exists (2.3 M
  instances). Simplify each instance as it is produced. Touches the verifier
  and the temporal scorer, which read the expanded set. Band: the two rows,
  and the replay pricing 0.28 recorded negative becomes affordable.
- **The ops × shared-monitor achiever index.** "packing the ops" builds an
  achiever table hundreds of millions of `u32`s wide on big preference tasks
  (46 s, GBs) — the `mem-cap` class on storage-complex (14 of 20) and
  pipesworld-metric-time (9). Virtual "every monitored op" achievers instead
  of one entry per op per monitor add. Band: storage-complex +8 to +14.

---

## Lane W — a declared wall makes the classical ladder worse

Unscoped in 0.28, fixtured already: `blocks(30)` solves unbudgeted in 0.19 s
and FAILS under a declared 0.5 s wall, because `FF_EHC_WALL_FRAC` hands EHC
0.12 s of a solve that needs 0.19. `FF_NO_EHC_WALLCAP=1` converts every
failing wall AND finishes inside it. The discrimination is the whole problem
(`laddertax` is the opposite shape at the same rung), and EHC's own progress
signal is arrival-shaped where LAMA's was not. **Measure first:** both
fixtures side by side, then `ipc2018-sat` and `ipc2023-sat` as subsets under
a progress-conditional EHC slice. **Kill:** `laddertax` goes red, or the
subsets read ≤ 0.

---

## Anti-pots — priced at zero, standing

Everything 0.28 listed, plus:

- **Any rows claim on the preference boards.** They are at 130/130 and
  100/100; the only thing left to claim there is points, and Lane 1 is judged
  on points.
- **A quality number from an overlay.** The 0.28 read stitched two engine
  hashes to estimate 569; the points measure is read on ONE engine or not
  reported.
- **"Fixed the referee" without `recheck28.py`'s population as the test.**
  147 rows are the fixture Phase 0.1 must reproduce from the rule alone.
- **Reopening `FF_TCONC`'s default.** The compression rung stands aside for
  the actor scheduler; the cabin crews read 109 / 63 / 47 and stay pinned.

---

## The cut rule

The 32-board `cut29` set, the same instrument; both declared measures read
on it under the rules above; the five 0.28 losses and Lane 1's cells re-run
on both engines before any is named; `recheck` is the referee's job now, not
a script's. Nothing is published until `main` has it and the pre-flight —
with the token's age on the checklist — is green.
