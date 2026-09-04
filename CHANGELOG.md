# Changelog

All notable changes to this project are documented here.

## [Unreleased]

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

## [0.25.0] - 2026-08-27 — The table grows to 32 boards — and the like-for-like 22 dips, and says so first

Two headlines BY DESIGN (roadmap-0.25 Phase 6): the grown table and the
like-for-like instrument, never blended. Full record:
[`docs/roadmap-0.25.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.25.md).

### Headline one — the grown table

**56% coverage across 32 IPC boards** (4,705/8,444), of which **665 are
certified optima** — the proof surface nearly doubles (386 → 665). Ten
boards enter: 2014 mco t2 (157/280) and t8 (164/280), 2018-opt
(89/240 ⚖️), 2023 sat (36/140) and 2023 opt (33/140 ⚖️), 2023
numeric-opt (81/400 ⚖️), 2026-opt FULL (80/260 ⚖️), and the three 2006
preference tracks on their full corpora — simple 90/130, qualitative
23/100, complex 9/108 (the first complex-preferences rows in this
planner's history; the Phase 2 entry). The denominator grows
6,366 → 8,444 and the total percentage DROPS on entry day exactly as the
roadmap said it would — a bigger honest table, not a regression.

### Headline two — the like-for-like 22: down 38, and the record names where

3,943/6,366 (61.9%) vs 3,981/6,366 (62.5%) at 0.24.0 — **−38 net**,
concentrated: **net-benefit 248→224 (−24)** and **propositional 369→358
(−11)** own −35 of it; 2023-numeric −8 (251→243) is third. Gains:
**metric-time 54→64 (+10)** and **time 77→79 (+2)** — BOTH under the
Phase 5 tier move (their boards moved 30 s → 60 s this cycle), so their
movement column carries budget-plus-engine, never engine alone; the
engine half of metric-time's +10 includes the two 0.25 bug fixes (the
zero-duration durative skip and the [TREL] relevance-mask hole).
seq-sat +3 (504→507); small ±1–3 elsewhere at the 60 s wall.

**Adjudications owed, hypotheses named (the 0.24 rule — never papered
over):** the prime suspect for net-benefit's −24 is the Phase 2
preference-tier ROUTER, a global change — the sweep header itself said
"the router change is global — watch for parity" before it ran.
Propositional's −11 sits entirely in near-wall timeouts (92 vs 81, zero
mem-caps), where borderline flips and the same router suspect both
apply. Both boards' final numbers came from CLEAN re-runs (~74% idle) —
contention does not explain them. Neither is root-caused in this
record; both are named for 0.26.

### The cycle's engine story

- **Wing II (Phase 3):** the conflict-rate bail is the refund that
  shipped (match-cellar i1 30.7→17.5 s, i2 31→1.2 s;
  `FF_NO_SAT_RATEBAIL` restores); the CEGAR pairing GUARD (a soundness
  fix, no hatch by design), layer-shift generalization
  (`FF_NO_SAT_LAYERGEN`); planning branching MEASURED NEGATIVE for
  default-on and shipped opt-in (`FF_SAT_BRANCH`). The wing's step-5
  verdict stands recorded: no board moved.
- **The metric-time decode (Phase 4)** found and fixed two real bugs,
  fixtures first; pathways stays 0/30 and its riddle carries to 0.26
  sharpened.
- **match-cellar canary: 40/40** at 0.23's exact costs — clean.

### The sweep, on the record

Three passes, 2026-08-26 → 08-27, on `m5-air`. Nine boards measured
under contention in pass 1 (Docker Desktop's VM, Steam, a wrangler
burst, a Gradle daemon — each named in the log) were refused WHOLE and
re-banked clean in passes 2–3 at ~74–77% idle. Nothing dirty was
promoted. Snapshot banked (`standings-history.json`, 7th snapshot).

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (26 earlier releases, 0.1.0–0.24.0).
