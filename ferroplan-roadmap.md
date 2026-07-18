# Ferroplan Roadmap — Toward an RPG Planning Brain with IPC6/IPC7 Coverage

**Audience:** the coding agent working in this repository.
**North star:** make Ferroplan the best possible planning engine for a
Rust RPG's decision-making, while picking up strong, honest coverage of
the IPC6 (2008) and IPC7 (2011) problem spaces along the way.

---

## How to use this document

Work the phases roughly in order. **Phase 0 is mandatory and gates
everything else** — do not start building Phase 1+ until you have
audited the current codebase and written up what actually exists,
because parts of this roadmap may already be done and other parts may
need a different approach than assumed here.

Each phase has: a goal, why it matters (with the RPG angle called out),
concrete tasks, and acceptance criteria. Treat the acceptance criteria
as the definition of done. After each phase, update `STATUS.md` (see
Phase 0) so the next run of the agent — or a human — can pick up cleanly.

Two rules that hold across all phases:

1. **Never regress the IPC5 baseline.** Ferroplan is currently
   competitive with SGPlan5 on IPC5. That baseline is a guardrail —
   every change must keep the IPC5 domains solving at their current
   coverage and quality. Add a regression gate for this early (Phase 0).
2. **Validate every plan.** Wire in an external plan validator (VAL or
   equivalent) so that any instance we claim to solve is machine-checked
   for correctness and reported cost. No self-graded plans.

---

## Strategic context (read before starting)

Ferroplan today is strong at **satisficing planning with preferences and
trajectory constraints** — the PDDL3.0 world that IPC5 rewards. Moving
toward IPC6/IPC7 is not "add more domains." It is crossing into
**cost-aware and cost-optimal heuristic search**. The competitive
frontier there is the Fast Downward / LAMA family of planners plus
portfolios — not SGPlan. The language delta is small (IPC6's PDDL3.1
mainly adds action costs and optional multi-valued object fluents), but
the *engine* delta is large: heuristics must estimate cost-to-go rather
than distance-to-go, and evaluation is scored on plan quality (total
cost), not just coverage or speed.

Happily, the two things that matter most for an RPG — **action costs**
(so NPCs weigh risk/time/resources) and **net-benefit / oversubscription
planning** (so a resource-bounded NPC chooses *which* goals are worth
pursuing) — are exactly the IPC6 additions. So the RPG direction and the
IPC6/7 direction reinforce each other.

### Two unknowns Phase 0 must resolve

The sequencing below assumes answers to these. Confirm them first:

- **Grounding representation:** does Ferroplan ground to a finite-domain
  (SAS+ / multi-valued variable) representation, or does it stay
  propositional (STRIPS-style binary facts)? This determines whether
  Phase 1 is "harden what exists" or "build the foundation."
- **Search paradigm:** is the current solver heuristic-guided search, or
  a constraint-partitioning / partition-and-resolve approach (SGPlan
  lineage)? Cost-sensitive heuristic search is the target; know where
  you're starting from.

---

## Phase 0 — Orientation & current-state audit *(mandatory, gates all)*

**Goal:** know exactly what exists before building, and stand up the
measurement and validation scaffolding the rest of the roadmap depends
on.

**Tasks**
- Inventory the parser: which PDDL requirements are already supported?
  (`:strips`, `:typing`, `:adl`, `:derived-predicates`,
  `:preferences`, `:constraints`, `:action-costs`, `:numeric-fluents`,
  `:object-fluents`, `:durative-actions`, etc.)
- Inventory the pipeline: grounding/instantiation strategy, internal
  state representation, successor generation, search algorithm(s),
  heuristic(s), plan reconstruction, and any existing anytime behavior.
- Answer the two unknowns above explicitly in writing.
- Locate or build the **benchmark harness**: pull the IPC benchmark sets
  (see Cross-Cutting → Benchmarking) and make it trivial to run
  Ferroplan across a domain and collect coverage, wall-clock, and plan
  cost.
- Integrate a **plan validator** (VAL) into the harness and CI.
- Establish the **IPC5 regression gate**: a stored baseline of current
  coverage/quality on the IPC5 domains that CI checks against.
- Write **`STATUS.md`**: current capabilities, the two unknowns'
  answers, which later phases are already partially done vs greenfield,
  and any surprises. This file is the living source of truth; update it
  at the end of every subsequent phase.

**Acceptance criteria**
- `STATUS.md` exists and accurately describes the codebase.
- The harness can run any IPC domain and emit coverage + cost + time.
- VAL validates solved instances automatically.
- The IPC5 regression gate is wired into CI and currently green.

---

## Phase 1 — Finite-domain (SAS+) grounding foundation

**Goal:** ground planning tasks into a compact multi-valued-variable
representation with mutex structure, the substrate every modern
heuristic and search below assumes.

**Why / RPG angle:** compact state and precomputed mutexes make
per-decision planning fast enough to run inside a game loop, and make
NPC world-models tractable. This is the enabling layer, not an optional
nicety.

**Tasks**
- Efficient grounding/instantiation of schematic actions and axioms.
- Invariant synthesis to discover mutex groups; convert groups of
  mutually exclusive binary facts into finite-domain (multi-valued)
  variables (Fast Downward-style translation).
- Fast successor generation over the finite-domain representation.
- Plan reconstruction back to a concrete PDDL action sequence.
- Preserve **derived predicates / axioms** through grounding (Ferroplan
  likely relies on these for trajectory constraints — do not lose them).
- (Optional, low cost, aligns naturally) parse `:object-fluents` so
  multi-valued state can be expressed directly in PDDL.

**Acceptance criteria**
- All IPC5 domains still solve — no coverage or quality regression.
- The finite-domain encoding is validated on tasks with known mutex
  structure (e.g., a variable per gripper/hand, per location occupancy).
- Reconstructed plans pass VAL.

> If Phase 0 finds SAS+ grounding already exists, this phase becomes
> *verify, harden, and add missing invariant/mutex coverage* rather than
> a build.

---

## Phase 2 — Action costs + cost-sensitive search

**Goal:** support `:action-costs` and make search reason about total
plan cost, with anytime improvement.

**Why / RPG angle:** action cost is the single most useful knob for a
game. Encode danger, time, stamina, gold, or noise as cost, and NPCs
start producing *weighted* plans — the believable "took the safer longer
road because the shortcut was risky" behavior — instead of any-valid
plan.

**Tasks**
- Parse the `:action-costs` requirement and `(minimize (total-cost))`
  metric. Enforce the IPC6 conventions: non-negative integer costs, a
  single `total-cost` effect per action, not inside conditional effects.
- Track accumulated cost through search; switch heuristic and search
  bias from distance-to-go to **cost-to-go**.
- Anytime behavior: retain the best plan found so far and keep searching
  to improve it until a time/tick budget is hit.

**Acceptance criteria**
- Solves IPC6 sequential domains with cost tracked; reported cost matches
  VAL.
- On a domain with non-unit action costs, Ferroplan finds strictly
  cheaper plans than a cost-blind breadth-first baseline.

---

## Phase 3 — LAMA-style satisficing configuration

**Goal:** the workhorse configuration — landmark + FF heuristics under
anytime weighted search. This is the best coverage-per-effort target and
the historical winner of the IPC6 sequential satisficing track; it also
holds up well on IPC7.

**Why / RPG angle:** fast, good-quality plans under a time budget mean
responsive NPCs. This is the config the game will lean on most.

**Tasks**
- A cost-aware **FF heuristic** (relaxed-plan based).
- **Landmark generation** and a **landmark-count pseudo-heuristic**
  (path-dependent).
- **Preferred operators / helpful actions**, with a dual open-list
  (preferred vs standard) search.
- **Multi-heuristic search** combining the landmark and FF estimates.
- **Weighted A\*** with iteratively decreasing weights (restarting /
  anytime), so quality improves the longer it runs.
- Lazy/deferred heuristic evaluation for throughput.

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
Comparatively few planners do it well — this is a genuine differentiator.

**Tasks**
- Parse soft goals with utility values and the net-benefit metric
  (maximize sum of achieved-goal utilities − total cost).
- Implement net-benefit search. Two viable routes — pick per Phase 0
  findings and prototype both if unsure:
  - **Compilation:** reformulate soft goals into "forgo" actions whose
    cost equals the foregone utility, reducing net-benefit to
    cost-minimization the Phase 2/3 machinery already handles.
  - **Direct partial-satisfaction search** with utility-aware relaxation
    heuristics.
- Handle partial-satisfaction correctly (achieving no soft goal is a
  legal, sometimes optimal, outcome).

**Acceptance criteria**
- Solves the IPC6 net-benefit domains (e.g., crew-planning) with
  reported net benefit validated.
- On a small RPG-flavored test domain, the planner demonstrably drops
  low-utility goals when the cost budget is tight and pursues them when
  the budget is loose.

---

## Phase 5 — Preferences & trajectory constraints as first-class game rules

**Goal:** exercise and harden the PDDL3.0 machinery Ferroplan already
has, and make sure it composes cleanly with action costs and
net-benefit.

**Why / RPG angle:** trajectory constraints are how you express behavior
rules — "always avoid the lava," "eventually reach the shrine," "never
let HP hit zero," soft stylistic preferences on how an NPC acts. You
already have the substrate from IPC5; the work is making it interoperate
with the new cost/utility objectives rather than sitting in a separate
code path.

**Tasks**
- Confirm full coverage of the trajectory-constraint modal operators:
  `always`, `sometime`, `at-most-once`, `sometime-after`,
  `sometime-before`, `within`, `always-within`, `hold-during`,
  `hold-after`.
- Confirm `is-violated` preference weighting in metrics.
- Ensure preferences and trajectory constraints combine correctly with
  `:action-costs` and with net-benefit objectives (shared metric
  evaluation, no double-counting).

**Acceptance criteria**
- IPC5 qualitative/complex-preference domains still pass at baseline.
- A combined test domain (action costs + preferences + soft goals)
  solves with a correctly computed metric.

---

## Phase 6 — Portfolio engine

**Goal:** run a small set of complementary configurations rather than
betting on one. Cheap to build once two or three configs exist, and
historically the thing that wins IPC6/IPC7 coverage.

**Why / RPG angle:** robustness. The game can't afford a single
configuration that faceplants on one category of quest/decision. A
portfolio degrades gracefully across problem types.

**Tasks**
- A configuration registry (each config = heuristic(s) + search + params).
- A **sequential portfolio scheduler** that time-slices the budget across
  configs.
- Shared global-best plan across configs (portfolio keeps the best plan
  any member has found — anytime at the portfolio level).
- (Later) per-domain or per-instance config selection based on simple
  task features.

**Acceptance criteria**
- Portfolio coverage on IPC6/IPC7 sequential-satisficing domains is at
  least as good as the best single configuration, and better on at least
  some domains.

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
- An admissible heuristic — start with **LM-cut** (landmark-cut; the
  standard sweet spot of quality vs cost).
- Optionally deepen with **pattern databases** (e.g., CEGAR/iPDB-style
  abstraction selection) and/or **merge-and-shrink** abstractions.
- **A\*** with the admissible heuristic; optimality-preserving pruning
  (e.g., partial-order reduction) as a stretch goal.

**Acceptance criteria**
- Solves IPC6/IPC7 sequential-optimal domains with costs matching known
  optima, verified by VAL.

---

## Phase 8 — Temporal planning *(conditional — gate on game design)*

**Goal:** durative actions and makespan optimization.

**Why / RPG angle:** only worth it if the RPG has genuine concurrency —
timed, overlapping actions on a shared clock. For turn-based or
single-agent-tick logic this is a large lift with little payoff.

**Do not start this phase** until there's an explicit decision that the
game needs concurrent timed action. If it does:

**Tasks**
- Parse `:durative-actions`; support makespan (`(minimize (total-time))`)
  and combined cost/time metrics.
- Temporal constraint handling and scheduling of overlapping actions.

**Acceptance criteria**
- Solves IPC6/IPC7 temporal-satisficing domains; schedules validated.

---

## Cross-cutting concerns (apply throughout)

**Benchmarking.** Use the IPC benchmark instances (the `potassco/
pddl-instances` collection is a consistent source): IPC 2008 covers 11
domains / 41 variants and IPC 2011 covers 19 domains / 54 variants on the
deterministic track. Domains worth targeting early include elevators,
openstacks, parcprinter, pegsol, scanalyzer, sokoban, transport,
woodworking (IPC6) plus barman, floortile, nomystery, parking, tidybot,
visitall (added in IPC7). **Score on quality, not just coverage:** the
IPC convention is a per-instance quality score of reference-cost divided
by your-plan-cost (capped at 1), summed across instances, under a fixed
per-instance time limit (30 minutes is the classic convention; use a
shorter budget too, to reflect game-loop reality). Track coverage,
quality score, and time separately.

**Plan validation.** VAL (or equivalent) in CI on every solved instance.
A plan we can't validate doesn't count.

**Regression safety.** The IPC5 baseline gate from Phase 0 runs on every
change. Treat an IPC5 regression as a build failure.

**RPG integration seam (ongoing, game-specific value).** Keep a clean
library API distinct from any CLI: `parse → ground → plan → plan as
action sequence`, callable from the Rust game. Design for **incremental
replanning** (the world changes; replan cheaply rather than from
scratch), and make a **time/tick budget** a first-class planner input so
the game can say "give me the best plan you have in N milliseconds." This
is where Ferroplan earns its keep as a game brain beyond IPC scores.

**Testing & docs.** Unit tests per component; golden-plan tests for
representative instances; property tests where feasible. Update
`STATUS.md` at the end of each phase.

---

## Sequencing & dependencies

- **Phase 0** gates everything.
- **Phase 1** is the foundation for Phases 2–7.
- **Phase 3** depends on Phase 2 (costs) being in place.
- **Phase 4** depends on Phase 2 and interoperates with Phase 5.
- **Phase 6** needs at least two of {Phase 3, Phase 4, Phase 7} to
  portfolio over.
- **Phases 7 and 8** are optional/late; Phase 8 is gated on an explicit
  game-design decision about concurrency.

**RPG-critical path, if forced to prioritize:**
`0 → 1 → 2 → 3 → 4 → 5`, then add `6`. Phases 7 and 8 are bonus.

---

## Note to the agent

Do not assume the codebase matches this document — read it first
(Phase 0) and report reality in `STATUS.md`. Where this roadmap and the
actual code disagree, the code wins and the roadmap gets a note. Keep the
IPC5 baseline green at all times. If the game's design constraints
(turn-based vs real-time, whether concurrency exists, what the tick
budget is) are unknown, flag that — it gates Phases 7 and 8 and shapes
the integration seam.
