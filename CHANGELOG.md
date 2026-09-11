# Changelog

All notable changes to this project are documented here.

## [Unreleased]

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

## [0.27.0] - 2026-09-10 — One lever in the engine, and an instrument that can finally finish

**61% coverage across 32 IPC boards** (5,122/8,444), **687 certified
optima** — **+134** over 0.26.0 on the same instrument. Full record:
[`docs/roadmap-0.27.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.27.md).

The engine changed in exactly one place this cycle. Most of the work went
into the thing that measures it, and that is the honest summary of what
0.27 is.

### The engine

- **The anchored successor generator** (`PackedTask::applicable_ops`).
  Expansion used to scan every grounded op to find the applicable ones.
  Now each op is anchored at its RAREST positive precondition, candidates
  are gathered from the state's true facts, and the result is sorted back
  into the scan's order — so the search it feeds is byte-identical, the
  same plans found in the same number of evaluations. Expansion per
  evaluation: labyrinth 234 → 34 µs (7×), parking 258 → 21 µs (12×),
  markettrader 10 → 5.6 µs. Wired into the classical, LAMA and novelty
  rungs.
- `apply()` gains an allocation-free path for ops with no conditional and
  no numeric effects, which otherwise cost four temporaries per successor.
- **Recorded negative:** the counter-based relaxed-graph build (reached
  facts decrement the ops that need them, no per-layer scan) measured
  1.78 → 2.02 ms on labyrinth and 0.92 → 0.96 on parking. Slower.
  Removed. What remains on these boards is the relaxation floor itself,
  ~1.7–2 ms per evaluation at 60–80k ops; moving it means firing fewer
  ops or evaluating fewer states, not a faster scan.

### Where it moved

simple-preferences +8.5 pts (119/130), 2014 seq-agile +6.4 (172/280),
qualitative-preferences +5.0 (51/100), and 2014 seq-sat / 2023 seq-sat /
2023 classical +4.3 each. 2018 seq-sat at 94/240 now places ~1st of 25
entrants by rate. One track went backwards: tempo-sat −0.3 pts.

net-benefit stands at **270/270**, but that is NOT claimed as a 0.27
gain. The like-for-like backfill now running — the v0.26.0 tag rebuilt
and re-measured on this box under the current referee — reaches 270/270
as well, so its published +3 was the instrument, not the engine. The
same control has so far moved 2026 numeric-opt's +1 to 0 for the same
reason. **Expect the same-instrument total to land a little under +134**;
it will be recorded when the backfill completes, against 0.26.0's own
re-measured numbers rather than its published table.

### The instrument (crucible R2)

Not shipped to crates.io — it is the harness — but it is why the numbers
above are worth reading. **This is the first sweep in the project's
history to reach a terminal state: 8,444 of 8,444 instances banked, zero
owed.** The 0.26 cut was taken by decision after six passes and five days
sixteen hours with 232 rows still owed.

The referee now judges each row by ITS OWN process rather than by the
box, which is what makes a clean terminal state reachable at all. Four
defects it found the hard way, each with its receipt:

- `cpu_ms` was **41.67× low** on every row ever recorded — Mach absolute
  time read as nanoseconds.
- The throttle never reached the child for the whole 0.26 sweep: the
  control channel's sender was dropped at construction.
- The **E-core defect**: under POLITE the harness put planners in
  Darwin's background band, where the same instance took **59.28 s
  instead of 4.52 s** — and banked, because ρ 0.956 is CPU share and a
  demoted process has all the share of a slow core. 1,709 rows affected.
- The canary locked onto a 3%-frequency boost clock and refused 553 rows
  as thermal across eleven boards. Its baseline is now the 25th
  percentile of recent solo readings.

### Honesty notes

- **11 instances that 0.26 solved, 0.27 did not.** Each was re-opened and
  re-run solo on a quiet box under the cut rule before promotion; these
  are the ones that failed again and are counted as real. Eight others
  looked like regressions and were not — they solved on the re-run, which
  is a 42% false-regression rate in the raw sweep and the reason the
  re-check exists.
- **The referee changed during the final passes.** ρ is CPU over wall,
  and every process pays a fixed ~0.3 s of fork, exec, linking and
  teardown that is wall without CPU — so below ~8 s no process, however
  well served, can reach ρ ≥ 0.95. Runs under that floor were being
  refused forever, and the set could not have completed. They are now
  judged by the box-wide window, which is what 0.26's instrument did for
  every row. Verified against the database rather than asserted: the same
  rows banked under 0.26 as `window`. The solved count did not move
  across either change — what banked were honest non-solutions.
- Coverage is measured at 60 s (300 s where a board says ENTRY), against
  official budgets that are typically 30× longer. The comparison is to
  ferroplan 0.26.0 on the same box, not to the competition.

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (28 earlier releases, 0.1.0–0.26.0).
