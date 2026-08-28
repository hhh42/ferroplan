# Changelog

All notable changes to this project are documented here.

## [Unreleased]

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

## [0.24.0] - 2026-08-20 — The SAT wing: the zero block gets its first nonzero row

The cycle that absorbed a solver instead of hand-rolling one, built the
first machinery in this planner's history that can crack the
temporal-machine-shop family, and caught its own regression before it
shipped. Full record:
[`docs/roadmap-0.24.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.24.md).

### Where this leaves the standings

**63% coverage across 22 IPC boards** (3,981/6,366), of which **386
are certified optima** — up from 62%/3,916/6,366/381 at 0.23.0.
+65 net across the (unchanged) board set. At-a-glance:
[`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md).

- **THE HEADLINE: temporal-machine-shop falls.** TMS-2011 i2 —
  SOLVED, VAL-valid, ~1 s (Mode::Sat, horizon 16, one STN
  refutation): the first TMS solve in this planner's history, on
  the family where every non-SAT entrant ever fielded scored zero.
  The zero block has its first nonzero row. slitherlink p01 falls
  to the classical face for a second first.
- **The cut's own canary caught a real regression before it
  shipped.** The 22-board sweep read match-cellar at 0/20
  (2014-tempo) and 7/20 (2011) against a 40/40 expectation — the
  promoted SAT rung was running against the WHOLE wall (h32
  ground its conflict budget in pure SAT conflicts, zero STN
  refutations, so the pre-registered thrash bail never saw
  anything to bail on) and starving the 0.02 s classical ladder of
  any time to run at all. Fixed: the promoted entry now gets
  `FF_SAT_PROMO_WALL_FRAC` (default 0.5) of the remaining wall as
  its own bounded slice, handing back cleanly on expiry so the
  ladder always gets its turn. Re-swept clean: match-cellar back to
  40/40 at 0.23's exact costs (750, 1060); TMS-2011 i2 unaffected
  (still 0.4 s).
- **ferroplan-sat**: varisat absorbed and owned — 5,012 lines, zero
  external dependencies, attribution preserved, a differential
  battery that proved itself RED against a lying stub and never
  trusts the solver's own models. One addition (conflict budgets);
  one named not-taken (planning-specific branching).
- **The wing**: ∃-step bounded-layer encoding with disabling
  chains, snap-event pairing for the temporal face, the existing
  STN scheduler as CEGAR teacher with a pre-registered thrash
  bail, a required-concurrency detector for early promotion, and
  honesty pinned at every exit (declines, horizons, budgets —
  never "unsolvable" without a proof).
- **Stage c**: `within`/`always-within` lowered onto a
  search-stamped clock — the 2006 constraints gate names nothing
  unenforced; six timed solves banked oracle-green; the
  constraints board itself moves 12/120 → 28/120.
- **onlycraft's docket reads paid**: both numeric-2026 variants
  (opt and sat) move 3/20 → 20/20, 34 of this board's 37-instance
  gain — the roadmap's open engine docket from 0.22's reallocation.
  Measured on this cut, mechanism not separately chased down this
  cycle (no targeted commit touches it) — worth confirming it holds
  under contention before calling it closed for good.
- **The basket**: the temporal search pays the wall (sokoban-t's
  honest exits); the a2 chain converts pathwaysmetric i2 at 173
  evals; the hash-join candidate lists clear the slitherlink gate
  (p03 grounding >60 s → 1.3 s); the 5A convergence fix recorded
  as a measured negative (nurikabe and spider are irreconcilable).
- **The game phase**: budget-stamped thinks with capped-vs-proven
  honesty on the MCP wire; the village tick loop 15.6 → 10.6 s at
  byte-identical evals; bazaar think latency halved. Mode::Sat
  reaches the wire by construction.
- **The contention-verdict methodology moved off raw idle%.**
  Whole-machine idle counts a board's OWN threads, so a fixed
  floor could never pass an mco `--threads 8` board even in an
  empty room (measured: 38-40% idle with <5% real competing load).
  The verdict now keys off named-competitor load instead — and
  caught a run the old metric missed (a stuck renderer process
  averaging 52% CPU across a 3h43m board, loadavg spiking to 123,
  masked by a still-above-floor median idle).

## Movement — all 22 boards, 0.23 promoted vs 0.24 promoted

| board | track | 0.23 | 0.24 | delta | what moved |
|---|---|---|---|---|---|
| ipc2026-numeric | 2026 numeric | 180/320 | 217/320 | +37 | onlycraft opt+sat 3/20→20/20 each (+34); settlers-snp +2; factory-robot +1 |
| ipc5-constraints | constraints (60 s) | 12/120 | 28/120 | +16 | stage c's timed-operator lowering continues into the 2006 corpus |
| ipc-opt-2008-11 (proof) | seq-opt | 281/550 | 287/550 | +6 | +6 certs |
| ipc5-prop | propositional | 366/450 | 369/450 | +3 | quality 0.90 |
| ipc67-netben | net-benefit | 246/270 | 248/270 | +2 | recovers both 0.23 wall losses |
| ipc2018-sat | 2018 seq-sat | 80/240 | 82/240 | +2 | |
| ipc67-temporal | tempo-sat 08+11 | 436/630 | 437/630 | +1 | match-cellar restored to parity (20/20) plus a net +1 elsewhere |
| ipc67-results | seq-sat 08+11 | 503/580 | 504/580 | +1 | |
| ipc2023-agile | 2023 classical | 36/140 | 37/140 | +1 | |
| ipc2023-agile-300s | 2023 agile ENTRY | 51/140 | 52/140 | +1 | |
| ipc5-time | time | 77/130 | 77/130 | = | |
| ipc5-metric-time | metric-time | 54/200 | 54/200 | = | |
| ipc2026-opt (proof) | 2026 numeric-opt | 22/60 | 22/60 | = | |
| ipc2023-numeric | 2023 numeric | 251/400 | 251/400 | = | |
| ipc2014-tempo | 2014 tempo-sat | 74/200 | 74/200 | = | match-cellar restored to parity (20/20); net board total unchanged |
| ipc7-mco-t2 | seq-mco t2 | 230/280 | 230/280 | = | |
| ipc7-mco-t4 | seq-mco t4 | 237/280 | 237/280 | = | |
| ipc7-mco-t8 | seq-mco t8 | 240/280 | 240/280 | = | |
| ipc2014-sat | 2014 seq-sat | 151/280 | 149/280 | −2 | |
| ipc2014-agile | 2014 seq-agile | 147/280 | 146/280 | −1 | |
| ipc2014-opt (proof) | 2014 seq-opt | 78/256 | 77/256 | −1 | |
| ipc2014-mco-t4 | 2014 seq-mco t4 | 164/280 | 163/280 | −1 | |
| **TOTAL** | | **3,916/6,366 (62%)** | **3,981/6,366 (63%)** | **+65** | optima 381 → 386 |

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (25 earlier releases, 0.1.0–0.23.0).
