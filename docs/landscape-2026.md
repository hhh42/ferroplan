# The modern planning landscape, 2026 — the 0.17 frontier memo

The 0.17 Phase 1 deliverable (`docs/roadmap-0.17.md`): where the
field went after the competitions ferroplan grew up against
(IPC-5/6/7, 2006–2011), what the winners actually run, and the
RANKED list of engine gaps — each with a mechanism sketch, the
evidence it wins, and an honest in-THIS-engine cost paragraph.
Phase 3 swings at the top of this list; Phase 2's baseline sweeps
are its referee.

## The field, competition by competition

**IPC 2014** (66 track-variant directories now local via
potassco/pddl-instances — including a sequential-MULTI-CORE track,
so ferroplan's mco entry extends backward too). Satisficing was won
by portfolio planners (IBaCoP family) over the LAMA-2011 baseline —
the era's lesson was PORTFOLIOS, which ferroplan already has
(`FF_PORTFOLIO`, budget-aware since 0.9).

**IPC 2018** (12 satisficing domains local: agricola, caldera,
data-network, flashfill, nurikabe, organic-synthesis, settlers,
snake, spider, termes, ...). Fast Downward Stone Soup won
satisficing; **BFWS(pref) won agile and BFWS variants were
satisficing runners-up** — the breakout idea:
**width/novelty-based search** (Lipovetzky & Geffner). Novelty of a
state = the size of the smallest atom tuple appearing for the first
time along the search (w=1: some single atom is new; w=2: some
pair). BFWS orders the open list by ⟨novelty, unachieved-goals⟩
with heuristics as tie-breaks; novelty is computed RELATIVE to a
partition (goal count, relevance count), which keeps the tables
small and the signal sharp. Polynomial variants (k-BFWS) prune
w>k outright and still solve a startling fraction of the corpus —
exploration structure, not heuristic accuracy, is doing the work.

**IPC 2023 classical** (7 new domains local, with official
per-instance reference PLANS and a `bounds.json` of best-known
costs: folding, labyrinth, quantum-layout, recharging-robots,
ricochet-robots, rubiks-cube, slitherlink). Satisficing AND agile
won by **Scorpion Maidu** (Scorpion + width search "with
forgetting" — novelty tables periodically reset) and **Levitron**
(Scorpion Maidu + **PowerLifted**, a LIFTED planner, in portfolio);
DALAI (disjunctive action landmarks) took a track as well. The
organizers' retrospective names the field's biggest struggle as
PDDL FEATURE SUPPORT — quantifiers, disjunctions, `imply`, negative
goal conditions — which is a ferroplan STRENGTH (the 0.10
DNF-static fix; full ADL on the 2008 openstacks board). Two of the
three winning ingredients are ideas ferroplan lacks (novelty,
lifted search); the third (strong classical heuristics/portfolios)
it has in kind.

**IPC 2023 numeric** (20 domains local with official sat/opt result
CSVs: counters, farmland, sailing, drone, expedition, hydropower,
markettrader, settlersnumeric, sugar, zenotravel, fo-* linear
variants, ...). Swept by **NLM-CutPlan** (Kuroiwa, Shleyfman,
Beck): numeric LM-cut — landmark-cut generalized to simple numeric
conditions/effects (linear expressions; constant-delta effects) —
over Numeric Fast Downward, with an **Orbit** variant (symmetry
orbit-space search — the same idea family as ferroplan's 0.14
orbits, validating that direction from the optimal side). The
satisficing baseline of record is ENHSP (interval-based relaxation
/ subgoaling heuristics, Scala et al.); Kuroiwa's lazy greedy BFS
with subgoaling relaxation is the satisficing-side sibling. The
"simple numeric" class covers most RPG resource math — quantities
and money moved by constant or recipe amounts — so this track's
heuristics are the village's heuristics.

## The ranked gap list

1. **Novelty/width signal in the classical ladder** (BFWS-class).
   Mechanism: per-state novelty against seen-atom tables
   partitioned by (unachieved-goal count[, relevance count]); order
   or prune by it; reset-on-restart ("forgetting") keeps tables
   honest across rungs. Evidence: agile winner 2018, inside both
   2023 winners; the single most proven post-LAMA satisficing idea.
   In-engine cost: MODERATE — a novelty table beside the visited
   set (facts are already dense bit-indices; a w=1 table is one
   bitset per partition cell, w=2 capped or skipped), a new rung in
   the classical ladder (the portfolio/ladder plumbing exists), no
   changes to h^FF. The going-in favorite, confirmed.
2. **Numeric heuristic upgrade** (subgoaling/AIBR class, NLM-cut's
   satisficing siblings). Evidence: the entire 2023 numeric podium;
   ferroplan's own audit (metric-time-2006 55/200; model-train-t
   0/30 last-mile-numeric wall — the SAME shape the 0.15 probe
   named). In-engine cost: SIGNIFICANT — a second numeric
   relaxation beside the FF-extension h (interval propagation per
   fluent; subgoal decomposition of comparisons), engine-visible on
   both corpora and the village. The 2023 numeric baseline sweep
   prices the distance first.
3. **Lifted / lazily-grounded search** (PowerLifted-class).
   Evidence: Levitron's winning half; the entire big-object
   problem class the village lives in. In-engine cost: LARGE
   (successor generation over schemas via joins instead of ground
   op tables) — priced by Phase 1's big-catalog stress test before
   any commitment; the cheap intermediate (lazy/on-demand
   grounding within the current architecture) may capture most of
   the village's need.
4. **Deferred evaluation + open-list alternation** (LAMA/FD
   machinery). Evidence: two decades of FD satisficing. In-engine
   cost: SMALL-MODERATE — evaluate-on-pop instead of on-generate
   under a flag; alternation between h^FF and novelty queues pairs
   naturally with bet #1. A supporting bet, not a headline.
5. **Dynamic derived predicates** (axioms). Evidence: several
   modern domains lean on them; ferroplan grounds static/stratified
   only. In-engine cost: MODERATE and long-deferred; goes in only
   if the Phase 2 sweeps show concrete coverage priced against it
   (the failure classifier will say).

## Assets now local (fetch scripted in `benchmarks/get-ipc.sh`)

- IPC 2014 (66 variants, potassco mirror) — includes seq-agile,
  seq-sat, seq-mco.
- IPC 2018 sat (12 domains, official bitbucket) + `cost_bounds.json`.
- IPC 2023 classical agl+opt (7 domains) + official reference
  plans per instance + `bounds.json`.
- IPC 2023 numeric (20 domains) + official sat/opt result CSVs.
- (Vendored earlier: the official IPC-5 results archive.)

Quality references therefore exist for the 2018/2023 classical and
2023 numeric sweeps from day one — no coverage-only asterisks on
the new standings tables except where WE fail to record the
currency (the makespan runner debt, still open).

## The big-catalog stress test (the village priced before it is built)

`benchmarks/bench/gen_catalog.py`: ONE gather rule + ONE make rule,
the whole catalog as static init data (the game's exact contract),
recipes a layered binary DAG, goal = the top item. `make` is
syntactically N³; a grounder that resolves the static needs1/needs2
joins grounds ~N ops.

**Monotone variant (pure grounding pressure):**

| N items | wall | peak RSS |
|---|---|---|
| 100 | 0.36 s | 10 MB |
| 1,000 | 0.35 s | 10 MB |
| 3,000 | 3.3 s | 15 MB |
| 10,000 | 37 s | 42 MB |

Static resolution holds — the shape is ~quadratic (not cubic), and
a 10,000-kind catalog grounds and solves in 37 s at 42 MB. A
REALISTIC village (hundreds of item kinds) grounds in well under a
second. **Verdict on gap #3: lifted search is NOT the village's
blocker** — demoted to a watch item (the quadratic term earns one
profile look in some later cycle).

**Consume variant (inputs deleted — the game's real semantics):**

| N items | wall | note |
|---|---|---|
| 30 | 0.00 s | len 111 |
| 100 | 1.5 s | len **895** — wandering re-gathering |
| 300 | TIMEOUT 60 s | the wall |
| 1,000 | TIMEOUT 120 s | — |

**This is the village's actual blocker, measured before the village
exists**: h^FF's delete relaxation reuses every gathered item
infinitely, so consumption makes the heuristic near-blind — the
search re-gathers in circles (the 895-step plan at N=100) and walls
by N=300. The fix class is exactly bet #1 (novelty-driven
exploration breaks h-plateaus without needing h accuracy) plus the
existing 0.15 trip-bound resource machinery (`FF_RESLM`) extended
to consumption counting. The ranking above is therefore MEASURED,
not just literature: the novelty rung is both the field's proven
idea AND the village's needed one.
