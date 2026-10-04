# Competition standings

ferroplan measures itself on 32 IPC boards — every deterministic track it
can enter from **IPC-5 (2006)** through **IPC-2026**: classical
satisficing, temporal, PDDL3 preferences and constraints, net-benefit,
numeric, the multi-core track under its wall-clock rule, and seven optimal
tracks where coverage is proof rate. Budgets are 60 s per instance (300 s
for the one official-budget agile entry), one thread, two instances at a
time on the sweep box, every reported plan externally validated by
[VAL](https://github.com/KCL-Planning/VAL). The tables below are
GENERATED from the raw per-instance sweep logs (`python3
benchmarks/standings.py`, and the crucible's own renderer must agree with
it byte for byte) and are refreshed against the final binary at every
release cut — scoreboards defend themselves.

Three kinds of honesty markers appear throughout:

- **Reference-scored** means scored against the official competition
  field. The IPC-5 preference boards are scored from the vendored official
  results archive (`benchmarks/IPC5-results.tgz` — see
  `benchmarks/ATTRIBUTION.md` for provenance) by the IPC quality score
  against SGPlan5's own `; MetricValue`, the currency those tracks were
  ranked on: level with the winner on rows, behind on points, and the
  table says so. The IPC-5 propositional, time and metric-time tracks are
  quality-scored by plan length and makespan against the archive field;
  the 2018 and 2023 classical boards against the best-known bounds.
- **Coverage-only** means no aligned reference exists: no official
  per-instance archive is vendored for IPC-6/7 and the 2014 corpus, and
  the constraints board records no quality currency.
- **vs field** (in [`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md))
  is a coverage-RATE placement against the competition's actual entrants
  at ~1/30th of the official budget — a floor, not a result;
  `docs/ipc-rankings.md` holds the provenance.

Failure classes are counted per unsolved instance: `timeout` (budget
exhausted), `mem-cap` (the runner's memory cap hit — tracked separately
from engine verdicts), `early-exit` (the engine returned without a plan
before its budget), `engine-reject/error` (a named rejection —
`hold-during` / `hold-after`, or a parse error), `spawn-fail` and
`VAL-RED` (the harness's own failures, never silently folded in).

The optimal tracks (⚖️) are entered since 0.19 under `Mode::Optimal` — A*
with an admissible LM-cut heuristic — and their coverage IS proof rate.
The sequential multi-core tracks are entered under their competition rule
(wall-clock with all cores; per-thread-count determinism still holds) on
the sweep box's 4P+6E cores, with the t8 row marked as oversubscribed.

{{#include ../../benchmarks/ipc-standings.md}}

## Reading the boards

- [`benchmarks/ipc5-scoreboard.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md)
  — IPC-5 simple preferences, ferroplan vs the official field.
- [`benchmarks/ipc5-qualitative-scoreboard.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-qualitative-scoreboard.md)
  — IPC-5 qualitative preferences, reference-grafted with the full
  W/T/L accounting.
- [`benchmarks/ipc67-results.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc67-results.md)
  / [`benchmarks/ipc67-temporal.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc67-temporal.md)
  / [`benchmarks/ipc67-netben.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc67-netben.md)
  — the standing IPC-6/7 scoreboards (seq-sat, tempo-sat,
  net-benefit), per-variant.
- [`benchmarks/ipc5-prop.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-prop.md)
  and siblings (`ipc5-time`, `ipc5-metric-time`, `ipc5-constraints`)
  — the 2006 deterministic-track sweeps, first entered in 0.16.
- `benchmarks/ipc7-mco-t{2,4,8}.md` — the multi-core rows.

Per-cycle history (what moved and why, cut by cut) lives in the
`docs/roadmap-0.*.md` records; this chapter is always the CURRENT
standing.
