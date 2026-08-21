# Changelog

All notable changes to this project are documented here.

## [Unreleased]

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

## [0.23.0] - 2026-08-16 — The temporal cycle, and the whole table on one box

The cycle that moved the temporal boards to their honest 60 s tier
with the budget/engine split proven per instance, opened the
constraints gate, re-entered the last six ghost boards — and for the
first time in this project's history, **every number in the table
shares one box.** The "Not re-baselined" section is deleted. Full
record:
[`docs/roadmap-0.23.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.23.md).

### Where this leaves the standings

**62% coverage across 22 IPC boards** (3,916/6,366), of which **381
are certified optima**. On the sixteen comparable boards: **+47**
(55 gains, 8 losses, every one named in the record); the six
re-entries add 1,002/1,450. At-a-glance:
[`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md).

- **The tier move, refereed exactly as the rule demands:** the
  v0.22.0 binary at 60 s gains +16 — dead center of the projection —
  and 0.23 banks 15 of those plus 9 engine gains (+24 across the two
  temporal boards, zero losses). The turn-and-open churn class
  retired as pre-registered. The budget caveat on every temporal
  placement halves (30× now, was 60×).
- **The constraints gate's first board:** 5/120 → 12/120, exactly
  the seven storage solves banked solo pre-sweep; the engine-reject
  class drops 100 → 70, leaving precisely the timed constituency
  that 0.24's stage c has since built.
- **The makespan quality columns debut** (the first temporal quality
  currency): time mean 0.80 (27W/3T/47L vs the official archive),
  metric-time **0.94** (43W/1T/10L; both openstacks variants sweep
  40W/0L at 1.00).
- **The mco sitting lands its four boards** (230/237/240/164 at
  t2/t4/t8/2014-t4) with its caveats carried loudly: t4 and t8
  banked under DEGRADED conditions verdicts (t8 can never read
  clean — eight own threads on ten cores — and a real half-core
  competitor ran all board long) and with VAL unavailable; t2 and
  2014-t4 fully VAL-green. Contention only ever depresses coverage.
- **The bills, read against the v0.21.0 backfill column:**
  org-synth-split i15 and hiking-agile i11 PAID (both return on the
  board itself); onlycraft's −6 is confirmed REAL engine cost
  (all six solve under the v0.21.0 tag on this box) and STANDS as
  0.24's open docket; the damping three stand engine-side;
  floor-tile-2011 i11 carried; nurikabe closed against 0.24's
  measured negative; the openstacks-2014 acquittal corroborated —
  the 0.21 board's 12/20 was itself the outlier.

## Movement — all 22 boards, 0.22 promoted vs 0.23 promoted

| board | track | 0.22 | 0.23 | delta | what moved |
|---|---|---|---|---|---|
| ipc67-temporal | tempo-sat 08+11 (60 s, was 30) | 419/630 | 436/630 | +17 | 9 budget + 8 engine (5 sokoban-t = Phase 6 MCV; 3 elevator-t = memory); 0 losses |
| ipc2014-tempo | 2014 tempo-sat (60 s, was 30) | 67/200 | 74/200 | +7 | 6 budget (turn-and-open i3/i4/i6 churn class retired) + 1 engine (satellite i15) |
| ipc5-constraints | constraints (60 s both cuts) | 5/120 | 12/120 | +7 | Phase 2 a+b: storage-time-constraints i1–i7; engine-reject 100→70 |
| ipc2014-agile | 2014 seq-agile | 141/280 | 147/280 | +6 | +hiking i11, +parking i2/i3/i5, +tetris i8/i12/i14; −parking i14 (wall) |
| ipc-opt-2008-11 (proof) | seq-opt | 277/550 | 281/550 | +4 | +5 LM-cut certs (elevator-11 i16, no-mystery-11 i5/i15, tidybot-11 i5/i11); −peg-solitaire-11 i16 |
| ipc2014-opt (proof) | 2014 seq-opt | 74/256 | 78/256 | +4 | hiking i17 (h^max+orbits), tidybot i8/i11, visit-all i16 |
| ipc2014-sat | 2014 seq-sat | 147/280 | 151/280 | +4 | openstacks i5, parking i5, tetris i8/i14 |
| ipc2018-sat | 2018 seq-sat | 79/240 | 80/240 | +1 | org-synth-split i15 (0.22 driver casualty returns, 59.99 s) |
| ipc2026-numeric | 2026 numeric | 179/320 | 180/320 | +1 | line-exchange-snp i5_5_90_10 |
| ipc2023-agile | 2023 classical (60 s baseline) | 36/140 | 36/140 | = | zero churn |
| ipc2023-numeric | 2023 numeric | 251/400 | 251/400 | = | zero churn; all 5 watchlist rows held |
| ipc2026-opt (proof) | 2026 numeric-opt | 22/60 | 22/60 | = | onlycraft i2 cert 16.07→1.0 s (fold) |
| ipc5-prop | propositional | 366/450 | 366/450 | = | quality 0.89→0.90 |
| ipc2023-agile-300s | 2023 agile ENTRY | 52/140 | 51/140 | −1 | +folding i8, +labyrinth i1; −recharging i17/−ricochet i7/−rubiks i6 (300 s wall churn) |
| ipc67-results | seq-sat 08+11 | 504/580 | 503/580 | −1 | −parking-2011 i15 (59.82→timeout 59.81) |
| ipc67-netben | net-benefit | 248/270 | 246/270 | −2 | −crew-planning i23 (wall), −woodworking i20 (mem-cap 33.9 s) |
| ipc5-time | time (NEW) | — (cloud 76/130, 0.16, incomparable) | 77/130 | new | makespan debut 27W/3T/47L, 0.80 |
| ipc5-metric-time | metric-time (NEW) | — (cloud 54/200, 0.19, incomparable) | 54/200 | new | makespan debut 43W/1T/10L, 0.94 |
| ipc7-mco-t2 | seq-mco t2 (NEW) | — (cloud 193/280, 0.16, incomparable) | 230/280 | new | wall-clock rule; VAL 230/230; clean |
| ipc7-mco-t4 | seq-mco t4 (NEW) | — (cloud 189/280, 0.16, incomparable) | 237/280 | new | DEGRADED verdict hand-banked; VAL unavailable |
| ipc7-mco-t8 | seq-mco t8 (NEW) | — (cloud 193/280, 0.16, incomparable) | 240/280 | new | oversubscribed by construction; DEGRADED (spotlight 52%); VAL unavailable |
| ipc2014-mco-t4 | 2014 seq-mco t4 (NEW) | — (cloud 107/280, 0.17, incomparable) | 164/280 | new | clean; VAL 164/164 |
| **TOTAL** | | **2,867/4,916 (58%)** | **3,916/6,366 (62%)** | **+47 comparable, +1,002 re-entry** | optima 373 → 381 |

- **The constraints gate opens** (Phase 2, stages a+b): at-end
  trajectory constraints fold as a TRAJ-END acceptance latch and the
  untimed monitors (always/sometime/at-most-once/sometime-before)
  ride the snap-compiled temporal path with a monitor AUDIT on the
  emitted schedule — storage-time-constraints i1–i7 solve solo where
  every one was an engine-reject; `within` now rejects by name only.
  Found on the way: a deterministic VAL crash class (SIGBUS, zero
  output) — the runner books it validation-unavailable, not
  plan-rejected.
- **The dockets close honest** (Phase 1): the openstacks ramp tax is
  ACQUITTED engine-side (interleaved v0.21/v0.22 binaries are
  eval-identical — the −3 was environmental); the damping bill's 3
  rows fail under every arm including NODAMP (the scoping's
  "recoverable" read was churn; bill stays open against the backfill
  column); fo-sailing's +7 pinned to the SUM half alone; the two
  dead flags leave the tree with the house law in their rustdoc.
- **Probe 1: goal-isomorphism symmetry — temporal DEAD** by both
  pre-registered reads (visited-class collapse 1.27× vs the 10× bar;
  best_h flat at the 110 floor): the ninth TMS negative and the
  cleanest. The arm ships classical-only behind `FF_ORBIT_ISO` with
  the round-trip fixture; TRPG-lite's gate opens per the
  pre-registration.
- **The optimal follow-through** (Phase 5): incremental LM-cut
  resumes failed probes (no-mystery i15 PROVEN 23 — the old
  certificate to the digit; child-snack i4 PROVEN by LM-cut+orbits,
  the engines' first joint certificate); the rep-folded numeric
  labels kill the per-eval tax (onlycraft i2: 17.4 s → 0.43 s to the
  same cost). 41/41 certificate slices, zero mismatches.
- **Probe 2: TRPG-lite — the tenth temporal negative, with the
  mechanism** (Phase 4): time-stamped relaxation lands behind
  `FF_TRPG`, reads clean on kiln-pack/match-cellar, and the
  pre-registered TMS read fails byte-identically — because the
  window facts a relaxation can soundly learn are provably not where
  the plateaus live (TMS's windows are never overrun; floor-tile and
  driver-log carry none). The temporal-bet ledger closes: those
  walls need different search, not better relaxation.
- **The temporal grounding wall, and MCV's confession** (Phase 6):
  the solve-path grounding arms the honest budget exit; the
  six-line mcv_order fix (full-cover producer tokens drowning the
  connectivity signal) drops sokoban-t i21's grounding from >65 s to
  4.2 s — the family re-attributes to search-side wall discipline
  (the temporal search has no wall checkpoints; named for 0.24).
  Indexed hash-joins REFUSED on a lower-bound simulation that still
  misses the gate. The last attribution sittings close satellite's
  pending row and pin markettrader as gradient-free (w_h 5→320
  changes nothing).
- **The sitting's desk half** (Phase 3): the ε-cap fails loud; the
  makespan quality column scores against the vendored IPC-5 archive;
  the tier move rides a per-row budget stamp with a promote-time
  registry gate; cut23-sweeps.sh carries all 22 boards with the mco
  methodology written where it renders.

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (24 earlier releases, 0.1.0–0.22.0).
