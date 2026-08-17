# Changelog

All notable changes to this project are documented here.

## [Unreleased]

Accumulating toward 0.24.0 — the SAT wing. Board claims land with
the cut sweep. Full record:
[`docs/roadmap-0.24.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.24.md).

- **THE HEADLINE: temporal-machine-shop falls.** TMS-2011 i2 —
  SOLVED, VAL-valid, ~1 s (Mode::Sat, horizon 16, one STN
  refutation): the first TMS solve in this planner's history, on
  the family where every non-SAT entrant ever fielded scored zero.
  The zero block has its first nonzero row. slitherlink p01 falls
  to the classical face for a second first.
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
  unenforced; six timed solves banked oracle-green.
- **The basket**: the temporal search pays the wall (sokoban-t's
  honest exits); the a2 chain converts pathwaysmetric i2 at 173
  evals; the hash-join candidate lists clear the slitherlink gate
  (p03 grounding >60 s → 1.3 s); the 5A convergence fix recorded
  as a measured negative (nurikabe and spider are irreconcilable).
- **The game phase**: budget-stamped thinks with capped-vs-proven
  honesty on the MCP wire; the village tick loop 15.6 → 10.6 s at
  byte-identical evals; bazaar think latency halved. Mode::Sat
  reaches the wire by construction.

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
