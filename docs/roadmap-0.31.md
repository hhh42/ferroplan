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
   cut30. Read the demotion rule against the pack tiers; a demoted row that
   the predecessor also failed banks `hot` like a thermal one.
0.4 **Text-mode wall arming** (engine, one line + fixture): `run_planner`
   grounds and tries `resolve::solve` before `crate::solve` arms
   `FF_TIME_LIMIT`; arm it at entry. Fixture: text mode, armed wall,
   returns inside it. (Owed from 0.30, "Probes only".)
0.5 **The per-rung matrix harness** (crucible + engine) — Lane A's
   instrument, built here because every later lane reads it too. Engine:
   `FF_RUNG_ONLY=<ehc|lama|novlight|novelty|bfs|partition|portfolio:<m>>`
   runs ONE rung to the full wall and reports `{rung, solved, time,
   evaluated, peak_rss}`; the knob already half-exists (`FF_NOVELTY_ONLY`,
   `--search bfs`, `--mode partition`) and is made uniform. Crucible: a
   `matrix` set that runs a stratified sample through each rung-env and
   stores rows keyed by `(cell, rung)` beside the engine hash. Sample:
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
  loss side larger than its gain. **Code written 2026-10-31**, unbuilt
  while the Lane B sweep holds the box: `parallel.rs` (members by thread
  count: ladder → driver → LAMA → bfs → EHC → novelty-light → lamanov →
  front, the matrix's marginal order beside the ladder — 683 / 694 / 701
  of 706 at two, three, four workers), the ladder factored into `ladder()`
  behind `plan_avoiding`'s gate, the hatch's rungs as `search::Rung`,
  novelty-light given the hard checkpoint the other rungs poll,
  `tests/parallel_rungs.rs`. Measure: build, fixtures, then the six mco
  boards through the crucible with `--env FF_PARALLEL_RUNGS=1`, compared
  against the banked 0.30.0 rows.
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
  the matrix (launched 2026-10-31 07:40, `probes-0.31/matrix/
  run-matrix-b.sh`), each read on its unique solves against the five.

---

## Lane B — a second signal: BFWS done the winners' way

**Claim:** a width-based rung in the configuration that won IPC 2018 agile
and sits inside both IPC 2023 classical winners moves the modern-corpus
timeouts. **Band:** 2018 seq-sat 99 → 115+, 2014 seq-sat / agile +10 each,
2023 seq-sat / classical +5 each — +25 to +60 rows. **Kill:** 0.17's —
budget-edge losses exceed gains under Lane A's table (not under a hand
slice; that question is answered) → the rung stays opt-in, recorded.

What `novelty.rs` is today, by its own header: width-1, partition
⟨unachieved-goal count⟩ only, after the LAMA rung, opt-in because of the
budget-edge tax. What the literature's winning configuration is:

- **BFWS(f5)** (Lipovetzky & Geffner, AAAI 2017): open list ordered
  ⟨w, #g, h⟩ with novelty w computed **relative to ⟨#g, #r⟩**, where #r is
  the number of atoms of the relaxed plan from the initial state reached
  so far — h^FF's relaxed plan, which the engine already computes on every
  evaluation. w ≤ 2 with the w=2 table capped (Singh et al. 2021,
  *Approximate novelty search*, for the Bloom-style approximation that
  keeps w=2 affordable).
- **Dual / k-BFWS** (Francès et al., IJCAI 2017): a polynomial 1-BFWS pass
  first (prune w > 1), then 2-BFWS, then the full search — the cheap
  passes solve a startling fraction and cost little.
- **Forgetting** (Scorpion Maidu, IPC 2023 abstract): the novelty tables
  reset periodically / on restart so a long search does not go blind.
- The FD-style variant (Katz et al., ICAPS 2017, *Adapting novelty to
  classical planning*) for the version that lives inside a LAMA-like
  ladder — the shape ours has.

**Mechanisms, in order:** (1) #r from the relaxed plan, the ⟨#g, #r⟩
partition, w ≤ 2 with a capped table, in `novelty.rs` on the flat arena
(0.30's `search_light` layout — the 0.17 rung died of memory at 12.5 M
nodes; the arena is why this is affordable now); (2) the k-BFWS prune pass
as its own rung ahead of the full one; (3) forgetting on a node-count
period. Each is a `FF_NOV_*` knob with the 0.30 behaviour as the hatch.
**Fixtures:** a plateau task h^FF cannot cross that 1-BFWS crosses (the
catalog-consume fixture's shape, which the header names); novelty-table
bytes per node pinned; determinism at any thread count (the rung's
existing contract). **Measured:** solo node-cost receipts on city-car,
child-snack, maintenance, woodworking i15/i16 (the 0.30 cells); then the
matrix sample for the new rung (one night) → Lane A's table →
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

**C.s — symmetry pruning in the optimal A* (added 2026-10-31, the free
rung).** `orbits.rs` already detects the task's structural symmetries
and canonicalizes states, and the satisficing best-first already dedups
on the canonical form; `optimal.rs` never reads it. Orbit search (Pochter,
Zohar & Rosenschein AAAI 2011; Domshlak, Katz & Shleyfman ICAPS 2012,
*enhanced symmetry breaking in cost-optimal planning*) is the same dedup
under A* — sound for the certificate as long as the plan is reconstructed
through the permutation trace, which is the one piece to build. The
autopsy that motivates it: the 895 unsolved seq-opt rows die on the
**wall** at 1.8 GiB median RSS, not in memory — the search is too wide
for the heuristic, and symmetry is the only width reduction that costs
no build time (C.0's receipt does not apply). Band: proofs +15 to +40 on
the domains the 2012 paper names (gripper, logistics-shaped, the
permutation-heavy 2008/2011 sets), read on `proven` only; kill: flat, or
any certificate VAL rejects. Fixtures: a toy with a known orbit count
expands exactly the orbit representatives; the reconstructed plan replays
under VAL; A* with orbits off is byte-identical to today.

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

- **Compile soft goals away** (Keyder & Geffner, JAIR 2009) and run the
  cost-aware anytime search the engine already has for net-benefit (where
  it stands at 270/270) — one search over the compiled task instead of
  seed → polish; the question is whether the compiled task's cost
  landscape beats the polish's candidate enumeration on complex.
- **Restarting anytime search** (Richter, Thayer & Ruml, ICAPS 2010 —
  RWA*) over the compiled task, the LAMA paper's anytime loop with
  restarts, so the wall after the first plan improves it rather than
  re-polishing.

**Band:** complex 54.1 → 65 of 100, qualitative 64.3 → 72 of 92, simple
flat. **Kill:** Lane 1b's — points flat with the wall spent. Probe first
on the ten worst-ratio cells (trucks, tpp, pathways); a lane only on a
probe that moves five of them.

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
