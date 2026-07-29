# Competition standings

ferroplan measures itself against three International Planning
Competitions — **IPC-5 (2006)**, **IPC-6 (2008)**, and **IPC-7
(2011)** — every deterministic satisficing track, swept at standard
budgets (60 s classical / 30 s temporal, three concurrent jobs) with
every reported plan externally validated by
[VAL](https://github.com/KCL-Planning/VAL). The tables below are
GENERATED (`python3 benchmarks/standings.py`) from the raw
per-instance sweep logs and are refreshed against the final binary at
every release cut — scoreboards defend themselves.

Two kinds of honesty markers appear throughout:

- **Reference-scored** means scored against the official competition
  field. The IPC-5 preference boards are scored from the vendored
  official results archive (`benchmarks/IPC5-results.tgz` — see
  `benchmarks/ATTRIBUTION.md` for provenance): per-instance
  `MetricValue`s of SGPlan5, HPlan-P, MIPS-XXL, MIPS-BDD and the
  rest of the 2006 field. The headline there: **ferroplan beats
  SGPlan5 — the track winner — 24W/4T/10L on the qualitative
  suite**, winning rovers, storage, and tpp outright and splitting
  openstacks (the graft's first pass, scored against a stale
  0.8-era ledger, read 12/3/23 — the correction narrative is on the
  board). The IPC-5 propositional track is quality-scored by plan
  length against the archive field.
- **Coverage-only** means no aligned reference exists yet: either no
  official per-instance archive is vendored (IPC-6/7), or the
  runner does not record the track's quality currency (makespan for
  the 2006 time tracks — a named runner debt).

Failure classes are counted per unsolved instance: `timeout` (budget
exhausted), `mem-cap` (address-space cap hit — environmental, tracked
separately from engine verdicts), `engine-reject/error` (instant
rejection — feature gaps such as the four timed modal operators land
here, by name), and `search` (died mid-flight before budget).

Optimal tracks are out of scope by design — ferroplan is a
satisficing planner — and the tables say so explicitly rather than by
omission. The IPC-7 sequential multi-core track is entered under its
competition rule (wall-clock with all cores; per-thread-count
determinism still holds) on the sweep box's 4 cores, with the t8 row
marked as oversubscribed.

{{#include ../../benchmarks/ipc-standings.md}}

## Reading the boards

- [`benchmarks/ipc5-scoreboard.md`](https://github.com/seanchatmangpt/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md)
  — IPC-5 simple preferences, ferroplan vs the official field.
- [`benchmarks/ipc5-qualitative-scoreboard.md`](https://github.com/seanchatmangpt/ferroplan/blob/main/benchmarks/ipc5-qualitative-scoreboard.md)
  — IPC-5 qualitative preferences, reference-grafted with the full
  W/T/L accounting.
- [`benchmarks/ipc67-results.md`](https://github.com/seanchatmangpt/ferroplan/blob/main/benchmarks/ipc67-results.md)
  / [`benchmarks/ipc67-temporal.md`](https://github.com/seanchatmangpt/ferroplan/blob/main/benchmarks/ipc67-temporal.md)
  / [`benchmarks/ipc67-netben.md`](https://github.com/seanchatmangpt/ferroplan/blob/main/benchmarks/ipc67-netben.md)
  — the standing IPC-6/7 scoreboards (seq-sat, tempo-sat,
  net-benefit), per-variant.
- [`benchmarks/ipc5-prop.md`](https://github.com/seanchatmangpt/ferroplan/blob/main/benchmarks/ipc5-prop.md)
  and siblings (`ipc5-time`, `ipc5-metric-time`, `ipc5-constraints`)
  — the 2006 deterministic-track sweeps, first entered in 0.16.
- `benchmarks/ipc7-mco-t{2,4,8}.md` — the multi-core rows.

Per-cycle history (what moved and why, cut by cut) lives in the
`docs/roadmap-0.*.md` records; this chapter is always the CURRENT
standing.
