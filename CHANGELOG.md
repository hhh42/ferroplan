# Changelog

All notable changes to this project are documented here.

## [Unreleased]

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
