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

---

## Instrument notes from lanes29 (for Phase 0's ledger)

- `sweep --max-passes 3` ran a FOURTH pass on every launch (an off-by-one
  in the pass cap); the extra pass re-runs owed rows on a box that just
  owed them, so it is wall spent, not rows lost. Fix with a fixture.
- A row solved by 0.28.0 AT the wall (spider-sat i17 at 60.0 s, labyrinth
  i9 at 58.5 s on its second attempt) is a coin the promoted raw records
  as heads; the loss side of any later engine flips it. The estimator's
  first-attempt view is the honest one for those cells.
- Both daytime phases spent most of their passes owing rows to the
  operator's load (game detection, memory pressure, thermal canary) --
  the instrument yielded as designed, and the rows it did bank are clean.
  Gain-side rows (each a solo minute) want the night.

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
