# Changelog

All notable changes to this project are documented here.

## [Unreleased]

### Added

- **A parallel rung portfolio, opt-in** (`FF_PARALLEL_RUNGS=1` at
  `--threads N ≥ 2`; 0.31 Lane A′, `parallel.rs`): the ladder runs as one
  worker exactly as it would alone, and N−1 of its rungs run beside it with
  the whole wall each — driver, LAMA, best-first, EHC, novelty-light, then
  Lane B's two, the order of their marginal cover on the Phase 0.5 matrix
  (the ladder 662 of 936 cells; with the driver beside it 683, plus LAMA
  694, plus best-first 701, the oracle 706). The first plan wins and the
  others are cancelled through the per-call budget's flag, which every
  rung polls at its batch boundary (novelty-light now too). Timing-
  dependent by construction — the one contract it breaks, "byte-identical
  for any worker count", is named in the module header — so it is opt-in
  and unmeasured until the mco subset sweep; the notes say it ran and which
  member won. The matrix hatch's solo rungs are one `search::Rung` enum
  now, shared by the hatch and the workers.

## [0.29.0] - 2026-10-15 — Points, and a relaxation that counts

0.28.0 closed the ROW gap to SGPlan5 on the IPC-5 preference boards and left
the POINT gap where it was, so this cycle declared two measures before any
lane ran: rows, by the first-attempt estimator against 0.28.0, and points, by
the IPC quality score on the preference boards. Four engine lanes and a
harness phase came out of that; two lanes were recorded negative. A
2026-09-30 read found that the first thing wrong with coins-2026 i18 was not
the heuristic at all but a code-generation regression in 0.28.0 itself.

### Measured

**66% coverage across 32 IPC boards** (5,555/8,444), **691 certified optima**
-- 8,444 of 8,444 instances banked in seven passes, then 24 rows re-opened
and re-measured by the re-check below. Full record:
[`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md),
[`docs/roadmap-0.29.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.29.md).

**Rows, against 0.28.0, on the estimator declared before the sweep** -- each
instance's FIRST attempt against its first:

| estimator | 0.28.0 | 0.29.0 | delta | 95 % CI |
|---|---:|---:|---:|---|
| **first attempt** (the claim) | 5,358 | 5,476 | **+118** | [+89, +149] |
| equal-N | 5,397 | 5,522 | +125 | [+95, +155] |
| banked -- what the boards show | 5,466 | 5,555 | +89 | [+67, +112] |

Every estimator agrees in sign. Banked reads lower than first-attempt because
the referee re-runs failures and 0.28.0's re-run rescued more of its failures
than 0.29.0's did (net -28). Of the cells both engines banked, 110 were gained
and 21 lost.

**Where it moved (rows):** 2023 numeric +27 (289/400, now ahead of the vendored
field's best, ENHSP at 267), metric-time +14 (119/200), 2014 seq-sat +7, the
2014 multi-core boards +6 / +6 / +7, IPC-2011 seq-sat +5 and tempo-sat +3, 2014
tempo-sat +2, 2014 agile +2, 2018 seq-sat +2, propositional +2. Down: 2023
seq-sat -1, 2026 numeric-opt (full) -1. 

**Points, the second declared measure** -- the IPC-5 quality score (best
metric / ours, over the cells SGPlan5 solved), SGPlan5's own score beside:

| track | rows | 0.28.0 | 0.29.0 | SGPlan5 |
|---|---|---|---|---|
| simple-preferences | 130/130 | 94.1 / 120.4 | **97.3 / 115.1** | 129/130 |
| qualitative-preferences | 100/100 | 59.2 / 92.1 | **64.6 / 91.4** | 100/100 |
| complex-preferences | 81/108 | 53.4 / 99.0 | 54.5 / 100.0 | 105/108 |

The headline lane moved points in the right direction on all three boards and
**landed under its pre-registered band** (simple 96 -> 105 of 120, qualitative
63 -> 75 of 92): ferroplan is closer to SGPlan5 on quality, not level with it.
Read the denominators: each cell's score is best-known over own, so a better
plan from either planner moves the other's total.

**The 21 cells 0.28.0 solved that 0.29.0 banked unsolved**, each re-run on
both engines before being named (0.29.0 with up to eight attempts; 0.28.0
rebuilt from tag `v0.28.0` under the crucible's own prefix, engine
`bb07d50d2015`, which is not byte-identical to the baseline's `89cfdc5f06ed`).
**Twelve reproduce**: the rebuilt 0.28.0 solves them and 0.29.0 does not in
any attempt -- `rovers-metric-time/28`, `pipesworld-metric-time/36`,
`settlers-snp-numeric-2026/5`, `ztalloc-sum-numeric-2026/9` and `/12`,
`sugar-numeric-satisficing/17`, `driver-log-temporal-satisficing/6`, and five
IPC-2011 multi-core cells (`elevator` 19 and 20 and `transport` 18 at two
threads, `elevator` 19 and `transport` 18 at four).
**Eight are noise**: the rebuilt 0.28.0 cannot re-solve them either
(`driver-log-temporal-satisficing/5`, `tidybot` 19, `elevator` 20 at four
threads, `labyrinth-satisficing/9`, `forestfire-numeric-2026/14`,
`gear-car-numeric-2026/8`, and `pipesworld-propositional/50` and
`counters-numeric-satisficing/16`, which 0.29.0 itself solved on a first
attempt that the referee voided as thermal / swap and could not reproduce).
**One is a wall coin**: `transport` 18 at eight threads is solved by the
rebuilt 0.28.0 twice in two and by 0.29.0 on a first attempt the referee
voided, then missed three times.

**What the failure classes did.** Coverage is the claim; the unsolved rows'
reasons moved with it. `mem-cap` fell on 2014 seq-sat (30 -> 4), 2018 seq-sat
(35 -> 24), 2014 tempo-sat (6 -> 2) and 2014 agile (25 -> 1), and rose where
the memory wall now ends a rung early: 2018 seq-opt (10 -> 21), 2023 numeric
(33 -> 52), 2023 numeric-opt, 2026 numeric-opt full (4 -> 24), 2014 multi-core
t8 (4 -> 11). On the optimal boards the same rows end unsolved either way.

**Not met.** Metric-time +14 against a +15 to +30 band, short by one. The
achiever index (Lane 4) did not move storage-complex (complex-preferences +1
row, band +8 to +14).
The classical memory lane (Lane 5) was measured, not built: a lazy frontier is
named (6-15 KB per evaluated node against a 200 B model) and left for 0.30.
Propositional stands at 386/450 against SGPlan5's 218/220.

### Added

- **`benchmarks/recheck29.py`** and **`benchmarks/promote-air29.sh`**: the
  cut's re-check (every banked solve whose price is missing, every solve under
  a slow canary near its wall, every cell the previous release solved) and
  promotion, for the next cut to copy.

### Changed

- **Temporal: an end-side `?duration` over a dynamic read compiles** (0.29
  Lane 2). A durative action whose duration reads a fluent some action assigns
  and whose END uses `?duration` -- rovers' `recharge`, lasting
  `(/ (- 80 (energy ?x)) (recharge-rate ?x))` and paying
  `(* ?duration (recharge-rate ?x))` at its end -- was SKIPPED by the snap
  compile ("never compiled wrong"), so the temporal search could never
  recharge, and the compression rung's plans, which do, failed the validator
  by the name of an action it never had. PDDL2.1 fixes `?duration` at the
  START, so the compile does the same: a hidden per-action fluent is assigned
  the duration expression by the start snap and stands in for `?duration` on
  the end side. Fixture: `tests/dur_fluent.rs`.
- **The relaxed plan pays for what it spends** (0.29 Lane 2). A `decrease` is
  a delete on a number and the relaxation dropped it: a plan of five drives at
  8 energy each read `energy >= 8` against a stock of 10 and was satisfied,
  so every state with the same drives left had the same h whatever its
  energy. Extraction now sums a selected plan's spend of each fluent its own
  preconditions read from below, against the stock in hand plus what the plan
  puts back, and prices a deficit through the best raiser. `FF_NO_CONSUME=1`
  restores the blind h; byte-identical where a plan spends nothing it reads.
- **One grounding per polish** (0.29 Lane 1). The preference polish grounded
  the task twice per attempt -- once to search a candidate, once to price it
  -- which on pathways (8 s a grounding) left it nothing to attempt. The
  hard-goal task is grounded once and each candidate swaps its goal facts in;
  the verifier is a prepared context (`verify::VerifyCtx`) that grounds once
  and replays many; with the route's hard-goal seed task in hand -- a plain
  grounding of the pair -- the polish grounds nothing at all. An `or` body is
  one goal per disjunct; held `(not ...)` bodies forbid their achievers in a
  candidate's search; an atom the grounder dropped reads from the init
  instead of as false. Pinned by fixtures counting the groundings (two, and
  zero with the seed task). The polish's candidate search prices the held
  preferences a plan would break as per-op penalties and runs best-first
  under them, so a candidate breaks the fewest it can. And the temporal
  preference scorer prices a plan on a grounding of the plan's own steps
  (`tcompress::specialise`) instead of the whole task -- 4.4 GB on
  pipesworld-complex before; `FF_SCORE_FULL=1` keeps the old grounding --
  pinned to it by a fixture scoring four plans both ways.
- **The achiever index is no longer ops x monitors** (0.29 Lane 4). The
  shared monitor block's adds belong to every monitored op, and the index
  carried one entry per op per monitor add -- hundreds of millions of `u32`
  on storage-complex, its `mem-cap` class. `add_by_fact` holds each op's own
  adds; `PackedTask::achievers` merges in every monitored op for a fact the
  block adds, ascending, each once.
- **The preference polish runs first, on the seed's own task** (0.29 Lane 1).
  It takes half the optimizer's wall share, clears its memory trip before the
  compiled grounding, is skipped where the seed's grounding peaked past a
  quarter of `FF_MEM_BUDGET_GB` or grounded slowly, and needs most preference
  bodies expressible. Its candidate search prices the preferences a plan would
  break as per-op penalties (`search::with_op_penalty`) instead of forbidding
  their achievers. `FF_PREF_POLISH_FRAC` (0.4) sizes the share.
- **Temporal node-cap refill** (0.29 Lane 3, `temporal::refill_ladder`) and
  **EHC slice extension** (Lane W, `search::ehc`): built, fixtures green,
  recorded negative on rows (constraints 28 -> 28 of 120; sat subsets 0 gained,
  0 lost) and kept for short walls. `FF_NO_TNODE_REFILL`, `FF_NO_EHC_EXTEND`,
  `FF_EHC_WALL_MAX_FRAC` (0.6).
- **Streamed expansion of the constraints simplifier** (0.29 Lane 4,
  `constraints::expand_simplified`): a member the static simplifier accepts is
  never stored. Byte-identical; storage still peaks at 5-6 GB. `FF_PREF_NO_STATIC`.

### Fixed

- **A 0.28.0 code-generation regression.** `heuristic::widen`, which Lane N's
  `needs` parameter had tipped LLVM into keeping an out-of-line call, cost the
  relaxed-graph build 11.0 us per evaluation against v0.27.1's 7.7 (+43 %) on
  the identical search trajectory. `#[inline(always)]` brings it to 8.5 us;
  coins-2026 i18 solo at 60 s went from 0/3 to 3/3. The heuristic's nine
  `FF_*` hatches are also read once per `Scratch` instead of up to six
  `std::env::var` reads per evaluation.

### Harness (the crucible; not part of any published crate)

- **A solve can be SUSPECT** (0.29 Phase 0.1). The referee re-ran a row that
  failed under bad conditions and never one that solved under them; a solve
  that came back without its price, or spent half its budget or more, is now
  judged by the signals about its own process and the box's clock (demoted,
  canary, swap growth, clock jump). `[referee] solved_wall_frac` (1.0 restores
  "a solve is a solve"). Pinned by the population the rule came from.
- **`publish.sh` asks how old the crates.io token is** before spending the
  pre-flight, and refuses a token file older than 80 days.
- **crucible** (0.29 Phase 0.5): `--max-passes` is a cap on passes,
  productive or not (it was consulted only after a pass that banked nothing,
  so a subset banking a few owed rows a pass ran on past it); the canary's
  factor is read against the box's own line at the width it ran at (schema
  v10 records the width), since a chip that has been running nine planners is
  throttled whether they are paused for the read or not, and against the solo
  line until that width has readings.

- **The README's standings block is a capability table.** The front page's
  generated block now says what the planner is good at and against whom --
  one row per capability (temporal, PDDL3 preferences, classical satisficing,
  optimal with proofs, numeric), each board's coverage beside its rank in that
  competition's field and the leader's count, and the preference boards' IPC
  quality beside their rows -- in place of the best-five-by-coverage list,
  every row of which read ~100 %. `benchmarks/manifest.toml` carries each
  board's `capability` / `front_name` (generated from `standings.py`, as
  `proof_track` is); `crucible standings --check` now covers the README block
  as well as the two tables; the IPC-5 simple-preferences field is
  archive-counted into `field-results.json`. The notes that still called the
  timed constraint operators "rejected by name" (constraints and
  complex-preferences rows; README Limitations; the book's standings intro)
  are corrected: `within` / `always-within` have been enforced on
  durative-action domains since 0.24, and only `hold-during` / `hold-after`
  are rejected.

## [0.28.0] - 2026-09-28 — Feasible first, better second

A 2026-09-20 read of SGPlan5's own IPC-5 solution headers found its MEDIAN
solve is **0.54 s** -- 89 % of its 860 solves finish inside 60 s, and of the
335 rows it solves and ferroplan did not, 143 took it under a second. The gap
on those boards was never the wall. On most of those rows a valid plan was in
hand, or milliseconds away, and the route had no way to return it. Five engine
lanes close that, and the harness learned to measure a question smaller than
a release.

### Measured

**65% coverage across 32 IPC boards** (5,466/8,444), **689 certified optima**
-- 8,444 of 8,444 instances banked, the second sweep to reach a terminal
state. Full record: [`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md),
[`docs/roadmap-0.28.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.28.md).

**Against 0.27.0, on the estimator declared before the sweep ran** -- every
instance's FIRST attempt against its first attempt, because that is the one
measurement both engines were given equally (0.27's own +115 was withdrawn as
a delta for want of exactly this):

| estimator | 0.27.0 | 0.28.0 | delta | 95 % CI |
|---|---:|---:|---:|---|
| **first attempt** (the claim) | 4,827 | 5,358 | **+531** | [+481, +579] |
| equal-N | 4,856 | 5,393 | +537 | [+487, +585] |
| banked -- what the boards show | 5,122 | 5,466 | +344 | [+307, +381] |

Every estimator agrees in sign, and this time the boards UNDERSTATE the
engine: the referee re-runs failures, 0.27.0 failed first time on 3,617 cells
and was re-run on 69 % of them (295 rescued), 0.28.0 on 3,086 and 32 % (110).

**Where it moved:** tempo-sat +10.8 pts (509/630, +68 rows),
complex-preferences +47.2 (80/108), qualitative-preferences +49.0 -- **100/100,
level with SGPlan5, the track winner** -- time +29.2 (126/130), 2014 tempo-sat
+22.0, metric-time +20.5, simple-preferences +8.5 (130/130), 2023 seq-sat
+5.7, 2023 agile-300s +4.3. Down: 2026 numeric-opt −1.7 (22/60), 2026 numeric
−0.6, 2023 numeric −0.5. On the six IPC-5 boards the lanes were aimed at,
379 -> 569 of 788; on the variants SGPlan5 entered, 316 -> 491 of 678 against
its 612 -- the gap 296 -> 121, most of it metric-time (66) and constraints (27).

**Seven cells 0.27.0 solved that 0.28.0 does not**, every one re-run on both
engines on a quiet box before it was called anything. Five are real:
`tetris-sequential-satisficing/15`, `sugar-numeric-satisficing/5`,
`coins-numeric-2026/18`, `line-exchange-snp-numeric-2026/3_5_90_100`, and
`sailing-wind-opt-numeric-2026/10` under the optimal mode (its satisficing
twin solved). Two are the box: v0.27.1 fails `hiking-sequential-agile/18`
and `rover-numeric-satisficing/15` today as well. Seventeen more read as
losses on the first pass and were not.

**Read the preference gains with what they are made of.** 76 of the six
boards' 569 solves are the EMPTY plan -- problems with no hard goal, every
goal a preference, where doing nothing is valid, VAL-accepted, and the
boards' standing convention (0.27.0's 379 carried 27) -- and five come back
without a metric. Coverage is not what IPC-5 ranked these tracks on. By its
quality score (best metric / ours, over the cells SGPlan5 solved): simple
94.8 -> 94.1 against SGPlan5's 120.4, qualitative 45.8 -> 59.2 against 92.1,
complex 20.1 -> 53.4 against 99.0. **The lanes closed rows and left points
where they were.**

**And a hole in the instrument, found in this read and left open.** The
referee re-runs a row that FAILED under bad conditions and never one that
SOLVED under them. Ten pathways-preferences-simple cells solved during a
52-hour stretch when a stray iOS Simulator held the box 2.4-6.7x slow -- the
empty plan, unpriced at the wall, on cells that price in 40 s alone -- and
were banked; simple-preferences quality read 82.0 until 147 such rows were
re-opened and re-measured (`benchmarks/recheck28.py`). A solve banked under a
slow canary near its wall should be SUSPECT like a failure is. Owed to the
crucible.

Regression reads before the cut, equal-N: over the 519 temporal cells the
published boards solved, **519 of 519**, summed solve time 0.91x, makespan
better on 181 cells and worse on 6; over the 58 cells that solve at a
resident size of 3.5 GB or more -- the only ones the new memory wall can
touch -- 58 of 58, 49 identical, one metric worse by 2, and eight that keep
their row and give up their metric.

### Added

- **The compression rung** (`tcompress`): a temporal task that needs no
  concurrency is planned as a classical one -- each durative action as one
  instantaneous action -- and a left-shift over the ops' read/write sets puts
  the plan back on the clock. The plan is validated against the ORIGINAL
  task before it is returned. It BANKS: the decision-epoch ladder then runs
  as a bounded quality chase and the smaller makespan wins, so a task that
  solved before returns the plan it returned before. Declines required
  concurrency, timed initial literals and trajectory constraints, and stands
  aside when `FF_TCONC=1` asks for the actor scheduler.
  `FF_NO_TCOMPRESS=1` restores 0.27; `FF_TCOMPRESS_WALL_FRAC` /
  `FF_TCOMPRESS_CHASE_FRAC` size the bet and the chase.
- **Incumbent zero** for PDDL3 preference optimization
  (`pddl3::metric_optimize_seeded`, `hard_goal_seed`, `close_seed`): the
  optimizer is handed a plan for the hard goals before it starts, as a FLOOR
  -- its own search is unchanged, and the seed is what comes back (with a
  note) only when it found nothing cheaper. Previously a run that ended
  before the optimizer's first plan reported `solved: false`.
  `FF_PREF_NO_SEED=1` restores 0.27; `FF_PREF_SEED_BOUND=1` also opens the
  branch-and-bound with the seed.
- `temporal::solve_scored` / `ScoredPlan` / `SoftScorer`: the preference
  score rides the solve instead of being computed after it.
- **Condition preferences on durative actions**: `(preference name (at start
  phi))` inside a `:durative-action`'s `:condition` now parses (every row of
  IPC-5 `tpp-preferences-complex` used to die at the parser). The search
  drops them, as PDDL3 allows, and the scorer counts one violated instance
  per APPLICATION -- at start before the start happening, at end before the
  end, over all across every state inside the interval.
- The anchored successor generator (0.27's classical lever) is wired into the
  temporal rung. Byte-identical by construction; measured, the scan it
  removes was 0.01-7.7 % of a temporal evaluation, so no coverage is claimed
  for it. `FF_NO_TSUCC=1` keeps the full scan for measurement.
- `FF_HTRACE=1`: the classical best-first `best_h` trace (evaluations and
  seconds at each improvement) on stderr. `FF_MEM_TRIP_FRAC`,
  `FF_GROUND_PHASES=1`: measurement knobs for the memory wall and the
  grounder's phases. All documented in the book's
  [tuning chapter](https://hhh42.github.io/ferroplan/tuning.html).

### Changed

- **Default temporal plans can be shorter, and more concurrent, than 0.27's.**
  The decision-epoch search laid a concurrency-free task out sequentially; the
  compression rung left-shifts independent work. On a domain that is LOCKLESS
  by design this overlaps everything the PDDL allows:
  `examples/cabin/crew-solo.pddl` went from makespan 109 to 47 with one worker
  on four jobs at once -- legal PDDL2.1, VAL-valid, and not a crew schedule. A
  resource that can do one thing at a time has to say so in the domain (a busy
  token), or be scheduled by the actor scheduler: with `FF_TCONC=1` the
  left-shift stands aside and the cabin crews read 109 / 63 / 47 exactly as in
  0.27. The game-embedding `Session` has its own temporal entry and is
  unaffected.

### Fixed

- **A plan in hand is reported inside the wall.** The temporal preference
  tiers banked a plan and then chased quality against the SAME wall, opened
  further ladder rungs after it expired, and re-grounded the task to score
  the result -- returning a valid plan at 21.9 s of a 20 s budget, which a
  runner that kills at the wall records as unsolved. Optional work now stops
  a reserve short of the wall (3 % of it, 0.5-3 s, plus a size term;
  `FF_REPORT_RESERVE_SECS`), the scorer is built before the chase, and a
  banked plan that cannot be scored in time is returned unscored, with a
  note, rather than lost. The same reserve covers the PDDL3 optimizer.
- The best-first loop read the clock once per 256-evaluation batch -- two
  seconds at the 8 ms an evaluation costs on a compiled preference task.
  Evaluations and expansions now read an armed deadline individually; a run
  that finishes inside its wall is unchanged, evaluation for evaluation.
- The preference-selection DFS, the optimizer's restart ladders and its
  legacy fallback no longer open work after the wall has expired.
- **Numeric heuristic, dead ends**: the relaxed-graph fixpoint test counted
  ANY bound movement as progress, so a consumable's un-read lower bound,
  drifting down for ever, sent every dead-end evaluation to the 2,000-layer
  cap. The test is now direction-aware (`FF_NO_NEED_DIRS=1` restores it).
  Heuristic values are identical; evaluations per second on
  `rovers-metric-time`-shaped tasks rise ~78x.
- **Memory is a wall too** (`ferroplan::mem`): the engine now MEASURES its own
  resident size (`/proc/self/status`; `task_info` on macOS) instead of only
  modelling it, and bounded work over a plan already in hand -- a quality
  chase, the grounding that prices a found plan, a rung's bet -- stops at 75 %
  of `FF_MEM_BUDGET_GB` and returns what was banked, where a runner's RSS
  watchdog used to kill the process and the plan with it. A first search is
  never cut by it. `FF_NO_MEM_WALL=1` restores 0.27. On a 16-bit toy task the
  unwalled chase reaches 8.2 GB in 39 s; walled, it is back in half a second
  at 209 MB. A trip is STICKY for the scope it happened in: what the scope
  opens next -- another tier's grounding, another search -- is refused at its
  first look, not after it has climbed back to the line. The grounder looks
  inside its interning loop as well as between phases, and EVERY grounding
  entry is armed inside bounded work, the preference scorer's included: a
  solved plan may now come back "NOT scored" where it used to be scored at a
  resident size over the declared budget. `FF_MEM_TRIP_FRAC` moves the line,
  for measuring it.
- The PDDL3 route plans its hard goals on the pair with SOFT trajectory
  constraints stripped: they cannot invalidate a plan, and their monitors are
  most of a qualitative-preference task.
- The temporal validator (and scorer) fired ends before starts within one
  epoch, which ordered a ZERO-duration step's end ahead of its own start and
  rejected every plan containing one.
- The text path no longer prints "problem proven unsolvable" for a PDDL3
  run that simply ran out of budget before its first plan.

### The site (deployed from `main`; not part of any published crate)

- **The browser demo can be saved for offline use.** A **Save offline** button
  in the demo's header registers a service worker that keeps the three demo
  pages, the example corpus and the WASM planner in the browser, so the demo
  opens and plans with no network. Opt-in: nothing is stored until asked. The
  copy is refreshed in the background on every online visit, a new deploy
  (keyed by commit) replaces it whole, the control reports what it holds and
  removes it on request, and the page is installable as an app. Verified
  end to end with a headless browser: save, offline reload, plan in the web
  worker offline, the live pages offline, update to a new build, remove.

### Harness (the crucible; not part of any published crate)

- **A subset is a sweep over fewer cells.** `crucible sweep --set S [--board
  ID].. [--only RE] [--rows FILE] [--prior unsolved|solved] [--name N]
  [--engine PATH]`, the same flags on `backfill` (minus `--engine`) and
  `compare` (plus `--lost FILE`). Same runner, referee, owed-row cascade,
  canary and database as a cut sweep; it stages under
  `benchmarks/probes/<name>/<ver>-<hash>/`, records no board pass, and shares
  its rows with the sweep that follows. Built after a week in which seven
  "misses" on one board turned out to be a foreign process at 210 % CPU
  beside an ad-hoc shell loop that could not know.
- Known, owed: `run.peak_rss` is a SAMPLED maximum. Two rows banked as solved
  at 4.5 and 5.9 GB under a 6 GB cap truly peaked at 6.6 and 6.4. `wait4`
  already returns `ru_maxrss`.

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (30 earlier releases, 0.1.0–0.27.1).
