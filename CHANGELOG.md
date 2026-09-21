# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.28.0] - 2026-09-21 — Feasible first, better second

A 2026-09-20 read of SGPlan5's own IPC-5 solution headers found its MEDIAN
solve is **0.54 s** -- 89 % of its 860 solves finish inside 60 s, and of the
335 rows it solves and ferroplan did not, 143 took it under a second. The gap
on those boards was never the wall. On most of those rows a valid plan was in
hand, or milliseconds away, and the route had no way to return it. Five engine
lanes close that, and the harness learned to measure a question smaller than
a release.

### Measured

Through the crucible (same referee, contention throttle and re-run rule as a
cut sweep), on the six IPC-5 boards the work was aimed at, 60 s, one thread:

| | 0.27.1 (published boards) | 0.28.0 |
|---|---:|---:|
| six IPC-5 boards, of 788 | 379 | **569** (+190, none lost) |
| the variants SGPlan5 entered, of 678 -- SGPlan5 solves 612 | 316 | **491** |

Read it with what it is made of. **46 of the +190 are the EMPTY plan**, on
problems with no hard goal at all (every goal a preference): valid,
VAL-accepted, and the boards' standing convention -- 0.27.1's carry 27 -- but
floor quality. 144 are plans that do something. And coverage is not what IPC-5
ranked these tracks on: by its quality score (best metric / ours, over the
cells SGPlan5 solved) ferroplan moved 94.8 -> 94.0 on simple preferences
(SGPlan5 119.4), 45.8 -> 59.2 on qualitative (84.8), 20.1 -> 50.6 on complex
(67.9). **The lanes closed rows and left points where they were.** The 569 is
an overlay of re-measured cells across two builds of this cycle, so it is an
estimate; the 32-board cut sweep is the instrument of record
([`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md)).

Regression read, equal-N (one banked row per cell per engine), over the 519
temporal cells the published boards solved: **519 of 519**, summed solve time
0.91x, makespan better on 181 cells and worse on 6. Over the 58 cells that
solve at a resident size of 3.5 GB or more -- the only ones the new memory
wall can touch -- 58 of 58, 49 identical, one metric worse by 2, and **eight
that keep their row and give up their metric** (they used to be scored at a
resident size over the declared budget, unnoticed). The full record, the
recorded negatives included, is
[`docs/roadmap-0.28.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.28.md).

### Added

- **The compression rung** (`tcompress`): a temporal task that needs no
  concurrency is planned as a classical one -- each durative action as one
  instantaneous action -- and a left-shift over the ops' read/write sets puts
  the plan back on the clock. The plan is validated against the ORIGINAL
  task before it is returned. It BANKS: the decision-epoch ladder then runs
  as a bounded quality chase and the smaller makespan wins, so a task that
  solved before returns the plan it returned before. Declines required
  concurrency, timed initial literals and trajectory constraints.
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

## [0.27.1] - 2026-09-11 — A budget the caller can set, and withdraw

No engine change. Coverage is unchanged from 0.27.0 (5,122/8,444) because
nothing here touches how the planner searches — the additions are inert
unless you set them.

### Added

Two fields on `Options`, both `None` by default:

- **`wall_ms: Option<u64>`** — this call's wall, in milliseconds. Armed at
  the top of `solve`, so it bounds parsing and GROUNDING as well as
  search. That is the budget the existing knobs could not express:
  `max_evaluated` caps evaluated states, and grounding runs before the
  first state exists, so a call capped at fifty thousand evaluations can
  still spend minutes. `FF_TIME_LIMIT` could not express it either, for
  the opposite reason — it is armed once per PROCESS from the first solve,
  so in a long-lived host it either bounds nothing or eventually refuses
  everything.
- **`should_continue: Option<Arc<AtomicBool>>`** — flip it to `false` to
  stop this call. Polled wherever the wall is: the grounding checkpoint
  (every 256 bindings), the best-first batch boundary, EHC's
  per-evaluation slice, and the temporal pop. `#[serde(skip)]`, since a
  live handle has no JSON form.

A stop from either returns `solved: false` with a note naming which budget
bound and where — `grounding stopped at the declared budget: …` or
`search stopped at the declared budget: …` — and never the word
"unsolvable". Running out of time is not a proof.

The budget is held per call, in a thread-local armed by an RAII guard, so
concurrent `solve` calls on different threads each carry their own and a
spent budget cannot leak into the next call on its thread.
`FF_NO_RUNG_WALLCAP` does not disable it: that hatch governs the
environment wall, and a budget passed in code outranks an environment
variable.

### Why this is a patch release

It answers a consumer blocked on it. `solve` was blocking and
uninterruptible, so a host that abandoned the work leaked that thread for
the life of the process; their own instrumentation recorded 213 solves
completed, then 2 abandoned, after which the pool was dead — 0 completed,
4 abandoned. The rest of the 0.28 cycle is unmeasured and stays on its
branch until it has been swept.

### Note

The search ladder rations a wall across its rungs, so a budget is not
simply "time until I stop" — each rung sees a fraction of it. A 30-block
instance that solves in 34 ms unbudgeted is stopped by a 120 ms wall and
solved under a 200 ms one. That is not new to this release: `FF_TIME_LIMIT`
of 0.12 s fails the same instance and 0.2 s solves it. Measure your own
domain rather than assuming a 250 ms wall buys 250 ms of search.

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (29 earlier releases, 0.1.0–0.27.0).
