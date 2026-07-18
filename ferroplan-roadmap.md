# Ferroplan Roadmap — Toward an RPG Planning Brain with IPC6/IPC7 Coverage

**Audience:** the coding agent working in this repository.
**North star:** make Ferroplan the best possible planning engine for a
Rust RPG's decision-making, while picking up strong, honest coverage of
the IPC6 (2008) and IPC7 (2011) problem spaces along the way.

> **Revision note.** The original draft of this document was written
> without access to the codebase and hedged on two "unknowns." This
> revision is audited against the code at v0.8.0: the unknowns are
> answered inline, phases that assumed greenfield work are re-scoped to
> what actually remains, and one phase (temporal) turned out to be
> largely shipped already.

---

## How to use this document

Work the phases roughly in order. **Phase 0 is still first and gates
the rest**, but it is smaller than originally drafted — much of its
scaffolding already exists. What remains of it is real, though:
external validation, the IPC6/IPC7 benchmark sets, and `STATUS.md`.

Each phase has: a goal, why it matters (with the RPG angle called out),
what already exists, concrete tasks, and acceptance criteria. Treat the
acceptance criteria as the definition of done. After each phase, update
`STATUS.md` (see Phase 0) so the next run of the agent — or a human —
can pick up cleanly. (Repo convention: per-release phase records live
in `docs/roadmap-<version>.md`; keep doing that too.)

Two rules that hold across all phases:

1. **Never regress the IPC5 baseline.** Ferroplan is competitive with
   SGPlan5 on IPC5 (see `benchmarks/ipc5-scoreboard.md` and
   `benchmarks/ipc5-qualitative-scoreboard.md`). CI already runs heavy
   IPC regression guards (`espc` / `ipc5_pref_metric`, `#[ignore]`d
   locally, exercised in a release CI step) — keep them green, and
   extend them as coverage grows.
2. **Validate every plan.** Ferroplan has an internal validator
   (`plan::validate_plan`, CLI `--validate`) under its own semantics.
   For anything we *claim* in a scoreboard, add external machine-checking
   with VAL. No self-graded plans in published numbers.

---

## Strategic context (read before starting)

Ferroplan today is strong at **satisficing planning with preferences and
trajectory constraints** — the PDDL3.0 world that IPC5 rewards. Moving
toward IPC6/IPC7 is not "add more domains." It is crossing into
**cost-aware heuristic search**. The competitive frontier there is the
Fast Downward / LAMA family plus portfolios — not SGPlan. The language
delta is small (IPC6's PDDL3.1 mainly adds action costs and optional
object fluents), and — good news from the audit — the *engine* delta is
smaller than the original draft feared: Ferroplan is already an
FF-family heuristic-search planner, so cost-sensitivity is an evolution
of the existing heuristic and search, not a paradigm change. What it is
**not** is a finite-domain (SAS+) planner — and that is a deliberate
architectural choice, not a gap (see Phase 1).

The two things that matter most for an RPG — **action costs** (so NPCs
weigh risk/time/resources) and **net-benefit / oversubscription
planning** (so a resource-bounded NPC chooses *which* goals are worth
pursuing) — are exactly the IPC6 additions. So the RPG direction and the
IPC6/7 direction reinforce each other.

### The two unknowns, answered (audit of v0.8.0)

- **Grounding representation: propositional, on purpose.** The grounded
  task is a data-oriented propositional core — bitset states over
  structure-of-arrays / CSR operator tables (`packed.rs`), built for
  streaming applicability, successor generation, and *data-parallel*
  batched heuristic evaluation. On top of it, `invariants.rs` already
  synthesizes **Helmert-style monotonicity invariants → sound mutex
  groups** — exactly what SAS+ would give as multi-valued variables —
  *without* moving the planner off its bitset state. The groups feed
  SGPlan/ESPC subgoal partitioning today. Phase 1 is therefore
  "extend and exploit the mutex layer," **not** "build SAS+."
- **Search paradigm: FF-family heuristic search.** Enforced
  hill-climbing with helpful-action lookahead, falling back to a
  deterministic, batch-parallel **weighted best-first search**
  (`--weight-g`/`--weight-h`, fixed batch size so plans are identical at
  any thread count). Delete-relaxation FF heuristic with helpful
  actions (`heuristic.rs`). Plus, layered on the same engine: an
  SGPlan-style partition-and-resolve mode, the **ESPC penalty-resolution
  anytime loop** for preference coordination, anytime branch-and-bound
  PDDL3 metric optimization, PDDL2.1 temporal planning with a
  decision-epoch pipeline and scheduler, goal **decomposition into
  contracts**, and a **`Session`** ground-once/replan-many API.

Other standing capabilities the original draft didn't know about:
numeric fluents (Metric-FF lineage), static derived axioms, PDDL3
trajectory-constraint monitors hardened across 0.6–0.8 (END
construction, shared monitor block), a benchmark harness
(`benchmarks/run.py`, `compare.py`, stored baselines in
`benchmarks/metrics/`), and IPC5 benchmark sets vendored under
`benchmarks/ipc/`.

---

## Phase 0 — Gap audit & measurement scaffolding *(first; smaller than drafted)*

**Goal:** stand up the pieces of measurement/validation that do *not*
yet exist, and write down current state so later phases stay honest.

**Already exists:** benchmark harness + baselines for IPC5; CI heavy
IPC regression guards; internal plan validation; per-release roadmap
records in `docs/`.

**Tasks**
- Pull the **IPC6 and IPC7 benchmark sets** (the
  `potassco/pddl-instances` collection is a consistent source) into
  `benchmarks/ipc/` alongside the IPC5 sets, and teach `run.py` /
  `compare.py` to emit coverage, wall-clock, **and plan cost** for them.
- Integrate **VAL** into the harness (and CI where feasible) as the
  external checker for scoreboard claims. Keep `--validate` as the fast
  inner-loop check.
- Write **`STATUS.md`**: current capabilities (start from the audit
  above), which later phases are partially done vs greenfield, and any
  surprises. Update it at the end of every subsequent phase.

**Acceptance criteria**
- `STATUS.md` exists and accurately describes the codebase.
- The harness can run any IPC5/6/7 domain and emit coverage + cost + time.
- VAL validates solved instances automatically for scoreboard runs.
- The existing IPC5 regression guards stay green.

---

## Phase 1 — Exploit the mutex layer *(re-scoped: harden, don't rebuild)*

**Goal:** extend and exploit the existing invariant/mutex-group
synthesis — **not** convert the planner to a SAS+ substrate.

**Why / RPG angle:** compact reasoning about "at most one of these is
true" makes heuristics sharper and per-decision planning cheaper inside
a game loop. Ferroplan already made the architectural call that mutex
groups live as an analysis layer over the propositional bitset core —
the bitset/SoA representation is load-bearing for its speed and
determinism. Respect that call; a Fast Downward-style representation
swap would be a rewrite of the engine's identity for benefits most of
the target heuristics (relaxed-plan, landmarks, LM-cut) don't need.

**Already exists:** `invariants.rs` — Helmert-style multi-predicate
monotonicity invariants, verified lifted per action, refined to a
fixpoint, checked against the initial state; emitted groups are always
sound. Consumed today by ESPC subgoal partitioning.

**Tasks**
- Measure invariant coverage on the IPC5/6/7 sets (there is prior art in
  `docs/invariants-measurement.md`) and close the gaps that matter —
  e.g. groups the refinement budget currently misses.
- Exploit groups beyond partitioning where it pays: mutex-based pruning
  in successor generation / reachability, sharper relaxed-plan
  extraction, and (Phase 7, later) the substrate for PDB-style
  abstractions.
- (Optional, low cost) parse `:object-fluents` so multi-valued state can
  be *expressed* in PDDL; compile it down to the propositional core.

**Acceptance criteria**
- All IPC5 domains still solve — no coverage or quality regression.
- Mutex coverage is measured and recorded on tasks with known structure
  (a variable per gripper/hand, per location occupancy).
- At least one consumer (pruning or heuristic) demonstrably improves a
  benchmark metric with groups on vs off.

---

## Phase 2 — Action costs + cost-sensitive search

**Goal:** support `:action-costs` and make search reason about total
plan cost, with anytime improvement.

**Why / RPG angle:** action cost is the single most useful knob for a
game. Encode danger, time, stamina, gold, or noise as cost, and NPCs
start producing *weighted* plans — the believable "took the safer longer
road because the shortcut was risky" behavior — instead of any-valid
plan.

**Already exists:** numeric fluents and metric machinery; the PDDL3
pipeline already compiles and anytime-B&B-optimizes metrics over
`is-violated` and `total-cost` terms. What's missing is the front door
and the heuristic: the parser does not accept the `:action-costs`
requirement, and the FF heuristic estimates *distance*-to-go, not
*cost*-to-go.

**Tasks**
- Parse `:action-costs` and `(minimize (total-cost))` per IPC6
  conventions: non-negative integer costs, a single `total-cost` effect
  per action, not inside conditional effects. Route it through `auto`
  mode detection (`features.rs`).
- Track accumulated `g`-cost through search (the weighted-BFS `g` is
  currently unit-step); make the **relaxed-plan heuristic cost-aware**
  (cheapest-achiever propagation instead of level-count).
- Anytime behavior for the plain-cost case: retain the best plan found
  and keep searching to improve it until a budget is hit. (The PDDL3
  B&B and ESPC loops already do this for metrics — extend the pattern,
  don't duplicate it.) Make a wall-clock/tick budget a first-class
  `Options` field while here (see Cross-cutting).

**Acceptance criteria**
- Solves IPC6 sequential domains with cost tracked; reported cost
  matches VAL.
- On a domain with non-unit action costs, Ferroplan finds strictly
  cheaper plans than a cost-blind baseline (its own 0.8.0 behavior).

---

## Phase 3 — LAMA-style satisficing configuration

**Goal:** the workhorse configuration — landmark + FF heuristics under
anytime weighted search. Best coverage-per-effort target; the historical
winner of IPC6 sequential satisficing and still strong on IPC7.

**Why / RPG angle:** fast, good-quality plans under a time budget mean
responsive NPCs. This is the config the game will lean on most.

**Already exists:** the FF heuristic with **helpful actions** (used in
EHC lookahead), weighted best-first with tunable `weight_g`/`weight_h`,
and deterministic batch-parallel heuristic evaluation. Note: classical
search has **no landmarks** today (the only landmark machinery in the
tree is temporal); that part is genuinely greenfield.

**Tasks**
- **Landmark generation** (RPG/delete-relaxation based) and a
  **landmark-count pseudo-heuristic** (path-dependent).
- **Preferred operators in best-first**: helpful actions exist but only
  EHC consumes them — add a dual open-list (preferred vs standard) to
  the weighted-BFS fallback.
- **Multi-heuristic search** combining the landmark and FF estimates.
- **Iterated weighted search** with decreasing weights (restarting /
  anytime), so quality improves the longer it runs.
- Throughput: LAMA's answer is lazy/deferred evaluation; Ferroplan's is
  batched parallel evaluation. Measure before porting LAMA's trick —
  deferred evaluation may fight the batch design for no gain.

**Acceptance criteria**
- Coverage and plan quality on IPC6/IPC7 sequential-satisficing domains,
  measured within a fixed time budget.
- Measurable quality improvement over the Phase 2 baseline on the same
  instances.

---

## Phase 4 — Net-benefit / oversubscription planning *(RPG crown jewel)*

**Goal:** support soft goals with utilities and an objective that
maximizes achieved-goal utility minus total cost — i.e., choose *which*
goals to pursue when you can't have them all.

**Why / RPG angle:** this is the most RPG-aligned capability in all of
IPC6. An NPC with limited time or resources deciding which quests/goals
are worth it, dropping low-value objectives under scarcity and picking
them up when resources are plentiful, *is* oversubscription planning.
Comparatively few planners do it well — a genuine differentiator.

**Already exists:** most of the compilation target. PDDL3 soft-goal
preferences with weighted `is-violated` metrics are compiled (forgo
machinery included — `pddl3.rs` builds "forgo" branches) and optimized
by anytime B&B over `is-violated` + `total-cost` terms. IPC6
net-benefit is close to a reformulation of this: soft goals with
utilities ≈ weighted preferences; forgo-cost = foregone utility.

**Tasks**
- Parse IPC6 net-benefit syntax (soft goals with utilities; maximize
  sum of achieved utilities − total cost) and normalize the objective
  into the existing minimize-metric form.
- Route it through the **compilation path first** (forgo actions costed
  at foregone utility, reducing net-benefit to the Phase 2/3 cost
  machinery). Only build direct partial-satisfaction search with
  utility-aware relaxation if the compilation route measurably falls
  short.
- Handle partial satisfaction correctly (achieving no soft goal is a
  legal, sometimes optimal, outcome — the empty plan must be a
  candidate).

**Acceptance criteria**
- Solves the IPC6 net-benefit domains (e.g., crew-planning) with
  reported net benefit validated.
- On a small RPG-flavored test domain, the planner demonstrably drops
  low-utility goals when the cost budget is tight and pursues them when
  the budget is loose.

---

## Phase 5 — Preferences & trajectory constraints × costs

**Goal:** make the PDDL3.0 machinery — Ferroplan's strongest suit —
compose cleanly with action costs and net-benefit.

**Why / RPG angle:** trajectory constraints are how you express behavior
rules — "always avoid the lava," "eventually reach the shrine," "never
let HP hit zero," soft stylistic preferences on how an NPC acts.

**Already exists:** this is the 0.6–0.8 arc. Trajectory-constraint
monitors, the END construction for hard monitors, the shared monitor
transition block, ESPC penalty coordination, `is-violated` metric
weighting — exercised against the IPC5 qualitative/complex-preference
sets with recorded scoreboards. The substrate is *done*; do not
re-litigate it.

**Tasks**
- Verify coverage of the modal operators against the IPC5/IPC6 sets:
  `always`, `sometime`, `at-most-once`, `sometime-after`,
  `sometime-before`, `within`, `always-within`, `hold-during`,
  `hold-after` — record which are exercised where.
- Ensure preferences and trajectory constraints combine correctly with
  `:action-costs` and net-benefit objectives: **one** shared metric
  evaluation, no double-counting between preference weights, forgo
  costs, and action costs.
- Keep ESPC's separation intact (penalties reorder search only; weights
  compute the reported metric) when costs join the metric.

**Acceptance criteria**
- IPC5 qualitative/complex-preference domains still pass at baseline.
- A combined test domain (action costs + preferences + soft goals)
  solves with a correctly computed metric.

---

## Phase 6 — Portfolio engine

**Goal:** run a small set of complementary configurations rather than
betting on one. Cheap once two or three configs exist; historically what
wins IPC6/IPC7 coverage.

**Why / RPG angle:** robustness. The game can't afford a single
configuration that faceplants on one category of quest/decision. A
portfolio degrades gracefully across problem types.

**Already exists:** the seed of it. `auto` mode already routes by
problem features (`features.rs`) across ff / pddl3 / temporal /
partition, and the ESPC and B&B loops keep anytime incumbents. What's
missing is running *multiple* configs on the *same* problem under a
shared budget.

**Tasks**
- A configuration registry (each config = heuristic(s) + search +
  params — `Options` is most of this already).
- A **sequential portfolio scheduler** that time-slices the budget
  across configs.
- Shared global-best plan across configs (portfolio keeps the best plan
  any member has found — anytime at the portfolio level, metric-compared
  consistently).
- (Later) per-domain or per-instance config selection based on simple
  task features — extending the `auto` routing that already exists.

**Acceptance criteria**
- Portfolio coverage on IPC6/IPC7 sequential-satisficing domains is at
  least as good as the best single configuration, and better on at
  least some domains.

---

## Phase 7 — Optimal planning track *(optional / credential)*

**Goal:** provably-optimal cost planning, to enter the sequential-optimal
subtracks.

**Why / RPG angle:** honestly the *least* necessary for a game — games
want fast and near-optimal, not provably optimal. Included because it's
the intellectually richest part of IPC7 and gives Ferroplan a serious
planner credential. Build it if the scope appetite is there; skip or
defer without guilt if the game is the priority.

**Tasks**
- An admissible heuristic — start with **LM-cut** (works over the
  delete-relaxation machinery Ferroplan already has; no SAS+ required).
- Optionally deepen with **pattern databases** — this is where the
  Phase 1 mutex groups finally pay as abstraction variables — and/or
  merge-and-shrink.
- **A\*** with the admissible heuristic (the weighted-BFS engine at
  `weight_g=weight_h=1` with an admissible h is the starting point);
  optimality-preserving pruning as a stretch goal.

**Acceptance criteria**
- Solves IPC6/IPC7 sequential-optimal domains with costs matching known
  optima, verified by VAL.

---

## Phase 8 — Temporal planning *(re-scoped: mostly shipped — benchmark it)*

**Goal:** ~~build durative actions~~ — **done**: PDDL2.1 durative
actions shipped (constant / parameter-dependent durations, duration
inequalities, timed initial literals), with a decision-epoch pipeline,
scheduler, makespan tracking, temporal plan validation, and goal
decomposition into contracts. The original draft's "do not start until
a game-design decision" gate is inverted: the capability exists; the
decision is how much more to *invest*.

**Remaining tasks**
- Run the IPC6/IPC7 **temporal-satisficing** sets through the harness;
  record coverage and makespan quality honestly (including required-
  concurrency instances, which are the hard class — expect and document
  gaps rather than hiding them).
- `(minimize (total-time))` and combined cost/time metrics as explicit
  objectives, once Phase 2's cost machinery lands.
- Extend `Session` (see Cross-cutting) to temporal domains if the game
  needs per-tick temporal replanning — v1 rejects them by design.

**Acceptance criteria**
- IPC6/IPC7 temporal-satisficing coverage measured and recorded;
  schedules validated (VAL's temporal mode where its stricter
  concurrent-numeric mutex semantics permit; `--validate` otherwise,
  with the difference documented).

---

## Cross-cutting concerns (apply throughout)

**Benchmarking.** Use the IPC benchmark instances (the
`potassco/pddl-instances` collection is a consistent source): IPC 2008
covers 11 domains / 41 variants and IPC 2011 covers 19 domains / 54
variants on the deterministic track. Domains worth targeting early:
elevators, openstacks, parcprinter, pegsol, scanalyzer, sokoban,
transport, woodworking (IPC6) plus barman, floortile, nomystery,
parking, tidybot, visitall (IPC7). **Score on quality, not just
coverage:** the IPC convention is per-instance reference-cost divided by
your-plan-cost (capped at 1), summed, under a fixed per-instance time
limit (30 minutes classic; also run a short game-loop-realistic budget).
Track coverage, quality score, and time separately — extend the
existing `benchmarks/` harness and scoreboard format rather than
inventing a parallel one.

**Plan validation.** Internal `--validate` for the inner loop; VAL for
anything claimed in a scoreboard. A plan we can't validate doesn't
count.

**Regression safety.** The IPC5 guards in CI run on every change; treat
an IPC5 regression as a build failure. Add IPC6/7 baselines to the same
mechanism as coverage lands.

**RPG integration seam.** Largely in place and should stay first-class:
the library API (`solve` / `parse` / `decompose` / `validate_plan`) is
clean and serde-serializable, and **`Session` already does
ground-once/replan-many** with `set_fact`/`set_fluent` per tick. Gaps to
close as phases land: a **wall-clock/tick budget as a first-class
`Options` input** ("best plan you have in N ms" — today's knobs are
node-count caps, which are deterministic but not time-denominated;
consider exposing both), `Session` support for PDDL3 and temporal
domains, and mutable goals in a session. This is where Ferroplan earns
its keep as a game brain beyond IPC scores.

**Testing & docs.** Unit tests per component; golden-plan tests for
representative instances; property tests where feasible. Update
`STATUS.md` at the end of each phase and keep the per-release
`docs/roadmap-<version>.md` records going.

---

## Sequencing & dependencies

- **Phase 0** first (it's mostly measurement scaffolding now).
- **Phase 1** is *supporting*, not gating: Phases 2–4 run on the
  propositional core as-is. Schedule Phase 1's exploitation work
  opportunistically; only Phase 7's PDB option truly depends on it.
- **Phase 3** depends on Phase 2 (costs) being in place.
- **Phase 4** depends on Phase 2 and interoperates with Phase 5.
- **Phase 6** needs at least two of {Phase 3, Phase 4, Phase 7} to
  portfolio over.
- **Phase 7** is optional/late. **Phase 8** is mostly done; its
  remaining work is benchmarking plus objectives that depend on
  Phase 2.

**RPG-critical path, if forced to prioritize:**
`0 → 2 → 3 → 4 → 5`, then add `6`, weaving Phase 1 in where it pays.
Phases 7 and the rest of 8 are bonus.

---

## Note to the agent

This revision was audited against v0.8.0, but code keeps moving — where
this roadmap and the actual code disagree, **the code wins and the
roadmap gets a note** (that's how this revision came to be). Keep the
IPC5 baseline green at all times. The known open game-design questions
that shape (not gate) the work: turn-based vs real-time, whether genuine
concurrency exists (affects further temporal investment), and the
per-tick planning budget (affects the `Options` budget work and
`Session` extensions). If those get answered, record them in
`STATUS.md`.
