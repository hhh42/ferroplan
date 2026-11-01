# Changelog

All notable changes to this project are documented here.

## [Unreleased]

### Added

- **A parallel rung portfolio, opt-in** (`FF_PARALLEL_RUNGS=1` at
  `--threads N ≥ 2`; 0.31 Lane A′, `parallel.rs`): the ladder runs as one
  worker exactly as it would alone, and N−1 of its rungs run beside it with
  the whole wall each — novelty-first LAMA, the driver with R refresh and
  forgetting, EHC, best-first, novelty-light, the front end, plain LAMA:
  the order of their marginal cover on the Phase 0.5 matrix with Lane B's
  rungs in the pool (the ladder 662 of 936 cells; with novelty-first LAMA
  beside it 712, plus the refreshing driver 747, plus EHC 750, plus
  best-first 753; the eight rungs' oracle 759). The first plan wins and
  the others are cancelled through the per-call budget's should-continue
  flag, which every rung polls at its batch boundary (novelty-light now
  too). Timing-dependent by construction — the one contract it breaks,
  "byte-identical for any worker count", is named in the module header —
  so it is opt-in and unmeasured until the mco subset sweep; the notes say
  it ran and which member won. The matrix hatch's solo rungs are one
  `search::Rung` enum now, shared by the hatch and the workers.

## [0.30.0] - 2026-10-29 — Memory, and two defects the rung exposed

0.29.0 moved rows and points together and left two things on the record:
the memory-capped tail of the classical boards (45 cells mem-capped, most
of them the LAMA and novelty rungs' node stores) and a pathways probe
whose six unsolved cells partition mode solved in seconds. This cycle
declared the same two measures before any lane ran — rows by the
first-attempt estimator against 0.29.0, points by the IPC quality score
on the preference boards — and a third, memory, read as `max_rss` on the
same rows. Six lanes ran; three were recorded negative by their own kill
clauses, and one of those found two defects in partition mode on the way
out.

### Measured

**66% coverage across 32 IPC boards** (5,568/8,444), **694 certified
optima** -- 8,444 of 8,444 instances banked in ten passes, then 21 rows
re-opened by the re-check and re-measured (all banked). Full record:
[`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md),
[`docs/roadmap-0.30.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.30.md).

**Rows, against 0.29.0, on the estimator declared before the sweep:**

| estimator | 0.29.0 | 0.30.0 | delta | 95 % CI |
|---|---:|---:|---:|---|
| **first attempt** (the claim) | 5,476 | 5,404 | **-72** | [-98, -48] |
| equal-N | 5,528 | 5,459 | -69 | [-94, -45] |
| per-run | 5,495.8 | 5,473.1 | -22.7 | [-39, -7] |
| clean-only | 546.3 | 544.7 | -1.7 | [-8, +4] |
| banked -- what the boards show | 5,555 | 5,568 | +13 | [+3, +24] |

**The estimators disagree in sign, so no delta is claimed.** The declared
estimator reads -72 and it is recorded as read. The sweep ran on a
heat-soaked fanless Air: 24 % of 0.30.0's runs were clean against 34 % of
0.29.0's, and on clean runs alone the two engines are level (-1.7
[-8, +4]); the boards' banked figure moves +13. Cells both engines banked:
**21 gained, 8 lost** -- caldera-sat i4, parking-agile i10, counters i12
and markettrader i5 (2023 numeric), elevator-temporal i12, tetris
multi-core i15, storage-complex-pref i16 / i18; ten more cells the first
read lost solved again on the re-check. One defect the first cut read found
is fixed before this sweep: novelty-light now keeps its states in a flat
arena (88 bytes a node against 229), so a tripped light rung no longer
leaves LAMA gigabytes of freed-but-resident nodes (woodworking i15 / i16,
lost on five boards, solve again at 13-16 s).

**Memory, the third measure (Lane 5, the lazy frontier).** The LAMA and
novelty rungs' open lists held a full state per entry; they now hold a
key, a parent and an operator, and regenerate the state on pop (128-bit
keys: 64-bit FxHash collided on rovers' quantised fluents). On the
180-cell memory subset: mem-cap verdicts 45 → 24, mean RSS down 20–50 %;
rows −1 [−6, +3] (child-snack i6 gained; caldera i3/i4 lost until the
memory wall learned not to refuse a rung entered above its line). A
memory win, rows flat — the engine change is kept.

**Points (Lane 1b, flat):** on the lane's read simple 97.3 → 97.3,
qualitative 64.6 → 64.3, complex 54.5 → 54.1; on the cut's boards 96.4 /
62.4 / 54.1 against SGPlan5's 115 / 92 / 100. The anytime polish
(the pass over the held preferences repeats from the plan it improved
while the share has wall left) buys nothing on the boards: the first
pass already prices every candidate the search can see. The candidate
search is the limit; it stays, pinned, for 0.31.

**Numeric (Lane 2b): +10 on the 2026 numeric board, −1 on 2023 numeric
(a 55 s coin), metric-time +0** — first +6 [−2, +14], banked +9 [+3, +15].
The consumption charge now prefers the achiever that does not overshoot
an equality goal; a "transfer" reading of a move-like effect cost −32 on
sugar / hydropower / markettrader and was withdrawn by bisection.

### The partition rung — out, and what it found

Under `--mode partition` the pathways propositional cells i17/i19/i21 and
their strips twins solve in 7–34 s; the classical ladder never reaches
them on any budget. A rung that tries the partition first on half the
wall gained those six on the propositional board and cost 26 first
attempts on the cells the ladder solves in the second half of its wall
(first-attempt −26 [−39, −15], banked −9 [−18, +0]) — the kill clause,
three board reads in a row. The rung stays in the engine opt-in
(`FF_PARTITION_RUNG=1`). The reads found two defects in partition mode,
both fixed and kept: **the merge cascade had no clock** — after its
budget expired, `resolve::solve` went on merging (99 merges at 0.7 s on
openstacks i30's hundred groups, to the process wall) — and **a ladder
entered after a bounded attempt judged its rungs against the whole wall**
(the 0.22 affordability gate read 49 % and skipped novelty-light, LAMA and
novelty the moment EHC's quarter was spent); it is now judged against the
wall it is handed.

### The instrument (Phase 0)

- The crucible's calibration gate waits for a clean line (foreign CPU
  under 25 %) before measuring the canary, stamps every canary reading
  with the foreign load beside it, and excludes stamped-foreign readings
  from the baseline — the 0.29 record's "thermal" verdicts on an idle
  night were calibration under a daytime indexer.
- `crucible engine` prints a content hash beside the blake3: a rebuild
  from the same source differs by 47 bytes of Mach-O `LC_UUID` and code
  signature, and the record now says when two engines are the same code.
- Folding is a grounding that never finishes (6.1 GB, 68 s past a 60 s
  wall), not a grounder defect; 0.29's probe used the wrong path. The
  grounding class (folding, caldera, organic-synthesis, storage-complex
  i19/i20, ~50 rows) is named for 0.31.

### Owed to 0.31

Classic text mode (`ff` without `--json`) runs its partition-first attempt
before the wall is armed; the boards and every probe use `--json`. A rung
that pays for itself must cost less than half the wall. The preference
candidate generator. The grounding class.

## [0.29.0] - 2026-10-15 — Points, and a relaxation that counts

0.28.0 closed the ROW gap to SGPlan5 on the IPC-5 preference boards and left
the POINT gap where it was, so this cycle declared two measures before any
lane ran: rows, by the first-attempt estimator against 0.28.0, and points, by
the IPC quality score on the preference boards. Four engine lanes and a
harness phase came out of that; two lanes were recorded negative. A
2026-09-30 read found that the first thing wrong with coins-2026 i18 was not
the heuristic at all but a code-generation regression in 0.28.0 itself.

### Measured

**66% coverage across 32 IPC boards** (5,555/8,444), **691 certified optima**
-- 8,444 of 8,444 instances banked in seven passes, then 24 rows re-opened
and re-measured by the re-check below. Full record:
[`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md),
[`docs/roadmap-0.29.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.29.md).

**Rows, against 0.28.0, on the estimator declared before the sweep** -- each
instance's FIRST attempt against its first:

| estimator | 0.28.0 | 0.29.0 | delta | 95 % CI |
|---|---:|---:|---:|---|
| **first attempt** (the claim) | 5,358 | 5,476 | **+118** | [+89, +149] |
| equal-N | 5,397 | 5,522 | +125 | [+95, +155] |
| banked -- what the boards show | 5,466 | 5,555 | +89 | [+67, +112] |

Every estimator agrees in sign. Banked reads lower than first-attempt because
the referee re-runs failures and 0.28.0's re-run rescued more of its failures
than 0.29.0's did (net -28). Of the cells both engines banked, 110 were gained
and 21 lost.

**Where it moved (rows):** 2023 numeric +27 (289/400, now ahead of the vendored
field's best, ENHSP at 267), metric-time +14 (119/200), 2014 seq-sat +7, the
2014 multi-core boards +6 / +6 / +7, IPC-2011 seq-sat +5 and tempo-sat +3, 2014
tempo-sat +2, 2014 agile +2, 2018 seq-sat +2, propositional +2. Down: 2023
seq-sat -1, 2026 numeric-opt (full) -1. 

**Points, the second declared measure** -- the IPC-5 quality score (best
metric / ours, over the cells SGPlan5 solved), SGPlan5's own score beside:

| track | rows | 0.28.0 | 0.29.0 | SGPlan5 |
|---|---|---|---|---|
| simple-preferences | 130/130 | 94.1 / 120.4 | **97.3 / 115.1** | 129/130 |
| qualitative-preferences | 100/100 | 59.2 / 92.1 | **64.6 / 91.4** | 100/100 |
| complex-preferences | 81/108 | 53.4 / 99.0 | 54.5 / 100.0 | 105/108 |

The headline lane moved points in the right direction on all three boards and
**landed under its pre-registered band** (simple 96 -> 105 of 120, qualitative
63 -> 75 of 92): ferroplan is closer to SGPlan5 on quality, not level with it.
Read the denominators: each cell's score is best-known over own, so a better
plan from either planner moves the other's total.

**The 21 cells 0.28.0 solved that 0.29.0 banked unsolved**, each re-run on
both engines before being named (0.29.0 with up to eight attempts; 0.28.0
rebuilt from tag `v0.28.0` under the crucible's own prefix, engine
`bb07d50d2015`, which is not byte-identical to the baseline's `89cfdc5f06ed`).
**Twelve reproduce**: the rebuilt 0.28.0 solves them and 0.29.0 does not in
any attempt -- `rovers-metric-time/28`, `pipesworld-metric-time/36`,
`settlers-snp-numeric-2026/5`, `ztalloc-sum-numeric-2026/9` and `/12`,
`sugar-numeric-satisficing/17`, `driver-log-temporal-satisficing/6`, and five
IPC-2011 multi-core cells (`elevator` 19 and 20 and `transport` 18 at two
threads, `elevator` 19 and `transport` 18 at four).
**Eight are noise**: the rebuilt 0.28.0 cannot re-solve them either
(`driver-log-temporal-satisficing/5`, `tidybot` 19, `elevator` 20 at four
threads, `labyrinth-satisficing/9`, `forestfire-numeric-2026/14`,
`gear-car-numeric-2026/8`, and `pipesworld-propositional/50` and
`counters-numeric-satisficing/16`, which 0.29.0 itself solved on a first
attempt that the referee voided as thermal / swap and could not reproduce).
**One is a wall coin**: `transport` 18 at eight threads is solved by the
rebuilt 0.28.0 twice in two and by 0.29.0 on a first attempt the referee
voided, then missed three times.

**What the failure classes did.** Coverage is the claim; the unsolved rows'
reasons moved with it. `mem-cap` fell on 2014 seq-sat (30 -> 4), 2018 seq-sat
(35 -> 24), 2014 tempo-sat (6 -> 2) and 2014 agile (25 -> 1), and rose where
the memory wall now ends a rung early: 2018 seq-opt (10 -> 21), 2023 numeric
(33 -> 52), 2023 numeric-opt, 2026 numeric-opt full (4 -> 24), 2014 multi-core
t8 (4 -> 11). On the optimal boards the same rows end unsolved either way.

**Not met.** Metric-time +14 against a +15 to +30 band, short by one. The
achiever index (Lane 4) did not move storage-complex (complex-preferences +1
row, band +8 to +14).
The classical memory lane (Lane 5) was measured, not built: a lazy frontier is
named (6-15 KB per evaluated node against a 200 B model) and left for 0.30.
Propositional stands at 386/450 against SGPlan5's 218/220.

### Added

- **`benchmarks/recheck29.py`** and **`benchmarks/promote-air29.sh`**: the
  cut's re-check (every banked solve whose price is missing, every solve under
  a slow canary near its wall, every cell the previous release solved) and
  promotion, for the next cut to copy.

### Changed

- **Temporal: an end-side `?duration` over a dynamic read compiles** (0.29
  Lane 2). A durative action whose duration reads a fluent some action assigns
  and whose END uses `?duration` -- rovers' `recharge`, lasting
  `(/ (- 80 (energy ?x)) (recharge-rate ?x))` and paying
  `(* ?duration (recharge-rate ?x))` at its end -- was SKIPPED by the snap
  compile ("never compiled wrong"), so the temporal search could never
  recharge, and the compression rung's plans, which do, failed the validator
  by the name of an action it never had. PDDL2.1 fixes `?duration` at the
  START, so the compile does the same: a hidden per-action fluent is assigned
  the duration expression by the start snap and stands in for `?duration` on
  the end side. Fixture: `tests/dur_fluent.rs`.
- **The relaxed plan pays for what it spends** (0.29 Lane 2). A `decrease` is
  a delete on a number and the relaxation dropped it: a plan of five drives at
  8 energy each read `energy >= 8` against a stock of 10 and was satisfied,
  so every state with the same drives left had the same h whatever its
  energy. Extraction now sums a selected plan's spend of each fluent its own
  preconditions read from below, against the stock in hand plus what the plan
  puts back, and prices a deficit through the best raiser. `FF_NO_CONSUME=1`
  restores the blind h; byte-identical where a plan spends nothing it reads.
- **One grounding per polish** (0.29 Lane 1). The preference polish grounded
  the task twice per attempt -- once to search a candidate, once to price it
  -- which on pathways (8 s a grounding) left it nothing to attempt. The
  hard-goal task is grounded once and each candidate swaps its goal facts in;
  the verifier is a prepared context (`verify::VerifyCtx`) that grounds once
  and replays many; with the route's hard-goal seed task in hand -- a plain
  grounding of the pair -- the polish grounds nothing at all. An `or` body is
  one goal per disjunct; held `(not ...)` bodies forbid their achievers in a
  candidate's search; an atom the grounder dropped reads from the init
  instead of as false. Pinned by fixtures counting the groundings (two, and
  zero with the seed task). The polish's candidate search prices the held
  preferences a plan would break as per-op penalties and runs best-first
  under them, so a candidate breaks the fewest it can. And the temporal
  preference scorer prices a plan on a grounding of the plan's own steps
  (`tcompress::specialise`) instead of the whole task -- 4.4 GB on
  pipesworld-complex before; `FF_SCORE_FULL=1` keeps the old grounding --
  pinned to it by a fixture scoring four plans both ways.
- **The achiever index is no longer ops x monitors** (0.29 Lane 4). The
  shared monitor block's adds belong to every monitored op, and the index
  carried one entry per op per monitor add -- hundreds of millions of `u32`
  on storage-complex, its `mem-cap` class. `add_by_fact` holds each op's own
  adds; `PackedTask::achievers` merges in every monitored op for a fact the
  block adds, ascending, each once.
- **The preference polish runs first, on the seed's own task** (0.29 Lane 1).
  It takes half the optimizer's wall share, clears its memory trip before the
  compiled grounding, is skipped where the seed's grounding peaked past a
  quarter of `FF_MEM_BUDGET_GB` or grounded slowly, and needs most preference
  bodies expressible. Its candidate search prices the preferences a plan would
  break as per-op penalties (`search::with_op_penalty`) instead of forbidding
  their achievers. `FF_PREF_POLISH_FRAC` (0.4) sizes the share.
- **Temporal node-cap refill** (0.29 Lane 3, `temporal::refill_ladder`) and
  **EHC slice extension** (Lane W, `search::ehc`): built, fixtures green,
  recorded negative on rows (constraints 28 -> 28 of 120; sat subsets 0 gained,
  0 lost) and kept for short walls. `FF_NO_TNODE_REFILL`, `FF_NO_EHC_EXTEND`,
  `FF_EHC_WALL_MAX_FRAC` (0.6).
- **Streamed expansion of the constraints simplifier** (0.29 Lane 4,
  `constraints::expand_simplified`): a member the static simplifier accepts is
  never stored. Byte-identical; storage still peaks at 5-6 GB. `FF_PREF_NO_STATIC`.

### Fixed

- **A 0.28.0 code-generation regression.** `heuristic::widen`, which Lane N's
  `needs` parameter had tipped LLVM into keeping an out-of-line call, cost the
  relaxed-graph build 11.0 us per evaluation against v0.27.1's 7.7 (+43 %) on
  the identical search trajectory. `#[inline(always)]` brings it to 8.5 us;
  coins-2026 i18 solo at 60 s went from 0/3 to 3/3. The heuristic's nine
  `FF_*` hatches are also read once per `Scratch` instead of up to six
  `std::env::var` reads per evaluation.

### Harness (the crucible; not part of any published crate)

- **A solve can be SUSPECT** (0.29 Phase 0.1). The referee re-ran a row that
  failed under bad conditions and never one that solved under them; a solve
  that came back without its price, or spent half its budget or more, is now
  judged by the signals about its own process and the box's clock (demoted,
  canary, swap growth, clock jump). `[referee] solved_wall_frac` (1.0 restores
  "a solve is a solve"). Pinned by the population the rule came from.
- **`publish.sh` asks how old the crates.io token is** before spending the
  pre-flight, and refuses a token file older than 80 days.
- **crucible** (0.29 Phase 0.5): `--max-passes` is a cap on passes,
  productive or not (it was consulted only after a pass that banked nothing,
  so a subset banking a few owed rows a pass ran on past it); the canary's
  factor is read against the box's own line at the width it ran at (schema
  v10 records the width), since a chip that has been running nine planners is
  throttled whether they are paused for the read or not, and against the solo
  line until that width has readings.

- **The README's standings block is a capability table.** The front page's
  generated block now says what the planner is good at and against whom --
  one row per capability (temporal, PDDL3 preferences, classical satisficing,
  optimal with proofs, numeric), each board's coverage beside its rank in that
  competition's field and the leader's count, and the preference boards' IPC
  quality beside their rows -- in place of the best-five-by-coverage list,
  every row of which read ~100 %. `benchmarks/manifest.toml` carries each
  board's `capability` / `front_name` (generated from `standings.py`, as
  `proof_track` is); `crucible standings --check` now covers the README block
  as well as the two tables; the IPC-5 simple-preferences field is
  archive-counted into `field-results.json`. The notes that still called the
  timed constraint operators "rejected by name" (constraints and
  complex-preferences rows; README Limitations; the book's standings intro)
  are corrected: `within` / `always-within` have been enforced on
  durative-action domains since 0.24, and only `hold-during` / `hold-after`
  are rejected.

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (31 earlier releases, 0.1.0–0.28.0).
