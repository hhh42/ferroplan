# Changelog

All notable changes to this project are documented here.

## [Unreleased]

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

## [0.26.0] - 2026-09-04 — The fallback learns the LAMA recipe, and the harness learns what it was doing wrong

**59% coverage across 32 IPC boards** (4,988/8,444), **685 certified
optima** — **+283** over 0.25.0 on the same 32-board instrument, the first
cut that can show movement against the grown table. Full record:
[`docs/roadmap-0.26.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.26.md).

### What moved, and why

- **F1 — the complete fallback carries the LAMA recipe** (preferred
  operators + the landmark term, default-on, `FF_NO_ENRICH=1` restores).
  Refereed on crucible at +8 ipc5-prop / +4 2018-sat against a +10–17 band
  (under-delivered, named), then +12/13 on its own witnesses under the
  hatch differential. On the full table it is the driver behind most of
  the +283: net-benefit **224→267** (the 0.25 −24 adjudication closes —
  crew-planning 9→30), the three IPC-5 preference boards **+58** between
  them (qualitative 23→46, simple 90→108, complex 9→26), propositional
  +22 (pathways 15→25), seq-sat +19, 2023-numeric +19.
- **F3 transport**: the rung that converts is the novelty driver, so its
  wall slice grows 0.30→0.50 (`FF_NOV_WALL_FRAC=0.30` restores): +6/+1
  solo, transport 2008/2011 rows across the seq-sat and mco boards.
  `FF_NUMPRE_TEMPORAL` ships opt-in at +1 (pathways-metric-time i1); the
  temporal pass ladder stops re-running passes it has already proven
  equivalent (`FF_NO_LADDER_DEDUP` restores).
- **Recorded negatives:** F2 YAHSP-style lookahead (opt-in, CLOSED), F4.1
  wall-denominated length polish, F4.2 the memory build (folding and
  elevator are grounding walls, not memory), F5 the 2014 config schedule
  (evaluation-cost tails and grounding-time rows, no schedule survives).
  The grounding checkpoint reads the clock every 256 bindings instead of
  8,192.
- **The proof-gap centerpiece was priced (Phase 0: three mechanisms, bands
  +1–2, +4–6/+2–3, +4–9 at 300 s) and NOT built** — the cycle's weight
  went to the instrument. The bands carry to 0.27 as riders.
- **The mem-cap classification fix**, landed after byte-parity was proven
  in both implementations (`standings.py`, crucible `referee.rs`): the
  0.24 label-hygiene suffix (`mem-cap (self-inflicted: …)`) had been
  filed under `early-exit` since 0.24. **60 rows across five boards move
  early-exit → mem-cap** (2023-numeric 56, one each on 2026-numeric,
  2014-mco-t4, 2014-mco-t8, propositional). Coverage untouched;
  attribution corrected.

### The sweep, on the record — and why 0.27 is a harness cycle

The first cut swept by **crucible** (F6): one process, the database as
the truth, every row committed in its own transaction, `kill -9` loses
nothing. It spawned 2026-08-30 14:07 and was **stopped 2026-09-04 after
six passes and 5 d 16 h** with 232 rows still owed — 44 of them solves
(kept; coverage is coverage), 184 timeouts measured under contention and
not re-measured (2014-agile 36, 2023-agile-300s 33, seq-opt 21, 2023-sat
11, 2018-sat 10, the two 2014 mco boards 17, the rest in ones and twos),
4 with no watcher coverage. The −2 on 2014-mco-t8 and −1 on 2026-opt-full
sit inside those rows. The referee that re-owed them measured the *box*
(any foreign process over 25 % pcpu) rather than the *run*; priced on
the sweep's own database, 89 % of the re-owed timeouts had used ≥ 90 %
of their wall as CPU. That referee, four defects found beside it
(`cpu_ms` 41.67× low from Mach units read as nanoseconds; the throttle's
sender dropped so SUSPENDED never reached a child; `tier::order` never
called; `jobs = 2` stamped on 1-wide rows) and the dashboard the sweep
never had are `crucible-spec.md` §R2 and `docs/roadmap-0.27.md`.

## Movement — all 32 boards, 0.25 promoted vs 0.26 promoted

| board | track | 0.25 | 0.26 | delta | what moved |
|---|---|---|---|---|---|
| ipc67-netben | net-benefit | 224/270 | 267/270 | +43 | crew-planning-net-benefit-opt-fluents 9→30; openstacks-net-benefit-opt-strips-negative-preconditions 24→30; woodworking-net-benefit-opt-fluents 21→27 |
| ipc5-qual-pref | qualitative-preferences (full corpus) | 23/100 | 46/100 | +23 | openstacks-qualitative 5→17; storage-qualitative 4→10; tpp-qualitative 5→8 |
| ipc5-prop | propositional | 358/450 | 380/450 | +22 | pathways 15→25; pipesworld-strips 39→42; tpp-strips 28→30 |
| ipc67-results | seq-sat | 507/580 | 526/580 | +19 | transport-strips 20→26; transport 2→7; elevator 14→17 |
| ipc2023-numeric | 2023 numeric | 243/400 | 262/400 | +19 | ext-plant-watering 4→10; sugar 9→11; fo-sailing 15→17 |
| ipc5-simple-pref | simple-preferences (full corpus) | 90/130 | 108/130 | +18 | storage-simple 6→13; tpp-simple 15→20; openstacks-simple 15→19 |
| ipc5-complex-pref | complex-preferences (full corpus) | 9/108 | 26/108 | +17 | trucks-complex 4→17; pathways-complex 2→6 |
| ipc-opt-2008-11 | seq-opt | 284/550 | 296/550 | +12 | barman-opt 4→7; woodworking-opt 5→7; scanalyzer-3d-opt 9→10 |
| ipc2014-sat | 2014 seq-sat | 150/280 | 161/280 | +11 | cave-diving 4→7; parking 3→5; tetris 11→13 |
| ipc7-mco-t2 | seq-mco t2 | 231/280 | 241/280 | +10 | transport-multi-core 4→7; elevator-multi-core 18→20; no-mystery-multi-core 15→17 |
| ipc7-mco-t8 | seq-mco t8 | 238/280 | 248/280 | +10 | no-mystery-multi-core 15→18; transport-multi-core 5→8; parking-multi-core 18→19 |
| ipc5-time | time | 79/130 | 88/130 | +9 | trucks-time 11→17; trucks-time-strips 13→15; storage-time 15→16 |
| ipc67-temporal | tempo-sat | 434/630 | 443/630 | +9 | peg-solitaire-t-strips 28→30; elevator-t 7→9; elevator-t-strips 28→29 |
| ipc7-mco-t4 | seq-mco t4 | 235/280 | 244/280 | +9 | transport-multi-core 5→8; no-mystery-multi-core 15→17; parking-multi-core 16→17 |
| ipc2023-agile-300s | 2023 agile ENTRY (300s) (300 s) | 50/140 | 58/140 | +8 | recharging-robots-agile 7→9; folding-agile 1→3; slitherlink-agile 5→7 |
| ipc2014-agile | 2014 seq-agile | 147/280 | 154/280 | +7 | cave-diving-agile 4→7; city-car-agile 2→4; openstacks-agile 10→12 |
| ipc2026-numeric | 2026 numeric (first board) | 217/320 | 224/320 | +7 | ztalloc-sum 9→11; petri-net 14→16; line-exchange-snp 8→10 |
| ipc2014-opt | 2014 seq-opt | 76/256 | 82/256 | +6 | transport-opt 4→6; hiking-opt 9→10; parking-opt 0→1 |
| ipc2014-mco-t2 | 2014 seq-mco t2 | 157/280 | 163/280 | +6 | cave-diving-multi-core 4→7; tetris-multi-core 11→13; city-car-multi-core 3→4 |
| ipc2014-mco-t4 | 2014 seq-mco t4 | 161/280 | 166/280 | +5 | tetris-multi-core 12→14; cave-diving-multi-core 5→7; city-car-multi-core 3→4 |
| ipc2018-sat | 2018 seq-sat | 82/240 | 86/240 | +4 | organic-synthesis-split 6→7; flashfill 12→13; agricola 0→1 |
| ipc2023-agile | 2023 classical | 37/140 | 41/140 | +4 | slitherlink-agile 3→5; folding-agile 0→1; labyrinth-agile 0→1 |
| ipc2023-sat | 2023 seq-sat | 36/140 | 39/140 | +3 | slitherlink 3→5; labyrinth 0→1 |
| ipc2014-tempo | 2014 tempo-sat | 76/200 | 78/200 | +2 | turn-and-open-t 4→5; map-analyzer-t 15→16 |
| ipc2018-opt | 2018 seq-opt | 89/240 | 91/240 | +2 | petri-net-alignment-opt 7→8; settlers-opt 8→7; caldera-opt 6→7 |
| ipc2023-numeric-opt | 2023 numeric-opt | 81/400 | 82/400 | +1 | hydropower 12→13 |
| ipc5-metric-time | metric-time | 64/200 | 64/200 | +0 |  |
| ipc5-constraints | constraints | 28/120 | 28/120 | +0 |  |
| ipc2026-opt | 2026 numeric-opt | 22/60 | 22/60 | +0 |  |
| ipc2023-opt | 2023 seq-opt | 33/140 | 33/140 | +0 |  |
| ipc2026-opt-full | 2026 numeric-opt FULL | 80/260 | 79/260 | -1 | petri-net 2→3; forestfire 8→7; gear-car 9→8 |
| ipc2014-mco-t8 | 2014 seq-mco t8 | 164/280 | 162/280 | -2 | tetris-multi-core 13→11; parking-multi-core 9→7; cave-diving-multi-core 5→7 |

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (27 earlier releases, 0.1.0–0.25.0).
