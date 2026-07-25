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

## Phase 3 — engine bets, memo-ranked (measured, per bet)

- Top-of-list bets from the Phase 1 memo get the cycle's swings —
  each fixtures-first, measured win or recorded negative, standard
  budgets, zero-regression rule intact, hatches for every default
  flip. The novelty rung is the going-in favorite; the memo can
  overrule it.

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
