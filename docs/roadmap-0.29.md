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
(`pddl3::polish_applies`), on both the API and the text path.

**What the share found on pathways-simple/16 (2026-09-30, solo, 60 s):**
with 15 s kept back the polish still gained nothing, and the named exits
say why, in order. (i) Its goal for an attempt conjoined every HELD
preference body, and on pathways those are `or`s: twenty of them are 2^20
goal disjuncts, and every attempt expired in the grounder's DNF -- held
bodies now ride along only when the DNF takes them at width one
(`dnf_flat`); the rest are guarded by the pricing. (ii) An attempt's slice
was cut to exactly its half-second floor and then compared against it, so
every candidate at the floor was skipped ("0 of 20 attempted"): the guard
now reads the wall before the cut. (iii) Grounding: the plain pathways task
grounds in 8.4 s, the baseline pricing alone cost 8.5 s of the 15 s window,
and six 1 s attempts all died in binding enumeration. The floor is now
twice the baseline's grounding time, and the SHARE is only taken from the
optimizer -- and the polish only started -- when the wall left is over
twice what the compiled task cost to ground (`polish_if_affordable`; on
pathways-simple/16 that is 17 s, so the cell behaves as 0.28 did, with the
reason logged). **The ceiling this names:** a polish that re-grounds the
task per attempt cannot reach the slow-grounding domains, and pathways is
where the simple band is lost (21 of 30 empty plans). The next step is one
grounding per polish with the goal facts swapped per attempt -- the
"scorer at plan size" work, mechanism (3) -- not more wall.

**MEASURED 2026-09-30 (`lanes29` pass 1, engine 17fc6d6f0bd8; the owed
re-runs of passes 2-3 can move single cells, not the shape):**

| board | 0.28.0 points | 0.29 | delta | better / worse cells | rows 0.28.0 → 0.29 | SGPlan5 |
|---|---|---|---|---|---|---|
| simple (130) | 94.1 | **96.0** | +2.0 | 27 / 6 | 130 → 130 | 115.3 |
| qualitative (100) | 59.2 | **63.3** | +4.1 | 19 / 0 | 100 → 100 | 92.0 |
| complex (105 priced) | 53.4 | 51.4 | −2.0 | 0 / 0 | 80 → 80 (+1 gained, pathways/27, unpriced) | 99.9 |

A measured win on the two boards the band names, under the band (simple
wanted 105, qualitative 75): the polish helps where its attempt can ground
(trucks, storage, tpp) and cannot reach pathways at all, which is where the
band is. Not one qualitative cell got worse. **Complex is a loss of the
Lane 1 kill kind, and not the polish's:** the −2.0 is four pipesworld
cells (i12, i14, i15, i16) that 0.28.0 scored post-hoc and 0.29 banks
"NOT scored: the wall or the memory budget left no room to build the
scorer" -- at 6-21 s of the 60, so it is the memory budget: their peak
RSS went 3.8-4.2 GB → 4.6-5.2 GB, across the memory wall's 4.5 GB trip
(0.75 × 6 GB), and the scorer, which grounds the original task, is refused.
**Probed solo the same day (`probes-0.29/lane3/probe-pipes.out`): not the
lanes'.** 0.28.0's own binary on i14 today: 4.50 GB, 5.9 s, NOT scored;
0.29 default and under every lane hatch (`FF_NO_TNODE_REFILL`,
`FF_PREF_NO_STATIC`, `FF_NO_EHC_EXTEND`, `FF_NO_PREF_POLISH`): 4.50 GB,
5.9 s, NOT scored -- byte for byte the same outcome. `FF_NO_TCOMPRESS=1`:
0.44 GB and unsolved, so the 4.5 GB is the compression rung's, whose plan
arrives with the process sitting exactly on the memory wall's trip; whether
the scorer then fits is the box's call, and the cut's 0.28.0 rows (11-15 s,
3.8-4.2 GB sampled, scored) were the other side of that coin. The complex
−2.0 is therefore pricing variance of four wall-edge cells, an instrument
finding to carry into the memory class: a banked plan whose scorer cannot
be built because the SOLVER's arena is still resident is Lane 4's second
mechanism's row, and the "scorer at plan size" (mechanism 3) is what
removes the grounding from the scorer altogether.

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

**MEASURED 2026-09-30 (`lanes29` phase B, the loss side, engine
17fc6d6f0bd8):** every row 0.28.0 solved on `ipc2023-numeric` (262) and
`ipc2026-numeric` (223) re-measured under the inlined build -- pass 1
banked 230 and 198 of them solved and not one as a failure; the rest (32
and 25) were owed to a box the operator was using and re-run in passes 2-3,
which banked **three failures of one shape: counters-numeric i12, i13,
i16** (0.28.0: solved at 39 / 25 / 36 s in best-first's first round) now
"mem-cap (self-inflicted: node byte target raised)" at 16-25 s -- the
0.22 refill re-entering a node-capped round with its byte target doubled
into watchdog territory, and the RSS watchdog taking it. Two readings fit:
the box (i16's first 0.29 attempt SOLVED at 32.8 s and was owed thermal;
a slower box cuts round 1 short at its wall-fraction checkpoint, and the
refill's deliberate overshoot does the rest), or the lane (faster
evaluations reach the same cap sooner). The solo probe on a quiet box
(`probes-0.29/lane2/probe-counters.sh`) decided it the same afternoon,
on the quiet box: **the box.** i12 -- 0.29: solved, 988,924 evaluations,
27.0 s, peak 6.77 GB, byte target raised x2 then x4; 0.28.0: solved,
988,924 evaluations, 27.3 s, peak 6.78 GB, the same two raises. i13 --
411,518 evaluations both, 20.5 s against 21.3 s, 6.82 against 6.87 GB,
one raise each. The inlined build runs the identical trajectory; what
banked the three rows was a refill that overshoots a 6 GB budget to
6.8 GB by design (0.22: "watchdog territory, never OOM-the-box") meeting
a box whose operator held the rest of the memory. Without the raise
(`FF_NO_NODECAP_REFILL=1`) neither cell solves at all (seven refill rounds
to the wall); without the refill, neither does. So the overshoot is what
solves counters, and it is a coin on a 16 GB box running two 6 GB jobs
-- a memory-class row for the ledger, not Lane 2's. The gain side (the 235 rows
0.28.0 did not solve, coins i18 among them, each a solo minute) is phase C,
queued behind this on the same engine; it runs in the box's idle gaps.

**VERDICT 2026-10-01 (`lanes29` phase C on the fixed build be638e381c26,
`crucible compare --a 89cfdc5f06ed --b be638e381c26`, the pre-registered
first-attempt estimator):**

| board | first-attempt B − A | 95 % CI | cells both banked | the cells |
|---|---|---|---|---|
| ipc2026-numeric (320) | **+2** | [+0, +5] | 1 gained, 0 lost | coins-2026 i18 (solved 2 of 3 attempts, 56.9 / 56.5 s, the third a 54.2 s miss; every attempt owed by the canary loop, so unbanked), line-exchange-snp 3_5_90_100 (banked, 52.9 s) |
| ipc2023-numeric (400) | −1 | [−5, +3] | 1 gained, 1 lost | gained zenotravel i20 (banked, 24.3 s); lost counters i13 -- the cell probed solo the day before: identical evaluations, peaks and refill raises under both binaries, both solve; the box's coin |

The tool declines a delta on both boards (the estimators disagree in sign
because the owed rows sit in `banked` and nowhere else); `first` is the
claim and it reads +2 and −1 with the intervals above. **What the lane set
out to do it did: the one real 0.28.0 loss, coins i18, is back** (0/3 →
2/3 at the wall's edge), with two more wall-edge numeric cells beside it
and nothing lost that was not a probed coin. **What the lane was NAMED
for -- the consumable-aware relaxation and the metric-time band -- was not
attempted this cycle**; the 24-of-110 metric-time shape and the rovers
fixture stand as written, and the lane stays open on them.


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
space, its evaluation budget or its wall.

**MEASURED 2026-09-30 (`lanes29` pass 1, engine 17fc6d6f0bd8) -- NEGATIVE
so far, the kill clause fires.** `ipc5-constraints`: **28 solved of 120,
the same 28** (84 banked, 36 owed to a box the macOS update daemon was
loading; their re-runs come with phase C). The note shapes moved the way a
spent wall moves them: "exhausted its budgets with N s of wall left" 40 →
30 rows (the survivors mostly 1-6 s left, two with 16-20 s: passes that
ended on their evaluation budget or the memory wall, not a node cap, so no
refill licence) and "stopped at the wall" 35 → 46. Spending the wall did
not solve one of them. Per the kill clause the note must now say what the
wall was spent on: an unsolved temporal verdict now ends "after refilling
the temporal node cap to xN" whenever a refill ran (`temporal::
refill_scale_reached`), built and gated after phase B so the loss side
stayed on one engine.

**And the note's first reading named the real defect (2026-09-30 17:40,
tpp-metric-time-constraints i11 solo under `FF_WALL_DEBUG`):** "exhausted
its budgets with 38 s of wall left after refilling the temporal node cap
to x64" -- six refill rounds, and `[tsearch] cap hit (nodes 400003 / max
400000)` on every one of them at 3 s. `temporal_node_cap` scaled the model
and THEN clamped it to the historical 400,000-node ceiling, so on a board
whose model already sat on the ceiling the x2..x64 were all 400,000: the
refill re-ran the identical pass six times. The fixture pinned the
`FF_TEMPORAL_NODE_CAP` path, which never meets the clamp. Fixed: clamp,
then scale (`scaled_node_cap`, unit-pinned on the ceiling shape), the
arena bounded by the measured memory wall the refill rounds already run
under. So the 28 → 28 above measured a no-op, not the mechanism. On the
fixed build (6dbb801) the same cell reads as designed: caps 400k → 800k →
1.6M → 3.2M nodes (x16 cut off by the wall at 917k), peak 3.8 GB under the
memory wall, "exhausted its budgets with 2 s of wall left after refilling
the temporal node cap to x16" -- the wall spent, the cell still unsolved.
The constraints board, re-measured on this engine (phase C, pass 1,
2026-09-30 18:40): **28 solved of 120, the same 28**, 71 failures banked
clean, 21 owed to the box. Spending the wall through a refill that now
really grows the arena (x2..x16 under the memory wall) solves none of the
40 cells the lane was named for. **Recorded NEGATIVE; the kill clause's
both halves are done** -- the wall is spent and the note says on what. The
mechanism stays (it costs nothing on a row that was failing anyway and
the fixture pins it), and the 40 cells' next question is why their
searches do not converge inside 3.2 M nodes, which is a heuristic
question, not a budget one.
The refill was also the suspect for the complex board's four unpriced
cells above; the same-day probe cleared it (0.28.0 reproduces them, and
`FF_NO_TNODE_REFILL=1` changes nothing on them).

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

**MEASURED 2026-09-30 (`lanes29`, engine 17fc6d6f0bd8) -- NEGATIVE on
memory, the band's two rows already solved.** Storage rows i17-i20, peak
RSS by `ru_maxrss` (the sampled peak agrees to the MB), 0.28.0's banked
attempt beside it:

| cell | 0.28.0 peak (banked attempt) | 0.29 peak | solved 0.28.0 → 0.29 |
|---|---|---|---|
| storage-qual i17 | 4.6 GB (3.6 s) | 5.0 GB (4.4 s) | yes → yes |
| storage-qual i18 | 5.2 GB (3.1 s) | 4.8 GB (4.7 s) | yes → yes |
| storage-qual i19 | 4.6 GB (6.2 s) | 5.2 GB (5.2 s) | yes → yes |
| storage-qual i20 | 5.3 GB (54.5 s, its 4th attempt) | 4.9 GB (5.9 s, first attempt) | yes → yes |
| storage-complex i17 | 5.4 GB | 4.7 GB | yes → yes |
| storage-complex i18 | 5.4 GB | 5.7 GB | yes → yes |
| storage-complex i19 | 6.5 GB | 6.3 GB | no → no |
| storage-complex i20 | 6.2 GB | 6.3 GB | no → no |

The peak is the same 5-6 GB with the expansion streamed, so the
materialised expansion was not where these cells' memory lives; the mem-cap
class on storage-complex is the second mechanism's (the ops × monitor
achiever index), which is not built. The streamed expansion stays as
correctness (byte-identical, fixture-pinned) and costs nothing; it is not a
Lane 4 win. i19/i20 on the qualitative board were already 0.28.0 solves
(rescues); 0.29 banks them first attempt, which is the 0.28 recheck's
"box" story, not this lane's.

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
stays green (17.8 s).

**MEASURED 2026-09-30 (`lanes29` phase B, the loss side, engine
17fc6d6f0bd8):** every row 0.28.0 solved on `ipc2018-sat` (97) and
`ipc2023-sat` (53) re-measured under the arrival-charged slice -- 91 and
29 banked solved, the rest owed to the operator's daytime load and re-run
in passes 2-3 -- which banked ONE failure: **spider-sat i17**, a cell
0.28.0 solved at 60.0 s exactly (the wall), unsolved at 60 s on its third,
clean attempt here -- and on `ipc2023-sat` **labyrinth i9**, 0.28.0's
second-attempt rescue at 58.5 s, unsolved three times. Two wall-edge cells:
a coin or the slice's doing; the solo probe (`probes-0.29/laneW/
probe-spider.out`, both cells, three reps an arm, quiet box) says coin:
**spider i17 solves at 60.00 s by LAMA in all nine runs** -- 0.29, 0.29
with `FF_NO_EHC_EXTEND=1`, 0.28.0 alike -- a plan that arrives exactly at
the wall and banks or not on the runner's clock; **labyrinth i9 fails in
all nine** ("EHC slice exhausted", the slice extended or not), so its
0.28.0 row was the rescue coin the estimator's first-attempt view already
discounts. The loss side is clean. The kill's second clause
("the subsets read ≤ 0") needs the gain side: the 230 rows 0.28.0 did not
solve are phase D, a quiet night's work, not chained behind the daytime
phases.

**VERDICT 2026-10-01 (phase D, the gain side, fixed build be638e381c26):
the subsets read 0 -- recorded NEGATIVE on rows.** `ipc2018-sat`: 0 of
the 143 unsolved rows solve (26 failures banked clean, the rest owed);
`ipc2023-sat`: 0 of 87. Both sides together: nothing gained, nothing
lost but two wall-edge coins the probes called. The mechanism stays as a
default on the fixture's evidence alone, which the 60 s boards cannot
see: a declared 1 s wall on blocks(50) is 0/3 with the fixed slice and
3/3 with the slice charged to arrival, and `laddertax` stays green -- an
API caller with a short wall gets the plan it was being denied. On the
boards it is a zero, and it is recorded as one.

---

## Where 0.29 stands (2026-10-01, main at the records' commit)

| lane | claim | measured (lanes29, engine beside each number in the records) | verdict |
|---|---|---|---|
| 1 preference quality | points toward SGPlan5 | simple **94.1 → 96.0**, qualitative **59.2 → 63.3** (0 cells worse), complex 53.4 → 51.4 (four wall-edge pricing coins, 0.28.0 reproduces them) | **win, under the band**; the ceiling is named (re-grounding per attempt; pathways at 8 s a ground) |
| 2 numeric | coins i18 back; the consumable relaxation | coins i18 0/3 → 2/3, 2026-numeric first +2 [0, +5], 2023-numeric −1 [−5, +3]; the 0.28 codegen regression found and fixed (`widen` inlined, −23 % per evaluation) | **the regression fixed; the relaxation open** |
| 3 constraints wall | +10 to +25 rows | 28 → 28 of 120 with the refill really growing the arena (first build's refill was a no-op: scale clamped) | **negative**, kill clause done both halves |
| 4 memory | storage i19/i20, the replay | storage peaks 5-6 GB before and after the streamed expansion | **negative on memory**; correctness kept |
| W EHC slice | sat subsets > 0 | 0 gained, 0 lost on 2018-sat / 2023-sat; blocks(50) at 1 s 0/3 → 3/3 in the fixture | **negative on rows**; kept for short walls |

Instrument findings carried into Phase 0's ledger below: the pass cap
that does not apply to subsets, the thermal canary measured at width
against a solo baseline, the wall-edge coins. Next for the cycle, in
order: the canary and the pass cap (every daytime number this cycle paid
for them), Lane 1's one-grounding polish, Lane 2's relaxation on the
rovers fixture, Lane 4's achiever index.

## Re-plan 2026-10-04 — significant gains on the competition

Agreed with the operator three days after the lanes' first round, with the
gap table below in hand. Two decisions and a re-ordering:

- **The headline is rows first: Lane 2's consumable-aware relaxation.** The
  largest measured pool is metric-time (24 of 110 on rovers/tpp/pathways,
  every miss "temporal ladder stopped at the wall"), and the same defect
  caps the numeric boards (markettrader 1/20, settlers 3/20, expedition
  5/20, sailing-wind 0/20). Lane 1's one-grounding polish (points) follows
  it, not the other way round.
- **A classical memory lane opens (Lane 5).** 82 mem-cap rows on 2014/2018/
  2023 seq-sat trip at a median 19–27 s with half the wall unspent
  (city-car 15, child-snack 10, maintenance 4; caldera 16,
  organic-synthesis 8+2, snake 5; folding 16, which trips at 7 s and is
  grounding). The classical ladder never arms the memory wall, so the runner's
  cap kills a search that could have handed its wall to a leaner rung.
- **The README's standings block is a capability table**, landed on `main`
  2026-10-04 with 0.28.0's numbers (what the planner is good at, its rank in
  each competition's field, the preference boards' points beside their rows);
  it regenerates at the cut. `benchmarks/manifest.toml` carries
  `capability` / `front_name`; `crucible standings --check` covers the README.

### The gap table the lanes are chosen from (0.28.0 boards vs field leaders, by rate)

| board | ours | leader | rows to leader's rate | pool shape |
|---|---|---|---|---|
| IPC-5 propositional | 384/450 | SGPlan5 218/220 | +62 | near-wall timeouts 57–59 s, 0 mem-cap (trucks 13/30, pathways 26/30, storage 26/30) |
| 2014 seq-opt (proof) | 82/256 | SymBA*-2 151/280 | +56 | wall; admissible-heuristic work, out of scope this cycle |
| IPC-5 metric-time | 105/200 | SGPlan5 151/200 | +46 | 76 "stopped at the wall" — Lane 2 |
| IPC-5 constraints | 28/120 | SGPlan5 47/80 | +42 | Lane 3 negative; a heuristic question, not budget |
| 2014 seq-sat | 173/280 | IBaCoP2 198/280 | +25 | 30 mem-cap + 77 timeouts — Lane 5 |
| complex-pref | 80/108 | SGPlan5 105/108 | +25 | 14 mem-cap (storage) — Lane 4; 13 wall (tpp) |
| 2011 seq-sat | 240/280 | LAMA-2011 250/280 | +10 | floor-tile 8/20, transport 11/20 |
| 2018 seq-sat | 97/240 | field mean 94 (winner not held) | — | 35 mem-cap + 108 timeouts — Lane 5 |
| 2023 seq-sat | 53/140 | — | — | 17 mem-cap (folding 16, trips at 7 s = grounding) |
| 2023 numeric | 262/400 | ENHSP ref 267/400 | +5 | 33 mem-cap (pathwaysmetric 10, markettrader 9, tpp 7) + 104 timeouts |
| preference points | qual 63/92, simple 96/120, complex 51/99 | SGPlan5 | points, not rows | Lane 1 |

Ahead of the leader already: time, tempo-sat 2008/2011, 2008 seq-sat,
net-benefit, 2026 numeric-opt (a tie). Those are the README's "good at" rows.

### The order from here

**Phase 0.5 (instrument) → Lane 2 (headline) → Lane 5 (classical memory) →
Lane 1 (one-grounding polish) → Lane 4 (achiever index) → the trucks probe →
the cut.** Fixtures first, band and kill pre-registered, every number through
the crucible with the engine hash beside it, `main` fast-forwarded when a
lane's record is written; sweeps at night, no cargo on the box while one runs.

**Phase 0.5.** (a) `--max-passes` on subsets: `sweep.rs::wait` checks
`passes >= max`, yet `lanes29` ran a fifth pass under `--max-passes 3`; find
whether the subset path never increments `passes` or bypasses `wait`; fixture
in `crucible/tests/subset.rs`, a permanently-owed row under `--max-passes 2`
runs exactly two passes. (b) The canary baseline at width: `Canary::calibrate`
takes solo readings and the running canary sits beside nine jobs (1.5× on a
quiet box), so packed passes owe every solo row and re-run them packed — a
loop; record the pack width with every reading and key the baseline percentile
(`Reader::canary_baseline`) by width, or take `baseline_n` readings at the
configured width; fixture: a width-9 reading against a width-9 baseline is
clean, against a solo one it is not; measured by re-running `lanes29`'s owed
rows and counting "thermal" owings per pass. (c) Record only: the failure-side
referee banked tetris i15 unsolved twice under conditions a 31 s solve could
not survive.

**Lane 2, the mechanism steps** (section above holds claim / band / kill):
(1) fixture `tests/numeric_consumable.rs` — the rovers shape at fixture scale,
a fluent every achiever of a needed fact decreases, stock enough for k firings,
goal needs k+1, RED on 0.28.0 by evaluation count with the expected h sequence
pinned (the plateau half of `a_dead_end_with_a_draining_consumable_is_a_fixpoint`);
(2) charge consumption in `build_rpg`: a fluent only ever decreased along
applied ops and read by a needed precondition bounds its achievers' firings by
stock / per-firing cost, so the relaxed plan's cost grows as the resource runs
short — value-identical where no consumable is read (Lane N's 123/123 pin);
(3) saturate growing bounds at the first layer no needed comparison can
change, instead of running to `LAYER_CAP`; (4) the probe's residue — cache the
six `FF_NUMPRE_*` reads in `relaxed_to_inner` (2 % of every evaluation; the
unit tests that set them in-process reworked first) and the residual +10 % per
evaluation; (5) the partition subplanner keeping its Phase-A subplans only if
(2)+(3) leave the band unmet and the solo probe says the search is what is
left. Measured: `sweep --board ipc5-metric-time --only rovers,tpp,pathways`,
then the loss side of `ipc2023-numeric` / `ipc2026-numeric`, `compare` on the
first-attempt estimator, evaluations/s before and after on rovers-metric-time
i10 solo.

### Lane 5 — the classical memory class (new)

**Claim:** the 82 mem-cap rows on 2014/2018/2023 seq-sat (and the same shape
on 2014 agile, 25, and 2023 numeric, 33) spend their remaining wall instead
of dying at the runner's 6 GB cap at 19–27 s. **Band:** +15 to +30 rows
across the three seq-sat boards on the first-attempt estimator, no row lost.
**Kill:** the rows end unsolved with the wall spent → recorded, and the note
says what the wall was spent on.

Day one measures where the memory goes (`probes-0.29/lane5/`): six
representatives solo under `FF_RES_DEBUG=1`, RSS after grounding and at the
trip — city-car i6, child-snack, maintenance; caldera i4, organic-synthesis;
folding. Two classes expected: the node store (20–27 s trips) and grounding
(folding). Mechanisms in order: (1) **arm the memory wall in the classical
rungs** — `search_from` already calls `MemWall::arm()`, but `arm()` is gated
on `BOUNDED_WORK` (a `ScopedDeadline` scope), so a plain classical solve runs
unarmed and the city-car rows die in weighted best-first at the runner's cap;
arm it whenever a budget is DECLARED, so a rung ends at 75 % with a "memory
wall" note and the ladder's next rung gets the rest of the wall with a fresh,
smaller table (EHC / novelty with a bounded closed set) instead of the
SIGKILL; fixture on the `tests/mem_wall.rs` pattern — a classical task under a
small `FF_MEM_BUDGET_GB` ends "memory wall" with a note, not a kill, and the
next rung runs. (2) **A leaner node store** — states once in an arena, the
closed set as 64-bit hashes with the collision list, parent/op as `u32`,
`Node` packed; bytes per node measured before and after on city-car i6.
(3) Grounding-bound rows (folding): record only, unless the probe shows the
packed-op tables are the peak, in which case they join Lane 4's achiever
work. Measured: `sweep --board ipc2014-sat --only city-car,child-snack,
maintenance`, `--board ipc2018-sat --only caldera,organic-synthesis,snake`,
`--board ipc2023-sat --only folding`, then the loss side of all three boards,
`max_rss` beside every row.

### Phase 0.5 — BUILT 2026-10-04 (crucible)

**The pass cap.** Not a subset defect: `--max-passes` was checked in the
runner's `wait`, which the loop consults only after a pass that banked
NOTHING -- a subset whose every pass banked a few owed rows never stalled,
so never saw the cap. It is `LoopConfig::max_passes` now, checked at the top
of every pass, with an `Event::Capped` before the `Stopped`; fixtures: a
queue banking one row a pass under `--max-passes 2` runs two passes and
stops owing ten, a queue draining under the cap is complete.

**The canary at width.** The canary pauses our planners for its two seconds,
but a chip that has been running nine of them is throttled either way, so at
width 9-10 on an idle box at 00:30 it read 1.50x its SOLO line and every
packed pass owed every solo row. Every reading now records its width (schema
v10, `canary.width`: how many of ours were attached when the read began; 0
at calibration), and the factor is read against the box's own p25 line at
that width once it holds `canary_baseline_n` readings -- seeded from the
database's last 100 at each width at sweep start -- and against the solo
line until then, which owes rows rather than banking under a line nothing has
measured (`sweep::canary_factor`, unit-pinned: 1.5 s at width 9 is 1.00x the
width-9 line and 1.50x without one; foreign load at width 9 still reads over
the line; the line never drops below solo). *Pre-registered risk:* a width's
first readings taken under foreign load set a lenient line; the p25 of five
and the database prior are the guards, and the "thermal" owings per pass are
what the next subset run reports.

### Lane 2 — the first finding was the route, not the heuristic (2026-10-04)

The consumption charge was built first (`heuristic::consumption_charge`,
hatch `FF_NO_CONSUME`, fixture `the_plan_pays_for_what_it_spends`: four
drives at 8 energy on a stock of 24 owe one recharge, h 4 -> 5; on a stock
of 8 they owe three, h 4 -> 7; a full tank is byte-identical). Then
rovers-metric-time i10 under `FF_WALL_DEBUG`:

    compression rung: a plan after 202 evals, 0.01 s
    left-shifted layout refused by the validator: `RECHARGE--STEP3` is not a durative action

The compression rung had the plan in 10 ms, in BOTH arms (193 evals blind).
The validator specialises the domain per plan step (`RECHARGE--STEP3`, a
parameterless copy) and replays through the snap compile -- and the snap
compile SKIPS every durative action whose END uses `?duration` when the
duration reads a fluent some action assigns ("end-side `?duration` over a
dynamic read: unsupported", never compiled wrong). Rovers' `recharge` lasts
`(/ (- 80 (energy ?x)) (recharge-rate ?x))` and pays `(* ?duration
(recharge-rate ?x))` at its end: the whole board recharges this way. So the
decision-epoch search could never recharge at all, and the one rung that
could was refused by name. **Fixed in `temporal::compile`:** PDDL2.1 fixes
`?duration` at the START, so a hidden fluent `(DUR-<action> ?params)`,
defined 0 at init for every binding, is ASSIGNED the duration expression by
the start snap and stands in for `?duration` on the end side and in the
invariant. Fixture `tests/dur_fluent.rs`: the action compiles, solves with
the durations fixed at each start (3.5 s and 2 s), validates; a drain between
a fill's start and end does not change what the fill pays, and a tampered
duration is refused as a duration, not a goal.

**Solo, 60 s, one thread (`probes-0.29/lane2b/probe.tsv`), both arms:**

| cell | 0.28.0 board | fixed build | `FF_NO_CONSUME=1` |
|---|---|---|---|
| rovers-metric-time i10, i12 | unsolved | **solved** (makespan 173, 105) | solved |
| rovers-metric-time i14, i16, i18 | unsolved | **solved** (186, 253, 177) | solved (182, 253, 212) |
| rovers-metric-time i20 | unsolved | unsolved: the compression rung finds no plan in 2,188 evals / 10 s, the temporal ladder stops at the wall | same |
| tpp-metric-time i10, i20 | unsolved | unsolved, "temporal ladder stopped at the wall" | same |
| pathways-metric-time i10, i20 | unsolved | unsolved, same note | same |

Five rovers cells of the eight probed are the compile fix's; the charge is
neutral on rows here (a better makespan on i18, a worse one on i14). Rovers
was 8/40 on the board; the board read is the crucible's. tpp and pathways
metric-time are NOT this defect: their note is the ladder's, and whether the
compression rung reaches a plan there is the next question for the lane.
The 0.28 residue note ("rovers best-h 28 -> 11 then flat") described the
compressed task's search on a plan the route then threw away.

**MEASURED 2026-10-04 22:15 (`lanes-1004/lane2`, engine 771d3808e167, the
metric-time rovers|tpp|pathways subset, 110 cells, two passes, `compare
--a 89cfdc5f06ed --b 771d3808e167`):**

| estimator | 0.28.0 | 0.29 | delta | 95 % CI |
|---|---:|---:|---:|---|
| first attempt (the claim) | 24 | 37 | **+13** | [+6, +20] |
| banked | 24 | 39 | +15 | [+8, +23] |
| equal-N | 24 | 37 | +13 | [+6, +20] |

Every estimator agrees in sign. 16 cells gained, 1 lost:
**rovers-metric-time i28**, which 0.28.0 banked solved and 0.29 did not in
three attempts -- owed a solo differential on both binaries (the 0.28.0
engine must be rebuilt from its tag first) before it is called anything.
The gains are the compile fix's (rovers); tpp and pathways stayed where they
were, as the solo probe said. The lane's band was +15 to +30 on the whole
200-cell board; this is the 110-cell subset it was named for, and reads at
the band's floor on `first`. The engine carried the consumption charge,
the cost-aware polish and the plan-sized scorer as well; none of those
touches this board.

### Lane 1 — one grounding per polish, BUILT 2026-10-04

The first build grounded twice per attempt (once to search, once to price)
and on pathways-simple/16 -- 32.7 s to ground the compiled task, 8 s the
plain one -- never started. Now: the route's hard-goal seed keeps its
grounded task (`pddl3::SeedTask`), and when it is a PLAIN grounding (no
monitor block, no END action -- the simple and qualitative boards) the
polish searches every candidate on it with the goal facts swapped in (the
packed tables sit behind `Arc`; a clone is the goal) and prices every
candidate on it through `verify::VerifyCtx::from_task` -- ZERO groundings;
otherwise `VerifyCtx::prepare` grounds once and the hard task is grounded
once (two), and a body the fast path cannot express (an `exists`) takes the
per-candidate grounding it always did. The polish's wall share is taken
whenever a second of wall is left, not `2 x ground_secs`. An `or` body is one
goal per disjunct, tried in order; the held `(not atom)` bodies forbid their
atoms' achievers in the candidate's search, minus what the candidate's own
relaxed plan needs, with an unforbidden search as the fall-back; an atom the
grounder dropped reads from the init when priced. Fixtures (tests/
pref_polish.rs, counted in a child process): the corridor polish grounds
twice without a seed task and NOTHING with one.

**Solo, pathways-simple/16, 60 s (`probes-0.29/lane1b-pathways16.err`):**
before, "[polish] not started: 0.00 s left, the task grounded in 32.71 s";
now the polish starts with 11-25 s left, reuses the seed's 1,705-op task,
and attempts **20 of 20** candidates. Every one prices WORSE (28.4-33.2
against the empty plan's 25.7): a product's plan must choose reagents, and
22 of the 42 preferences are `(not (chosen ...))`, held by the empty plan
and worth more together than the product (1.2-1.4). With all 22 forbidden
no candidate is reachable; with the relaxed plan's needs exempted (1,155 of
1,705 ops still forbidden) none is either -- the relaxed plan names fewer
reagents than a real plan must choose. **The mechanism's ceiling on
pathways is named: the candidate search is not cost-aware.** A plan that
chooses the FEWEST held-false atoms is what an improving step there needs
(action costs = the broken preferences' weights, through the ladder's
cost term), the next step for this lane. The boards' points are the
crucible's read; the first step's gains (trucks, storage, tpp) are
untouched by this one.

**MEASURED 2026-10-04 23:14 (`lanes-1004/lane1s`, engine 771d3808e167,
ipc5-simple-pref, 130 cells, two passes):** rows **130 / 130** both engines,
0 lost, first-attempt +3 [-1, +8]. Points (`lane1/quality.py` on the stage):
**94.1 → 95.8** (+1.7; the first round's polish read 96.0), 29 cells better /
9 worse, 2 solved unpriced (pathways/27, storage/19), 22 empty plans. The
polish gained on 27 cells (openstacks 18, trucks 6, tpp 2, storage 2). The
nine worse: seven storage cells (i5 25 → 47, i8 104 → 252, i9 525 → 577,
i11, i12, i14, i7), openstacks i4 (16 → 23) and **trucks i13, whose 0.28.0
optimum of 0 became 5**. The shape: the optimizer now hands the polish 40 %
of its remaining wall whenever a second is left, and on storage the
preference bodies are `exists` forms the fast path cannot express, so every
candidate took the per-candidate grounding, gained nothing, and the
optimizer had lost the wall that found its better plans. **The fix, written
tonight, compiled when the chain ends:** the share is taken only when the
seed task is plain AND some violated-able body is expressible by the fast
path; otherwise the optimizer keeps its wall and the polish runs on what
is left, as 0.28 did. Re-read on `--only storage|trucks|openstacks`.

**MEASURED 2026-10-05 00:21 (`lanes-1004/lane1q`, engine 771d3808e167,
ipc5-qual-pref, 100 cells, one pass):** rows **100 / 100** both engines, 0
lost, first-attempt +2 [+0, +5]. Points **59.2 → 65.2** (+6.0; the first
round read 63.3), 25 better / 6 worse, **0 solved unpriced** (0.28.0 had 5),
13 empty plans; the polish gained on 28 cells (rovers 12, trucks 11, storage
2). The six worse are all rovers (i1 68 → 98, i2 33 → 61, i3 29 → 82, i4
26 → 55, i7 38 → 46, i18 6256 → 7203): rovers' metric carries a numeric
term beside the violations, and the optimizer that chased it lost the wall
the polish share took -- the simple board's storage shape again, so the
same share gate (plain seed task AND an expressible body) is its fix, to be
re-read on `--only rovers` with it. Against SGPlan5's 92.8 the gap is now
27.6 of a band that asked for 75.

**The share gate, MEASURED 2026-10-05 02:51 (`lanes-1004/gate1s`, engine
d4f103fe60a4, simple-pref `storage|trucks|openstacks`, 60 cells):** the gate
changed NOTHING on these cells (0 worse, 5 better against the night's
engine); against 0.28.0 the same seven are worse (storage i7, i8, i9, i11,
i12, i14; trucks i13 0 → 4), and worse than that: **storage i18 is LOST**
(`mem-cap` at 45.9 s; 0.28.0 solved it at 4149) and storage i19/i20 come
back unpriced ("memory budget reached before grounding began"). The seed
task, held alive for the polish through the compiled task's 5-6 GB
grounding, is the memory; the up-front 40 % share, not the gate, is the
points. So the third shape, written at 03:00 and compiled when the
qualitative gate subset ends: **the polish runs FIRST** -- on incumbent
zero, on the seed's own plain task, under `polish_frac` of the wall, zero
groundings -- its plan is the optimizer's seed, the seed task is dropped
before the compiled grounding, and the optimizer keeps its whole wall. The
Lane 1 kill clause fires on storage i18 for the engines 771d3808e167 and
d4f103fe60a4; neither ships. The qualitative gate subset (`gate1q`, rovers|
trucks, 40 cells, 03:31) said the same: 40 / 40 rows, 23 better / 5 worse
against 0.28.0, the five the same rovers cells (i1, i2, i3, i4, i18) --
the gate changed nothing there either.

**The polish FIRST, MEASURED 2026-10-05 04:06 (`first1s`, engine
a3a38f0c694e, the same 60 simple cells):** rows **60 / 60 -- storage i18
is back** (the seed task dropped before the grounding), 26 better / 6
worse, points +0.3 on the subset (the share version read +1.6). The slice
is the cost: 40 % of the wall before the compiled grounding left storage
i16-i20 UNPRICED (the 5-6 GB grounding no longer fit) and the optimizer
short on trucks (i11 0 → 2, i13 0 → 6, storage i7/i11/i12/i14 a few points
each); the polish-first fired on 38 of 60 cells, the post-optimizer polish
on one. Next (written 04:40): the first polish takes half the share (0.2,
`FF_PREF_POLISH_FIRST_FRAC`) and is skipped where the seed task itself
took over a tenth of the wall to ground -- re-read on the same 60.
On the qualitative subset (`first1q`, rovers|trucks, 40 cells, 04:57) the
polish FIRST is the right shape outright: **40 / 40, points 19.4 → 26.2**
on the subset (the share version read 22.5), 25 better / 1 worse (rovers
i15, 3294 → 3398), 0 unpriced -- the five rovers regressions are gone.

**Half the share, MEASURED 2026-10-05 05:55 (`half1s`, engine 3279a9e045ed,
the same 60 simple cells):** 60 / 60 rows, 25 better / 5 worse, points +0.6
on the subset; storage i17-i20 still UNPRICED (four, from five) and trucks
i11/i13 still 2 and 6 against optima of 0 -- the 12 s the first polish
takes before the compiled grounding is what storage's 5-6 GB grounding and
trucks' optimizer were short of, and the seed-grounding skip (a tenth of the
wall) does not see storage coming because its seed task grounds fast. The
polish first fired on 35 of 60. Where the three shapes stand on the simple
subset against 0.28.0: share-after +1.6 but storage i18 LOST; polish-first
+0.3; half share +0.6, no row lost. On qualitative the polish first is +6.8
with nothing lost. A skip rule that sees the compiled task's size coming
is the next (and last) turn of this knob before the boards decide.

### Lane 1, mechanism 3 — the scorer at plan size, BUILT 2026-10-04 (evening)

`temporal::score_soft` grounded the whole snap-compiled task to price one
plan: 4.4 GB on pipesworld-complex, 12.7 s on pathways-complex i20, and the
four complex cells the first round banked "NOT scored: the wall or the
memory budget left no room to build the scorer". `SoftScorer::score` now
specialises the domain to the plan's own steps (`tcompress::specialise`,
the validator's construction, shared), compiles and grounds THAT -- `steps`
ops -- and replays on it; an atom the small grounding never reaches reads
from the init (`eval_formula_init`). The last grounding is cached by the
plan's distinct steps, so the chase's second score of the same plan is a
replay, as before. `FF_SCORE_FULL=1` keeps the full grounding. Fixture
`tests/score_plan_size.rs`: on a tank domain with goal, numeric, static,
trajectory (`sometime`, `at-most-once`) and `total-time` terms, the solver's
plan and three hand-built ones price IDENTICALLY on both groundings
(violated set, satisfied count, metric), and the numbers are asserted
outright. The complex board is tonight's read.

**MEASURED 2026-10-05 01:29 (`lanes-1004/lane4`, engine 771d3808e167, the
whole ipc5-complex-pref board, 108 cells, three passes):** rows banked
**80 → 81** (+1 [+0, +3]: pathways i27 gained, none lost); the first-attempt
estimator reads −2 [−7, +3], the estimators disagree in sign, so no delta is
claimed and the banked +1 is the standing read. Points **53.4 → 53.7**;
solved unpriced **3 → 2** -- pipesworld i18 and storage i17 are PRICED now
(the scorer at plan size; the first round's four unpriced pipesworld cells
are none), pathways i27 and storage i18 remain; `mem-cap` 17 → 14 (the
achiever index's doing, on storage); trucks i1 worse, 0 → 4, the share
shape again. The lane-4 band (storage-complex +8 to +14) is NOT met: the
index took three mem-caps off the class and converted no row.

### Lane 4 — the achiever index, BUILT 2026-10-04

`add_by_fact` carried one entry per op per shared-monitor add (ops x
monitors). It now holds each op's OWN adds; `shared_add[f]` marks the facts
the monitor block adds and `monitored_ops` names the ops, and
`PackedTask::achievers(f)` merges the two ascending, each op once (the old
buckets held an op twice when its own add and its own conditional add both
named the fact). All ten readers walk `achievers`. Fixture
`tests/achiever_index.rs`: on a monitored task the walk equals the op scan
for every fact, the table is exactly the own adds, and the shared facts
route to every monitored op; without a block the walk is the table's row.
Measured where the class lives: storage-complex and pipesworld-metric-time
through the crucible (`max_rss` beside every row) -- not yet run.

### Lane 5 — the classical memory class, MEASURED 2026-10-04

Six representatives solo, one thread, peak RSS by `/usr/bin/time -l`, at a
node cap of 1 (the grounding's footprint) and 200,000 evaluations
(`probes-0.29/lane5/measure.tsv`; the 120 s alarm cut three):

| cell | grounded | at 200k evaluations | class |
|---|---|---|---|
| city-car i6 | 31 MB | 2.9 GB (alarm at 120 s) | the search |
| child-snack i10 | 21 MB | 4.9 GB (alarm) | the search |
| maintenance i4 | 8 MB | 1.2 GB in 1.2 s | the search: ~6 KB per evaluated node |
| caldera i4 | 5.9 GB in 44 s | 5.8 GB | the grounding |
| organic-synthesis i8 | 3.6 GB and still grounding at 120 s | same | the grounding |
| folding i3 | the grounder exits at once: "wall budget exhausted during binding enumeration" with the whole wall unspent | -- | a grounder defect, named below |

Two classes, as expected, and the search class's number is the finding:
6-15 KB per EVALUATED node against the per-node byte model's ~200 B. The
best-first search stores every GENERATED successor's state in its arena
at generation (deferred evaluation: a pushed `Node` holds the full
`State`), so the arena is the frontier times the branching factor, not the
closed set -- which is why city-car dies at 6 GB in 20 s. The mechanism
this names is a LAZY frontier: an open node holds (father, op, key) and
regenerates its state on pop, the exact duplicate check kept against the
closed states and a hash against the open ones -- a branching-factor cut
in bytes per node, measured on city-car i6 before any row is claimed. Not
built this cycle; the measurement is the record. The grounding class
(caldera, organic-synthesis) is the grounder's binding enumeration, Lane
4's neighbour, not a node store. **Folding is a third thing:** the
grounder returns "wall budget exhausted" in 0.0 s with FF_TIME_LIMIT=20,
so the 16 folding `mem-cap` rows at 7 s on the board are a grounder that
gives up, not one that runs out; the next question is why `GroundWall`
trips on it at once.

### The trucks / propositional probe — RUN 2026-10-05 01:29-02:02 (record only)

Solo, one thread, three reps at 60 s and one at 300 s, engine 771d3808e167,
on the night's idle box (`probes-0.29/trucks/probe.tsv`):

| cell | 60 s (3 reps) | 300 s | reading |
|---|---|---|---|
| trucks-propositional i10 | unsolved at 1.95-2.18 M evaluations | **solved**, 2.92 M | a near-miss: 60 s buys ~2.1 M of the 2.9 M it needs -- speed, 1.4x |
| trucks-propositional i15 | unsolved, 0.54 M | unsolved, 3.79 M | a lost search |
| trucks-propositional i16 | unsolved, 0.50 M | unsolved, 3.15 M | a lost search |
| storage-propositional i27 | unsolved, 0.24-0.26 M | unsolved, 2.33 M | a lost search |
| storage-propositional i28 | unsolved, 0.27-0.28 M | **solved**, 0.99 M | speed, 3.7x: the ladder reaches 0.27 M in a minute here against 2.1 M on trucks i10 -- storage's evaluations are eight times dearer |
| pathways-propositional i17, i18 | not run: the script asked for one shared domain file and pathways ships one per instance | | a probe defect, re-run owed |

So the propositional pool is two things. Two of five probed cells are speed
(1.4x and 3.7x), which a faster evaluation or a wider box buys and which
SGPlan5's 1,800 s bought outright; three are searches that do not converge
in five minutes, which no budget buys -- the heuristic question Lane 3's
constraints cells and the compressed tpp search also pose, and the next
cycle's. The per-evaluation cost difference (storage 4.5k/s against trucks
35k/s) names the first thing to profile.

`probes-0.29/trucks/` (as scoped): trucks-propositional i10/i15/i16, pathways-prop
i17–19, storage-prop i27–30, solo at 60 s and at 300 s, three reps, `sample`
on one. The question is whether these are wall-edge speed
(field-gaps-0.26 §1e: 57–59 s timeouts) or lost searches. Trucks is weak on
every track it appears on (prop −15, time −19, preference quality 5.6 of
18.5), so a mechanism found here pays on several boards; the record names it
for 0.30 unless it is a one-line fix.

## Instrument notes from lanes29 (for Phase 0's ledger)

- `sweep --max-passes 3` does not hold: every launch ran a fourth pass,
  and phase C ran a FIFTH before it was stopped by hand -- the cap is not
  an off-by-one, it is not applied to a subset's passes at all. With the
  canary loop below, an uncapped run re-owes the same rows for ever. Fix
  with a fixture that counts passes on a subset.
- A row solved by 0.28.0 AT the wall (spider-sat i17 at 60.0 s, labyrinth
  i9 at 58.5 s on its second attempt) is a coin the promoted raw records
  as heads; the loss side of any later engine flips it. The estimator's
  first-attempt view is the honest one for those cells.
- Both daytime phases spent most of their passes owing rows to the
  operator's load (game detection, memory pressure, thermal canary) --
  the instrument yielded as designed, and the rows it did bank are clean.
  Gain-side rows (each a solo minute) want the night.
- **The thermal canary reads 1.50x its baseline at width 9-10 with no
  foreign load at all** (quiet hours, 00:30, idle 2.5 h): the canary job
  runs beside nine others and is measured against a solo baseline, so a
  packed pass owes every solo row it touches ("thermal 94" on
  ipc2023-numeric's pass 2 at night), and the owed rows are re-run packed
  again under the same conditions. A loop, not a slow box. Phase 0's
  ledger: the canary's baseline must be taken at the width it runs at,
  or the owed re-runs must run solo; until then, read owed rows through
  the estimator's per-run view, not the banked one.

## Instrument notes from the 2026-10-04 build (for Phase 0's ledger)

- `tests/ehc_extend.rs::a_run_that_keeps_arriving_keeps_its_slice` is a
  TIMING fixture (blocks(50) under a 1 s wall through `ferroplan::solve`,
  EHC needs ~0.34 s of the 0.6 s slice ceiling) and it read RED 1-3 runs in
  3 on this afternoon's box with `mediaanalysisd` at 214 %, then
  `hybridsearchd` and `ANECompilerService` at 90-100 % beside it (load
  averages 7-12). The engine is not the cause: the real `ff` under
  `FF_TIME_LIMIT=1` solves the cell by EHC 3 of 3, unwalled it runs the
  identical 21,767 evaluations on this build and on 0.28.0, 0.375 s against
  0.358 s wall. The suite otherwise reads 394 passed. The fixture wants a
  quiet box, as every timing fixture here does; it is not re-calibrated on a
  loaded one.
- Under Timberborn at 185-200 % CPU (the afternoon's box), pathways-simple
  i16 through `ff --json` at `FF_TIME_LIMIT=60`: the compiled task's
  grounding checkpoint expired mid-enumeration and the process still
  returned at **81 s** -- 21 s past the wall -- with the empty plan,
  unpriced. The report reserve is sized for a quiet box; a grounding that
  trips its checkpoint on a loaded one spends the teardown past the wall.
  The runner kills at 60 s, so the row would read `mem-cap`/timeout, not
  this; an instrument note for the memory class, with the receipt in
  `probes-0.29/lane1b-pathways16.out`.
- **The overnight chain of 2026-10-04 (`probes-0.29/lanes-1004/`)** launched
  three times. 16:56: suspended at once by the throttle (Timberborn at
  185 %); its canary rows for engine 7caf433e532d, calibrated under the
  game at 2.58 s, deleted. 18:03: the waiter launched it beside a stray
  folding grounding of mine (foreign 301 %), calibrating at 1.775 s; rows
  kept (a handful at width 1), its canary rows deleted, relaunched 18:17
  on engine 771d3808e167 at **1.270 s** with Mail and Spotlight indexing at
  315 % foreign -- still not a solo-quiet line, so the canary is LENIENT
  tonight and `rho` is the contention referee. The per-width lines
  (Phase 0.5) fill from this run's readings. Read tonight's owed/thermal
  counts with that in mind; a re-run of the owed rows on a cold morning
  box is the clean read. **The idle line, measured at 02:03 when the
  gate re-read calibrated on the empty box: 0.520 s** -- the night's
  1.27 s was 2.4x lenient, the 18:03 launch's 1.775 s 3.4x, the game's
  2.58 s 5x. A calibration taken beside ANY foreign load is a lenient
  night; the crucible should refuse to calibrate while the throttle reads
  foreign load, and wait -- owed to Phase 0's ledger.
- Lane 5's probe wrote `evaluated = None` for every row: `ff --json` carries
  the count under `statistics.evaluated_states`, not a top-level key. The
  peaks and seconds are what the record uses.

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
