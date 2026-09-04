# Changelog archive

Releases older than the two most recent. The live
[`CHANGELOG.md`](CHANGELOG.md) keeps `[Unreleased]` plus the newest two;
everything before that lands here, newest first, verbatim and unedited.

`publish.sh` reads release notes from BOTH files, so archiving a version
never breaks `--release-only <old-version>`.

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

## [0.22.0] - 2026-08-08 — The coverage cycle, and the re-entries that ran hot

The cycle scoped as "think big" on solving coverage: a fresh
per-domain decode of all thirteen 0.21 boards, four numbered pots
(the 645-instance classical-satisficing centerpiece, 503 optimal-proof
timeouts, 344 temporal failures, 318 numeric-satisficing timeouts),
and one gate that outranked all of them — markettrader i3's VAL-RED,
zeroed in Phase 1 before any coverage lever was pulled. Full record:
[`docs/roadmap-0.22.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.22.md).

- **The gate passed, and the boards' one VAL-RED is not ours**
  (Phase 1): VAL type-check-refuses markettrader's instances before
  reading any plan (undeclared fuel fluents from a commented-out
  metric); the plan hand-replays valid. The harness knows both
  typecheck signatures now; the VAL-RED class zeroes with no board
  edits.
- **The charge's -8 bill, paid** (Phase 1): per-achiever gap-SUM
  damping (MAX built first, measured, recorded as the negative)
  recovers ext-plant-watering i7/i13, delivery i18/i19, rover i19
  while sailing/pathwaysmetric/tpp receipts stay byte-identical.
  `FF_NUMPRE_NODAMP=1` restores 0.21 exactly. counters/zenotravel's
  three re-attributed to the old-binary column.
- **Grounding holds a budget** (Phases 1+2): block-grouping i3's
  4^21-conjunct DNF balloon (76 s past a 60 s wall) becomes a 7.8 s
  honest exit; 2048's 67-74 s enumeration zombies end by 60.6 s;
  partition mode finally narrates under `FF_WALL_DEBUG`. Notes say
  "stopped at the declared budget" — never "unsolvable".
- **The optimal ladder's third lesson** (Phases 3+4): the root gate
  gains a margin (city-car's six thin-ratio losses gate b-branch
  again), the sprint slice scales down where landmark structure is
  unambiguous (scanalyzer-08 i4: PROVEN at 6.0 s vs 23.2 s), LM-cut
  runs as a bounded probe with h^max RESUMING its preserved open
  list on failure — and the proof boards learn numbers: an
  interval-RPG layer bound, admissible fail-closed behind a
  reject-by-name audit, takes sailing-wind-opt i8 from 2.86M blind
  expansions to 132k (+numRPG names itself in the PROVEN note). The
  differential gained the board-budget mode that catches what 90 s
  forgiveness hid — and promptly convicted 11 of the 12 named
  slice-losses on the old binary.
- **The partitioned driver takes the novelty slot** (Phase 5B): the
  centerpiece — subgoal-feature partition, width-2 pair novelty, an
  h-free driver queue (3.3× states per slice-second where the old
  rung paid per-pop h), numeric gap-bucket features opt-in.
  FF_NOV_OLD restores the 0.21 rung byte-identically; the cut's
  old-binary column is the pre-registered referee.
- **The symmetry engine** (Phase 6): orbit-canonical visited keys,
  optimal and satisficing — child-snack-opt i1 goes from three
  releases of node-cap walls to PROVEN OPTIMAL cost 21, VAL-valid;
  a 20-certificate orbit-active sample re-certifies 20/20 at exact
  costs; child-snack-sat i6 converts at 15.9 s. barman honestly
  deflates to +0–4 (the goal pins most shots).
- **Grounding scale** (Phase 7): MCV join ordering (byte-identical
  by construction) and a threshold-routed fixpoint — 2048 i8 goes
  from 74 s enumeration zombies to a 203-step solve inside the
  wall; block-grouping's 4^21 or-goal balloon compiles factored.
  The audit found solved products to 1.62e12, so the route bar rose
  to 1e13 and caldera's pot is forfeited on the record rather than
  risked.
- **The wall is spent on a clock** (Phases 2+5A): time-based
  checkpoints in every rung, a teardown reserve so huge-arena runs
  report instead of dying mid-drop, progress-conditional LAMA slice
  (0.25) + novelty slice (0.30) + per-rung-entry affordability.
  gear-car i6 converts at 57.8 s; sailing-wind-opt's node-cap
  early-exits end honestly (conversion negative recorded);
  tidybot/openstacks casualties clean.
- **The gate batch, adjudicated** (pre-cut): the board-budget
  differential on the final binary read 11 slice-loss regressions
  down to 6 (sprint-resume recovered five); the b-flip proofs landed
  — city-car i8 PROVEN cost 76 (v0.19's certificate, to the digit)
  and tetris i5 PROVEN cost 30. block-grouping i3 and 2048 i9 solve
  in hundredths of a second; org-synth i01 routes and solves. The
  fixpoint route's threshold moved 1e8 → 1e13 after the audit found
  currently-solved products up to 1.62e12 — caldera's pot forfeits
  on the record rather than risking the sokoban-t regression class.

### Where this leaves the standings

**58% coverage across 16 IPC boards** (2,867/4,916), of which **373
are certified optima**. At-a-glance:
[`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md);
per-track detail:
[`benchmarks/ipc-standings.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc-standings.md);
rough field placement per year/track (new this cut):
[`docs/ipc-rankings.md`](https://github.com/hhh42/ferroplan/blob/main/docs/ipc-rankings.md).

On the thirteen boards comparable to 0.21 (same 4,076-instance
denominator), coverage moves **2,153 → 2,248 (+95)** — the floor of
the cycle's own +80–190 ambition band, not the stretch. Three boards
re-entered after being cloud-era/unbaselined for several cycles
(propositional, net-benefit, constraints) and overshot expectations
by a wide margin — net-benefit alone reaches **92%**, this cut's
strongest board. Folded in, the sixteen-board headline lands at the
cycle's *stretch* target (58%), for a different reason than priced:
the re-entries running hot, not the engine phases running hot.

Two boards moved backward and are named rather than netted away:
2014 seq-agile −1 (142→141), 2014 tempo-sat −3 (70→67). Every other
comparable board held or gained; 2014 seq-opt's **+16** (58→74,
+6.2 pts) is the cycle's single biggest mover.

The sweep itself: sixteen boards, one clean pass, zero contended
re-runs — every board's measured conditions verdict reads `clean`.

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (23 earlier releases, 0.1.0–0.21.0).

## [0.21.0] - 2026-08-04 — The numeric cycle, and the ladders that pay their own way

The cycle that took the sailing wall down, closed a temporal debt
carried since 0.18, and repaired the −26 coverage regression the v0.19
backfill exposed in 0.20 — while keeping every win 0.20 had bought.
Full record:
[`docs/roadmap-0.21.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.21.md).

### Where this leaves the standings

**53% coverage across 13 IPC boards** (2,153/4,076), of which **354 are
certified optima** — on the optimal tracks coverage IS proof rate.
At-a-glance: [`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md);
per-track detail: [`benchmarks/ipc-standings.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc-standings.md).

Against 0.19.0 — re-measured on the SAME machine, so the comparison is
engine-to-engine — the twelve comparable boards move **1,943 → 2,132,
+189**:

| board | 0.19 | 0.20 | **0.21** |
|---|---|---|---|
| 2026 numeric | 124 | 121 | **165** |
| seq-opt (08/11) ⚖️ | 235 | 250 | **275** |
| 2023 numeric | 193 | 194 | **229** |
| 2014 seq-agile | 114 | 103 | **142** |
| 2014 seq-sat | 115 | 110 | **138** |
| seq-sat (08/11) | 472 | 473 | **486** |
| 2018 seq-sat | 63 | 53 | **70** |
| 2014 tempo-sat | 65 | 66 | **70** |
| 2023 classical | 30 | 27 | **32** |
| 2023 agile ENTRY (300 s) | 49 | 48 | **51** |
| tempo-sat (08/11) | 419 | 419 | 416 |
| 2014 seq-opt ⚖️ | 64 | 56 | 58 |

Two boards remain behind 0.19 and are not netted away: **tempo-sat −3**
(within the ±4 band re-measurement showed on this box) and **2014
seq-opt −6**, which is entirely `city-car` — the one domain where the
optimal root gate does not recover what 0.20's unconditional
quarter-budget sprint cost. Both are 0.22 work.

**Every board in this release was measured under recorded conditions.**
This box is a laptop, and contention only ever depresses coverage — so
it invents regressions and hides gains. Each board now carries a
`conditions.json` (median idle, load, swap, and the competing processes
by name); a board measured below 65% median idle is refused rather than
banked, and the driver re-measures it at the next quiet window. All 13
boards here are verdict `clean`, 67.8–74.2% median idle. Two apparent
regressions in the first pass (tempo-sat −19, the 300 s entry −3)
turned out to be contention and vanished on clean measurement.

- **The numeric-precondition charge** (Phase 3): extraction now
  charges a selected op's unsatisfied numeric preconditions through
  the existing achiever machinery — sailing-numeric i1 goes from a
  5,000,048-eval cap-out to a 174-step solve at 29,203 evals;
  block-grouping i1 (a 0/20 domain) solves in 24 evals via the new
  one-sided Eq charge. Hatch `FF_NO_NUMPRE`; numeric novelty lands
  opt-in behind `FF_NUMNOV`; temporal groundings deliberately keep
  0.20's heuristic. The capped-search text no longer claims "proven
  unsolvable".
- **The optimal ladder learns the clock** (Phase 4): under an armed
  `FF_TIME_LIMIT`, a root informativeness gate decides whether LM-cut
  earns the remaining wall or h^max keeps the full budget, and the
  h^max sprint is time-boxed (`FF_OPT_SPRINT_FRAC`, default 0.4).
  scanalyzer-08 i4: PROVEN cost 24 inside the wall vs 0.20's 60 s
  kill mid-sprint. No armed wall ⇒ bit-identical to 0.20. Hatch
  `FF_OPT_NO_ROOTGATE`; h-memo on re-opened states kept (−4.6%
  evaluated, expansions identical).
- **The static-fluent fold** (Phase 6): defined-static, irrelevant
  fluents fold to constants and the fluent tables compact out of
  every stored node — data-network i12 drops 3,683 → 209 bytes/node
  (17.6×), tpp i12 24,418 → 4,672 (5.2×) — with plans, eval counts
  and expansion order byte-identical (hatches `FF_NO_FLUENT_FOLD`,
  `FF_NO_FLUENT_COMPACT`). The session `set_fluent` contract is
  pinned with a fixture whose teeth are proven. `FF_MEM_BUDGET_GB`
  tells the engine its byte budget on kernels without a workable
  RLIMIT_AS (macOS), so the retained-state cap trips internally and
  the refill loop spends the wall the RSS watchdog used to eat.
- **The ladder tax** (Phase 5): under an armed budget, EHC and
  novelty-light get wall-denominated slices (`FF_EHC_WALL_FRAC` 0.25,
  `FF_NOVLIGHT_WALL_FRAC` 0.10) instead of op-scaled/fixed-pop
  budgets — the repair for the −26 the v0.19 backfill exposed.
  hiking-2014 i6: 55.5 s (half a second inside the kill line) →
  20.3 s, same plan; openstacks i1 keeps its EHC-direct solve. No
  armed budget ⇒ byte-identical. Hatch `FF_NO_EHC_WALLCAP`; rung
  narration under `FF_WALL_DEBUG`.
- **Temporal emission is sound on the witness** (Phase 7): the two
  same-slot bubble repairs become one per-slot topological order
  with cross-kind guard edges — map-analyzer's three VAL-RED rows
  (the only temporal VAL failures on the twelve boards, 0.20's
  honest negative) go GREEN: solo referee 13/13 VAL-valid.
- **The h-surgery bet dies its pre-registered death** (Phase 8): the
  end-gated interval credit probe landed, priced a snap pair as one
  unit (pinned), and BOTH reads failed — the village stool contract
  still dies at 200k evals, and TMS's best_h floor re-levels
  110→174 without breaking. Fifth negative on this wall; the ledger
  line dies with a sharper localization; the probe stays dormant
  behind `FF_H_ENDGATE`.
- **Harness**: the IPC-2026 -opt pairs get a proof-track board
  (`ipc2026-opt`, cut21-sweeps.sh + promote-air21.sh); multipart
  instance names keep their full identity in the JSONLs; the
  early-exit class is closed (the classifier's timeout line moved to
  the refill loop's 90% re-entry floor).

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (22 earlier releases, 0.1.0–0.20.0).

## [0.20.0] - 2026-08-01 — The guidance cycle, cut on new silicon

The cycle that set out to improve search GUIDANCE — and then had to move
house mid-cut. Phases 1–5 landed on the old cloud container; the cut
itself, and every board in it, was run on an M5 MacBook Air. That
migration is not a footnote: **every scoreboard number in this release
was re-measured from scratch on the new machine**, and none of them may
be read against a 0.19 number. Faster silicon inflates coverage at a
fixed time budget, so a cloud→Air "improvement" would be hardware, not
progress. Full record: [`docs/roadmap-0.20.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.20.md)
and [`docs/roadmap-0.21.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.21.md).

### Where this leaves the standings

**48% coverage across 12 re-baselined IPC boards** (1,917/4,016), of
which **306 are certified optima** — on the optimal tracks coverage IS
proof rate. seq-sat 473/580 (82%), tempo-sat 419/630 (67%), 2023
numeric 194/400, seq-opt 250/550. At-a-glance:
[`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md); per-track detail:
[`benchmarks/ipc-standings.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc-standings.md).

Two boards deserve calling out. **482 of 485 temporal plans validate**
under VAL across the IPC-6/7 boards (419/419 and 473/473 green); the
only three failures are the map-analyzer rows this cycle already
recorded as an honest negative. And the **IPC-2026 numeric corpus gets
its first board — 121/320, with ZERO engine-rejects across 16 domains
the planner had never seen.**

### Spend the whole wall (Phase 1)

- The runner records elapsed wall for UNSOLVED rows, and the standings
  classifier now separates a graceful exit AT an armed `FF_TIME_LIMIT`
  from a true fast reject. The old columns overstated rejects and
  understated timeouts on every budget-armed board.
- **The refill loop**: after ladder exhaustion with >10% of a declared
  wall remaining, the search re-enters GREEDIER (w_h ×4, max_eval ×4,
  at most 6 rounds). An engine holding a time limit should not return
  unsolved with double-digit budget unspent. Hatch: `FF_NO_REFILL=1`.

### LM-cut, and an admissibility bug it uncovered (Phase 2)

- **A 0.19 soundness repair first.** h^max iterated only unconditional
  adds, so a goal reachable only through a `(when ...)` effect was
  labelled unreachable — an OVERestimate, and A* certified wrong optima
  (pinned witness: "PROVEN cost 100" where the optimum is 11). The
  relaxation now runs over an achiever list. The differential says the
  252 shipped 0.19 certificates were not corrupted in practice: the bug
  was real, its bite was not.
- **LM-cut** (Helmert & Domshlak 2009) over the achiever graph, as a
  two-rung ladder — an h^max sprint on a quarter of the node budget,
  then LM-cut on the full one. The PROVEN note names its prover.
  Hatches: `FF_NO_LMCUT`, `FF_NO_HMAX_SPRINT`.
- **Priced honestly, by differential.** 13 certificates carry the LM-cut
  prover label, but re-running exactly those instances with
  `FF_NO_LMCUT=1` on the same box shows four fall to the h^max sprint
  anyway — so LM-cut's UNIQUE contribution is **9 of 306 certificates
  (2.9%)**. No instance is lost by running it (`hatch-only 0`), so the
  two-rung ladder costs nothing. Against the phase's 554-instance
  ambition that is a small pot; it is also real, free and correctly
  wired, which is a different verdict from "does not pay".

### The novelty-LIGHT rung (Phase 3)

visit-all-2014 — the canonical width-2 domain, dispatched in
milliseconds by BFWS-class planners — took 35 s here, and forcing the
existing novelty rung changed nothing. The decode: that rung IS
BFWS-shaped, and spent all 35 s on per-pop `relaxed_helpful` calls a
width-1 structure never needed. So: `novelty::search_light`, IW(1) +
goal count with ZERO heuristic evaluations. **visit-all-2014 i1 35 s →
under 1 s**, and the domain now scores 20/20. Cap priced at 300k pops
(~1 s ladder tax). `FF_NOVLIGHT` / `FF_NO_NOVLIGHT` / `FF_NOVLIGHT_ONLY`.
The cycle also named what it did NOT expect to move — transport,
parking, cave-diving — and all three duly stayed at 0/20.

### Retained-state compression (Phase 4)

The visited structures stored a full StateKey per inserted node,
duplicating what the node arena already held. They are now hash → node
index buckets, with collisions settled exactly against the arena.
Dedup verdicts and expansion order byte-identical. RSS at an identical
forced cap: city-car 133.9 → 113.2 MB (−15%), block-grouping-numeric
169.9 → 124.2 MB (−27%).

### The debt basket (Phase 5)

- **tpp-numeric's early exhaustion, closed**: every probed instance is
  a node-cap trip, not open-list exhaustion — no completeness hole.
- **drone-numeric's 16 VAL-RED rows, attributed to VAL**: its parser
  fails on any drone problem with two objects of the location type,
  before a plan is judged. The runner now records `val: null`
  (validation unavailable), which is not the verdict "plan rejected".
- **The sailing class, named**: sailing / markettrader / pathwaysmetric
  share a genuine numeric-reachability wall (interval/AIBR-class), on
  the record for a numeric cycle. Confirmed on the Air, and again on
  IPC-2026's sailing-wind-sat (0/20) — instances this cycle never saw.
- **An honest negative**: the ε-separation START-vs-provider surgery
  landed a same-slot pin, but the three map-analyzer VAL-RED rows are
  NOT that shape — the repair belongs in the temporal emission layer.
  Carried forward with a sharper decode.

### The MCP server grows a memory (`session_*`, on rmcp)

The library has had a rich `Session` API since the many-minds cycle — fork,
observe, elapse, timed facts, budgeted rethink — and the MCP server exposed
none of it. An agent could ask `solve` a question but could not keep a world
open: every step re-sent the whole domain and paid grounding again. That is now
fixed, and the server moved onto [`rmcp`](https://crates.io/crates/rmcp), the
official MCP Rust SDK, to do it.

- **Ten session tools.** `session_open` grounds a world ONCE and returns a
  handle; then `session_set` (facts / fluents / scheduled timed facts / goal, in
  one call), `session_observe` (returns only the SURPRISES — sightings that
  contradicted belief), `session_elapse`, `session_apply_start`,
  `session_replan` (optionally budgeted), `session_state`, `session_list`,
  `session_close`. The loop is: open once, then *tell it what changed* →
  *rethink*.
- **`session_fork` — the many-minds primitive over the wire.** A fork shares the
  grounded world and owns its beliefs and goal, so two minds can disagree about
  whether they are done. `session_state` reports `world_bytes` (shared, paid
  once) against `mind_bytes` (what one more fork costs) — pinned by a test that
  moves the fork, checks the parent did not move, and asserts both still report
  the same world.
- **On rmcp.** Framing, capability negotiation, tool-schema derivation and the
  error conventions now come from the SDK; tool input schemas are DERIVED from
  the Rust parameter types and cannot drift from the code. This is where the
  `schema` feature below pays off end to end: `solve`'s `options` advertises its
  real knobs instead of an opaque object, pinned by
  `protocol.rs::solve_advertises_a_typed_options_schema`.
- **Behaviour changes worth naming.** The server now enforces the MCP lifecycle
  — `initialize` must precede `tools/call`, per spec, where the hand-rolled loop
  was permissive. Requests are served concurrently and the two expensive calls
  (grounding, search) run off the runtime, so one deep search cannot stall other
  sessions; ordering dependent calls is the client's job, as in any JSON-RPC
  service. And **this crate's MSRV is now 1.88** (rmcp's), overridden locally so
  the LIBRARY keeps the workspace's 1.74 — an MCP server is a tool you run, not
  a dependency you compile into something old.
- The stateless four (`solve` / `parse` / `validate` / `decompose`) answer
  exactly as before, including `solved: false` as a normal answer and tool
  failures as readable `isError` results. 13 protocol/session tests drive the
  real binary over stdio.

### Uptake from downstream (thanks, Sean Chatman)

Two self-contained improvements adopted from
[seanchatmangpt/ferroplan](https://github.com/seanchatmangpt/ferroplan), which
runs this planner as the deterministic core of a Claude Code agent control
plane and pushed hard on the surfaces below. Credit to Sean for both the
patches and the pressure-testing.

- **`schema` cargo feature** (off by default) derives
  `schemars::JsonSchema` on `Options`, `Mode`, and `Search`, so MCP servers
  and other tooling get a *typed* configuration schema instead of an opaque
  `Value`. `schemars` is an optional dep: default builds — and
  `ferroplan-wasm`/`-cli`/`-bevy` — pull nothing new. Defended by
  `tests/api.rs::schema_feature_types_the_options_surface`.
- **Three more wasm bindings** on `WasmSession`: `set_timed_fact` (schedule an
  exogenous flip `dt` from now), plus `world_bytes` / `mind_bytes` for the
  shared-world vs per-fork memory split the bazaar demo wants.

### The move to new hardware, and three bugs it exposed

Porting the harness to macOS/ARM was supposed to be paperwork. It found
three things that would each have ruined a sweep:

- **`RLIMIT_AS` cannot be set on macOS at all.** It reports INFINITY and
  rejects every `setrlimit` with EINVAL. Raised inside a `preexec_fn`,
  that surfaced as a spawn failure — and the runner's retry then booked
  **every instance** as `spawn-fail`. The twelve-board sweep would have
  burned ~5.6 hours producing 4,016 garbage rows that looked like
  environmental fork failures. Now probed once, side-effect-free.
- **The per-job memory cap got a new instrument**: a 0.25 s RSS
  watchdog, since the address-space cap is unavailable. On this path the
  mem-cap column measures RESIDENT bytes — a different instrument
  reading the same column, recorded wherever it is used.
- **The IPC-2026 corpus lost three instances to its own normalizer**: a
  0-indexed `p000.pddl` collapsed to an empty instance number, which the
  runner then died on mid-listing, taking the board with it. Fixed at
  source; the runner now skips un-numbered files loudly rather than
  crashing.

Also: `benchmarks/get-val.sh` builds again (CMake 4.x removed the
pre-3.5 compatibility VAL's CMakeLists declares).

### VAL's other refusal, and 15 instances it was hiding

VAL has more than one way to decline a domain. 0.19 taught the runner
`"Parser failed"`; `data-network-2018` and `factory-robot-2026` instead
say `"Problem in domain definition!"` — and say it against an EMPTY
plan, so VAL never judged our plans at all. Those rows arrived as
`val: false`, and since the standings drop a rejected plan from
coverage, **the standings table read 15 instances lighter than the
boards beside it** (2018-sat 46 vs 53; 2026-numeric 113 vs 121). One
sweep, two artifacts, disagreeing.

`val_check` now tests a list of unavailability signatures, and a VAL
*timeout* returns `null` rather than `false` for the same reason.
[`benchmarks/val-availability.py`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/val-availability.py)
probes every domain and currently names four VAL cannot ingest.

### Release notes you can actually read

The front page had accumulated **sixteen "What's new" blockquotes —
~308 of 684 lines, 45% of the README** — so a visitor met a year of
history before learning what the planner is. The changelog had reached
22 releases and 1,919 lines.

- [`scripts/release-notes-roll.py`](https://github.com/hhh42/ferroplan/blob/main/scripts/release-notes-roll.py)
  keeps `[Unreleased]` plus the newest two releases in both places;
  older changelog sections move verbatim to
  [`CHANGELOG-ARCHIVE.md`](https://github.com/hhh42/ferroplan/blob/main/CHANGELOG-ARCHIVE.md). `publish.sh` reads
  release notes from BOTH files, so archiving never breaks
  `--release-only <old-version>`.
- **[`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md)** is new: every track banded and
  sorted, proof tracks marked, cloud-era boards held separate and
  excluded from the headline. Generated by `standings.py`, which
  patches the README headline in the same run so the shop window cannot
  drift from the boards.
- `benchmarks/standings-history.json` banks per-release numbers, each
  tagged with the BOX it ran on. Improvements are only ever computed
  between snapshots from the same hardware; where no comparable
  predecessor exists the table says "baseline" instead of inventing a
  delta.

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (21 earlier releases, 0.1.0–0.19.0).

## [0.19.0] - 2026-07-31 — The contest cycle

Improve the standings on every entered track and enter the one the
project always fenced off — by direct request (cycle record in
`docs/roadmap-0.19.md`).

### The reject audit (~120 instances back from the front door)

- **Negative number literals** (`(= (d p0) -370)`) now lex; the
  sailing/fo-sailing/fo-counters reject cluster parses and searches.
- **Implicit `(total-cost) = 0`** — the PDDL 3.1 `:action-costs`
  convention: agricola, flashfill, and settlers (60 IPC-2018
  instances that silently returned zero facts) ground and solve.
- **Named verdicts**: an unsolvable-at-grounding result now says WHY
  in `Solution.notes` ("goal fact (X) is unreachable: no surviving
  grounded action adds it").
- Reject columns: 2018-sat **60 → 0**, 2023-numeric **60 → 1**.

### The optimal tracks, entered (`Mode::Optimal`)

- A* + admissible cost-labeled h^max over the same packed task,
  **proof-or-nothing**: a plan is returned only with an optimality
  certificate; caps are inconclusive, exhaustion certifies
  UNSOLVABLE past the delete relaxation. Constant and static-fluent
  action costs; the rest reject by name. `--mode optimal`.
- First entries: **2008 seq-opt 114/270, 2011 seq-opt 90/280, 2014
  seq-opt 48/256 — 252 certified optima**, every plan VAL-green,
  costs cross-checked against the independent cost-sweep oracle and
  literature. The h^max walls (floor-tile, parking, barman) are
  named; classical LM-cut is the recorded next bet.

### The numeric-heuristic swing (+52/−1)

- Linear numeric goals (`(>= (+ (* 2 (x)) (y)) (d))` — the 2023
  numeric track's staple) now get a repetition-counting gradient:
  `linearize` + ⌈gap / combo-delta⌉ charges, running only where the
  old bare-fluent path punted. **2023-numeric 129 → 181 solved
  (valid 113 → 165)**: farmland +17, fo-farmland +17, counters +8.
  One named casualty (tpp-metric-time i4, `FF_NO_NUMH` hatch).

### Ladder, memory, and emission

- **Novelty by default under a budget**: with `FF_TIME_LIMIT`
  declared, the width-1 novelty rung runs by default (0.18's gated
  +4/−0 referee; `FF_NO_NOVELTY` opts out; budget-less behavior
  byte-identical). At the cut this compounded to **+16/−0 on
  2018-sat** (30 → 50 valid over the cycle) and **+11 on the
  580-instance seq-sat flagship** (441 → 452, its first movement in
  three cycles).
- **The node cap can now see the memory limit**: the retained-bytes
  target clamps to 60% of the actual `RLIMIT_AS` — tiny-state
  numeric searches stop dying to the OOM killer before the internal
  cap fires (the numeric board's 105-row mem-cap class, attributed
  to search-state growth, NOT grounding).
- **Emitted-duration reconciliation**: final plans replay and clamp
  state-dependent durations to their domain expressions at emitted
  start times (never half-correcting). The map-analyzer witnesses
  refused the fix and decoded the debt one level deeper (ε-shifted
  starts also precede propositional providers) — named 0.20 work.

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (20 earlier releases, 0.1.0–0.18.0).

## [0.18.0] - 2026-07-29 — The living-village cycle

Correctness debt paid first, then the village made live and visible,
with the budget-aware ladder as the cycle's engine bet (cycle record
in `docs/roadmap-0.18.md`).

### The ε-emission order inversion, fixed (0.17's named debt)

- `epsilon_separate` now repairs SAME-SLOT end groups by invariant
  relation before emission — if one end's deletes hit another's
  invariant-positives, the protected end emits first; cycles defer to
  the existing STN-consistency veto, and zero-slack geometries keep
  the recorded raw-times fallback. Fixture:
  `benchmarks/bench/eps-cross-*` (minimized match-cellar shape) pinned
  as a unit test on the emission pass itself.
- **match-cellar-2014: VAL 0/20 → 20/20** — the whole red cluster
  green, coverage and plans byte-stable. The 630-instance
  2006/2008/2011 tempo board: **zero movement instance-by-instance**.
  2014 tempo-sat standing: valid **42 → 62 of 200**.
- map-analyzer's 3 reds survived and REFUTED the 0.17 hypothesis —
  solo-check decoded them as **state-dependent duration drift**
  (duration expressions read fluents; an ε-shifted start crosses a
  fluent write; VAL fails the duration constraint). Named 0.19 debt
  with witnesses.

### The village, alive (tick loop + screens)

- **`examples/village_live.rs`**: the tick-loop economy over
  `benchmarks/village/` — one authoritative world `Session`, workers
  HIRED by goal contract (fork + restrict + `set_goal`), validity as
  the free suffix replay on a probe fork carrying the worker's own
  contract, dispatch via in-flight durative starts, interval ends
  firing from `elapse`, and a mid-run theft forcing a drift rethink.
  Measured: two workers, three contracts, one theft survived —
  `benchmarks/village-live.md`.
- **`web/village-live.html`**: the same loop LIVE in the browser —
  map, economy sparklines, contracts and visible intentions per
  worker, theft/till disruption buttons — over new `WasmSession`
  verbs (`apply_start`, `elapse`, `set_fluent`/`fluent`,
  `restrict_contains`, `plan_valid_json`).
- **Plan introspection** (`introspect` module + the solver demo's
  "Explain this plan"): causal links (last-achiever replay over the
  solver's own grounding), invariant spans (`over all` conditions
  from the original schema, arguments substituted), preference
  breakdown (final-state goal prefs + verify-oracle trajectory
  prefs).

### A seven-cycle-old corpse, found by the new smoke test

- On wasm32, `NODE_CAP_TARGET_BYTES = 8 << 30` silently wrapped to
  ZERO (32-bit usize; shl drops high bits) — every default-cap wasm
  solve (all of temporal, the classical best-first fallback) had been
  dead since 0.8, invisible behind EHC-solvable demos and the
  explicit budgets of Session thinks. Fixed with a width-guarded
  2 GiB 32-bit ceiling (64-bit byte-identical); the wasm demo's
  temporal examples went unsolved → solved.
  `crates/ferroplan-wasm/smoke.js` (headless-Chromium page smoke) is
  now part of the cut drill.

### The budget-aware ladder (the novelty referee's next idea)

- `FF_TIME_LIMIT=<secs>` tells the engine its REAL wall budget; a
  bounded classical rung (LAMA, novelty) is entered only while more
  than 40% of the budget remains, so late-ladder rungs stop starving
  the complete fallback near the budget edge — the mechanism behind
  the novelty referee's −51. Unset ⇒ byte-identical to 0.17.
  `benchmarks/ipc67.py` passes its per-instance timeout
  automatically; `FF_WALL_DEBUG=1` narrates the gate's verdict.
- **The referee, re-run at the cut** (all eight gate-touched classical
  boards): base boards neutral within noise (the 580-instance flagship
  variant-for-variant identical; every casualty solo-verified as
  contention noise, not gate tax), and **the novelty rung under the
  gate scores +4/−0** where 0.17's ungated verdict was +7/−51 — the
  tax is gone when the budget is declared. `FF_NOVELTY` stays opt-in;
  default-on-under-`FF_TIME_LIMIT` is the recorded 0.19 candidate.

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (19 earlier releases, 0.1.0–0.17.0).

## [0.17.0] - 2026-07-27 — The frontier cycle

The push toward "best PDDL planner in general," measured against the
modern field for the first time — and the abstract village domain
the RPG simulation builds on (cycle record in `docs/roadmap-0.17.md`).

### The landscape, mapped with receipts

- **`docs/landscape-2026.md`**: where planning went after 2011 — the
  IPC 2018/2023 winners run novelty/width search and lifted planning
  over FD-class machinery; NLM-CutPlan's numeric LM-cut family swept
  the 2023 numeric track (its Orbit variant validating ferroplan's
  symmetry direction from the optimal side); the ranked engine-gap
  list with an honest in-this-engine cost per idea.
- **Four corpora fetched, normalized, and scripted** (`get-ipc.sh`):
  IPC-2014 (66 variants), IPC-2018 sat (+official cost bounds),
  IPC-2023 classical (+official reference plans and best-known
  bounds), IPC-2023 numeric (+official result CSVs). Seven new
  runner tracks; 1,820 instances.

### First standings on the modern field

- IPC-2014: seq-sat **95/280**, seq-agile **94/280**, tempo-sat
  **42/200**, seq-mco entered at t4. IPC-2018 sat: **30/240** with
  the first bounds-scored quality column. IPC-2023 classical:
  **26/140** at the 60 s baseline (quality ties best-known on 11 of
  26 solved). IPC-2023 numeric: **112/400** — ferroplan's first
  number on the modern numeric track. All rows generated into
  `benchmarks/ipc-standings.md` with failure classes counted.
- Four VAL-red clusters decoded per-cluster: two VAL-side (VAL's
  parser rejects the drone-numeric and data-network domains), two a
  real engine bug **named to the mechanism** — the ε-emission order
  inversion (same-epoch end pairs, legally reordered internally,
  get their emitted ends crossed by ε-staggered starts; the same
  family as 0.14-ext's ε mutex gaps). Named correctness debt;
  leads 0.18.

### The novelty rung (and its referee)

- Width-1 BFWS-style novelty search ships as an OPT-IN third
  classical rung (`FF_NOVELTY=1`): after EHC and LAMA fail, a
  bounded novelty-first exploration for where the relaxed gradient
  is flat or wrong. The referee A/B across five classical boards
  flipped it off-by-default: **+7 gained, −51 lost** at wall-clock
  budgets — the rung's wall-time tax ahead of the complete fallback
  prices out budget-edge instances (the gen-skip arithmetic,
  repeated). The six h-dies-outright gains are real and reachable
  via the flag; the budget-aware ladder is the recorded 0.18+ idea.
  With the flag off the classical path is byte-identical to 0.16.0,
  so every standing scoreboard carries forward.

### The village (the game's domain, owned here)

- **`benchmarks/village/`**: the abstract crafting-economy core by
  direct request — ONE gather / make / buy / sell rule, every piece
  of content (items, recipes with required-vs-consumed quantities,
  tool/station gates, prices, travel times) as INIT DATA; content
  packs extend catalogs, never rules. Craftsman and workshop rungs
  solve on defaults with tests pinning the forced chains;
  **`examples/village.rs`** demonstrates hiring as a Session goal
  contract (fork → restrict to own labor → `set_goal`; re-hire is
  another `set_goal`).
- The big-catalog stress test (`benchmarks/bench/gen_catalog.py`)
  priced village scale before the village was built — including its
  own correction: grounding holds to 10,000-item catalogs (37 s /
  42 MB); the draft "consumption wall" was a fixture artifact
  (exponential-depth plans), and at honest depth the consumable
  profile solves in 2.65 s at N=3000. Depth is the enemy; width is
  not — baked into the village design.
- Finding for the fence ledger: the village exhibits the
  start-credit plateau in miniature (gather-spam floods the pruned
  pass) — the h-surgery bet now has a game-shaped witness.
- The village live page + plan introspection views severed WHOLE to
  0.18: the page needs the live tick-loop village under it.

## [0.16.0] - 2026-07-25 — The standings cycle

A measurement release: zero engine-code changes, and the largest
standings correction in the project's history (cycle record in
`docs/roadmap-0.16.md`).

### The standings, made honest and scripted

- **`benchmarks/ipc-standings.md`** — one table per competition
  (IPC-5/6/7), every deterministic satisficing track, generated by
  **`benchmarks/standings.py`** from the raw sweep logs (never
  hand-edited; failure classes counted per track: timeout /
  mem-cap / spawn-fail / engine-reject / search). Rendered live as
  the book's new **Standings** chapter; README's Benchmarks section
  reorganized around the three competitions.
- First-ever sweeps of the never-entered tracks: IPC-5
  propositional **354/450** (quality-scored by plan length vs the
  official field: 52W/48T/164L, mean 0.91), time **76/130**,
  metric-time **55/200**, constraints **5/120** (100 instant
  rejects = the four timed modal operators, priced at last); IPC-6
  net-benefit refreshed at full scale **217/270**.
- **IPC-7 sequential multi-core: entered** — t2 **193/280**, t4
  **189/280**, t8 **193/280**, competition wall-clock rules. The t4
  dip decoded to runner fork failures under memory pressure; the
  runner now retries spawns and classifies persistent failures
  `spawn-fail` (environmental).

### The verdict the official archive flipped

- **`benchmarks/IPC5-results.tgz` is vendored** (hand-retrieved; the
  Wayback Machine holds only a 301 for it — this copy doubles as
  preservation) and the qualitative-preferences board is
  reference-grafted from its MetricValue headers, parser
  cross-validated to exact agreement on the simple-preferences
  board.
- The graft's first read — 12W/3T/23L vs SGPlan5 — measured the
  board's stale 0.7/0.8-era ferroplan column and forced a
  re-measurement on today's defaults: **ferroplan beats SGPlan5,
  the IPC-5 qualitative track winner, 24W/4T/10L**, winning rovers
  (7–1), storage (6–2), and tpp (6–1–1) outright. Seven cycles of
  engine work (the 0.5.1 barrier default, 0.6 selection layer, 0.10
  DNF static resolution) had already closed the recorded tpp rout —
  unmeasured until now. p01 regression ceilings re-locked to the
  new metrics.
- Residuals named on the board: tpp p07 (57 vs 49), trucks p04/p06
  quality, and trucks p07/p08 — demoted from wall to budget-bound
  (p07 solves at ~1100 s, metric 6, on pure defaults).

### The gaps, priced

- Every large IPC-6 gap now carries its fence by name in the audit
  record (model-train's numeric last-mile, transport's
  route-structure, sokoban's tie-break lottery, the mem-cap class);
  the one unfenced candidate (crew-planning net-benefit) probed to
  a genuine search wall. The raises route through the 0.17
  frontier-cycle engine bets (`docs/roadmap-0.17.md`, authored this
  cycle: modern-corpus expansion, novelty-rung favorite, the
  village domain).

## [0.15.0] - 2026-07-24 — The seen-and-scheduled cycle

One hard research bet, two capability ships, one platform piece —
every phase traced to a recorded debt or diagnosis (cycle record in
`docs/roadmap-0.15.md`).

### The probe that rewrote the hard bet (and the win it found)

- The kiln-pack fixture family disproved "window packing" as
  machine-shop's wall before anything got built (near-linear evals,
  N=2..12) — doom-pruning already owns window overruns.
- What the TStats probe found instead: 81% of TMS candidates were
  orbit-permutation duplicates, generated then discarded.
  **Generation-side stabilizer skipping** ships as an OPT-IN hatch
  (`FF_ORBIT_GEN=1`): an op is never generated when a state-fixing
  member swap — verified against cross-member facts, fluents, and
  the pending agenda — maps it to an already-generated sibling.
  2.4× real evaluations at equal budget on TMS — and zero new
  solves, while the cut's sweep referee found the per-expansion
  stabilizer scan cost match-cellar 9 solved instances. Default
  OFF; the canonical-key pre-dedup (0.14) stays default-on.
- The wall itself, named to the decimal: the **start-credit
  plateau** — h^FF pays for interval starts on firing while they
  deliver nothing until their ends land; best_h pins at 110 across a
  13× budget ladder, and four ordering schemes measured four
  negatives (`FF_TLIFO`, `FF_TB_FREE_G`, `FF_TAGENDA_W_PRUNE` stay as
  hatches). The named fence: end-gated interval credit inside the
  relaxation — h surgery, a future cycle. The second witnesses have
  DIFFERENT walls, now on file: storage-t is spatial feasibility
  (3,494 invariant-blocked heads at best_h 20), model-train is
  last-mile numeric (best_h 6, nothing blocked).

### Numeric over-all invariants, closed

- The 0.14 transition guard covered propositional conjuncts only;
  the fuel-gap fixture proved the numeric bait (drain past the floor
  mid-interval, refill before the end — both endpoint checks pass,
  VAL-red). Grounded `Comp` conjuncts now ride the InvMap: a happening
  that moves a read fluent re-evaluates the comparison and only an
  actual true→false FLIP blocks — above-floor drains sail through by
  construction. Suite-pinned; watch domains clean.

### Belief and observation (the game capability)

- **`Session::observe(&[(fact, value)]) -> surprises`**: sighted
  facts snap to observed truth (same fences as `set_fact`; a bad
  batch moves nothing), unsighted facts stay believed, the return is
  exactly the news. A mind's session IS a belief state, now formally.
- **Fog in the bazaar** (`bazaar_live`, `claims + fog` rows): truth
  in an authoritative session, per-stall change ledgers, own-stall
  observation each turn + partner-stall on arrival, claims public.
  Measured: bounded fog overhead, +1-tick theft discovery via the
  arrival channel, an unscripted WINNER INVERSION (information
  asymmetry reshuffles outcomes, deterministically), and the named
  pathology — false dormancy under fog — recorded as the next policy
  layer.

### The live in-page bazaar (platform)

- **`WasmSession`**: fork / set_goal / restrict+claims mask / think
  (with internal plan walk) / set_fact / observe / goal_met in the
  browser. `bazaar-live.html` is a LIVE loop now, not a canned
  replay: policy toggles, a mid-run steal button, belief-drift
  badges — headless-verified against the native trace.
- The page flushed a real bug: `Instant::now()` panics on wasm and
  the engine timed itself unconditionally — any solve reaching the
  best-first fallback died in the browser. Fixed lib-wide with a
  wasm-frozen clock shim (native behavior byte-identical) plus a
  panic hook that makes future wasm panics readable.

### Also

- Orbits for tresolve/Session: recorded structural negative (contract
  subgoals and actor masks name members; runtime worlds are invisible
  to lifted detection) — both keep None deliberately.

## [0.14.0] - 2026-07-23 — The living-bazaar cycle: the population runs

0.13 built a population of minds; 0.14 makes them live together — the
tick loop driven end-to-end, contention prevented rather than
survived, and worlds that carry schedules and running intervals
between thinks (cycle record in `docs/roadmap-0.14.md`).

### The tick loop, measured (`bazaar_live`)

- N actor-scoped forked minds, one authoritative world, serial tick
  order — so conflict attribution is EXACT (a break found at a mind's
  turn can only be rival-caused) and the whole simulation replays
  byte-identical at any thread count. Emits the live-loop section of
  `benchmarks/bazaar-thinks.md`.
- **`Session::restrict_ops(keep)`** — the actor-scoping correctness
  primitive: a mind plans only its OWN actions; a rival's moves reach
  it as `set_fact` drift, never as plan steps. Plumbs to the
  `forbidden` masks both engines already carried; replays and
  `replan_following` prefixes reject forbidden steps; forks inherit.
- **`Session::goal_met()`** — the pure state test ("is it done"),
  suite-pinned as distinct from a zero-budget think ("could I still
  plan"); the first loop draft confused the two and silently marked
  an idle mind successful.
- Measured: disjoint goals 4/4 met, zero conflicts, quiescent in 3
  ticks / 0.5 ms; overlapping goals in the one-way barter economy are
  MUTUALLY DESTRUCTIVE (1/4 met — stolen rungs cannot come back), so
  contention needed prevention, not post-hoc replanning.

### Contention, prevented (claims)

- All loop-side policy over the `restrict_ops` primitive: a CLAIM is
  an item a rival's active plan still intends to receive; minds mask
  claimed takes before thinking and WAIT (not dormancy) while blocked.
  On the new jointly-satisfiable crossed-chain fixture
  (`bazaar-chain-x2m`): naive = both goals met but the raided mind
  pays 6 conflicts / 387 evals / churn 12; claims = zero conflicts,
  one think each, 21 evals, churn 0.

### The scheduled world (`set_timed_fact` / `elapse`)

- Clock-RELATIVE world events ("in 5 units the market closes") ride
  into every think as think-relative timed happenings and into
  `plan_still_valid` replays; `elapse(dt)` decays the schedule and
  fires due events, mirrors synced. Plans beat closing windows or
  fail honestly; thinks WAIT through outages with scheduled repairs
  (pending events seed the heuristic session-side; the CLI/corpus
  paths are byte-identical — spot-verified against the temporal
  baseline).
- The static fence held twice over: grounding strips statics from
  runtime preconditions, so scheduling one could not soundly change
  behavior — refused, with the domain contract documented (an
  exogenous-changeable fact must be touched by some domain action).
- Recorded limit: a goal whose enabler exists ONLY via events never
  grounds — an honest construction error.

### In-flight intervals (`apply_start`) — the at-rest fence lifted

- **`Session::apply_start("(fire urn)")`**: the world begins a
  durative action NOW; thinks happen MID-INTERVAL (plans cover what
  remains, never restart the running action, and are valid THROUGH
  every pending end — a think can even be pure waiting: zero steps,
  makespan = the pending end's moment). `elapse` fires due ends with
  their own at-end effects, RETIRING 0.12's mirror-the-end-effects
  idiom; ends broken by drift are reported, effects dropped. Landed
  with ZERO engine changes — a running interval is a root-agenda
  happening, the machinery scheduled events already used.

### The visible bazaar

- The browser demo gains `bazaar-live.html` — a replay of a real
  deterministic tick-loop run (naive vs claims, from
  `bazaar_live --trace`) — and the wants-gated bazaar domain in the
  solver picker.

### Extension: the research phases (7–11, absorbed from the withdrawn 0.15 plan)

- **Temporal soundness, the endpoint gap closed.** `over all`
  invariants were checked at interval endpoints only; a delete +
  re-add BETWEEN them slipped through both checks and failed VAL —
  pinned minimally by `benchmarks/bench/kiln-gap-*.pddl` (a bake
  spanning a scheduled outage of its invariant fact). Every happening
  — starts, classical ops, fired ends, TILs — is now vetted
  diff-wise against all pending intervals' grounded invariants;
  same-epoch ties scan the equal-time agenda for a legal firing
  order; and nodes whose agenda head can never legally fire (its
  unconditional effects break the invariant of an interval outliving
  the head's epoch) are pruned at birth. Numeric invariant conjuncts
  and non-conjunctive shapes remain endpoint-only (recorded limit).
- **Object-symmetry orbits** (`orbits.rs`, `FF_NO_ORBIT=1` reverts):
  when objects or goal PAIRS are interchangeable — identical init
  profiles, symmetric goals, grounded task closed under relabeling
  (verified per template family) — the temporal visited key is
  canonicalized under member permutation. Machine-shop's five pair
  orbits (state-space divisor 8.7×10⁸) collapse: 5,632 vs 13,657
  stored nodes at equal eval budgets, ~3× wall throughput from
  deduping on the canonical key BEFORE paying for the heuristic.
  Sound-semantics coverage save: turn-and-open i1 solves in 15 s
  with orbits, times out without. TMS itself stays 0/20 at 30 s —
  the residual wall is the invariant-blind relaxation, named for a
  future cycle. Probe: `examples/orbit_probe.rs` (+
  `FF_ORBIT_DEBUG`, `FF_TEVAL_BUDGET`).
- **Temporal follow-biased rethinks**: `replan_following` now works
  on the temporal path (happening replay on the ε grid to the first
  inapplicable step, in-flight tail think with the carried agenda,
  prefix + shifted tail) — drift repair keeps commitments in a
  timed world too.
- **The semantic-landmark rung, measured negative with the mechanism
  named** (`FF_RESLM=<w>` hatch, defaults byte-identical): the
  ⌈demand/capacity⌉ trip bound over detected counter resources adds
  no gradient h^FF doesn't already carry — 220 instances per arm,
  identical solve sets everywhere; transport-class walls need
  drive-level route structure, and storage never had the capacity
  shape at all (recorded in `docs/roadmap-0.14.md`).
- **Docs reworked as a platform**: the mdBook gains the Session
  chapter (game-embedding flagship), every 0.9–0.14 knob in the
  tuning table, refreshed scoreboard story, and the bazaar-live
  demo page; `publish.sh` carries all three crates + the wheel
  build; `CLAUDE.md` codifies the working agreements.

## [0.13.0] - 2026-07-21 — The many-minds cycle: one world, a population of planners

0.12 proved one agent thinking in one world; 0.13 closes the distance
between "a session" and "a population" (cycle record in
`docs/roadmap-0.13.md`).

### Retargetable goals (`Session::set_goal`)

- **Swap the goal without regrounding**: any ground conjunction over the
  already-interned fact space — atoms, negated atoms where grounding
  created the `(NOT ...)` mirror, numeric comparisons — via the same
  `(:goal ...)` grammar. Errors BEFORE touching the current goal on
  unknown atoms/fluents (statics are compiled away — a session cannot
  want what its world cannot express), missing mirrors, compiler
  `RUNNING-*` tokens, non-ground terms, and ADL connectives (those
  compile at grounding time). Classical and temporal sessions both;
  `plan_still_valid` answers against the CURRENT goal.
- **The visited key grows with the goal**: a retarget onto a fluent no
  precondition/goal previously read re-runs the relevance closure, so
  state keys get finer, never coarser — replay soundness and t1 ≡ t8
  determinism hold across retargets.
- **Latent mirror bug fixed** (0.11-era): `set_fact` left the
  complementary `(NOT (p ...))` mirror STALE when flipping its base,
  silently mis-evaluating every op with a negative precondition on a
  session-set fact. Mirrors now sync in both directions, and the
  `RUNNING-*` fence looks through `(NOT ...)`.

### Shared world, many sessions (`Session::fork`)

- **N minds cost ONE grounding**: the grounded payload — CSR operator
  columns, names, achiever indexes, the monitor block, the temporal
  compilation, the session's lookup maps — sits behind `Arc` inside
  the same read API, so `PackedTask` clones are Arc bumps plus the
  seven small per-mind state fields (facts, fluents, goal, relevance).
  No search code changed. `fork()` starts from the parent's CURRENT
  state and goal, then diverges freely — no shared tie-breaks, no
  cross-mind writes; forked t1 ≡ t8 suite-pinned.
- Measured (`many_minds` example, vendored bazaar): world load once
  ~1.9 s / ~516 MB transient peak; **12 forks + 12 retargets ≈ 0.0 ms
  and +0.0 MB RSS** (~0.4 KB private state per mind); the old way paid
  ~1.7 s + ~40 MB retained PER MIND. `world_bytes()` / `mind_bytes()`
  give embedders the shared-vs-private split.

### The barter think benchmark (the game track's scoreboard)

- Vendored `bazaar-chain` fixtures (wants-gated barter, 12 holders ×
  40 items): a vendor releases goods only for the item it wants, so a
  depth-k goal forces a k-hop trade-up chain — ≥ k trades in ANY plan.
  `benchmarks/bazaar-thinks.md` is generated by the `bazaar_thinks`
  example: every cell an independent forked mind (`fork` + `set_goal`
  + one bounded think).
- The curves: **solo chains are heuristic-transparent** (k+1 evals,
  sub-ms at any depth — an NPC can chase an 11-hop chain every tick);
  the contended two-chain fixture (invisible to the delete relaxation)
  shows honest budget-exhaustion onset marching 1k → 4k → 16k evals
  with depth, quality optimal at every solving budget, ~460 ms for an
  11-hop contended think.

### Follow, don't dither (`Session::replan_following`)

- A bounded rethink BIASED toward the broken plan: replay the
  still-applicable prefix (zero search), search only the tail — churn
  confined to what drift actually broke. Goal met mid-prefix cuts
  there; no tail found falls back to an unbiased rethink (combined
  eval counts — the bias can cost budget, never completeness).
  Temporal sessions delegate to the plain think (a timed prefix isn't
  at-rest). Measured on scripted drift: deep-hole churn 1 at 3 evals
  vs churn 16 at 2,899 evals unbiased; one probed drift has the
  unbiased rethink exhausting 64k evals where following answers in 6.

### Temporal engine

- **Symmetry reduction in the decision-epoch search** (default on,
  `FF_NO_TSYMM=1` reverts): the pending-interval agenda keeps a
  CANONICAL (time, op) order — same-epoch starts of interchangeable
  intervals used to mint N! visited keys for one pending multiset —
  and a start that changes nothing while its identical end is already
  pending (re-firing a lit kiln, re-baking a baking piece) is skipped
  as a redundant interval copy (sound for plain-STRIPS ends).
  Corpus verdict (630-instance sweep, all solves VAL-green): total
  **387 → 388**, with **match-cellar 6→10** (+4), floor-tile 3→4,
  elevator-numeric 28→29; one named casualty, parking #16, a
  tie-break shift recoverable under the hatch. temporal-machine-shop's
  wall survives both levers (diagnosis sharpened: the residual is
  goal-paired PIECE-subset state symmetry, out of scope for
  agenda-level reduction — recorded in the roadmap).
- `bench_temporal.py` now handles the per-instance-domain IPC layout
  (parc-printer-2011's `domains/domain-N.pddl`); the shared-file
  assumption silently failed every instance of those variants.

### Examples

- `many_minds` (the population story), `bazaar_thinks` (the game
  scoreboard generator), and `game_think` Act 3: a forked trader plans
  a 3-hop chain, drift makes the desire impossible (honest unsolved
  verdict), and the NPC settles for the reachable rung via `set_goal`.

## [0.12.0] - 2026-07-20 — The game cycle: temporal thinks, drift-stable plans, fixpoint grounding

The release where the engine starts serving its actual customer — the
game from STATUS.md's recorded design answers (cycle record in
`docs/roadmap-0.12.md`). The corpus was the measuring stick; this
cycle the stick did its job.

### The temporal Session (genuine concurrency, thought about in bounded time)

- **`Session` accepts durative domains**: snap-compile + stratified
  grounding ONCE, then every `replan`/`replan_budgeted` runs the
  bounded decision-epoch ladder from the current AT-REST world state
  and returns a timed plan (per-step time/duration, makespan, real
  evaluated-state counts). The world between thinks carries no running
  intervals: `set_fact` fences the compiler's `RUNNING-*` tokens, and
  TILs are rejected at construction (they pin the absolute clock;
  thinks are clock-relative). The demand tier is read once at
  construction for stable per-session behavior.
- **The temporal path gains an EVAL BUDGET** (it had only node caps):
  a think's budget now spans the WHOLE pass ladder, charged serially
  per evaluation batch — deterministic at any thread count — and the
  memory target plumbs to the temporal node cap. A budget-exhausted
  think returns `solved: false` honestly. Duration tables rebuild per
  think, so `set_fluent` on an op-unmodified fluent flows into
  parameter-dependent durations instead of staying frozen.
- Suite-enforced: overlapping-interval concurrency, drift replans,
  tiny-budget honesty, t1 ≡ t8 timed-step determinism, both fences.

### Follow before you rethink

- **`Session::plan_still_valid(plan, from_step)`**: replay the
  remaining suffix against the current world state (classical
  op-by-op; temporal via the validator's happening replay), ending in
  the goal test. Exact, zero search — irrelevant or even helpful
  drift keeps the agent following its plan for FREE; only a broken
  suffix spends a think. The scripted-drift test pins the contract:
  follow, helpful drift, breaking drift — exactly two thinks
  end-to-end.

### Fixpoint grounding (the fixtures chose the design)

- **Reached-restricted fixpoint grounding** (`ground_fixpoint`, the
  `Session`'s temporal grounding entry; `FF_NO_FIXPOINT_GROUND` falls
  back to stratified): every action joins its positive dynamic
  literals against the atoms reached so far, rounds to fixpoint —
  enumeration tracks the REACHABLE op set instead of the typed
  product, subsuming the 0.10 stratification. elevator-11 p04,
  same-binary A/B: **31.6 s / 5.7 GB → 6.9 s / 48.8 MB (~117× less
  transient), identical task dims**. The surviving op set is
  identical, but doomed candidates are never enumerated, so fact-id
  first-reference order shifts and search tie-breaks move with it —
  the corpus A/B measured that as real sokoban-t coverage (stratified
  4/10 vs fixpoint 1/10 on i1–i10), so the CORPUS solve paths stay on
  stratified grounding and only the `Session` (the game track, where
  the memory win is the point and no scoreboard baseline is
  disturbed) grounds via fixpoint. The residual elevator-11 tail is
  SEARCH-bound (p05 solves solo at 49 s under fixpoint — formerly a
  grounding OOM), joining the recorded guidance family.
- **The bazaar fixtures** (vendored: the game's any-for-any trade
  economy): measured DENSE-reachable — 197k of 211k typed candidates
  are real ops, so no grounder can shrink it; ground-once makes it a
  5.5 s / 644 MB world-load cost and thinks stay pure search.
  Classified and recorded.

### Corpus debts

- parc-printer-t diagnosed (complete-pass start-spam, ~2,076 pending
  intervals per node — the TMS family; the cheap ordering experiment
  measured negative, `FF_TAGENDA_W` stays opt-in).
- `ipc67.py --score-against PRIOR.jsonl`: self-relative IPC-formula
  quality scoring for regression tracking (explicitly NOT an official
  IPC score — the corpus carries no reference costs).
- turn-and-open at realistic budgets: 1/20 at 120 s, val-green —
  search-bound as classified.

## [0.11.0] - 2026-07-20 — The guidance cycle: three honest negatives and the think API

The cycle that attacked the one wall class 0.10 left standing — the
heuristic — and reported what it found (cycle record in
`docs/roadmap-0.11.md`). Three principled guidance transfers were
implemented, measured at the scoreboard baselines, and concluded
NEGATIVE; each ships opt-in with its diagnosis recorded, and the
conclusion is itself the cycle's finding: **the remaining walls
(transport, storage-t, TMS, model-train, floor-tile) need a genuinely
different heuristic — red-black or semantic landmarks over numeric
structure — not reweightings of what exists.** Alongside, the
game-embedding **budgeted-think API** ships, and its determinism test
caught and fixed a real budget leak.

### The budgeted-think API (the game track)

- **`Session::replan_budgeted(max_evaluated, memory_mb)`**: a think is
  a BOUNDED call on a ground-once `Session` — eval budget (the
  deterministic unit, never wall clock) plus a retained-memory target
  (`SearchCfg.node_bytes_target` through the per-node byte model). A
  budget-exhausted think returns `solved: false` honestly; identical
  budgets give identical plans at any thread count (suite-enforced).
- **EHC budget-leak fix**: the determinism test caught EHC's internal
  op-scaled cap ignoring `max_eval` — a 1-eval think solved anyway.
  The caller's eval budget now bounds EHC too.
- **`examples/game_think.rs`**: the episodic walkthrough — think →
  follow → world drifts → rethink → honest tiny-budget verdict.

### The guidance experiments (all opt-in, all defaults bit-identical)

- **Temporal LAMA rung** (`FF_TLAMA=1`): fact-landmark bitsets +
  landmark-dominant key as a bounded pass in the temporal ladder.
  Three shapes measured (key-term / unbounded rung / 50k-bounded
  rung); none positive. Diagnosis: snap tasks' landmarks are
  RUNNING-token chains that accept in path order regardless of
  choices — no branching signal, unlike the classical barman
  landmarks that made the 0.9 rung win.
- **Lax helpful fallback** (`FF_LAX_HELPFUL=1`): the temporal
  Start-filter DOES empty nonempty helpful sets on END-led relaxed
  plans (storage's stored sets averaged 0.0 — mechanism confirmed),
  but repairing it RESTRICTS expansion exactly where the empty set
  previously meant a recovering full scan: zero new solves.
- **Classical landmark-count term** (`FF_CLM=<w>`): transport solve
  sets identical, visit-all untouched (EHC path), floor-tile worse.
- **Measurement honesty**: an A/B against the rebuilt 0.10 scoreboard
  binary proved same-day sokoban-t deltas ENVIRONMENTAL (the box ran
  ~40% slower than the scoreboard day; the borderline band flips with
  it) — wall-clock scoreboards inherit box variance; the eval-count
  budgets the engine uses internally do not.

Default-path behavior is unchanged from 0.10.0 (all experiments
hatched off; the EHC budget bound only binds when a caller sets an
eval budget below EHC's op-scaled cap), so the 0.10.0 scoreboards
remain current.

## [0.10.0] - 2026-07-19 — The walls fall where they can: grounder truth, temporal recoveries

The frontier cycle (cycle record in `docs/roadmap-0.10.md`; per-item
records in `STATUS.md`). Every remaining next-cycle agenda item is now
either SHIPPED with measured coverage or ANSWERED with a recorded
diagnosis — no wall is left unclassified.

### Grounder: the fact space tells the truth

- **Fact-space compaction**: Phase C interned atoms from every raw
  candidate op; reachability pruned the ops but their fact ids sized
  every state bitset. On temporal snap tasks the gap was catastrophic —
  elevator-08-t p22 minted **2.35 M facts for ~7 k live** (287 KB per
  state, 8 GB RSS, dead at any budget). Facts now compact to the
  reached/referenced/goal set after reachability with a monotone,
  order-preserving renumber (`FF_NO_FACT_COMPACT`); classical tasks are
  bit-identical (their raw references survive).
- **Stratified Phase B grounding** (temporal path): snap END actions
  ground join-restricted to the atoms their STARTs actually produce,
  through the existing static-literal pruning — the 470 k raw END
  candidates of elevator-08-t p22 are never enumerated
  (`FF_NO_STRAT_GROUND`). p22: unsolvable-at-any-budget → **~26 s**,
  transient 8.0 → 4.1 GB.
- **DNF static resolution** (`FF_NO_DNF_STATIC`): fully-bound
  never-added literals resolve against init during expansion and a True
  disjunct absorbs its disjunction — killing the 2^k conjunct explosion
  of `forall (imply (static …) (dynamic …))` preconditions. Folding a
  literal AWAY additionally requires never-DELETED (delete-only phase
  facts like `TRAJ-PLANNING` keep gating — the constraints suite
  enforced the asymmetry). **openstacks-ADL 6/30 → 30/30; the temporal
  twins swept 30/30 + 30/30 (+71 instances from one fix).**
  Instance-7 previously died at 15 GB mid-grounding.

### Temporal: recoveries and honest classifications

- **Byte-aware temporal node cap** (`temporal_node_cap`, the classical
  model + agenda/key extras; `FF_TEMPORAL_NODE_CAP`) replaces the
  byte-blind 400 k count.
- **Shift-invariant visited keys**: on TIL-free tasks the agenda keys
  by pending-end DELTAS, not absolute times — retimed permutations of
  one logical state finally dedup (`FF_TEMPORAL_ABS_KEY`). A minimal
  turn-and-open repro (now a suite test) proved same-epoch chaining
  already handles start-inside-an-interval concurrency — **no
  semantics gap**. Coverage at the 30 s baseline: **sokoban08-t
  7→10/30, sokoban11-t 0→2/20, floor-tile11-t 0→3/20; turn-and-open
  0→1/20 at 60 s** — all VAL-validated. elevator-08-t 19→22/30 and
  elevator-11-t 0→3/20 (grounding fixes above).
- **PDDL2.1 `?duration` in expressions + state-dependent durations**:
  the parser accepts `?duration` in expression position (reserved
  pseudo-fluent), the snap compiler substitutes the duration expression
  (exact at START; end-side only under full inertia, else the action is
  skipped — never compiled wrong), and durations reading assigned
  fluents resolve per expansion against the node's state (side table on
  `Kind::Start`; validator checks bounds at the start happening).
  Proven end-to-end by the `durexpr` fixture. model-train now parses,
  grounds, and searches — still 0 solved: its wall moved to guidance.
- **Walls classified, recorded**: storage11 explored 3 M nodes with a
  live 2.2 M heap — no exhaustion, no semantics gap, a pure
  h^FF-guidance wall (same family as transport11/model-train, whose
  attribution — h is the delete-relaxation floor, 3.6× thread scaling,
  guidance not throughput binds — was measured this cycle);
  temporal-machine-shop drowns in genuine-concurrency interleavings
  (~47 pending ends per node).

### Scheduling, validation, quality

- **Budget-aware portfolio**: the ladder runs to its natural end on the
  FULL eval pool before diversification spends anything — portfolio
  coverage ≥ default by construction (`FF_PORTFOLIO_SLICED` restores
  doubling). All five recorded losses recovered to exact default parity
  and the no-mystery diversification win kept (~428 ≥ 427 ≥ old 416).
- **Temporal VAL in the runner**: `ipc67.py` validates tempo-sat plans
  (timestamped rendering, `-t` at ff's 0.001 ε) and immediately caught
  a real bug — same-instant numeric write-write passed the fact-only
  mutex test. `epsilon_separate` now counts numeric footprints
  (write-write + write-read) and separates up to 2000 happenings.
  elevator-numeric val 1/3 → 3/3; every sweep this cycle is val-green.
- **Per-job memory cap** (`--mem-gb`, default RAM/jobs): a memory spike
  kills its own job with a `mem-cap` note instead of inviting the OOM
  killer to execute siblings.
- **Length-anytime within one search** (`FF_LEN_ANYTIME=1`, measured
  and default-OFF): the drain cost 9 instances of coverage at the 60 s
  budget against 4 shorter sokoban plans and zero gains on
  floor-tile/visit-all — recorded negative, same verdict class as
  0.9's improve_length.

## [0.9.0] - 2026-07-18 — The IPC6/IPC7 arc opens: costs, benefit, landmarks, portfolio

The general-planning cycle (`ferroplan-roadmap.md`; cycle record in
`docs/roadmap-0.9.md`). ferroplan learns the IPC-2008/2011 satisficing
objectives — real action costs, net benefit — grows a LAMA-style landmark
rung on BOTH execution paths, fixes two grounder walls that made whole
domains unsolvable, and gains a sequential portfolio mode. Vendored
costs-subset scoreboard: **35/54 (0.8.0) → 54/54 at a 240 s library-path
budget** (49/54 at the quick 30 s single-thread tier), with every
reported cost VAL-validated where VAL is available; net-benefit subset
**16/16 with the benefit reported everywhere** (was: empty plans, no
metric); barman11 and tidybot11 — never solved before this cycle — go
**0/4 → 4/4 each**. The IPC5 preference/qualitative baselines held green
throughout (19 heavy guards).

### Added

- **Costs × preferences composition verified** (roadmap Phase 5,
  `tests/costs_prefs.rs`): action costs and weighted `is-violated`
  terms share ONE metric evaluation — the satisfy-vs-forgo decision
  flips exactly at the weight boundary, nothing double-counts, and a
  hard `always` monitor stays enforced while the combined metric is
  optimized.

- **LAMA-style satisficing rung** (roadmap Phase 3, `landmarks.rs` +
  `lama.rs`): fact landmarks by first-achiever backchaining over the
  relaxed planning graph (sound, memory-light — no quadratic landmark
  table), counted PATH-DEPENDENTLY (per-node accepted-bitset) as a
  second heuristic beside FF, with **preferred-operator boosting** via a
  dual open list (helpful-action successors ride a favored heap; LAMA's
  core recipe). Runs as a BOUNDED middle rung — after EHC gives up,
  before the complete weighted fallback — so it can only add coverage;
  `FF_NO_LAMA=1` restores the two-rung ladder and explicit
  `--search bfs` never enters it. Same determinism contract as the main
  engine (fixed batches, order-preserving parallel h, serial insertion).
  Recorded: **barman11 p01 solves for the first time at any tested
  budget** (105 steps, cost 258); parking11 p01 and floortile11 p01 drop
  from >130 s / >10 s to seconds on the library path.
- **IPC6 net-benefit / oversubscription planning** (roadmap Phase 4):
  `maximize` metrics NORMALIZE onto the existing PDDL3 minimize B&B
  (extracted at scale −1; the dropped affine constant rides
  `Compiled::metric_konst` so the reported metric is the original
  net-benefit value — `maximize (- 70 X)` optimizes `minimize X` and
  reports `70 − X`). `cost_monotone` now accepts increases by provably
  non-negative STATIC expressions (sums/products/quotients of
  non-negative constants and static fluents — elevators'
  `(travel-fast ?f1 ?f2)`, crew-planning's
  `(* (/ (payloadact_length ?pa) 10) (+ (crew_efficiency ?c ?d) ...))`)
  instead of constants only, which previously bounced three of the four
  vendored net-benefit domains to an empty-plan fallback. The empty plan
  stays a legal candidate (utilities that don't pay are forgone — the
  oversubscription semantics). Vendored netben subset: **16/16 solved,
  all VAL-validated, net benefit reported on every instance** (crew08
  nets 1988–2160 of its 3335 ceiling). Pure-cost problems WITHOUT
  preferences now route to the classical `costs.rs` path on the text
  planner too, matching the library API's routing.
- **IPC6 `:action-costs` on the classical path** (roadmap Phase 2,
  `costs.rs`): `(:metric minimize <fluent>)` is detected, the plan's cost
  is REPLAYED (never estimated) and reported as the metric, and an
  **anytime cost-improvement sweep** — bounded branch-and-bound ordered by
  accumulated cost, guided by the cost-augmented relaxed plan
  (`relaxed_costed`: selected-op cost + length, so zero-cost regions keep
  a gradient) — trades plan length for cost after the untouched EHC /
  best-first machinery finds its first plan. Recorded: elevators08 p01
  cost 100 → 54; the sweep's budget stays proportionate to the solve
  (`FF_COST_SWEEP_EVALS` overrides, 0 disables). `--satisfice` reports
  the cost without sweeping; maximize / compound metrics are never
  silently claimed. Uncapped sweep exhaustion reports **proven optimal**.
  The PDDL3 text path's unsupported-metric fallback (e.g. fluent-valued
  cost increases, which fail its constant-only `cost_monotone` check) now
  routes through this path instead of dropping the metric.
- **Phase 0 scaffolding** (roadmap): vendored IPC-2008/2011
  action-costs + net-benefit benchmark subsets (`benchmarks/ipc/costs/`,
  `benchmarks/ipc/netben/`), external **VAL validation of every solved
  plan** in `benchmarks/run.py` (`FERROPLAN_VAL`; exit 1 on any failure),
  a full-corpus IPC6/7 runner (`benchmarks/ipc67.py` + `get-ipc.sh`,
  `get-val.sh`), and `STATUS.md` as the roadmap's living source of truth.
- **Sequential portfolio mode** (roadmap Phase 6, `portfolio.rs`;
  `Mode::Portfolio` / `--mode portfolio`): four complementary classical
  members — the default ladder, the LAMA rung alone, plain best-first at
  w_h 3 and 1 — time-sliced over ONE shared evaluated-state pool with
  doubling restart slices; deterministic by construction (fixed member
  order, eval-count slices). Coverage-first: the first plan any member
  finds returns with the winner named in `Solution.notes`, and a complete
  member's un-capped exhaustion settles unsolvability early. Measured at
  the 30 s single-thread tier: EXACT coverage parity with the default —
  49/54 with the identical unsolved set — meeting the acceptance
  criterion's "at least as good as the best single configuration"; the
  "better on some domains" half is NOT yet demonstrated on the vendored
  subset (after this cycle's frontier fixes, no curated instance is left
  where the default faceplants and another member wins — the full-corpus
  `ipc67.py` run is the recorded venue for that half). Temporal and
  preference problems fall back to their own machinery, exactly like
  `auto`.
- Web demo: three examples from the new suites join the picker —
  elevators08 under BOTH objectives (action costs: the sweep takes cost
  100 → 54; net benefit: soft goals with utilities, the empty plan
  legal) and barman11 p01 (the landmark-rung story, marked hard for
  single-threaded WASM).

### Fixed

- **A type-cycle hang on legal PDDL**: a domain that redeclares the
  built-in root type (`(:types ... object ...)` — IPC-2011 tidybot does)
  recorded the self-edge `OBJECT → OBJECT`, and every type-hierarchy
  walk spun forever BEFORE grounding — the planner never got to work at
  any budget. Cyclic `(:types ...)` is now rejected BY NAME; the walks
  are hop-bounded as defense in depth (programmatically built domains).
- **Join-style grounding**: the binding enumeration checks each static
  precondition literal at the FIRST level where its variables are bound,
  pruning whole subtrees, instead of enumerating the full cartesian
  product and post-filtering — tidybot11's 9-parameter grid actions
  ground 91.6 s → 2.8 s with a byte-identical grounded task (the
  surviving binding order is unchanged by construction). tidybot11 goes
  **0/4 → 4/4** (11 s / 124 s / 6 s / 6 s at 4 threads, every plan
  oracle-replayed to goal).

### Changed

- **The text path runs the library's ladder where it matters** (the two
  recorded unification gaps, both closed): the partition cascade's
  MONOLITHIC endpoint uses the full EHC → LAMA → complete best-first
  ladder, and its per-subgoal solves became bounded probes (100k evals)
  with a per-subgoal LAMA rung (`landmarks_for` / `lama::search_subgoal`
  recompute landmarks per (start, subgoal) pair). A subgoal unsolvable in
  isolation used to burn the FULL eval budget proving it before every
  merge; bounded probes only make merges happen sooner, and solvability
  is unchanged by construction. barman11 p01 on the text path:
  never-finishes at any tested budget → **57 s**.
- **Iterated-weight length improvement ships OPT-IN as a measured
  negative** (`FF_LEN_SWEEP_EVALS`, unset/0 = off — byte-identical
  first-found behavior): the restart ladder over the new
  `SearchCfg::g_bound` incumbent-length pruning is sound and
  deterministic but pays ~1.8% (visitall p01: 226 → 222) at ~28× the
  solve's evals — below the polish doctrine's price. The recorded next
  ideas: a within-one-search length-anytime, or landmark-guided
  restarts.

## [0.8.0] - 2026-07-18 — Pay the Costs: linear goals, shared monitors, ESPC on structure

0.7 moved the fence and wrote down the bill: goals exponential in the
monitor count, a monitor tax multiplied across every ground action, and a
penalty pass that drowned in the states it widened. 0.8 pays those costs
(`docs/roadmap-0.8.md`, Phases 1–3). The monitor compilation is now LINEAR
where it was exponential — hard-constraint acceptance rides one
forced-terminal action instead of a goal-DNF product (storage hard fixture:
59,969 ops → 921) — and SHARED where it was multiplied: the transition
block grounds once instead of per ground op, which erases both recorded
15 GB grounding OOMs outright (storage qualitative p07: 313 ms / 109 MB;
p08: 676 ms / 174 MB) and gives the suite's last two uncovered storage
instances their first-ever metrics, reported == verified exact. ESPC now
engages on real once-only achievement structure instead of monitor
artifacts, so the storage tail runs on PURE DEFAULTS — the scoreboard's
two documented `FF_NO_ESPC=1` rows lose their env footnote, and coverage
rises from 36/40 to 38/40 with every remaining gap still named. Every
change keeps a restore hatch and the constraint-free path stays
byte-identical.

### Added

- **The END construction** (0.8 Phase 1): hard trajectory monitors' S_n
  acceptance moves off the goal onto one synthetic forced-terminal
  `TRAJ-END` action — every real action requires the init-true
  `TRAJ-PLANNING` phase fact; `TRAJ-END` deletes it, adds `TRAJ-ENDED`,
  and latches one `TRAJ{i}-ACC` fact per hard monitor via a conditional
  effect whose condition reads exactly S_n (the observation-offset
  contract's third leg, relocated intact). The compiled goal becomes all
  positive literals, so the grounder's exponential goal-DNF product
  (one synthetic REACH-GOAL op per DNF disjunct — 3^10 = 59,049 on the
  recorded storage fixture) never fires: 59,969 ops → 921, grounding
  2.16 s → 0.77 s, conditional effects up only by the linear ACC latches
  (+30). Soft acceptance deliberately does NOT move — preference wrappers
  keep their S_n bodies in the goal, the entire PDDL3 metric stack prices
  them unchanged, and the metric locks held byte-identical (the exact
  interaction the 0.7 deferral feared never materializes). The synthetic
  step is stripped from every reporting surface only when the constraint
  gate compiled — the constraint-free path never changes — and the
  reserved-name fence grows `TRAJ{n}-ACC`, `TRAJ-PLANNING`, `TRAJ-ENDED`,
  and the `TRAJ-END` action name. `FF_NO_TRAJ_END=1` restores the 0.7
  goal-side acceptance byte-for-byte.
- **The shared monitor block** (0.8 Phase 2): monitor transitions are
  fully ground and byte-identical for every binding of every action, yet
  0.7 stored them per ground op in four simultaneously-resident copies —
  the monitor-count × ground-action product that OOM'd storage
  qualitative p07/p08 during grounding on a 15 GB box. The transitions
  now travel as `Domain.monitors` plus a per-`Action` `monitored` flag,
  ground and intern ONCE, and every consumer iterates them through
  `PackedTask::cond_effs` in the exact 0.7 suffix order — apply, the
  relaxed heuristic, reachability, inertia, achiever buckets, and the
  temporal/session scans see identical effective semantics. Measured:
  p07 grounds in 313 ms at 109 MB peak (2.1M effective conditional
  effects now virtual: 1,291 shared entries + one bit per op), p08 in
  676 ms at 174 MB; the 10-monitor hard fixture grounds with ZERO
  overhead (78 ms vs 78 ms unconstrained). First-ever metrics follow:
  **p07 = 200, p08 = 261, both reported == verified exact** on the
  independent trajectory oracle. `FF_NO_COND_SHARE=1` restores the
  per-action append.
- **A deterministic search memory backstop** (0.8 Phase 3): `search_from`
  gains an insertion cap alongside `max_eval` — the retained `nodes` and
  `visited` stores grow one full-state entry per INSERTED successor while
  the eval budget counts only popped nodes, the exact geometry of the
  recorded exit-137s. The cap derives from a documented 8 GiB byte target
  over static task dimensions (never RSS, never wall clock; the count is
  serial, so t1 ≡ t8 by construction), returns the anytime incumbent or
  an honest `capped` verdict, and sits far above every green fixture's
  retained size. `FF_SEARCH_NODE_CAP` overrides it (`0` disables).
- Measurement probes `examples/ground_probe.rs` (gate + ground + peak
  RSS) and `examples/verify_plan.rs` (independent-oracle replay of a plan
  file), plus new heavy locks: the grounding fixtures now LOCK the
  one-extra-op shape, and storage qualitative p05 is locked on PURE
  DEFAULTS at its `FF_NO_ESPC` metric (47), reported == verified.

### Changed

- **ESPC engages on structure, not artifacts** (0.8 Phase 3): the
  deadline-pair detection no longer scans the shared monitor block, whose
  conditional adds are trajectory-monitor bits riding every action —
  pairing them made ESPC engage on monitor-compiled tasks and then OOM
  its monolithic tightening pass on the monitor-widened states
  (dmesg-confirmed ~16 GB inside one pass, below every eval budget).
  Monitor-artifact-only tasks now fall through to the closure optimizer —
  exactly the behavior the 0.7 scoreboard documented per-row as
  `FF_NO_ESPC=1` — while real deliverables (openstacks' per-op
  conditional adds) keep their pairs untouched; the simple-preferences
  ESPC locks hold byte-identical. Storage qualitative p05–p08 all
  complete on pure defaults. `FF_ESPC_TRAJ_PAIRS=1` restores the 0.7
  monitor-artifact pairing.
- The 0.7 roadmap's gated stretch phases — constraint-aware search
  guidance, constraints on the temporal path, temporal selection — did
  not ship in 0.8.0 and carry forward as the 0.9 agenda
  (`docs/roadmap-0.8.md` Phases 4–5, unchanged gates). The timed
  operators, the temporal path, and `Session` keep their named
  rejections.

## [0.7.0] - 2026-07-17 — Trajectories: enforce the constraint, price the preference

The release that retires the project's oldest fence. Since 0.4.1 every PDDL3
`(:constraints ...)` block was cleanly rejected; 0.7 compiles the six untimed
modal operators into monitor automata and ENFORCES them on the classical
path — hard constraints as goal conjuncts, soft `(preference name ...)`
constraints priced through the existing metric stack with zero optimizer
changes — and vendors the IPC-5 *qualitative-preferences* track (5 domains ×
8 instances) as the measured proof: 36 of 40 instances produce a plan and a
metric, the independent verifier reproduced the metric EXACTLY on every one
of the 11 instances it was run against (all five p01s plus six larger spot
checks), and every gap has a named reason on the new scoreboard. The verifier itself came out stronger:
it now grounds quantified preference bodies, making it authoritative on the
qualitative suite and on 5 of 6 simple-preferences domains. What 0.7 does
not enforce still rejects BY NAME (timed operators, the temporal path,
`Session`), with `FF_CONSTRAINTS_REJECT=1` restoring the old blanket
rejection outright. Constraint-free inputs are untouched.

### Added

- **PDDL3 trajectory constraints — the hard untimed operators are now
  ENFORCED on the classical path** (0.7 Phase 1, `docs/roadmap-0.7.md`).
  `always`, `sometime`, `at-most-once`, `sometime-after`, `sometime-before`,
  and `at end` compile into monitor automata (new `constraints` module):
  0-ary monitor facts plus conditional-effect transitions on every action,
  with the goal conjoined on the automaton's accepting condition. `forall`
  constraints expand at the constraint level (bodies stay ground); the init
  state S_0 counts for the trajectory (evaluated at compile time), and
  `sometime-before` is strict ("strictly earlier"). Wired at every gate:
  `solve`, `decompose`, `run_planner`, `run_ff`.
- **Independent trajectory oracle**: `verify::verify` folds the ORIGINAL
  constraint semantics over its replay (never the compiled monitors) —
  `Verified` gains `constraints_met` + `constraint_failures`, and
  `plan::validate_plan` now requires `constraints_met` for `Valid`: a plan
  that reaches the goal but breaks a constraint is `Invalid`, with the
  violated operators named.
- Heavy `#[ignore]` grounding-cost fixtures (hard overlays on vendored
  IPC-5 storage/trucks instances). Recorded (release build): trucks p03,
  3 monitors — 1,065→1,083 ops, +12,780 conditional effects, ground
  8→~50 ms; storage p05, 10 monitors — 920→59,969 ops, +36,800 conditional
  effects, ground ~80 ms→~1.2 s. The storage blow-up quantifies the
  roadmap's predicted goal-DNF risk: the monitors' end-state acceptance
  checks make the compiled goal's DNF exponential in the monitor count
  (3^10 = 59,049 synthetic REACH-GOAL disjunct ops, exactly), with the
  END-action construction recorded as the known fix if real workloads bite
  (`docs/roadmap-0.7.md`). Constraint-free inputs are untouched (the gate
  is a no-op), so this cost is opt-in with the feature.

- **Soft constraint-preferences are ENFORCED and PRICED** (0.7 Phase 2).
  `(preference name <constraint>)` inside `(:constraints ...)` lowers to the
  same monitor automata plus a goal-side `(preference name <acceptance>)`,
  so the whole PDDL3 metric stack (Keyder–Geffner collect/forgo pricing, the
  exact-closure optimizer, the selection layer) scores trajectory
  preferences with **no optimizer changes**. The PDDL3 instance boundary is
  honored exactly: `forall` OUTSIDE a preference multiplies instances
  sharing the name (`(is-violated name)` counts violated instances), while
  `and`/`forall` INSIDE the preference body stay ONE instance, violated at
  most once (adversarial review caught the initial per-member split — the
  verifier shared the expansion, so only a semantics-level review could).
  Anonymous preferences get deterministic `TRAJPREF{n}` names, with the
  generated monitor/name namespace guarded: a user predicate or preference
  inside it (e.g. `traj0-viol`, which could silently clear a hard
  violation) is rejected by name. Weight defaults match goal preferences
  exactly (no metric → 1 each, metric-unreferenced → 0), pinned by tests.
  `run_ff` closes `:derived` axioms before the constraint gate whenever a
  `(:constraints ...)` block is present (its constraint-free classic
  pipeline is untouched).
- **The IPC-5 qualitative-preferences suite is vendored and scored**
  (`benchmarks/ipc/qualpref/{openstacks,rovers,storage,tpp,trucks}`, 8
  instances each, from the potassco mirror — the track ran 5 domains; there
  is no qualitative pathways). All 40 instances parse, gate, and compile
  with no rejection; **36 of 40 produce a plan + metric** (31 on pure
  defaults within 300 s, 3 more — openstacks p07/p08 and trucks p06 —
  within 600 s, and storage p05/p06 under a documented `FF_NO_ESPC=1`
  env), reported == verified held exactly on all 11 oracle-checked
  instances, and every gap has a named reason on the board — storage p07/p08 exceed 15 GB during
  grounding, trucks p07/p08 exceed the search budget. See
  `benchmarks/ipc5-qualitative-scoreboard.md` (self-scored: the official
  reference archive is unreachable from the dev container; the board
  documents both graft-in paths) and the heavy locks in
  `tests/ipc5_qual_metric.rs`.
- **Constraint-side static simplification** (in `constraints::compile`,
  same `FF_PREF_NO_STATIC=1` hatch as the 0.5 goal-preference pass):
  constraint bodies are partially evaluated against static predicates +
  init, and instances whose fold verdict is statically ACCEPTED are
  dropped before monitor compilation — statically VIOLATED instances are
  never dropped. This is what makes the qualitative storage instances
  compile at all (p03: 1,548 of 1,554 instances dropped; without it,
  quadratic `forall` preferences OOM grounding). Planner-side only: the
  verifier folds the unsimplified semantics, so the oracle stays
  independent.
- **The independent verifier is now authoritative on quantified preference
  bodies**: `verify::verify` grounds formula-level `forall`/`exists` (both
  in constraint bodies and in goal-preference bodies) before scoring, folds
  every soft constraint-preference over the replayed trajectory, and reports
  per-instance verdicts (`Verified::constraint_prefs`). reported ==
  verified is asserted exactly on every qualitative domain and on 5 of 6
  simple-preferences domains (rovers stays validity-only: its metric folds
  a numeric term the preference verifier doesn't recompute).

### Changed

- What stays rejected is now rejected **by name**: the four timed operators
  (`within`, `always-within`, `hold-during`, `hold-after`), any constraint
  on the temporal path (Phase 3), and `Session` (grounds once and replans
  from mutated states — a compiled monitor's S_0 baking would go stale).
  The 0.4.1 blanket rejection survives behind `FF_CONSTRAINTS_REJECT=1` (a
  restore hatch that restores *rejection*, not ignoring — no setting
  silently drops a constraint).

## [0.6.0] - 2026-07-15 — Selection: solve the choice, then plan to it

The forensics release. `docs/forensics-tpp.md` proved the remaining
tpp/pathways tail was never a search problem — on zero-action-cost domains,
plan quality is decided by WHICH jointly-satisfiable preference subset the
end state lands in, and SGPlan5's tpp p05 score is the closed-form optimum of
that selection. 0.6 answers it twice over: the guidance BARRIER flip (keep
init-satisfied preferences visible — the storage 8/8 domain sweep vs SGPlan5,
totals 234 vs 547) and the SELECTION layer (solve the subset choice exactly,
then plan to it — tpp p06 ties SGPlan5 exactly, and rovers' totals lead
widens to 4862 vs 5632 because selection also picks which samples are worth
their traverse cost). Suite tally vs the IPC-5 winner: **19W / 16T / 13L**
(0.4.0 shipped at 14/11/23), with three domains led under both quality
conventions (openstacks, storage, rovers) and trucks ahead on totals — all
on pure defaults, deterministic at any thread count. Two levers were measured
and honestly retired along the way (a weight-aware barrier variant; three
selection-shaped attempts at trucks' shared-timeline scheduling, which moves
to 0.7 as temporal selection). Every default change keeps a restore hatch
(`FF_PREF_NO_BARRIER`, `FF_PREF_NO_SELECT`).

### Added

- **The selection layer** (`selection.rs` + the closure loop's selection
  seed; `FF_PREF_NO_SELECT=1` restores 0.5.1) — the 0.6 headline, built
  from the tpp forensics: on preference domains, plan quality is largely
  decided by WHICH jointly-satisfiable preference subset the end state
  lands in, so ferroplan now solves that selection EXACTLY (a variable per
  invariant mutex group, Eq/Neq atoms coupling compiled `(NOT p)` facts to
  their groups, DFS branch-and-bound with a deterministic node cap) and
  plans to the chosen facts as a hard-goal target: singleton pre-probes ban
  supply-capped facts (on tpp they re-discover the market caps exactly),
  at most two joint attempts (per-fact bans cannot repair counting
  infeasibility), the exact tail closes, and the incumbent feeds the normal
  tightening loop. The seed's bounded evals stay OUTSIDE the tightening
  budget, like the legacy EHC seed (charging them starved storage p08,
  83 → 104 → fixed). The selection bound is admissible, so `final == bound`
  can prove optimality. Measured (defaults, deterministic, t1≡t8):
  **tpp p05 89 → 80** (the solver's bound reproduces the forensics'
  79 optimum; the +1 is one `p-drive` application, outside end-state
  selection), **p06 104 → 101 — an exact tie with SGPlan5**, p07 110 → 103;
  **rovers p02 596.7 → 502.2, p03 935.3 → 847.4, p08 998.1 → 740.9**
  (selection picks which samples are worth their traverse cost; the rovers
  totals lead widens to 4862.0 vs 5632.5). Storage's 8/8 sweep, pathways,
  trucks, and openstacks hold exactly. Suite tally vs SGPlan5:
  **19W / 16T / 13L**.

### Changed

- **Init-satisfied preferences are kept in the satisfaction guidance** (was:
  excluded since 0.4.0's barrier-free change). Plan forensics on tpp p05
  (`docs/forensics-tpp.md`) showed the exclusion makes the search blind to
  high-weight TRAP preferences — `not (stored goods1 level3)` is satisfied at
  init, so the guidance rewarded trampling it for a cheaper positive
  preference, and every restart-ladder profile inherited the blindness; the
  entire 93-vs-79 gap on that instance was this one decision. Re-measured on
  the 0.5 engine: keeping them takes **storage p05–p08 from 31/121/124/148 to
  25/43/60/83 — an 8/8 domain sweep vs SGPlan5** (totals 234 vs 547) — plus
  tpp 89/104/110/129 and pathways p06 11, at the cost of pathways p05 alone
  (6 → 6.5, a win becoming an exact tie). Suite tally vs SGPlan5:
  19W/15T/14L. `FF_PREF_NO_BARRIER=1` restores the 0.4–0.5.0 exclusion.

### Added

- `docs/forensics-tpp.md` — the tail-gap forensics: on zero-action-cost
  domains quality is pure end-state selection; SGPlan5's tpp p05 79 is
  derived as the closed-form selection optimum (per-goods stored level under
  supply caps + the 16-weight coupling constraints); the identified 0.6
  lever is exact selection planned as hard goals.

## [0.5.0] - 2026-07-14 — Closing on first: three IPC-5 domains on the defaults

The 0.5 roadmap ("First Place") executed end-to-end, shipped with its honest
verdict. On the vendored IPC-5 simple-preferences suite, **pure defaults** —
one configuration, no env vars, deterministic at any thread count — ferroplan
now **leads SGPlan5 under BOTH quality conventions (per-instance wins AND
domain totals) on three of the six domains**: openstacks (wins p04–p08, 271
vs 326), storage (wins p01–p07, 447 vs 547), and rovers (wins p04/p06/p07/
p08, exact ties p01/p05, 5301.6 vs 5632.5). trucks leads on totals (23 vs 31)
with instances drawn; tpp and pathways stay with the IPC-5 winner. Suite-wide
the instance tally is **19W / 14T / 15L** — more wins than losses against the
contest winner for the first time (0.4.0: 14/11/23). The 4-of-6 bar this
release aimed at was not met, so the claim is "closing on first," not first —
the remaining gap is exactly the tpp/pathways p05–p08 tails, measured
direction-bound (identical at 4× budget) and resistant to every lever below.
Full ledger: `benchmarks/ipc5-scoreboard.md`; the executed plan:
`docs/roadmap-0.5.md`.

### Changed

- **ESPC graduated: deterministic eval budget, default-on where it bites.**
  The penalty loop's outer budget converts from wall-clock to an evaluated-
  state pool (`FF_ESPC_EVAL_BUDGET`, default 6M) threaded through every inner
  search — thread-count and machine independent, exactly the contract
  `FF_PREF_EVAL_BUDGET` set for the B&B. `features::espc()` defaults ON (it
  engages only on deadline-pair structure — a verified no-op elsewhere);
  `FF_NO_ESPC=1` opts out; `FF_ESPC_TIME_MS` is demoted to an optional
  additional wall cap that applies only when set. The graduated default
  openstacks row reproduces the old opt-in row exactly (19/23/17/16/21/22/
  66/87; worst wall ~63 s on p04).
- **Folded numeric metrics route through the exact-closure optimizer** (was:
  legacy compiled-goal B&B). The 0.4.0 verdict that the closure path measures
  worse on rovers ("tiny-epsilon tightening churn") was an artifact of
  first-improvement restarts, which the anytime sweeps removed; with the
  routing flipped, rovers goes 935.3/653.5/1018.2/485.5/523.3/664.6/402.2/
  979.9 → **811.3/596.7/935.3/418.7/483.6/655.7/402.2/998.1** — a full
  domain lead. `FF_PREF_NUMLEGACY=1` restores the pre-0.5 split.
- **Anytime sweeps + a diversified restart ladder in both preference B&B
  loops** — the two remaining scoreboard levers, measured and landed. Each
  bounded metric sweep now tightens its bound **in place** on every acceptance
  and keeps draining (a restart happens once per eval cap, not once per
  improvement; `FF_PREF_GREEDY=1` restores first-improvement sweeps). Measured
  alone this changed no metric — the large-instance plateau was never restart
  churn but a **guidance limit** — so a capped no-improvement sweep now
  rotates the open-list weights through a fixed half-cap **profile ladder**
  (h-greedy → h-heavy → g-heavy → pure-h) under the same bound before the
  final all-remaining escalation (`FF_PREF_NO_RESTARTS=1` disables). Fully
  deterministic and thread-count independent. On the IPC-5 suite
  (`benchmarks/ipc5-scoreboard.md`): **storage now beats SGPlan5 on p01–p07
  and on the domain total** (46/145/200/263 → 31/121/124/148 on p05–p08),
  **pathways p05 flips to a win** (8.5 → 6 vs 6.5), tpp p05–p07 −4/−12/−14,
  trucks p03 1→0 and p06 6→1, openstacks default-path p01 42→23, rovers p04
  559.9→485.5 (0.1 from a tie). Cost, recorded honestly: tpp p08 +1,
  openstacks p03 +1, rovers p02 +56.8 — all already-losing instances.
  Instance tally vs SGPlan5: 14W/11T/23L → **17W/12T/19L**. The opt-in
  `FF_ESPC` openstacks path is untouched (spot-checked identical).

### Added

- `heuristic::relaxed_plan_cost` — a cost-aware relaxed plan (sums the
  selected ops' `increase` effects on a cost fluent), and an experimental
  **forgo-aware seed** built on it (`FF_PREF_SEED=1`): price each
  preference's completion from the initial state and pre-forgo those priced
  over their weight in one extra seeded solve. Measured **neutral** on rovers
  (the estimates fire correctly, but the EHC seed already lands at the same
  incumbent; identical metrics on/off across p01–p08) — default off, kept as
  the substrate for completion pricing inside the search.
- **Partitioned closure seed** (`FF_PREF_SEED3=1`, experimental, default
  off): ESPC increment 3 generalized past deadline pairs — mutex-conflict-
  pruned preference components composed into an incumbent by P3-masked,
  sibling-protected stages before the tightening loop. The composition
  genuinely works (tpp p05 composes 99 vs the 105 init-tail) but measured
  **neutral on finals**: the anytime+ladder loop reaches the same metric from
  either starting bound. Kept as the substrate for per-stage λ pricing (0.6).
- The 0.5 roadmap (`docs/roadmap-0.5.md`), now annotated with the executed
  outcome per phase.

## [0.4.1] - 2026-07-06 — Trajectory-constraint safety and a docs correctness pass

A correctness point release. It closes one silent-correctness footgun — PDDL3
trajectory `(:constraints ...)` were parsed but enforced by nothing, so a hard
constraint was accepted and dropped — and runs a documentation once-over that
retires the pre-0.4.0 "we trail SGPlan6" story the docs still told in places. No
engine or plan-quality change to any solve that succeeds today; the only behavior
change is that a domain declaring trajectory constraints now errors instead of
being silently mis-solved.

### Changed

- **PDDL3 trajectory constraints are now rejected instead of silently ignored.**
  The modal `(:constraints ...)` operators (`always`, `sometime`, `at-most-once`,
  `sometime-after`/`-before`, `within`, `hold-during`/`-after`) were parsed into
  the AST but enforced by no solving path, so a hard constraint was accepted and
  dropped. Every public entrypoint (`solve`, `decompose`, `Session::new`, the `ff`
  CLI) now returns a clear error (new `SolveError::Unsupported`) when a domain or
  problem carries one. Goal `(preference ...)` soft goals are unaffected — they
  live in the goal formula, not in `:constraints`, and the PDDL3 metric path still
  handles them.

### Added

- `ferroplan-py`: `temporal` mode, for parity with the `ferroplan-wasm` binding.
- Library examples `decompose.rs` and `validate_plan.rs` (the two advertised
  public APIs that had no runnable Rust example).
- An `examples/README.md` index (feature-by-feature map + reading order) and a
  `book/src/tuning.md` reference collecting the full `FF_*` env-knob family.

### Docs

- Corrected stale/contradictory documentation left over from before 0.4.0: the
  README's ESPC "not yet built" limitation (it shipped), the SGPlan5/SGPlan6
  baseline mix, the book's `results`/`metric-quality`/`pddl3`/`temporal` pages
  (which still told the pre-0.4.0 "we trail SGPlan6" story and marked timed
  initial literals / duration inequalities unsupported), the non-compiling
  `library.md` example, and the `village` example's false "`:derived` is rejected"
  claim. Archived the 0.2.1 roadmap.

## [0.4.0] - 2026-07-03 — Preference metrics: ferroplan takes on SGPlan5

The PDDL3 preference-metric release. On the vendored IPC-5 simple-preferences
suite (p01–p08, six domains, vs the official SGPlan5 results — see
`benchmarks/ipc5-scoreboard.md`), ferroplan goes from a distant quality 2nd to
**leading the IPC-5 winner on two domains** (openstacks via the opt-in
`FF_ESPC` partitioned penalty loop; storage on the plain defaults), **ahead on
the trucks total**, at **small-instance parity on tpp and pathways**, with
**full 48/48 coverage** (storage was 2/8) — every result deterministic and
thread-count independent.

Bumped to 0.4.0, not 0.3.1: the preference-metric default path changed (the
exact-closure optimizer replaces the compiled-goal B&B; wall time now scales
with the eval budget instead of stopping at the first failed probe) and the
public API grew (`SearchCfg::w_c`, `features::espc()` /
`set_espc_override`). Every behavior change has a restore hatch:
`FF_PREF_COMPILED`, `FF_PREF_NO_STATIC`, `FF_PREF_BARRIER`,
`FF_PREF_NO_ESCALATE`, `FF_ESPC_MONO`; budget via `FF_PREF_EVAL_BUDGET`.

### Added
- **Budget-escalating B&B retry — the eval budget becomes a real contract,
  lifting five of six IPC-5 preference domains at the default settings.**
  Both preference-metric optimizers (closure and legacy) treated one capped
  300k-eval tightening probe that found no cheaper plan as terminal, abandoning
  the optimization with most of `FF_PREF_EVAL_BUDGET` unspent — and the
  per-iteration cap was pinned at 300k, so raising the budget changed nothing
  (measured: 16x budget, identical results). A capped failure now retries the
  same bound with ALL remaining budget (deterministic eval counts, so plans
  stay thread-independent; `FF_PREF_NO_ESCALATE=1` restores the old behavior;
  the legacy loop also gains the budget accounting it never had). Measured at
  defaults: tpp p04 36 -> 35 (SGPlan5 tie, completing p01-p04 parity), tpp
  p05/p07/p08 97/131/146; trucks p07 19 -> 12 (now ahead of SGPlan5's 24 by
  half); storage p05/p06/p08 46/145/263; openstacks default p01 49 -> 42;
  rovers p02 659.3 -> 596.7 and p05 649.9 -> 523.3. Wall time now scales with
  the budget (trucks p08 ~163 s at 4 threads; lower `FF_PREF_EVAL_BUDGET` to
  trade quality for speed).
- **`SearchCfg::w_c` — experimental metric-cost open-list ordering** (default
  0.0 = priority key bit-identical), settable via `FF_PREF_COST_WEIGHT`. Built
  as the designed rovers lever and measured to be a dead end there: every
  non-zero weight collapsed rovers to the all-forgo floor (accumulated cost
  ordering buries deep goal-reaching prefixes), so the default stays 0
  everywhere and the field is documented as experimental. Additive public-API
  change to `SearchCfg` (constructors default it).
- **Exact-closure metric optimizer (new default for preference metrics) —
  storage flips from 2/8 coverage to beating SGPlan5 on p01–p05; tpp and
  pathways reach SGPlan5 parity on their small instances; trucks p08 drops
  133 → 10.** Three coupled changes to the PDDL3 path, each with a restore
  hatch:
  - *Static preference simplification* (compile): a preference whose phi is
    statically true (e.g. an `imply` over a static relation that never holds
    for that binding) can never be violated, so it is dropped before the
    Keyder–Geffner expansion — storage's quadratic forall-preference shrinks
    ~90–97% (p03: 1601 → 53 live instances; p08: 62k raw). Reported metrics
    are unaffected (the verifier scores from the original goal).
    `FF_PREF_NO_STATIC=1` restores blind expansion.
  - *Exact-closure metric search* (optimize): the anytime B&B now searches
    REAL states only, accepting a state iff the real hard goal holds and
    `cost-so-far + closure(state) < bound` — `closure` being the exact weight
    the deterministic `P3END`/collect/forgo phase tail pays from that state —
    instead of searching a compiled goal of hundreds/thousands of bookkeeping
    facts with a satisfaction-blind heuristic. The first incumbent is the tail
    applied to the initial state (instant coverage on any pure-preference
    instance); the tightening budget is a deterministic evaluated-state count
    (`FF_PREF_EVAL_BUDGET`, default 2M), so plans are thread-count
    independent, and un-capped exhaustion still proves optimality. Folded
    numeric metrics (rovers) deliberately stay on the legacy compiled-goal
    B&B; `FF_PREF_COMPILED=1` forces it everywhere. Multi-disjunct phis
    (`imply`/`exists`) now close correctly (the collect-op map kept one
    arbitrary disjunct before).
  - *Barrier-free DNF guidance*: the open-list satisfaction penalty now
    evaluates each preference's full DNF (so `imply`/`exists` preferences
    guide at all) and skips preferences already satisfied in the initial
    state — penalizing their transient dips walled off every improving
    trajectory (tpp's weight-16 `p4A` made metric 16 unreachable from 21).
    `FF_PREF_BARRIER=1` restores the old shape.

  IPC-5 defaults (release, 4 threads, all ≤ 60 s): tpp 16/24/29/36/101/116/
  133/148 (ties SGPlan5 p01–p03), storage 3/5/6/9/48/148/200/272 (beats
  SGPlan5 p01–p05; was 8/12 then nothing), trucks 0/0/1/0/0/6/19/10 (wins
  p01/p07), pathways 2/3/3/2/8.5/12.9/12.5/20.2 (ties p01–p04), openstacks
  default 49/40/29/41/67/86/153/370 (`FF_ESPC` row unchanged at 19/…/87),
  rovers unchanged. See `benchmarks/ipc5-scoreboard.md`.
- **Partitioned ESPC (opt-in `FF_ESPC`) — ferroplan now beats SGPlan5 on
  openstacks p04–p08.** The PDDL3 preference-metric penalty loop
  ("increment 2" of `docs/espc-preferences-spec.md`) couples its per-trigger λ
  schedule to a partitioned search instead of one monolithic B&B per penalty
  setting: subproblems come from the goal-interaction components of the real
  (non-compiled) goal, the shared renewable-resource variable (openstacks'
  `stacks-avail`) is excluded from component formation and priced as a global
  constraint by λ, each stage's goal is enriched with its own preference
  deliverables (the per-stage quality pressure a cost bound can't provide on
  cost-flat stage plans), the compiled `P3*` bookkeeping is closed by an exact
  phase tail, and leftover budget runs an incumbent-bounded monolithic polish
  (the "never worse than the plain B&B" floor). IPC-5 openstacks p01–p08 at
  the same 90 s budget: 42/43/55/66/81/90/151/227 →
  **19/23/17/16/21/22/66/87**, ahead of the IPC-5 winner SGPlan5 on p04–p08
  (26/36/33/67/123) — deterministic (3/3 identical runs, thread-count
  independent) and typically stall-terminated in 4–60 s. The default path is
  untouched (`FF_ESPC` stays opt-in; the other five IPC-5 preference domains
  are verified no-ops); `FF_ESPC_MONO=1` reproduces the previous monolithic
  loop. New WASM-safe toggle: `features::espc()` / `set_espc_override`.

### Fixed
- **Bevy Animator: "Animate this plan" always showed the embedded demo.** The
  Solver web page writes the domain, problem, and already-solved plan to
  `localStorage['ferroplan.handoff']` before navigating to the Animator — but no
  Rust code ever read it back, so the Animator always loaded its embedded demo
  regardless of what was actually solved and selected. `webhandoff.rs` now reads,
  parses, and applies the handoff at startup (scene + the pre-solved plan,
  autoplaying immediately — no re-solve, so it can't disagree with what the
  Solver page reported); falls back to the embedded demo if there is no handoff
  or it fails to parse. Verified in headless Chromium: no handoff → embedded
  demo; a real handoff → the handed-off domain/problem with its plan already
  playing; a corrupted handoff → clean fallback, no panic.

## [0.3.0] - 2026-07-02 — Solver depth: escalation, parallelism, sessions

A temporal goal that used to fail in ~45 s can now solve in ~30 ms (default-on
goal-relevance pruning); a search that used to just fail now escalates through two
more rungs before giving up (the Full demand tier, then the decomposer); and a
caller embedding the planner in a live loop gets a proper `Session` API instead of
re-grounding every tick. Measured on the 75-instance RPG temporal corpus:
**65 → 73 solved, zero regressions on anything that already solved.**

Bumped to 0.3.0, not 0.2.3: this release adds a new public API (`Session`) and
changes default `solve()`/`ff` behavior for temporal domains — an instance that
previously failed fast can now take substantially longer before returning
`solved: false`, because the escalation ladder tries harder before giving up
(`FF_NO_ESCALATE` restores the single-pass pre-ladder behavior). Two correctness
fixes are included too: a validator/replay bug on `:derived`-axiom domains, and a
domain-authoring bug in the `rpg-world` example (`bread-line` was unsolvable by
construction).

### Added
- **`Session` — ground once, replan many.** The embedding API for callers that
  re-solve the same world every tick (a game's villagers, a simulation loop):
  `Session::new` parses, compiles `:derived` axioms, and grounds ONCE; the session
  then holds the *current world state* — mutate it with `set_fact`/`set_fluent`
  (plus `fact`/`fluent` readbacks) as the world evolves and `replan()` solves from
  wherever it stands, paying only the search. Measured on `villagers`: a
  tick-sized contract (`errand`) drops **223 µs → 22 µs per replan (~10×)**; a
  search-dominated instance (`township`) is break-even, as expected — size
  per-agent contracts small (the decomposer's whole job) and the tax vanishes.
  Static facts are rejected with an explanatory error (grounding bakes them in;
  flipping one could require never-enumerated operators), as are temporal and
  PDDL3-preference inputs (v1 scope). See `examples/replan.rs`.

### Solver
- **Goal-relevance pruning graduated to the default tier.** Previously it rode the
  opt-in `FF_TDEMAND` Full tier only; the default search could exhaust its node
  budget in goal-irrelevant unbounded accumulators (`food=1,2,3,…`) on
  feature-rich domains. Measured trigger: on the rpg-world bread-line hub,
  `flour >= 2` — a 5-step till→plant→irrigate→harvest→mill chain — **failed after
  ~45 s; it now solves in ~30 ms** under defaults. The pass structure gains an
  **unmasked complete backstop** (helpful/sound → full/tight → full/sound →
  full/unmasked), so completeness is now *unconditional* — a hypothetical mask bug
  can cost time, never coverage. `FF_NOREL` disables pruning alone;
  `FF_NO_TDEMAND` still restores the pristine pre-v0.2 path.
- **Static unproducibility check — fail unsolvable goals in microseconds.** If a
  positive goal fact has no adder anywhere in the grounded task, or a `>=`/`>`
  numeric goal's fluent has no effect that could ever raise it, the temporal
  search (and every decomposer contract) now reports unsolvable immediately
  instead of exhausting every pass — bread-line's unproducible goal went from a
  **~45 s** exhaustive failure to **~9 ms**. Sound and conservative: an effect
  counts as a potential raiser unless it provably never raises; the check never
  changes a found plan.
- **Validator/replay fix: `:derived` domains.** Every solve path compiles derived
  axioms into init facts before grounding — but `plan::validate_plan` (the CLI
  `--validate`), `verify::verify`, and `trace::trace` replayed against the **raw**
  problem, so on axiom-using domains (e.g. rpg-world's `(:derived (reachable …))`)
  they wrongly rejected valid plans ("problem grounds to unsolvable" / "unknown
  action") and the GUI animator couldn't trace them. All three now run
  `derived::compile` first (identity when a domain has no axioms).
- **rpg-world domain fix: the bread economy.** `bake-bread` produced `meals`
  directly, leaving the `bread` fluent with **no producer** — so `hard/bread-line`
  was unsolvable-by-construction (violating the hard-set's "solvable in principle"
  contract) and `plate-spread` was dead code. `bake-bread` now yields bread
  (cook bonus included); meals keep their direct path via `cook-meal`, and the
  bread→plate-spread→meals chain is live. `bread-line` now solves and validates
  under default options.
- **On-failure escalation ladder.** When the default-tier monolithic temporal
  search fails, `temporal::solve` now retries at the **Full demand tier**
  (predicate-goal seeding), then hands the goal to the **decomposer** — each rung
  runs only after the previous one failed, so no instance that solves today can
  change its plan; the ladder spends extra time on would-be failures to convert
  them into solves. Ladder gains (all plans independently `--validate`d):
  `crew-solo`/`crew-pair`/`skilled-specialists` at the Full rung (makespans
  109/152/198 — matching their documented flagged solves, now flag-free),
  `order-8`/`order-12` and `found-village` at the decomposer rung. The tier is
  now threaded explicitly through the search (no racy global overrides), the
  decomposer's own monolithic fallbacks are **skipped on the ladder path** (the
  ladder already exhausted that exact search at both tiers — and this is also
  what makes the ladder recursion-free), and TIL-bearing compositions stay safe
  (the decomposer hard-validates before returning). `FF_NO_ESCALATE` — or
  `features::set_escalate_override(false)` in-process (WASM) — disables the
  ladder alone; `FF_NO_TDEMAND` still restores the pristine pre-v0.2 path.
- **Parallel temporal search.** The decision-epoch search now evaluates successor
  heuristics **in parallel** (the `threads` option previously only parallelized
  grounding on the temporal path). Successors are generated serially, batch-
  evaluated across workers (one relaxation `Scratch` per worker; frontiers under
  128 stay on the serial path with zero new allocation — per-round fan-out has to
  amortize against a full unpruned op scan to win), then enqueued serially in
  input order — so the heap and visited-set evolve exactly as before and **plans
  are byte-identical for any thread count**, verified by a corpus-wide
  determinism sweep at `--threads 1/2/4/8` (65 instances, 0 mismatches).
  Measured honestly: the win is modest (~4% on exhaustion-bound searches, ~0 on
  typical solves) — the temporal search is dominated by its serial successor-gen
  / dedup / heap machinery, so this lays the plumbing without changing the
  performance story; the corpus-visible speed lever remains the ladder + pruning.

**Measured** on the full temporal corpus (rpg suite + hard + contracts, cabin,
villagers — 75 instances): **65 → 73 solved, zero losses, zero makespan changes
on previously-solving instances** (pruning graduation +2, escalation ladder +6).
The hard set is now 12/12 — 10 under plain defaults (was 3/12 when authored) and
the two big conjunctive orders via the ladder's decomposer rung. The remaining
corpus misses are `crew-trio` and `skilled-crosstrained`, which resist every
rung — the honest border.

### Benchmarks & docs
- **IPC-5 openstacks: the opt-in `FF_ESPC` gap to SGPlan5 re-measured, ~5× → ~3×.**
  A fresh measurement (`FF_ESPC=1 FF_ESPC_TIME_MS=90000`, 4 cores) narrows the
  scoreboard's headline quality gap: 42/43/55/66/81/90/151/227 vs. the prior
  default row 63/66/62/66/138/129/278/608 across p01–p08, no instance regresses.
  The loop is budget-sensitive — at the *default* 15 s only p01/p02/p06 improve
  on the same box.
- **`docs/espc-preferences-spec.md`: the general-path ESPC blocker has been
  built.** A 2026-07 revisit found that the multi-predicate mutex-group
  synthesis added since the original "deferred" decision (`invariants.rs`) now
  recovers exactly the `(STACKS-AVAIL n)` guidance variable a faithful
  cross-domain ESPC needs — the specific gap the deferred design cited as
  blocking. What remains is "increment 2": coupling the `espc.rs` penalty
  schedule to the partitioned search (subproblems from goal-interaction
  components, global constraints on shared mutex variables). Not yet
  implemented; recorded as the concrete next step.

## [0.2.2] - 2026-06-30 — GUI & tooling

A GUI- and tooling-focused release: the web surfaces and the native Bevy app get a
shared "forge" visual identity, the animator gains a real timeline UI (a scrubbable
transport bar) plus a temporal timescale (Gantt) view, the engine is brought up to
current dependencies, and the publish pre-flight is fast again. No solver/library
API or behavior changes — `ferroplan` / `ferroplan-cli` are functionally identical
to 0.2.1 (dependency refresh only).

### Added
- **Animator transport bar** (native Bevy GUI) — a play/pause button, a scrubbable
  timeline (click or drag to seek, one notch per step), a molten progress fill +
  playhead, and a step/time readout. Mirrors the keyboard controls so the animator is
  usable with the mouse alone.
- **Temporal timescale (Gantt) view** — temporal plans (overlapping durative actions
  the graph can't tween) are now legible: each durative action is a bar on a shared
  plan-time axis, greedily lane-packed so non-overlapping actions share a row, coloured
  by the acting object, with a cyan "now" line swept by the transport playhead. Toggle
  with **T**.
- **Duration-aware playback + active-edge highlight** — classic plans dwell on each
  step in proportion to its `duration`; temporal plans sweep their whole makespan in a
  fixed wall-clock time (relative durations preserved); the edge a mobile is traversing
  at the current timeline position is recoloured molten and thickened.

### Changed
- **"Forge" visual identity** across all three surfaces — the Solver web demo, the
  Bevy visualizer/animator web shell, and the native GUI are restyled to a shared
  dark / molten / cyan palette, and the logo is retinted to match (cyan start, molten
  target).
- **Bevy 0.15 → 0.19** — the GUI is migrated to current Bevy (rendering split into
  `*_render` feature crates, the `Projection` enum, and the `BorderColor` /
  `BorderRadius` / `FontSize` / `ScrollPosition` API changes). Building the GUI now
  needs Rust ≥ 1.95; the published library keeps its 1.74 MSRV (it has no Bevy
  dependency).
- **Dependencies modernized** — `thiserror` 1 → 2, `criterion` 0.5 → 0.8, `pyo3`
  0.24 → 0.29, `wasm-bindgen` pinned to 0.2.126, and the rest brought current.

### Fixed
- **Fast publish pre-flight / `cargo test`** — two IPC-benchmark regression guards
  (`espc` ~346 s, `ipc5_pref_metric` ~175 s) are now `#[ignore]`d, so the default test
  run (and `publish.sh`) finishes in seconds. They remain gated: CI runs them in
  release (`cargo test --release -p ferroplan -- --ignored`), and `RUN_HEAVY=1
  ./publish.sh` (or `cargo test -- --include-ignored`) includes them on demand. No
  assertions changed — only when they run.
- **Bevy GUI black screen on launch** — the 0.19 render features (`bevy_ui_render`,
  `bevy_gizmos_render`, `bevy_sprite_render`) weren't enabled, so the ECS data was
  there but nothing drew.

## [0.2.1] - 2026-06-26 — "The Bridge"

The engine release (0.1) made ferroplan fast and correct; 0.2 makes the README's
bet real and inspectable: the proven temporal heuristics are on by default, temporal
coverage goes deeper (duration inequalities + timed initial literals), and a goal too
big for the one-shot search is **automatically decomposed** into solvable,
individually-verified contracts.

### Added
- **`parse` API — syntax-check PDDL without solving.** `ferroplan::parse(src)`
  auto-detects domain vs problem, validates syntax, and returns a serde-serializable
  `ParseReport` (ok/error-with-line, name, requirements, and a structure summary:
  types/predicates/functions/actions or objects/init/goal/metric/TIL counts) — fast
  feedback for an authoring loop or editor tooling, no grounding or solving. Exposed
  as a **`parse` MCP tool** too.
- **MCP server (`ferroplan-mcp`)** — a Model Context Protocol server exposing
  `solve`, `validate`, and `decompose` to an LLM agent over stdio, so the agent can
  *author and supervise* PDDL and let the planner run deterministically (the README's
  bet, made operational). A self-contained newline-delimited JSON-RPC 2.0 loop — no
  async runtime, deps limited to `serde`/`serde_json` — that returns the structured
  `Solution` / `Decomposition` as tool results, reports tool failures as `isError`
  results (so the agent can correct its PDDL), and never panics on input. Integration
  tests drive the built binary end to end. (`publish = false` for now; not in the
  crates.io release set yet.)
- **Goal decomposer — `decompose` API + `ff --decompose`** (the README's bet, made
  inspectable). A temporal goal too big for the one-shot search is split into ordered
  sub-contracts — each small enough to solve whole and individually verified — then
  stitched into one validated plan. This surfaces the partition-and-resolve engine
  (previously only the `FF_TDECOMP` flag, which returned just the flat plan) as a
  first-class, typed, serde-serializable `Decomposition { contracts, plan, monolithic }`
  where each `Contract` names its sub-goal (`(order o1), (order o2)`, `coin >= 15`),
  its sub-plan, and its offset in the stitched timeline. A goal that can't be split —
  or whose split doesn't validate — falls back to a single monolithic contract,
  reported honestly. `ff --decompose` prints the breakdown (text or `--json`).
  Demonstrated on `examples/rpg-world/hard/order-8` & `order-12` (8 / 12 contracts),
  which the one-shot temporal search fails on. `ferroplan::decompose(domain, problem,
  &Options)`; `tresolve::solve` now delegates to the recording `decompose` (the
  `FF_TDECOMP` plan path is unchanged).
- **Timed initial literals (PDDL2.2)** — `(at <time> <literal>)` in `:init` (including
  `(at <time> (not <literal>))`) now schedules an exogenous fact change at a fixed
  absolute time, disambiguated from the ordinary `(at ?x ?y)` predicate by a numeric
  first argument. Each TIL compiles to a synthetic 0-arg applier action (so its fact
  is grounded and a goal reachable only via a TIL isn't pruned as a relaxed dead end);
  the decision-epoch search fires it from a pre-seeded agenda at its time, the STN
  re-timing floors TIL-gated actions at their scheduled instant so they can't slide
  before their gate, and the in-crate validator replays TILs up to the plan horizon.
  Off the temporal path, TILs are inert (heap key byte-identical).
- **Temporal duration inequalities** — `:duration` now accepts `(>= ?duration L)`,
  `(<= ?duration U)`, and `(and ...)` ranges in addition to the fixed
  `(= ?duration e)`. The decision-epoch search commits to the **shortest feasible**
  duration (the lower bound), and the in-crate temporal validator accepts any
  duration within `[min, max]` (a fixed `=` collapses the range to a point,
  recovering exact-equality). Durations remain constant or parameter-dependent.
  (IPC temporal domains aren't vendored — licences — so this is exercised by
  crafted inequality domains + `temporal::validate`; the fixed-duration RPG corpus
  is unchanged, 26/27 suite.)

### Changed
- **Temporal demand guidance is now on by default** (graduated from the opt-in
  `FF_TDEMAND`). The default is a new **`Numeric`** tier: demand is seeded from
  *numeric goals only* — the measured multi-round win (`steel ≥ 2`, `grain ≥ 10`,
  `coin ≥ 15`). Validated on the RPG `suite/` + `hard/` corpus: **26 → 34/39
  solved, no regression** vs. the old default, and crucially *without* the makespan
  regression a blind graduation would cause — the previously-coupled
  predicate-goal-threshold seeding reads a renewable-pool guard (`(>= (avail) 1)`,
  net-zero) as accumulation demand and serializes concurrency domains (a unit
  `crew` pool of 2 went concurrent-~5 → serialized-~10). That structural/predicate
  half — plus goal-relevance pruning — now rides an explicit **`Full`** tier
  (`FF_TDEMAND`), which additionally solves the one structural build
  (`gather-build`) the numeric default gives up (decomposer territory per
  `examples/BORDERS.md`).
  - Opt out entirely with **`FF_NO_TDEMAND`** (heap key bit-identical to 0.1.0).
  - Library / WASM callers: `features::set_overrides` is now tri-state-backed
    (`true` / `false` are definitive; new `features::clear_overrides` returns to
    default + env), and the active tier is queryable via `features::demand_mode()`
    (`Off` / `Numeric` / `Full`).

## [0.1.0] - 2026-06-24

Initial public release.

### Added
- Data-parallel FF planner core (bitset / CSR, parallel grounding + heuristic).
- **Enforced hill-climbing (EHC)** with helpful actions and a weighted-best-first
  fallback — the default, ~3× faster than best-first and metric-ff-class on
  classical/ADL (geomean 0.21× → 0.66× Metric-FF).
- **Configurable `Options`** (library-first; CLI flags + JSON map to the same
  fields): `mode`, `search`, `helpful_actions`, `weight_g/weight_h`, `threads`,
  `max_evaluated`, `optimize`.
- ADL: conditional effects, `forall`/`exists`, object equality.
- Numeric fluents (Metric-FF style).
- **Derived predicates / axioms** (`:derived`, static / stratified) — closed into
  the initial state via a datalog fixpoint.
- PDDL3 soft-goal preferences (incl. `forall`-quantified and precondition
  preferences) with anytime branch-and-bound metric optimization. IPC-5 coverage
  on par with SGPlan6 (39/48).
- **PDDL2.1 temporal**: durative actions with `at start`/`over all`/`at end`
  conditions & effects, constant or parameter-dependent durations, required
  concurrency, and ε-separation; decision-epoch search; IPC temporal plan output
  with makespan. Plans validated against VAL on real IPC domains (44/45 valid);
  an independent in-crate validator (`temporal::validate`).
- SGPlan-style partition-and-resolve mode.
- **ESPC penalty-resolution loop** (`FF_ESPC`, opt-in) — SGPlan's Extended
  Saddle-Point Condition adaptive penalty coordination, applied to the PDDL3
  preference metric path. It penalizes, on the *concrete* state, once-only
  conditional achievements that fire without delivering (openstacks: a product
  made while its orders still wait — a permanently lost preference the
  delete-relaxed heuristic is blind to), and adapts a **per-trigger** penalty
  across an outer loop, keeping the best plan as an anytime incumbent. Iteration 0
  runs the penalty-free B&B as a floor, so the loop can only improve, never
  regress. Narrows the metric-quality gap on openstacks at the default budget
  (p01 63→42, p02 66→43, p05 138→81, p06 129→90, p08 608→227); a larger
  `FF_ESPC_TIME_MS` / more threads improves the hardest instances further
  (e.g. p07 278→142). The loop is wall-clock-bounded (default 15 s, tunable) and
  always returns its incumbent inside that budget, so it never loses coverage
  under a harness timeout. Inert on every domain without the make-deadline
  structure — including the whole numeric/temporal RPG corpus — and bit-identical
  to the prior default when off. Auto-tunes per instance (no manual weight); never
  claims optimality. See `docs/espc-preferences-spec.md`.
- **Temporal converging-resource demand guidance** (`FF_TDEMAND`, opt-in) — the
  ESPC concrete-state idea ported to the durative/numeric (RPG) search. It regresses
  the numeric goal down the recipe DAG to a TOTAL per-resource demand (`steel ≥ 2` ⇒
  ingots/coal/ore ≥ 2, logs ≥ 4 — bridging snap-compiled start/end the way the
  landmark extractor does) and guides on cumulative availability (init + produced,
  clamped), which survives consumption across rounds. This is the gradient the
  delete-relaxed heuristic lacks once ≥2 contributions converge on a goal quantity
  (see `examples/BORDERS.md`). Phase-1 key only — phase 2 stays byte-identical, so
  completeness holds. Measured on the RPG corpus: **+8 instances solved (26→34/39),
  all plans validated, no regressions**, cracking three shapes the relaxation went
  flat on — multi-round converging DAGs (tech-steel/bronze), cyclic resource regen
  (farmstead `grain≥10`), and multi-path numeric goals (mint-fortune/trade `coin≥N`).
  Off by default (heap key bit-identical when unset).
- **Temporal partition-and-resolve decomposer** (`FF_TDECOMP`, opt-in) — the SGPlan
  partition loop (`resolve.rs`) brought to the durative/numeric path for the
  conjunctive/structural goals the demand term can't crack. A reusable
  `temporal::solve_from(start, goal, forbidden)` subplanner (the temporal analog of
  `solve_subgoal_avoiding`) lets the decomposer partition the world goal into
  contracts, solve each from the running composed state, splice the timed subplans
  strictly sequentially (each offset past the prior makespan + an ε seam), and MERGE
  groups on conflict down to a monolithic `temporal::solve` — so it is solvable
  EXACTLY when the monolithic search is (completeness preserved). Same-epoch
  happenings order on an ε-grid-rounded key (ends before starts) so the offset
  concatenation validates without re-separation. Measured: solves the large mixed
  conjunctive goals `order-8`/`order-12` (RPG temporal 34→36/39), every composed
  plan validated, zero regressions, default path byte-identical. Remaining fails
  (`found-village`, `gather-build`) reduce correctly to a *pre-existing* predicate-
  build (`build-house`/village-shape) search blowup — the next target, separate from
  the decomposer. Groundwork for it (predicate-goal demand seeding; predicate-
  precondition contract regression) is in place behind the same flag.
- **Temporal goal-relevance pruning** (rides `FF_TDEMAND`; `FF_NOREL` disables) — a
  backward closure from the goal marks every op that can contribute (adds/deletes a
  relevant fact or increases a relevant resource, transitively pulling in its
  preconditions and consumed resources); non-contributing ops are pruned from BOTH
  search phases. Fixes the predicate-build blowup: the diagnosis showed phase 1
  (helpful actions) gets stuck under delete-relaxation (the agent is relaxed-
  omnipresent, so travel is never "helpful"), and the COMPLETE phase 2 then drowns in
  goal-irrelevant unbounded accumulators (`forage-food`/`gather-herbs` → food=1,2,3,…).
  Pruning to the relevant subspace lets the search solve instead of exploding. Two
  masks drive three passes — helpful(sound) → full(TIGHT) → full(sound): the SOUND
  mask keeps every producer of a relevant resource (completeness-preserving, the final
  backstop); the TIGHT mask keeps only each resource's single best-yield producer, so
  marking `planks` relevant pulls in `saw-planks` but NOT the alternative producer
  `haul-cargo` (which would otherwise drag the whole logistics subsystem in and
  re-explode). Off by default (empty masks ⇒ op set bit-identical, original two-pass
  behavior). Solves `gather-build` AND `found-village` (RPG temporal 36→38/39); every
  plan validated, no regressions, full suite green. The lone remaining miss,
  `bread-line`, is a pre-existing relaxed dead-end unrelated to relevance.
- **Concurrent temporal scheduling** (`FF_TCONC`, opt-in) — a scheduling phase
  (`tsched`) for durative plans. The decision-epoch search is action-count-guided, so
  it lays actions out sequentially and more workers never shortened the makespan; this
  repacks the found plan onto the domain's actor-objects — one job per worker at a
  time, each action starting as early as its consumed resources and prerequisite
  predicates allow — to minimize makespan. The multi-actor search is flaky, so it
  searches a single-actor reduction and reassigns the plan across the real crew. Every
  rescheduled plan is run through `temporal::validate` and kept only if shorter, so it
  can only improve a plan, never produce a wrong one; default path byte-identical.
  Showcase (`examples/cabin`): a durative crew build where 1→2→3 workers cut makespan
  109→63→47 on the same job.
- **Worker skills** — a task's actor-referencing precondition (e.g. `(smith ?w)`) is
  read by the scheduler as a required capability, so skill-gated tasks are assigned
  only to workers who have them (location is handled the same way); the single-actor
  reduction becomes a super-worker (union of all skills) so the search still finds the
  plan, and a task needing a skill no worker has is correctly unsolvable. Shown in
  `examples/cabin/crew-skilled` (sawyer/smith routing) and a "forge order" where the
  smith is the bottleneck — two extra labourers barely move it (65→62) but a second
  smith at the same crew size cuts ~a third (65→44).
- **WASM feature overrides** (`crate::features`) — the env-gated temporal switches
  (`FF_TDEMAND`/`FF_TDECOMP`/`FF_TCONC`) reachable from non-CLI callers via a process
  override OR'd with the env read (env *writes* panic on `wasm32`), surfaced through
  the WASM `plan(domain, problem, mode, flags)` `flags` arg — so the browser demo runs
  the demand guidance, decomposer, and concurrent scheduler too.
- Library API returning structured, `serde`-serializable results.
- `ff` CLI: drop-in `-o/-f` text, `--json`, `--json-request` job I/O, full
  strategy flags.
- **Robust** against malformed input — pathological/deeply-nested PDDL returns a
  typed error, never a panic.
- **SAS+ / mutex groups** — Helmert-style multi-predicate invariant synthesis,
  feeding SGPlan-style subgoal partitioning + resolution.
- **General metric terms** — the metric optimizer folds monotone numeric fluent
  terms (e.g. rovers' `(sum-traverse-cost)`) into total-cost, so all six IPC-5
  simple-preferences domains are scored, rovers included.
- **Bindings (reach)** — `ferroplan-wasm`: run the planner in the browser via
  WebAssembly with a self-contained "try it" demo (no server/install);
  `ferroplan-py`: a pyo3 **abi3** wheel (`import ferroplan; ferroplan.plan(domain,
  problem)`), one wheel for CPython 3.8+. The core stays pure Rust.
- mdBook documentation site; cross-planner comparison harness (`compare.py`),
  temporal+VAL harness (`bench_temporal.py`), and benchmark results vs
  Metric-FF / SGPlan6 / VAL.
- **Worked-domain corpus + coverage borders** (`examples/`) — a ~120-action
  crafting/economy domain (`rpg-world`) with validated contracts, a flavor-×-scale
  `suite/`, an adversarial `hard/` batch, and an `industrial-city` decomposition
  showcase; plus `logistics` (transshipment) and `jobshop` (machine-scheduling,
  scales to 100 jobs) domains. `examples/BORDERS.md` is a measured map of where
  one-shot planning solves vs. where a goal must be decomposed into contracts. Also
  `villagers` — the generic, data-driven recipe model a live game embeds (3 actions:
  walk/gather/craft, recipes as `:init` data; the abstract counterpart to rpg-world) —
  and `cabin`, a deep linear build (fell→mill→smith→glaze→raise, ~52 steps) with a
  durative "parallel crew" twin showing makespan vs. crew size and worker skills.
- **Claude Code skill** (`.claude/skills/ferroplan`) — PDDL-authoring guidance, a
  CLI/feature reference, and six per-feature examples each re-verified to solve,
  enforcing an author → run → read-the-plan loop.
- **GUI / web** — per-type procedural icons (incl. a machine icon for scheduling
  domains) and relation-colored edges (rail vs road vs stage routing). The in-browser
  WASM demo is a **two-level picker** (choose a domain, then a problem graded
  simplest→most-complex), with an execution toggle (**Web Worker** — responsive +
  cancelable — or main thread, for environments that block workers), solve-on-button
  so a heavy problem never auto-freezes the tab, and per-example **feature flags** that
  enable the demand guidance / decomposer / concurrent scheduler in-browser. Includes a
  `border` example that shows where one-shot planning gives out.

### Performance
- **Grounding** — restrict each parameter's domain by its static unary
  preconditions before enumerating; fixes untyped cartesian-product blowup
  (gripper p02 658µs→247µs, 2.65×; large untyped grounding 1.56s→~0). See
  `docs/perf-notes.md`.
- **EHC** — work cap scaled by op count so large-but-easy instances finish in
  EHC's near-greedy arm instead of unpruned best-first (gripper-250 `--mode ff`
  2.16M evals/33s → 32k/0.86s, 38×).
- **Temporal search** — a weighted-`g` heap key plus two-phase helpful-action
  pruning (a pruned `g+h` phase, then the original complete pure-`h` phase) takes
  multi-step long-chain contracts from timeout to instant. A numeric-threshold
  landmark term (phase-1 key only, so the complete pass is byte-identical) then
  restores the heuristic gradient on converging recipe DAGs — a from-scratch ingot
  and the metallurgy benchmark go from no-plan to instant, and deep accumulations
  get 10–60× faster. No regression on the existing temporal suite.

### Known limitations
- Numeric domains trail Metric-FF (EHC falls back to best-first on some).
- IPC-5 preference metric *quality* on the hardest instances still trails SGPlan6;
  retroactively, ferroplan places ~2nd in the field (SGPlan5 swept). The opt-in
  ESPC penalty-resolution loop (`FF_ESPC`, see above and
  `docs/espc-preferences-spec.md`) narrows the openstacks gap substantially
  (~11–63% per instance) but does not close it — reaching SGPlan's level needs a
  dedicated minimum-open-stacks scheduler, not a relaxation-guided search. ESPC is
  off by default while the cross-domain sweep matures.
- The metric branch-and-bound does not scale to instances with hundreds of
  preferences (e.g. storage p05+) — the Keyder–Geffner compilation grows large.
- Temporal coverage is search-limited on the largest *monolithic* instances; the
  intended path past the border is decomposition into contracts (see
  `examples/BORDERS.md`).
- Not supported: duration inequalities, timed initial literals, continuous (`#t`)
  effects, and *dynamic* derived predicates (static / stratified axioms are
  supported).
