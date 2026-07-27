# ferroplan 0.17 roadmap — the frontier cycle

Scope set 2026-07-24, mid-0.16, by direct request. The goal, stated
plainly: **be the best PDDL planner in general** — and the reason,
equally plainly: the planner serves a village-scale RPG simulation
(developed in a separate project) whose domain logic is ABSTRACT —
one pickup rule, one make rule, one hire rule, with parameters
carrying what kind of item, tool, skill, and price. General
excellence and the game are the same bet: a village of craftsmen is
a big-object, numeric, temporal, multi-mind planning workload, and
every gap the modern corpus exposes is a gap the game will find too.

Four framing decisions, locked by direct answers:

1. **Corpus expansion: IPC 2014 / 2018 / 2023 classical + IPC 2023
   numeric.** HTN (HDDL) and probabilistic (RDDL) are OUT — different
   input languages, different engines; a second front we're not
   opening.
2. **Ferroplan owns the abstract RPG core.** The village domain
   (rules, reference catalog, fixtures, benchmark, demo) lives here
   as a first-class domain; the game project consumes and extends it
   with content packs.
3. **Contracts stay behind the cross-mind fence.** Hiring is the
   proven bazaar pattern — the loop spawns/retargets a worker's
   `Session` with a goal contract; claims + observation coordinate.
   Planner-native negotiation remains rejected.
4. **Visualization: the village live page AND plan introspection
   views.** Search introspection stays probe-side.

Runs after 0.16 closes (standings cycle: audit → committed raises
including the qualitative-tpp selection extension → standings docs →
cut).

## What the first research pass already established (sources on file)

- **IPC 2018 classical**: Fast Downward Stone Soup won satisficing;
  **BFWS-Preference won agile** and BFWS was satisficing runner-up —
  novelty/width-based search is THE proven post-LAMA satisficing
  idea, and ferroplan has no novelty signal anywhere in its ladder.
  (ipc2018-classical.bitbucket.io; Francès et al., "Best-First Width
  Search in the IPC 2018".)
- **IPC 2023 classical**: Scorpion Maidu and Levitron won
  satisficing; the organizers' retrospective (Taitler et al., AI
  Magazine 2024) names the field's biggest struggle as **PDDL
  feature support** — quantifiers, disjunctions, `imply`, negative
  goal conditions. That is a ferroplan STRENGTH (the 0.10 DNF-static
  fix took openstacks-ADL 6/30 → 30/30); showing up with full ADL
  may be worth real coverage against the modern field.
  (ipc2023-classical.github.io; dataset:
  github.com/ipc2023-classical/ipc2023-dataset.)
- **IPC 2023 numeric**: NLM-CutPlan variants swept every subtrack;
  **ENHSP** (Scala et al. — interval-based relaxation / subgoaling
  heuristics) is the baseline system of record. Ferroplan's numeric
  h is an FF extension; this track is the honest judge of it.
  (ipc2023-numeric.github.io.)
- Corpus reachability from this container: github.com hosts the 2023
  datasets and potassco/pddl-instances (through IPC 2014); the 2018
  corpus lives on bitbucket (reachability to verify in Phase 1).

Candidate engine bets these facts nominate (Phase 1 RANKS them,
Phase 3 swings at the top of the list — hypotheses, not
commitments):

- **Novelty/width rung** (BFWS-class): a novelty measure beside
  h^FF in the ladder — the one proven idea the engine entirely
  lacks.
- **Deferred evaluation + multi-queue alternation** (LAMA/FD
  machinery): standard wins on large branching factors; ferroplan
  evaluates eagerly on one queue with helpful-action bias.
- **Numeric heuristic upgrade** (AIBR/subgoaling class): the
  metric-time-2006 55/200 and the model-train last-mile mechanism
  already point the same direction the 2023 numeric track measures.
- **Lifted / lazily-grounded search** (PowerLifted-class): abstract
  rules × village-scale object catalogs = exactly the grounding
  blowup lifted planning exists for. This bet is the game bet.
- **Dynamic derived predicates**: the recorded limitation; several
  modern domains lean on axioms.

## Phase 1 — the landscape memo (research with receipts)

- Deep pass over the modern satisficing/agile/numeric literature and
  the three competitions' per-domain results; deliverable
  **`docs/landscape-2026.md`**: per-idea mechanism sketch, evidence
  of wins, and an honest "what it would cost in THIS engine"
  paragraph — the ranked gap list Phase 3 obeys.
- Fetch the corpora (2023 classical + numeric from github; 2014 via
  potassco; 2018 route verified or mirrored), extend `get-ipc.sh`,
  dry-enumerate every track with `ipc67.py --list`-class eyes.
- The abstract-rule stress test: measure ferroplan's grounding and
  search on synthetic big-catalog instances of the EXISTING rpg
  example (one make rule × N item types × M objects) — the first
  honest read on where village scale breaks the engine, BEFORE the
  village domain is built.

### Recorded — the memo is written, the corpora are local, and the stress test renamed the bet

`docs/landscape-2026.md` delivered with the ranked gap list; all
four corpora fetched and verified (IPC 2014 via potassco — 66
variants including a 2014 seq-mco track; IPC 2018 sat — 12 domains
+ official cost bounds; IPC 2023 classical — 7 domains + official
reference PLANS per instance + best-known bounds; IPC 2023 numeric
— 20 domains + official result CSVs). Quality references exist for
every new corpus from day one.

The big-catalog stress test (fixture:
`benchmarks/bench/gen_catalog.py`) settled the rankings with
numbers — after correcting ITSELF once, on the record: **grounding
is NOT the village's blocker** (10,000-item catalogs ground+solve
in 37 s / 42 MB; lifted search demoted to a watch item), and the
first-draft "consumption wall" (wandering 895-step plans, N=300
timeout) turned out to be a FIXTURE artifact — the draft DAG grew
deeper with N, making minimal plans exponential under consumption.
Depth-capped to the shape real crafting has (4 layers, width
scaling), the consumable village profile solves in 2.65 s at
N=3000: **nothing blocks the village at realistic scale.** The
novelty rung keeps its Phase 3 slot on the FIELD case (the
2018/2023 corpora, where it lives inside every modern winner); its
referee is the corpus A/B, and the fixture lesson — depth is the
enemy, width is not — goes straight into the Phase 4 village
design.

## Phase 2 — first standings on the modern corpus

- Sweep IPC 2014/2018/2023 satisficing (+ agile timing discipline
  where the track defines it) and IPC 2023 numeric satisficing at
  standard budgets, VAL on everything; extend the standings tables;
  classify every failure (feature gap / search wall / budget edge /
  mem-cap) exactly as the 0.16 audit does for the older corpus.
- Expectation set honestly: the modern field is Fast-Downward-class
  engines with two decades of satisficing machinery; the first sweep
  is a BASELINE, not a challenge. The deliverable is knowing the
  distance, per domain, with the failure classes named.

### Recorded — seven tracks entered, the distance known, four red flags decoded

All seven sweeps ran at standard budgets against the 0.16.0-behavior
binary (VAL on everything; full rows in `ipc-standings.md`,
generated): IPC-2014 seq-sat **95/280**, seq-agile **94/280**,
tempo-sat **42/200** (+23 VAL-RED — see below), seq-mco t4 entered;
IPC-2018 sat **30/240** with the first bounds-scored quality column
(0W/1T/13L vs best-known, mean 0.72); IPC-2023 classical **26/140**
at the 60 s baseline budget (bounds quality 0W/11T/15L, mean 0.88 —
ties on 11 of 26!); IPC-2023 numeric **112/400**, ferroplan's first
number on the modern numeric track. The distance to the modern
field is now measured, not imagined: 2018-hard domains (agricola,
organic-synthesis, snake) are where two decades of FD machinery
show; the numeric track lands right where the memo's #2 bet
(subgoaling-class numeric h) points.

**The VAL-RED classes, solo-probed and decoded, one per cluster:**

- **drone-numeric (16) and data-network-2018 (8): VAL-side, not
  engine.** VAL's parser rejects both DOMAINS outright ("Parser
  failed to read file" / "Problem in domain definition!") — our
  plans never got judged. Classified harness-gap; the runner keeps
  them VAL-RED rather than quietly counting them solved (honesty
  over coverage), with the note that a VAL upgrade or per-domain
  exemption is runner work, not engine work.
- **match-cellar-2014 (20) and map-analyzer-2014 (3): a REAL
  engine bug, named to the mechanism — the ε-EMISSION ORDER
  INVERSION.** The 2014 match-cellar's tight 5/2 packing makes a
  mend's end and its match's end land on the SAME internal epoch;
  the tie-scan legally fires mend-end before match-end (the
  kiln-gap machinery working as designed) — but the ε-separation
  EMISSION pass staggers the STARTS, so the emitted mend end
  (start+ε+2.0) crosses the emitted match end by exactly ε, and
  VAL reads the invariant as broken for 0.001. Internally sound,
  emitted unsound: the pass must preserve internal same-epoch
  order for ends riding on shifted starts. Same family as the
  ε-separation mutex gaps that led the 0.14 extension; **named
  correctness debt, leads 0.18** — not rushed into this cut's
  temporal engine (a fix invalidates every temporal board and
  demands full re-sweeps).

## Phase 3 — engine bets, memo-ranked (measured, per bet)

- Top-of-list bets from the Phase 1 memo get the cycle's swings —
  each fixtures-first, measured win or recorded negative, standard
  budgets, zero-regression rule intact, hatches for every default
  flip. The novelty rung is the going-in favorite; the memo can
  overrule it.

### Recorded — the rung is built; the referee sweep closes the phase

**The novelty rung ships** (`crates/ferroplan/src/novelty.rs`):
width-1 BFWS-style greedy best-first — open list ordered by state
novelty first (a fact never seen in the unachieved-goal-count
cell), h second — with the LAMA rung's proven skeleton (dual
preferred/normal heaps, deferred parent-h, deterministic funnel,
t1 ≡ t8 pinned by test) as the classical ladder's THIRD bounded
rung: it runs only after EHC and LAMA both give up, so it can only
add coverage per instance. `FF_NOVELTY_ONLY=1` probes it. Two
design lessons paid for and recorded: a FINER novelty partition
(an early draft added parent-h to the cell) makes nearly
everything novel and measurably degenerates to plain h-greed
(byte-identical plans), and the fixture that motivated the rung
corrected itself (Phase 1 record) — so the rung's REFEREE was the
corpus A/B, novelty binary vs the committed baselines across five
classical boards.

**The referee's verdict: OPT-IN (`FF_NOVELTY=1`), by the gen-skip
arithmetic.** Per-instance "can only add coverage" is not
per-BUDGET: the rung burns up to 400k evals of wall time ahead of
the complete fallback, and at wall-clock budgets that tax priced
out every budget-edge instance that used to fall through and
solve. The full diff: 2018-sat **+3/−1** (39 vs 38), 2023
classical **0/0**, 2014-sat **0/−6**, seq-sat **+1/−34** (408 vs
441!), prop-2006 **+3/−10** — **+7 gained, −51 lost**. The gains
are REAL — six instances where h dies outright and novelty-first
exploration finds the door — and stay reachable via the flag; the
tax is structural. Why the LAMA rung survives the identical
structure: its win rate carries its tax; novelty's does not, on
these corpora. The recorded next idea (0.18+, not this cycle): a
BUDGET-AWARE ladder — spend the novelty rung only when the
remaining wall budget affords it, or interleave rungs on one clock
instead of sequencing them — which is also exactly what the agile
track's scoring rewards. With the flag off the classical path is
byte-identical to 0.16.0, so every standing scoreboard remains the
cut's scoreboard.

## Phase 4 — the village (the abstract core, owned here)

- **The domain**: abstract verbs only — `pickup`, `make`, `hire`,
  `sell`-class rules whose parameters (item kind, tool requirement,
  skill, consumed inputs, produced outputs, price) carry ALL the
  content; recipes/catalogs are INIT DATA, not new actions. Numeric
  fluents for quantities and money; durative where labor takes time.
- **The fixtures ladder**: lone craftsman (gather → craft → sell) →
  toolchain workshop (tools made of parts made of materials — deep
  make-graphs) → full village (N craftsmen of different trades, a
  marketplace, hired labor via Session goal contracts, the bazaar
  loop as the world driver). Each rung a fixture + test + measured
  scoreboard entry (evals, grounding size, tick latency).
- **The point of the scaling rung**: it feeds Phase 3's
  lifted/lazy-grounding evidence directly — the village IS the
  big-object benchmark.
- The game project consumes this domain; content packs extend the
  catalogs without touching the rules.

### Recorded — the village stands, three rungs deep

**`benchmarks/village/`** ships the abstract core exactly as
committed: ONE gather / make / buy / sell (plus walk and
pickup-tool), every piece of content — items, recipes with
required-vs-consumed quantities (`req1/req2` vs `qty1/qty2`; a
fixture input like a chisel has req 1, qty 0 — the qty-only first
draft let carving happen without a chisel, caught and fixed), tool
and station gates, prices, travel times — as INIT DATA. Rung 1
(craftsman: tools → gather → two-recipe chain → market, makespan
21) and rung 2 (workshop: buy iron, forge the chisel carving
requires, makespan 61) solve on defaults, tests pinning the forced
chain structure. Rung 3 (`examples/village.rs`) demonstrates the
HIRE mechanism: workers fork the one grounded world scoped to
their own labor, a hire is `set_goal`, a re-hire is another —
mara's decoy contract correctly forces her through the
iron-buy/forge/carve chain. The scaling dimension is priced by the
depth-capped catalog stress test (width to N=3000 comfortable).

**Finding for the fence ledger**: the communal village exhibits the
START-CREDIT PLATEAU in miniature — h^FF pays for each GATHER
start on firing, gather-spam floods the pruned pass (19 pending
gather intervals on one popped node), and a 200k think dies where
a 1M think sails. The h-surgery bet (end-gated interval credit)
now has a GAME-SHAPED witness next to TMS on its file.

## Phase 5 — the screens (severable)

**SEVERED to 0.18, as one coherent deliverable**: the village live
page needs the live tick-loop village underneath it (the thing
rung 3 deliberately did not build), and shipping the loop and its
page together — with plan introspection views beside them — is one
experience, not two halves. The severance is the roadmap's own
marking ("severable") exercised, not a cut corner: rung 3 proves
the hire mechanism; the page shows it moving, next cycle.

## Phase 5 — the screens (severable)

- **Village live page**: bazaar-live's successor — map + timeline of
  the economy, craftsmen with visible intentions (their current
  plan), stock and money flows, contracts in flight, the steal
  button's descendants (disrupt a delivery, poach a worker).
- **Plan introspection views**: for any solved instance — temporal
  Gantt (intervals, ε-orderings, invariant spans), classical causal
  chain, preference satisfaction/violation breakdown. Makes the
  planner legible beyond this repo.

## Phase 6 — cut 0.17.0

The standing template: scoreboards (old AND new corpora) against the
final binary with A/B attribution, casualties named and solo-checked,
mem-cap separate, records complete, full pre-flight, finish in main;
the user publishes.

## Deferred, on the record

- **HTN (HDDL) and probabilistic (RDDL) tracks**: rejected by direct
  decision — different languages, second front.
- **Planner-native multi-agent / cross-mind planning**: the fence
  holds; Sessions + goal contracts is the chosen mechanism.
- The 0.15/0.16 carried list (h-surgery end-gated credit, transport
  route-structure fence, continuous `#t`, fixpoint/stratified
  unification, belief-aware dormancy) — unchanged.
