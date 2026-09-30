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

### Phase 0.1 — BUILT 2026-09-29 (`b4d2778`)

`judge`'s first line is no longer `if f.solved { Banked }`. A solve is
CLOCK-SENSITIVE when it came back unpriced/unscored or spent
`solved_wall_frac` (0.5) of its budget or more; such a solve is judged by
demoted, canary, swap and clock jump -- the process-and-clock signals a
failure is judged by -- and by nothing else. Calibrated on cut28's database
before it was written: the rule owes 115 of the 123 solves `recheck28.py`
re-opened by hand and banks the other eight, all unpriced rows measured on a
healthy box whose re-run then said the same three times, scored three and
lost two; on the boards as banked today it would re-run 62 of 5,466 solves.
The box-wide WINDOW was pre-registered as a condition and is NOT one: 489 of
the 720 half-budget solves ran under a dirty window, and re-running 7 % of the
boards every pass buys nothing the canary does not. The population is the
fixture (`crucible/tests/fixtures/cut28-reopened-solves.json`); the spec's
R2.1 table has the row; `crucible/preflight.sh` clean (once red on
`exec_supervision::a_short_spinning_child_is_not_undercounted`, a 120 ms
CPU-spin measurement under the parallel test build -- 3 of 3 alone).

---

### Phase 0.2 — BUILT 2026-09-29 (`3fe71b6`, the boards in the commit after)

`crucible compare` prints the estimators over every attempt of every cell
both engines measured -- banked / any / first / equal-N / per-run / clean-only
with cell-bootstrap intervals, the re-run trigger table, the decomposition of
`any`, and a same-sign verdict line. On cut28 it reproduces
`attempts-estimator.py` to the row on every point estimate (first +531,
equal-N +537, per-run +452.6, banked +344; the intervals within a few rows,
a different seeded generator); the Python is retired.

The three preference boards' quality cell is the IPC-5 quality score against
SGPlan5's `; MetricValue` -- `IPC score vs SGPlan5: 94.1 / 120.4 over 130
cells (29W/19T/82L; 0 solved unpriced)`, `59.2 / 92.1 over 100 (25W/3T/67L; 5
unpriced)`, `53.4 / 99.0 over 105 (13W/4T/59L; 2 unpriced)` -- ported in
lockstep in `standings.py` and `crucible-publish`, and `crucible standings
--check` renders the regenerated `ipc-standings.md` byte for byte. Lane 1's
measure now lives where the boards are published, not in a receipt script.

### Phase 0.4 — BUILT 2026-09-29 (`f1bd757`, `3fe71b6`)

The no-progress warning (no child, not SUSPENDED, not held, ten minutes:
`!! no planner running for N min ... the runner may be stuck`, and an
event); `ru_maxrss` from `wait4` on every row (`run.max_rss`, schema v9,
bytes) beside the sampled `peak_rss`; `publish.sh` refuses a crates.io token
file older than 80 days before spending the pre-flight.

### Phase 0.3 — RUN 2026-09-29 (`probes-0.29/bisect-losses/`)

Each of the five real losses, solo at 60 s, one thread, three reps, on 0.28.0
under every lane hatch and on v0.27.1 (`run.sh`; `read.py`):

| cell | 0.28.0 | + any hatch | v0.27.1 | verdict |
|---|---|---|---|---|
| tetris-sat i15 | **3/3** (30–45 s) | 3/3 | 0/3 | **not a loss** -- the cut and the re-check both banked it unsolved |
| sugar i5 | 0/3 | 0/3 | 0/3 | the box, or best-of-N: v0.27.1 fails it here too |
| line-exchange-snp 3_5_90_100 | 0/3 | 0/3 | 0/3 | same |
| sailing-wind-opt i10 (optimal) | 0/3 | 0/3 | 0/3 | same |
| **coins-2026 i18** | 0/3 | 0/3 | **3/3 at 54 s** | **REAL, and no hatch restores it** |

So the five are ONE. The pre-registered fork's third arm fires: coins i18 is
Lane 2's first fixture, and it is a wall-edge cell -- v0.27.1 needs 54 of the
60 s -- so the first question Lane 2 asks of it is whether 0.28.0 is slower
on the same search (Lane N's direction-aware fixpoint is value-identical and
was measured faster, not slower; the per-evaluation deadline reads and the
report reserve were not) or searching differently. Not answered here.

Two things about the instrument, though, that are. **tetris i15 solves on
0.28.0 every time alone and was banked unsolved twice** -- by the cut's
cascade and by the re-check's quiet-night pass -- which means the failure
side's referee let a timeout bank under conditions that made a 31 s solve a
60 s miss, and the SUSPECT rule (prior solved, solo attempt < 2) had been
spent by then. And **three of the five "real" losses were the box's** even
after a same-night differential: v0.27.1 solves none of them alone today.
The published 0.27.0 rows for them were rescues, which is the +344 / +531
story told cell by cell.

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

### Lane 1 — BUILT 2026-09-29/30 (`pddl3::polish`; `tests/pref_polish.rs`)

Mechanism (1), the plan-side step: from the banked plan, the violated
appendable preferences (goal preferences, soft `sometime` / `at end`
members) heaviest-first, each planned as a hard goal from the plan's end
state with the held ones kept as goals too, priced by the verifier, kept iff
the metric fell. `FF_NO_PREF_POLISH` hatch; `FF_PREF_POLISH_EVALS` (20,000)
per attempt. Fixture: a corridor whose optimizer cannot reach two cheap side
trips inside its budget -- metric 11 → 3, `SIDE1`/`SIDE2` gained, a kept
preference stays kept, the hatch keeps 11 (4 tests). First subset
(`probes-0.29/lane1/`, engine 43097c3e63c3): simple **94.1 → 96.6** (17
better / 1 worse), qualitative **59.2 → 59.7**, complex 53.4 → 51.2 (rows
100 → 98 on qualitative, 6 owed; unpriced 2 → 5 on complex). Diagnosis of
the small gain: on pathways-simple/16 no `[polish]` line at all -- the
optimizer runs to the report reserve and the polish starts with nothing.
So the optimizer now runs under a tightened deadline that leaves the polish
`FF_PREF_POLISH_FRAC` (0.4) of the remaining wall whenever a polish applies
(`pddl3::polish_applies`), on both the API and the text path. Re-measured
in the `lanes29` subset below.

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

### Lane 2 — the first question answered 2026-09-30 (`probes-0.29/lane2/`)

**Slower on the same search.** coins i18 at 20 s, solo, one thread: 0.28.0
and v0.27.1 run the identical trajectory (novelty-light 326,257 nodes,
novelty-driver 431,768, the same h), but 0.28.0's relaxed-graph build costs
11.0 µs per evaluation against 7.7 (+43%) -- 1,204,680 round-1 evaluations
against 1,716,551 -- so round 1 never leaves enough wall for the greedier
refill round (w_h 20), and that round is what solves the cell. Not the
toolchain: v0.27.1 rebuilt today on the same rustc reproduces 7.69 µs. Not a
hatch (Phase 0.3; `FF_NO_NEED_DIRS` is +0.4% on the fixed binary). Not the
per-evaluation deadline reads (equal clock samples in both profiles). The
`sample` profiles name it: `heuristic::widen` is the top self-time frame in
0.28.0 and absent from v0.27.1, and `nm` shows the symbol only in the 0.28.0
binary -- Lane N's `needs` parameter tipped LLVM into keeping `widen` an
out-of-line call, once per applied op per layer.

**Fix:** `#[inline(always)]` on `widen`, the reason in its doc comment.
8.5 µs per evaluation; coins i18 solo at 60 s: **0/3 → 3/3 at 54.8–55.0 s**
(v0.27.1: 3/3 at 54 s; 5,749,216 evaluations all three reps). A wall-edge
cell, so it banks only on a quiet box. No fixture: a timing assertion flakes
and a symbol-table assertion pins a mangled name; the receipt is the probe
directory and the numeric boards' rows, measured through the crucible with
the rest of the cycle. Left on the record: the residual +10% (candidates:
the batch closure's per-node deadline read, the `needs` branch), and ~2% in
`getenv` from `relaxed_to_inner`'s six `FF_NUMPRE_*` reads per evaluation,
whose unit tests set those variables in-process.

The consumable-aware relaxation itself (mechanisms a–c above) is not built
here; it opens on the rovers fixture with this regression out of the way.

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

### Lane 3 — BUILT 2026-09-30 (`temporal::refill_ladder`; `tests/tnode_refill.rs`)

The temporal ladder's node cap is a thread-local scale (`NODE_CAP_SCALE`)
that `refill_ladder` doubles, round after round, while more than a second of
wall remains and the scale is under 64× -- each round under
`tighten_deadline(0.0)`, so the memory wall is armed for it; no refill at
all without a declared wall. Called where `solve_prefless` used to decline and
after the compression rung's second attempt. `FF_NO_TNODE_REFILL` hatch;
`FF_TEMPORAL_NODE_CAP` is scaled too. Fixture: a 12-link chain under
`(always (safe))` with the cap forced to 6 and escalation off -- hatched, it
ends "exhausted its budgets with"; refilled, it solves and the note says
"temporal node cap refilled x". The first build refilled on ANY empty
ladder and `tests/ladder_dedup.rs` caught it (an unsolvable ring ran its
quartet twice over): the refill is now licensed only by a pass that ended
on its node cap (`NODE_CAP_TRIPPED`), never by one that exhausted its
space, its evaluation budget or its wall. Measured on `ipc5-constraints`
in `lanes29`.

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

### Lane 4 — streaming expansion BUILT 2026-09-30 (`constraints::expand_simplified`; `tests/constraints_stream.rs`)

The first mechanism. `expand` is now `expand_with(.., None)`; the monitor
compile calls `expand_simplified`, which threads the static simplifier
(`static_simplifier`, `None` under `FF_PREF_NO_STATIC`) through
`walk`/`walk_members` so every member is simplified as it is produced and
an accepted one is never stored. Fixture: 6×6 instances of a preference
body over a static predicate -- the streamed expansion is byte-identical
(`Debug`) to `expand` + `simplify_static` and keeps 12 of 36 members; the
hatch keeps them all. First build's plumbing stopped at `walk` (the members
were pushed unsimplified: 36); the fixture caught it. Measured on the
storage rows of the qualitative/complex boards in `lanes29`, memory by
`ru_maxrss` (Phase 0's `max_rss` column). The achiever index (second
mechanism) is not built.

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

### Lane W — BUILT 2026-09-30 (`search::ehc`; `tests/ehc_extend.rs`)

The slice is charged against the last improvement, not against entry: on
each `bfs_improve` arrival the slice deadline moves to now + slice, under a
ceiling of `FF_EHC_WALL_MAX_FRAC` (0.6) of the wall. `FF_NO_EHC_EXTEND`
hatch. Fixture recalibrated on the M5 (blocks(30) now solves in 0.03 s):
blocks(50), EHC alone 0.34 s, at a 1 s wall with the fixed slice a tenth of
it -- fixed: 0/3, "EHC slice exhausted", the ladder does not rescue it;
arrival-charged: 3/3 at 0.34 s, "solved by EHC". `ladder_rungs_pay_the_wall`
stays green (17.8 s). Measured on `ipc2018-sat` and `ipc2023-sat` in
`lanes29`.

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
