# ferroplan 0.31 roadmap — the ladder learns, a second signal, and the optimal half

Scoped 2026-10-23, while the `cut30` re-sweep on `ff 0.30.0 [b02061eded16]`
is still running. The operator's brief: *"I don't care the level of effort
… I'm game to do any combination … knowing that some will probably fall
out as we go."* So this is a long cycle with several lanes, ordered so the
cut can be taken after any of them lands, and so each later lane runs on
the instrument the earlier one built. All 0.31 work lives on `engine-0.31`
in its own worktree (`/Users/harold/ferroplan-0.31`, created when Phase 0
opens); every completed phase pushes to `main` as well as the branch
(`CLAUDE.md`).

The discipline is 0.30's, unchanged: fixtures first; a measured win or a
recorded negative; every claim through the crucible (`crucible-spec.md` R3
subsets for questions smaller than a release, the 32-board sweep for the
cut); the comparative estimator declared before the sweep runs; sweeps at
night, no cargo on the box while one runs; the hot-timeout bank's bias
(one-sided, conservative on gains) stated beside every number read under
it. The reading list for the cycle is `docs/reading-list-0.31.md`; the
papers themselves are in `docs/papers/`.

**Baseline for `compare`:** the banked 0.30.0 engine — `b02061eded16` if
the running re-sweep promotes, else whatever `promote-air30.sh` banks.
(Fill in at Phase 0; never a rebuild.)

---

## The finding this cycle is built on

Three cuts in a row: 0.28 +190 on the six IPC-5 boards it aimed at, 0.29
+118 first-attempt over the 8,444, 0.30 **flat** (first read −16 [−41, +7];
Lane 5 rows −1, Lane 6 −26 and killed, Lane 2b withdrawn, Lane 1b flat).
0.30 did what it set out to do on memory — `mem-cap` rows 45 → 24 on the
Lane 5 subset, mean peak RSS down 20–50 % on every search-class variant —
and the rows did not move. The sentence that matters is Lane 5's close:
*the best-first beneath the rungs runs its 35–45 s on what the rungs leave
and reaches no plan.* The wall is now spent honestly, in memory the engine
can afford, and the **signal runs out**. Plumbing inside the current
heuristics is what 0.30 measured to zero; it is not a headline again.

Where the 2,889 unsolved rows sit (`benchmarks/ipc-standings.md`, 0.29.0):

| pool | unsolved | what the engine brings there |
|---|---:|---|
| optimal tracks (2008/2011/2014/2018/2023 seq-opt, 2023/2026 numeric-opt) | **~1,215 (42 %)** | A* + LM-cut + an h^max sprint. One admissible heuristic. Never had a lane. |
| modern satisficing timeouts (2014/2018/2023 sat, agile, classical) | ~460 | EHC → LAMA → bounded best-first; the novelty rung is w=1, ⟨#g⟩ only, after LAMA, opt-in |
| constraints | 92 | two negatives (0.28, 0.29 Lane 3); the trajectory constraint is a monolith on the compiled task |
| grounding class (folding, caldera, organic-synthesis, storage-complex) | ~50 | the grounder alone sits at 5.9 of 6 GB |
| preference **points** (complex 54.1/100, qualitative 64.3/92) | — | the polish's candidate generator is the limit (Lane 1b's close) |

And one pattern the record has paid for three times: a rung killed by the
**slice of the wall it was given**, not by what it does — the 0.17 novelty
rung (−51 budget-edge rows against +7 gained), the 0.28 seed-ladder
tightening (rovers-qual i20 lost at 80 %), 0.30's Lane 6 partition rung
(−26, 16 of them rescued on re-run; six pathways cells solved under it in
7–34 s). The engine's largest unmeasured parameter is its time table
(`FF_EHC_WALL_FRAC` 0.25, `FF_LAMA_WALL_FRAC` 0.25, `FF_NOVLIGHT_WALL_FRAC`,
`FF_NOV_WALL_FRAC`, the polish and seed shares, the opt sprint fractions —
twenty-eight `*_FRAC` / `*_BUDGET` knobs in `crates/ferroplan/src`), every
one hand-set from one or two cells.

What the engine already has that the lanes below lean on, checked in the
source on 2026-10-23: sound **mutex groups** from Helmert-style invariant
synthesis (`invariants.rs` — "exactly what a SAS+ representation would
give as multi-valued variables"); an **evaluation-budget portfolio** with
doubling slices (`portfolio.rs`, four fixed members, never wall clock);
the **flat-arena** node store and the **memory wall** (0.30); h^FF's
relaxed plan and helpful actions on every evaluation (`heuristic.rs`);
LM-cut with the conditional-effect repair (`optimal.rs`).

---

## The declared measures (pre-registered, before any lane runs)

1. **Rows**: first attempt against first attempt over the 8,444, `crucible
   compare --a <0.30.0 banked> --b <cut31 hash>`; equal-N beside it; banked
   coverage published as the boards' standing number, never as a delta; no
   delta claimed where the estimators disagree in sign. The hot-timeout
   bank's one-sided bias is named beside every number.
2. **Points**: the IPC quality score on the three preference boards against
   SGPlan5's `; MetricValue`, on ONE engine. Lane E is judged on this.
3. **Proofs**: the certified-optimum count across the seven opt boards
   (691 on 0.29.0) — a subset of rows, named separately because Lane C
   is judged on it and a plan without a certificate is not a row there.

Receipts that are not claims: Lane A's **held-out gain** (the table built
without a domain, read on that domain); Lane B's **node cost** and
**novelty-table bytes** on named cells; Lane C's **heuristic build time
distribution** (seconds before the first A* expansion) on every opt cell.

---

## Phase 0 — the instrument, and what the cut owed (blocking)

0.1 **The cut30 record closed.** `compare --set cut30 --lost`, `recheck30.py`
   on the lost cells, `promote-air30.sh`, the standings snapshot, the
   CHANGELOG — the 0.30 cut's own checklist, finished before 0.31 reads
   anything. The baseline hash above is filled in here.
0.2 **A canary pinned to a fixed binary** (crucible). Today the canary is
   keyed by engine hash and never reaches its five readings per width, so
   every engine re-learns the box's per-width line. Pin it to the banked
   0.30.0 `ff` (or a dedicated canary binary) so the line carries across
   engines. Fixture: two engines, one canary history.
0.3 **Demotion at day width 8** (crucible): demoted rows were owed 4.7 h on
   cut30. The scoping note proposed banking a demoted timeout the
   predecessor also failed, like `hot`; read against the referee's own
   receipt that was withdrawn — a demoted child runs on an E-core at ~13×
   slower (`referee.rs`, the 0.27 E-core defect), so such a bank would
   miss any gain the cell needed a tenth of its wall for, a bias an order
   of magnitude beyond `hot`'s 1.4×. **Built instead: the hold.** While
   the operator is at the keyboard (the only time the throttle demotes),
   a solo row the predecessor failed — the likely wall-long timeout —
   waits, and the predecessor's solves take the day width; the hold lifts
   the moment the box idles. Nothing is lost that a demotion would not
   have lost; what is saved is the re-run. `hold_failed_by_day = false`
   restores 0.30. (`sweep.rs::run_batch`, `config::Scheduler`.)
0.4 **Text-mode wall arming** (engine, one line + fixture): `run_planner`
   grounds and tries `resolve::solve` before `crate::solve` arms
   `FF_TIME_LIMIT`; arm it at entry. Fixture: text mode, armed wall,
   returns inside it. (Owed from 0.30, "Probes only".)
0.5 **The per-rung matrix harness** (crucible + engine) — Lane A's
   instrument, built here because every later lane reads it too. Engine:
   `FF_RUNG_ONLY=<ehc|novlight|lama|driver|novelty|bfs|partition|portfolio>`
   (and Lane B's `front`, `lamanov`, `driverx`) runs ONE rung with the
   whole remaining wall and the whole eval budget and returns ITS verdict,
   never the next rung's (`search.rs`; fixture `tests/rung_only.rs`); the
   JSON notes carry `rung: solved by <name>` and, under the hatch,
   `ground: N.NNNs` so the allocator can subtract the grounding the ladder
   pays once. Crucible: no new set — `sweep --env FF_RUNG_ONLY=<rung>
   --name matrix31-<rung>` on a subset (`select.rs`); the env is part of
   a board's identity, so each rung's rows land on their own board row
   and the real board is never touched. `benchmarks/matrix-sample.py`
   picks the sample from the banked baseline's rows and prints the five
   (eight, with Lane B's rungs) sweep commands; `benchmarks/ladder-table.py`
   is the allocator, with the held-out read beside the in-sample one, and
   writes the table `FF_LADDER_TABLE` loads (`ladder.rs`; fixture
   `tests/ladder_table.rs`). Sample:
   every domain of the satisficing/agile/mco boards, ≤ 12 instances a
   domain spread over the instance index (~1,200 cells; the operator
   accepted either this or a 600-cell half — the half is the fallback if
   the box is loud, at the cost of a coarser held-out read); seven
   rung-envs; ≤ 60 s each; ~140 child-hours serial, two quiet nights at
   width 8 (agreed 2026-10-23).
   The matrix is a **timing** measurement on a fanless box: rows read hot
   (canary ≥ 1.37×) are re-run, never averaged, and the slice currency is
   **evaluations** where a rung is budget-driven (`portfolio.rs`'s
   contract) with the evals/s receipt converting to seconds.
0.6 **Grounder bytes-per-op profile** (the grounding class, ~50 rows): one
   receipt per cell — ground ops, facts, bytes resident after grounding,
   bytes per op — on folding i1–i5, caldera i3–i6, organic-synthesis
   i5–i8, storage-complex i19/i20. A lane only if the receipt names a fat
   field (the 0.28 residue suspected the `packing the ops` achiever index).

---

## Lane A (headline) — the ladder learns its slices

**Claim:** a slice table learned from the per-rung matrix beats the
hand-set fractions on held-out domains, and no board loses more than it
gains. **Band:** +20 to +50 rows first-attempt over the satisficing /
agile / mco boards (the three recorded slice deaths sum to ~75 budget-edge
rows). **Kill:** held-out gain ≤ 0 on the matrix, or any board's loss side
larger than its gain side on the subset sweep → the table is recorded and
the fractions stay.

**Mechanism** (Helmert, Röger & Karpas 2011 — *Fast Downward Stone Soup*;
Seipp et al. 2015 — *Cedalion*, the per-step "most coverage per second
appended" refinement): from the matrix `t[cell][rung]`, greedily append
the (rung, slice) pair that adds the most cells solved per second of wall
until the wall is spent; the result is an ordered ladder with slices, per
track class (seq-sat / agile / mco-tN / temporal — the temporal ladder is
`tcompress` + the DE chase and gets its own small matrix only if the
classical table lands). The engine side is small: the ladder already
takes its slices through `rung_slice(...)`; a table keyed by track class
replaces the defaults, every `FF_*_FRAC` keeps overriding it, and
`FF_LADDER_TABLE=0.30` restores the hand-set fractions (the hatch, and the
A/B arm). **Leave-one-domain-out** is the referee: the table built without
domain D is what D is scored on; the published table is built on
everything, and its number is the held-out one, never the in-sample one.

**Fixtures:** (1) the allocator on a synthetic matrix with a known optimum;
(2) a ladder under `FF_LADDER_TABLE=0.30` byte-identical to 0.30.0 on the
corridor fixtures; (3) a ladder under a two-rung table that runs the
second rung at exactly its slice (wall debug receipt).

**Measured:** held-out on the matrix first (the receipt); then
`sweep --board ipc2014-seq-sat,ipc2018-seq-sat,ipc5-prop --engine <hash>`
through the crucible; `compare` first-attempt against the baseline. What
the mco boards do under a table built on single-thread rows is read, not
assumed (threads change evals/s, not the order).

**What it also buys:** every later rung in this cycle (B's BFWS, C's
heuristic rungs in the opt ladder, D's red-black rung) gets its slice from
the matrix rather than from a coin; the matrix is re-run on the sample for
the new rung only (one night, not two).

### Lane A — the matrix read, 2026-10-31: the kill clause fired

The matrix ran 2026-10-29 15:31 → 10-31 04:13 (36.7 h; ehc 2.1 h, novlight
7.4 h, lama 9.9 h, driver 6.4 h, bfs 10.8 h) on `ff 0.30.0 [6facbec9163c]`
(engine-0.31 at db38ce2, the fixtures run first — three red, all three
fixture defects, none the engine's), 1,008 cells × 5 rungs, packed at
width 9–10 with about half the rows dirty-timed; 936 cells carry every
rung. `ladder-table.py` learned ehc 19 / novlight 1 / lama 4 / driver 7 /
bfs 28 (`probes-0.31/matrix/table.txt`): in-sample 523, **held-out 505
against 513 = −8**. The kill clause: the table is recorded, the fractions
stay. But the number that matters came from the database, not the script
(`benchmarks/matrix-read.py`):

| on the 936 cells | solved |
|---|---:|
| one rung, the whole wall: ehc / novlight / lama / driver / bfs | 266 / 362 / 547 / 503 / 555 |
| **the banked 0.30.0 ladder** (`b02061eded16`, the same cells) | **662** |
| best pair side by side (lama + driver) | 662 |
| **oracle** — any rung inside the wall | **706** |
| cells the ladder solves that no single rung does | 0 |
| unique solves: ehc / novlight / lama / driver / bfs | 15 / 3 / 25 / 39 / 19 |

`ladder-table.py`'s "defaults 513" is a *simulation* of the hand-set
fractions under restart semantics; it mis-models a ladder whose rungs hand
state to each other (EHC's committed prefix, the seeded rungs, the
best-first on what they leave), and the banked rows say 662. Read against
662 the finding is: **the ladder sits at 94 % of its own rungs' ceiling.**
The fastest rung varies *within* 72 of the 78 variants, which is why a
table keyed on the domain cannot generalise (−8) and why no static
re-slicing the simulation can express beats the composition. The 44 cells
between the ladder and the oracle (4.7 %) are reachable only by running
rungs **side by side**, each with the whole wall.

What this closes and what it opens:

- **Closed:** the slice table, and with it every "give rung X more of the
  wall" lane — priced at zero by measurement, the three recorded slice
  deaths notwithstanding. Phase 0.5's instrument is the win: a candidate
  rung is now judged in 2–10 h on the sample (its *unique* solves, cells no
  standing rung reaches), not in a 36-h sweep.
- **Opened, in Lane A's slot:** the **parallel rung portfolio on the mco
  boards** (ipc2014-mco-t2/t4/t8, ipc7-mco-t2/t4/t8: 1,680 rows, 367
  unsolved, every one a wall death). Today `--threads N` only splits a
  batch's h-evaluation (`search.rs` line 21); the matrix says lama + driver
  side by side already equals the ladder, four rungs reach 703 of 706. The
  mechanism: at threads ≥ 2 the ladder's rungs run as workers with the full
  wall each, first plan wins, the others cancelled through `CallBudget`'s
  flag; the mode is opt-in until measured because it breaks the
  "byte-identical for any worker count" contract, which the record must
  say. **Band:** +4 % of the mco cells (≈ +40–80 rows) on the subset sweep,
  no single-thread board touched. **Kill:** < +20 rows, or any t2 board's
  loss side larger than its gain. **Code written 2026-10-31, built and
  fixtures green 2026-11-01:** `parallel.rs` (members by thread count:
  ladder → LAMA-novelty → the refreshing driver → EHC → bfs → novelty-light
  → front → LAMA, the matrix's greedy marginal order beside the ladder with
  all three Lane B rungs in the pool — 712 / 747 / 750 / 753 of the eight
  rungs' 759 at two, three, four, five workers; the plain driver is ninth
  and seatless), the ladder factored into `ladder()` behind
  `plan_avoiding`'s gate, the hatch's rungs as `search::Rung`, novelty-light
  given the hard checkpoint the other rungs poll, `tests/parallel_rungs.rs`.
  One defect found by the fixture on the first build: the workers' flag
  was armed with the polarity of a STOP flag where the budget reads a
  should-continue flag, so every worker was born withdrawn — the fixture's
  "no member solved in 0.00 s" is the receipt, and the reason the fixture
  exists. Measure next: the six mco boards through the crucible with
  `--env FF_PARALLEL_RUNGS=1`, compared against the banked 0.30.0 rows.
- **A probe, not a lane — run 2026-10-31, negative:** instance-level
  rung prediction (`benchmarks/rung-predictor.py`, output
  `probes-0.31/matrix/predictor.txt`, the feature table `features.csv`).
  Forty-eight features read off the PDDL text (requirements, type /
  predicate / action counts and arities, precondition and effect sizes,
  quantifiers, conditional effects, object / init / goal sizes, a log
  grounding estimate — no grounding, no search), one random forest per
  rung ("does this rung solve the cell"), the rung with the highest
  probability given the whole wall:

  | policy on the 936 cells | solved |
  |---|---:|
  | banked ladder | 662 |
  | best constant rung (bfs) | 555 |
  | the predictor, **leave-one-family-out** (an unseen domain) | **536** |
  | the predictor, 10-fold over cells (known domains, unseen instances) | 670 |
  | … plus the family name as a feature | 675 |
  | best rung per family **by name**, leave-one-cell-out | 680 |
  | oracle | 706 |

  Gating the predictor on its own confidence and falling back to the
  ladder otherwise reaches 656 on unseen domains at the strictest gate
  (65 overrides) — still under the ladder. The reading: the text features
  carry no signal that transfers to a domain the forest has not seen (536
  is under the best *constant* rung); on known domains they are a proxy
  for the name, worth +8, and the name itself is worth +18 — and a table
  keyed on the benchmark's names is memorising the benchmark. Closed: a
  learned single-rung choice, in any feature set cheaper than running the
  rungs. What remains of the 44 cells is reachable only side by side.
- **Lane B's judge is now fixed:** `lamanov`, `front`, `driverx` through
  the matrix (launched 2026-10-31 08:12, `probes-0.31/matrix/
  run-matrix-b.sh`), each read on its unique solves against the five.
  **First read, 2026-10-31 (lamanov, every cell reached, rescues still
  landing):** coverage **635** of 936 with the whole wall against plain
  LAMA's 547; **28 unique solves** no standing rung reaches (parking 7,
  pathways 4, caldera-split 4, trucks 3, storage 2, no-mystery 2, one each
  on rovers, data-network, flashfill, settlers, snake, spider); 50 cells
  the banked ladder does not solve; head to head, 97 cells it solves that
  plain LAMA does not against 9 the other way, and on the 538 both solve
  the medians are 3.8 s against 3.3 s. The six rungs' oracle is 734; the
  ladder with LAMA-novelty beside it 712. Two consequences, both for when
  the box is free: the portfolio's first worker is LAMA-novelty (the order
  is re-drawn in `parallel.rs`), and Lane B's own claim — `FF_LAMA_NOV=1`
  as the ladder's default, the rung in its slice — is now a subset sweep
  on the six boards against the banked 0.30.0 rows, not a matrix question.
  Its kill clause stands as written (budget-edge losses against gains,
  the nine LAMA-only cells among them). `front` and `driverx` follow.
  **Final read, 2026-11-01 06:29 (every rung on every cell, rescues
  landed; `matrix-b.log`):**

  | rung, whole wall | solved | unique of 8 | beyond the ladder |
  |---|---:|---:|---:|
  | LAMA-novelty (`lamanov`) | 636 | 22 | 50 |
  | driver + R refresh + forgetting (`driverx`) | 587 | 18 | 50 |
  | front end, w > 1 pruned (`front`) | 387 | 2 | 17 |
  | plain LAMA / plain driver | 547 / 503 | 2 / 1 | — |

  The eight rungs' oracle is **759** (the five's 706); greedy beside the
  ladder: 662 → + lamanov 712 → + driverx **747** → + EHC 750 → + bfs 753
  → + novelty-light 755 → + front 757 → + LAMA 758 → + driver 759. The two
  strong rungs STACK: 50 cells each beyond the ladder, 15 shared, 85 in
  union. Head to head the Lane B forms cover their plain forms — LAMA-
  novelty 98 cells plain LAMA does not against 9; driverx 112 against the
  driver's 28 — at a small median cost on the cells both solve (3.84 s v
  3.34 s; 2.05 s v 1.80 s). driverx's unique cells: transport-agile 5,
  data-network 4, spider 3, one each on rovers, settlers, child-snack ×2,
  transport-sat, ricochet-robots. The front end is a weak member — 328
  cells behind the oracle, 2 unique — and sits seventh, not worth a sweep
  of its own. **Consequences:** (1) the portfolio's first two workers are
  LAMA-novelty and driverx (done, above); (2) Lane B's ladder claim is
  now BOTH knobs as the ladder's defaults — `FF_LAMA_NOV=1` in LAMA's
  slot and `FF_NOV_REFRESH=1 FF_NOV_FORGET=1` in the driver's — one subset
  sweep on the six matrix boards against the banked 0.30.0 rows, kill
  clause as written; (3) the front end stays a hatch (`FF_RUNG_ONLY=front`)
  with no default-on claim. **The ladder sweep launched 2026-11-01 06:51**
  (`probes-0.31/laneb/run-laneb-ladder.sh`, engine `8efe4c05f005`, the
  six boards' 1,970 rows, subset `laneb31-ladder`); read with
  `benchmarks/subset-read.py --engine 8efe4c05f005 --baseline b02061eded16
  --env FF_LAMA_NOV` over the six boards. The portfolio's mco sweep
  (`probes-0.31/par/run-par-mco.sh`) follows it, on whichever ladder the
  read leaves as the default.

---

## Lane B — a second signal: BFWS done the winners' way

**Claim:** a width-based rung in the configuration that won IPC 2018 agile
and sits inside both IPC 2023 classical winners moves the modern-corpus
timeouts. **Band:** 2018 seq-sat 99 → 115+, 2014 seq-sat / agile +10 each,
2023 seq-sat / classical +5 each — +25 to +60 rows. **Kill:** 0.17's —
budget-edge losses exceed gains under Lane A's table (not under a hand
slice; that question is answered) → the rung stays opt-in, recorded.

**Corrected on reading the code and the papers (2026-10-23).** The
scoping note called the novelty rung "w=1, ⟨#g⟩ only"; that is the 0.17
h-guided rung, which 0.22 REPLACED at its slot with the h-free DRIVER
(`novelty::search_driver`). The driver already is BFWS(f5) in all but
name: partition ⟨#g, r⟩ with R the relaxed plan's need-facts from the
root (`r_partition_facts`, capped 256), width 2 over R-pairs, key
⟨rank novel-1 < novel-2 < non-novel, #g, |R|−r⟩, lazy frontier, 400k-eval
cap, default-on after LAMA at half the remaining wall. So what the winners
have that the ladder lacks is not f5. Reading the three papers
(`docs/papers/`, the extracted mechanism sections):

- **Dual BFWS** (L&G 2017, Table 2: 225/280 on IPC-2014 against LAMA's
  171 and BFWS(f5)'s 192): a FRONT END that is f5 with every w > 1 state
  PRUNED — polynomial, succeeds or fails fast (151 of the 225 on its own)
  — then a BACK END that is LAMA's machinery with novelty FIRST:
  **f6 = ⟨w_{hL,hff}, help, hL, w'_{hff}, hff⟩**, delayed evaluation,
  helpful/non-helpful split. The paper's own ablation is the receipt:
  pushing a novelty preference in front of ⟨hL, hff⟩ took 99 → 149.
- **#r recomputed on progress** (L&G 2017's definition): the relaxed plan
  behind #r is recomputed whenever a state lowers #g against its parent,
  not once at the root. The driver computes R once.
- **Forgetting** (Scorpion Maidu 2023, the satisficing winner; its single
  most important configuration `hnew` was LAMA's four queues + a
  type-based queue + ONE novelty queue w ≤ 2 whose tuple tables are
  cleared "whenever the search makes progress", with the open list
  bucket-shifted so old states fall behind; `hnew` alone 84/140 against
  LAMA's 80 on the 2023 domains at 30 min).

**Mechanisms, in order, each a `FF_NOV_*`/`FF_LAMA_*` knob with 0.30 as
the hatch:** (1) **novelty-first LAMA** — `lama.rs` gains a 3-level
novelty w_{hL,hff} (partition by the ⟨lm-count, parent-h⟩ pair; atom
tables on the arena; pairs over the R list as the driver does) as the
LEADING key term ahead of its ⟨W_FF·h + W_LM·lm⟩ key, both heaps; the
f6 shape, delayed evaluation already in place. (2) **the pruned front
end** — the driver with non-novel states pruned (rank 2 dropped), as a
rung AHEAD of LAMA beside novelty-light, bounded by construction
(|F|²·|G| states), slice from the matrix. (3) **R refresh + forgetting**
in the driver: R recomputed on a #g improvement (one relaxed plan per
improvement, not per pop — the driver stays h-free in cost); on each
improvement the tuple tables are cleared and the heap is re-keyed one
bucket back (Maidu's milder variant). **Fixtures:** the `deceptlawn`
shape (`tests/ladder_wall.rs`) for (1) — LAMA grinds the lawn, novelty-
first LAMA walks the corridor; a two-goal task where the front end fails
fast and cheap for (2); a task whose root relaxed plan is wrong for the
second goal for (3). Table bytes per node pinned; determinism at any
thread count (the rungs' existing contract). **Measured:** solo
receipts on city-car, child-snack, maintenance, woodworking i15/i16
(the 0.30 cells), barman / parking / transport from the paper's table;
then the matrix sample for the new rungs (one night) → Lane A's table →
`sweep --board ipc2014-seq-sat,ipc2014-agile,ipc2018-seq-sat,ipc2023-seq-sat`.

---

## Lane C — the optimal half

Forty-two percent of the unsolved rows, one admissible heuristic. Every
optimal-track winner since 2014 is one or both of: **saturated cost
partitioning over abstraction heuristics** (Scorpion: projections / PDBs
and Cartesian abstractions — Seipp & Helmert JAIR 2018; Seipp, Keller &
Helmert JAIR 2020; pattern selection by hill-climbing iPDB, Haslum et al.
AAAI 2007, or by CEGAR, Rovner et al. ICAPS 2019) and **symbolic
bidirectional search** on BDDs (Torralba et al. AIJ 2017; SymK / SymBD,
IPC 2023 abstracts). Ragnarok, the 2023 optimal winner, is a portfolio of
exactly these. The numeric side's leader is NLM-cut (Kuroiwa et al.,
ICAPS 2021 / JAIR 2022) — a generalization of the LM-cut we have, but
the leader sits at 102/400 at the official budget, so its 60 s ceiling
is low; it is C's last step, not its first.

**C.0 — the price, before any commitment (a probe, one week).**
(a) **Variables from mutex groups**: how many of the opt boards' cells
get a full cover (every fact in some group, plus singletons) from
`invariants::synthesize`, how many variables, how long. The abstraction
heuristics need a multi-valued view; this is the only place the engine
could already have one. (b) **One PDB, one cell**: a single projection
onto the goal variables, built explicitly, used as h in the existing A*,
on 2008 seq-opt's unsolved cells solo — does it prove anything LM-cut
could not in 60 s, and what did the build cost? (c) **Build-time
budget**: the distribution of "seconds before the first expansion" on
every opt cell under LM-cut today (the receipt the lane is judged
against: a heuristic that eats the median cell's wall building itself
is the kill). Record in `probes-0.31/opt/`; the lane opens on (a) and
(b) together, and names C1 or C2 first from them.

**C.s — symmetry pruning in the optimal A* (added 2026-10-31; CORRECTED
2026-11-01 on reading the code).** The 10-31 bullet said `optimal.rs`
never reads `orbits.rs`. False: the optimal A* has dedup'd its visited /
closed / best-g lookups on orbit-canonical keys since 0.22 Phase 6 L1
(`api.rs` arms `detect_classical_iso` on every unconstrained opt call;
`optimal.rs` `key_of` → `canonical_skey`; nodes stay concrete, so the
certificate needs no permutation trace — that piece already exists by
construction). What the engine HAS is orbit search over OBJECT
symmetries: interchangeable members with identical init profiles, closed
under every transposition, goal set σ-invariant (or goal-isomorphic under
`FF_ORBIT_ISO=1`, default off, probe-negative on TMS). What Pochter,
Zohar & Rosenschein (AAAI 2011) and Domshlak, Katz & Shleyfman (ICAPS
2012) have beyond it: the FULL automorphism group of the problem
description graph via bliss — predicate and structural symmetries that
are not object relabelings (mirror grids, symmetric goal positions,
symmetric operator schemas) — and in 2012 search in the orbit space
proper (successors generated from the canonical representative, not just
keys canonicalized). Neither paper is vendored; the 10-31 pointer to
`docs/papers/` was wrong too. **What is unknown, and is the probe:** on
the 895 unsolved seq-opt cells, how many get an orbit at all today, how
large the group is, and how many of the rest a PDG automorphism search
would find — detection narrates only under `FF_ORBIT_DEBUG` (not in the
notes), so the receipt needs ff on the cells (box time, after the Lane B
sweep) and a note line first. The lane opens only on that count; the
band and kill stand as written, read on `proven` only, with "A* with
orbits off is byte-identical" already the 0.22 contract.

**C.1 — abstractions + saturated cost partitioning.** Claim: a PDB
collection with SCP, added as an opt rung before LM-cut's A* (or as a
max with it), raises the proof count. Band: +40 to +80 proofs across the
seven opt boards (2008 157 → 175, 2011 140 → 160, 2014 82 → 100, 2018
94 → 105 as the shape). Kill: build time eats the wall on the median
cell (C.0's receipt), or proofs flat with the wall spent in A*.
Mechanisms, in order: explicit projections over the variable cover (a
PDB is a Dijkstra over the abstract space; sizes capped by a byte
budget under the memory wall); iPDB hill-climbing pattern selection;
saturated cost partitioning in a fixed order with the order chosen by
the diversification from the JAIR 2020 paper; Cartesian CEGAR
abstractions only if PDBs land and the receipt says the wall is left.
Fixtures: a PDB on a toy task equal to the known optimal abstract
distances; SCP admissibility on a two-heuristic example with a known
sum; A*'s certificate unchanged under the new h (every proof is
re-verified by the existing VAL path).

**C.0 / C.1 BUILT 2026-10-23 (engine-0.31, unmeasured — the sweep holds
the box).** `vars.rs`: the multi-valued cover from `invariants::synthesize`
(greedy disjoint, largest first, singletons, a `none` value per variable);
`FF_VARS_DEBUG=1` prints the cover's receipt after grounding on both
routes and `probes-0.31/opt/vars.py` runs it over the opt boards' cells
(C.0 (a)). `pdb.rs`: the projection onto a pattern — value tests for
preconditions, set/clear for effects, numeric dropped, conditional
effects on the pattern made nondeterministic (refused past three on one
op), goal values as the abstract goal — enumerated in a mixed-radix
index, reverse graph as CSR in two passes, one backward Dijkstra;
refused (never partial) past a state cap, an edge cap or a deadline.
`FF_OPT_PDB=1` builds the GOAL PATTERN (goal variables, smallest domains
first, under `FF_OPT_PDB_CAP`, 2^20) in `solve_optimal` under a quarter
of the remaining wall, installs it for every engine of the ladder, and
the A* max()es it with LM-cut / h^max (`heuristic` names `+PDB`); the
notes carry the build receipt — variables, abstract states, edges,
seconds — which is C.0 (b) and (c) in one line, or `pdb: not built
(why)`. Fixtures `tests/pdb.rs`: admissibility against exact distances
on the agent ring, the certificate unchanged with the table installed,
action costs honoured, a `when`-only route kept finite. Not built yet:
iPDB hill-climbing, the collection, saturated cost partitioning — judged
after the receipt over the seven opt boards says what the goal pattern
alone buys and costs.

**C.2 — symbolic bidirectional blind search.** Independent of any
heuristic; the strongest single technique on a different set of domains
from C.1 (the AIJ 2017 paper's own split). Needs a BDD package: price a
Rust crate against a small in-house one (the variable order is the
whole game — the SAS+ cover from C.0 is what it orders). Opens only
after C.1 is measured; its own band is written then, from the AIJ
paper's domain table against our opt loss lists.

**C.3 — NLM-cut** for numeric-opt (2023 numeric-opt 82/400, 2026
numeric-opt). Last, because the ceiling is low; the JAIR 2022 paper's
simple-condition subset matches most of the engine's "simple numeric"
class. Band written from C.0-style receipts on the numeric-opt loss
list.

**Measured:** the seven opt boards in full through the crucible
(`sweep --board ipc2008-seq-opt,…`), the proof count and `compare`
first-attempt; `max_rss` beside every row (0.30's finding: the opt
rungs now mem-cap where they timed out).

---

## Lane D — red-black relaxation (the principled consumption charge)

0.29's consumption charge bought +27 rows on 2023 numeric and lost five
cells that `FF_NO_CONSUME=1` restores — because h^FF cannot see a
consumable being spent, and the charge is a one-sided patch for it. The
literature's answer to exactly that blindness is **red-black planning**
(Domshlak, Hoffmann & Katz, AIJ 2015; Katz et al. AAAI / ICAPS 2013;
Mercury, IPC 2014 seq-sat runner-up): keep some variables "black" (real
semantics — the consumed ones), relax the rest. It needs the same
multi-valued view as Lane C, so it rides C.0's variable cover; if the
cover is cheap this is the satisficing payoff of the same groundwork.

**The mechanism, from the papers (read 2026-10-23; `katz-etal-2013-red-
black-relaxed-plan.pdf`, `who-said-relax-all.pdf`).** Over the variable
cover of `vars.rs`: (1) the DOMAIN TRANSITION GRAPH of each variable —
arcs (d, d') per op with eff[v] = d' and pre[v] = d or v unconstrained,
each arc labelled with the op's OUTSIDE condition (its other
preconditions) and OUTSIDE effect (its other effects); (2) a variable is
RSE-INVERTIBLE when every arc (d, d') has an inverse arc (d', d) whose
outside condition is contained in the forward arc's outside condition ∪
outside effect — testable per variable, polynomial; (3) the PAINTING:
candidates are the invertible variables; paint black a subset whose
black causal graph (arcs v → v' when some op has v in pre ∪ eff and v' in
eff) is ACYCLIC, every other variable red — the paper's strategies pick
black by causal-graph level or by how many relaxed-plan conflicts a
variable carries, and keep painting red until the cycles are gone; (4)
THE HEURISTIC, `UNRELAX`: take h^FF's relaxed plan in layer order
(`heuristic::linearize`); walk it keeping the BLACK variables' real
values and the red ones accumulating; before each action whose black
precondition is not met, insert an achieving sequence — a path in the
black variable's DTG restricted to arcs whose outside conditions are
already true (red: accumulated; black: current) — and likewise for the
black goal at the end; h = the length (cost) of the repaired plan.
Theorem 2 of the AAAI paper: with RSE-invertible black variables and an
acyclic black causal graph the repair always exists, in polynomial time.
Mercury's receipt (IPC 2014 seq-sat runner-up) is this heuristic in LAMA's
place; the HSDIP 2014 "DAG" variant paints more and is in
`icaps14-hsdip-proceedings.pdf`.

**Claim:** a red-black h on a small black set (the invertibility-based
painting from the AAAI 2013 paper; DAG painting from the 2014 HSDIP
paper) as a rung — or as LAMA's second heuristic — moves the delete-heavy
satisficing domains and the charge's five lost cells. **Band:** +10 to
+25 rows on 2014/2018 seq-sat plus the five; **kill:** per-evaluation
cost > 3× h^FF's without rows (Lane 6's receipt shape: evals/s solo,
one thread). Fixtures: the five charge-lost cells (`tests/
consume_transfer.rs` is already the shape, `#[ignore]`d); a red-black plan
on a two-variable task that the relaxed plan gets wrong by construction.
Opens after C.0; closes before the cut if C.1 is still building.

---

## Lane E — preference points from the candidate generator

Lane 1b closed flat with the diagnosis: a second polish round has nothing
to buy because the first already prices every candidate the search can
see. The mechanism is upstream of the polish. Two literature shapes to
probe against the three boards, with SGPlan5's per-instance metric as the
referee:

- **The compile is already in the engine** (checked 2026-10-23:
  `pddl3.rs` is the Keyder & Geffner compile since 0.4 — `collect_p` /
  `forgo_p` with the metric's weights folded into `total-cost`, the
  optimizer an anytime branch-and-bound over it, `solve_subgoal_bounded`).
  So the probe is not whether to compile; it is what SEARCH runs over the
  compiled task after the seed: today a B&B with a bound, then the polish
  over candidates.
- **Restarting anytime search** (Richter, Thayer & Ruml, ICAPS 2010 —
  RWA*; `richter-etal-2010-joy-of-forgetting.pdf`): restart from the
  initial state at each new incumbent with a lower weight, keeping the
  heuristic cache, instead of continuing the bounded search — the paper's
  finding is that continuing from a stale open list is what makes anytime
  search slow to improve. The candidate: RWA* over the compiled task in
  the optimizer's wall share, the polish after it as now. The probe reads
  the ten worst-ratio cells with the incumbent's metric per restart.

**Band:** complex 54.1 → 65 of 100, qualitative 64.3 → 72 of 92, simple
flat. **Kill:** Lane 1b's — points flat with the wall spent. Probe first
on the ten worst-ratio cells (trucks, tpp, pathways); a lane only on a
probe that moves five of them.

**Groundwork read, 2026-11-01 (banked 0.30.0 rows, the archive's
SGPlan5 metrics, `standings.py`'s own `_ipc_score`; no box time):** the
three boards sit 94 points behind SGPlan5 pairwise, and the split is
**30 coverage / 64 quality**. Coverage is two variants: storage-complex
(16 cells unsolved of 20) and tpp-complex (12 of 20). Quality is
concentrated: trucks-qualitative +13.8, tpp-qualitative +10.6,
trucks-simple +10.5, trucks-complex +5.8, pathways-simple +5.7,
pathways-complex +5.4, rovers-qualitative +4.2 — trucks alone is 30 of
the 64. The worst cells are all one shape: **SGPlan5 reaches metric 0
(every preference satisfied) and ours does not** — trucks-simple 6/14/
15/16/17, trucks-qualitative 4/5, tpp-complex 3, openstacks-simple 15/18
— so a zero-violation plan exists and the search over the compiled task
does not find it in the wall: Lane E's question exactly. Note what the
optimizer already does (`pddl3.rs`, the B&B loop): a diversified RESTART
ladder — four weight profiles tried in turn when a sweep stalls, under
the SAME bound, reset to the default on an improvement
(`FF_PREF_NO_RESTARTS=1` disables it). RWA* is the other way round:
restart on each IMPROVEMENT, from the root, with a LOWER weight, the
heuristic cache kept — the probe is that difference, not "add
restarts". Two cautions for the referee. (1) `pref_points` is pairwise against SGPlan5, which is right:
a best-of-field read is poisoned by MIPS-XXL's `.soln` headers, which
carry `MetricValue 0.00` on 18 openstacks-simple and 5 pipesworld-complex
plans whose bodies are ordinary 12–23 step plans (an unfilled header,
not a zero-violation plan) — never switch the referee to best-of-field
without dropping those. (2) We BEAT SGPlan5 on openstacks-simple
(−4.4), storage-qualitative, rovers-metric-simple and pipesworld-complex,
so the gap is not a uniform quality deficit; it is trucks/tpp/pathways,
where deadlines and ordering preferences make the compiled `total-cost`
landscape deceptive for a bounded B&B. The autopsy's SECOND half — which
preferences our trucks plans violate and whether the violation is the
seed's or the polish's — needs the plans themselves (ff runs; the notes already count satisfied
and violated preferences, the plan says which), i.e. box time, and should run on the
ladder Lane B leaves as the default, not on 0.30.0's.

---

## Probes only — recorded, never claimed this cycle

- **Constraints (28/120).** The IPC-5 constraints track's other
  medallists compiled the trajectory constraints to automata and gave the
  heuristic the automaton's progress (Baier, Bacchus & McIlraith, AIJ 2009;
  HPlan-P's IPC-5 abstract). The 0.30 probe (never run — Phase 0 and the
  lanes took the cycle) is this cycle's: on tpp / pipesworld constraints
  i1–i5 solo, the compiled task's shape, and whether an `always` /
  `sometime-before` compiles to something a landmark can see. A lane for
  0.32 only if the record names a mechanism.
- **Lifted search** (PowerLifted, Corrêa et al. ICAPS 2020; Levitron's
  half of the 2023 win). Demoted in 0.17 because the village grounds
  fine; the grounding class (0.6) is the first evidence either way.
- **Per-instance selection** (Delfi / the Hapori portfolios, IPC 2023).
  Lane A's table is per track class; a per-instance table on cheap
  features (ops, facts, goal count, variables) is the next step only if
  A lands and the matrix shows domains that want different orders.

---

## Anti-pots — priced at zero, standing

Everything 0.28–0.30 listed, plus:

- **An in-sample table.** Lane A's number is the held-out one; a table
  scored on the cells it was built from is a receipt of nothing.
- **A rung slice set by hand for a lane in this cycle.** B, C and D take
  their slices from the matrix; "it solved these six cells at 30 s so
  give it half the wall" is what killed Lane 6.
- **A proof count that includes plans without certificates.** Lane C's
  measure is `proven`; an A* that stops at a bound is a satisficing row,
  counted there.
- **An opt heuristic read without its build time.** The receipt is
  seconds before the first expansion, per cell; a heuristic that proves
  more on the cells it finishes building for and times out on the rest
  is read on the board, not on the cells.
- **Reopening the pinned knobs.** `FF_TCONC`, the memory-skip quarter,
  the consumption charge's semantics stay where 0.30 left them; Lane D
  adds a rung beside the charge, it does not re-litigate it.

---

## The order, and where the cut can fall

```
Phase 0 (cut30 closed; canary; demotion; text wall; MATRIX harness; grounder receipt)
   → Lane A  (table; held-out; subset sweep)              ← cut possible here
   → Lane B  (BFWS f5 + k-BFWS + forgetting; matrix row; sweep)   ← cut possible here
   → C.0 probe (variables; one PDB; build-time receipt)
   → Lane D  (red-black on C.0's cover)   ‖  Lane C.1 (PDB + SCP)   ← cut possible here
   → Lane E probe → lane
   → C.2 / C.3 as the receipts allow
   → the cut
```

**Re-ordered 2026-10-31 on the matrix read** (Lane A's record above: the
ladder at 94 % of its rungs' ceiling, the table a negative, the unsolved
pools autopsied — opt dies on the wall at 1.8 GiB median, numeric-opt in
memory at 4.9 GiB median, mco on the wall with threads splitting only
h-evaluation):

```
Phase 0 done (the matrix is the instrument; the table recorded)
   → Lane B's rungs through the matrix (lamanov / front / driverx; unique solves)
   → Lane A' parallel rung portfolio on the mco boards (opt-in mode; subset sweep)  ← cut possible here
   → Lane C, the headline:
        C.0 probe (variables; one PDB; build-time receipt)
        C.s symmetry pruning in A* from orbits.rs (built, unused there; the free rung)
        C.1 PDB + SCP                                                          ← cut possible here
        C.3 NLM-cut stays last (the field's leader is 102/400 at the official budget:
            the numeric-opt 317 die in memory, but the ceiling there is low)
   → Lane D (red-black on C.0's cover)  ‖  Lane E probe → lane
   → the cut
Beside the cycle, a product track: the re-entrant Thinker (actor → resumable → real-time),
measured in milliseconds per think, never in rows — docs/thinker.md.
```

The cut may be taken after Lane A, after Lane B, or after D/C.1, at the
operator's call; what has landed is published as 0.31 and the rest moves
to 0.32's roadmap with its receipts. The point of the order is that each
cut is a real one: the ladder table alone is a release, and so is the
table plus a rung measured under it.

## The cut rule

The 32-board `cut31` set, the same instrument; the three declared
measures read on it; the cells 0.30.0 lost and every lane's named cells
re-run on both engines before any is named; `recheck` is the referee's
job. The baseline for `compare` is the banked 0.30.0 engine, never a
rebuild. Nothing is published until `main` has it and the pre-flight —
with the token's age on the checklist (the crates.io token dates from
2026-09-29, ~90-day limit, so it expires around 2026-12-28 and a cut
after that needs a new one first) — is green.
