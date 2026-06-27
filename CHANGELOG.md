# Changelog

All notable changes to this project are documented here.

## [0.2.1] - 2026-06-26 — "The Bridge"

The engine release (0.1) made ferroplan fast and correct; 0.2 makes the README's
bet real and inspectable: the proven temporal heuristics are on by default, temporal
coverage goes deeper (duration inequalities + timed initial literals), and a goal too
big for the one-shot search is **automatically decomposed** into solvable,
individually-verified contracts.

### Added
- **MCP server (`ferroplan-mcp`)** — a Model Context Protocol server exposing
  `solve`, `validate`, and `decompose` to an LLM agent over stdio, so the agent can
  *author and supervise* PDDL and let the planner run deterministically (the README's
  bet, made operational). A self-contained newline-delimited JSON-RPC 2.0 loop — no
  async runtime, deps limited to `serde`/`serde_json` — that returns the structured
  `Solution` / `Decomposition` as tool results, reports tool failures as `isError`
  results (so the agent can correct its PDDL), and never panics on input. Integration
  tests drive the built binary end to end. (`publish = false` for now; not in the
  crates.io release set yet.)
- **Goal decomposer — `decompose` API + `ff --decompose`** (the README's bet, made
  inspectable). A temporal goal too big for the one-shot search is split into ordered
  sub-contracts — each small enough to solve whole and individually verified — then
  stitched into one validated plan. This surfaces the partition-and-resolve engine
  (previously only the `FF_TDECOMP` flag, which returned just the flat plan) as a
  first-class, typed, serde-serializable `Decomposition { contracts, plan, monolithic }`
  where each `Contract` names its sub-goal (`(order o1), (order o2)`, `coin >= 15`),
  its sub-plan, and its offset in the stitched timeline. A goal that can't be split —
  or whose split doesn't validate — falls back to a single monolithic contract,
  reported honestly. `ff --decompose` prints the breakdown (text or `--json`).
  Demonstrated on `examples/rpg-world/hard/order-8` & `order-12` (8 / 12 contracts),
  which the one-shot temporal search fails on. `ferroplan::decompose(domain, problem,
  &Options)`; `tresolve::solve` now delegates to the recording `decompose` (the
  `FF_TDECOMP` plan path is unchanged).
- **Timed initial literals (PDDL2.2)** — `(at <time> <literal>)` in `:init` (including
  `(at <time> (not <literal>))`) now schedules an exogenous fact change at a fixed
  absolute time, disambiguated from the ordinary `(at ?x ?y)` predicate by a numeric
  first argument. Each TIL compiles to a synthetic 0-arg applier action (so its fact
  is grounded and a goal reachable only via a TIL isn't pruned as a relaxed dead end);
  the decision-epoch search fires it from a pre-seeded agenda at its time, the STN
  re-timing floors TIL-gated actions at their scheduled instant so they can't slide
  before their gate, and the in-crate validator replays TILs up to the plan horizon.
  Off the temporal path, TILs are inert (heap key byte-identical).
- **Temporal duration inequalities** — `:duration` now accepts `(>= ?duration L)`,
  `(<= ?duration U)`, and `(and ...)` ranges in addition to the fixed
  `(= ?duration e)`. The decision-epoch search commits to the **shortest feasible**
  duration (the lower bound), and the in-crate temporal validator accepts any
  duration within `[min, max]` (a fixed `=` collapses the range to a point,
  recovering exact-equality). Durations remain constant or parameter-dependent.
  (IPC temporal domains aren't vendored — licences — so this is exercised by
  crafted inequality domains + `temporal::validate`; the fixed-duration RPG corpus
  is unchanged, 26/27 suite.)

### Changed
- **Temporal demand guidance is now on by default** (graduated from the opt-in
  `FF_TDEMAND`). The default is a new **`Numeric`** tier: demand is seeded from
  *numeric goals only* — the measured multi-round win (`steel ≥ 2`, `grain ≥ 10`,
  `coin ≥ 15`). Validated on the RPG `suite/` + `hard/` corpus: **26 → 34/39
  solved, no regression** vs. the old default, and crucially *without* the makespan
  regression a blind graduation would cause — the previously-coupled
  predicate-goal-threshold seeding reads a renewable-pool guard (`(>= (avail) 1)`,
  net-zero) as accumulation demand and serializes concurrency domains (a unit
  `crew` pool of 2 went concurrent-~5 → serialized-~10). That structural/predicate
  half — plus goal-relevance pruning — now rides an explicit **`Full`** tier
  (`FF_TDEMAND`), which additionally solves the one structural build
  (`gather-build`) the numeric default gives up (decomposer territory per
  `examples/BORDERS.md`).
  - Opt out entirely with **`FF_NO_TDEMAND`** (heap key bit-identical to 0.1.0).
  - Library / WASM callers: `features::set_overrides` is now tri-state-backed
    (`true` / `false` are definitive; new `features::clear_overrides` returns to
    default + env), and the active tier is queryable via `features::demand_mode()`
    (`Off` / `Numeric` / `Full`).

## [0.1.0] - 2026-06-24

Initial public release.

### Added
- Data-parallel FF planner core (bitset / CSR, parallel grounding + heuristic).
- **Enforced hill-climbing (EHC)** with helpful actions and a weighted-best-first
  fallback — the default, ~3× faster than best-first and metric-ff-class on
  classical/ADL (geomean 0.21× → 0.66× Metric-FF).
- **Configurable `Options`** (library-first; CLI flags + JSON map to the same
  fields): `mode`, `search`, `helpful_actions`, `weight_g/weight_h`, `threads`,
  `max_evaluated`, `optimize`.
- ADL: conditional effects, `forall`/`exists`, object equality.
- Numeric fluents (Metric-FF style).
- **Derived predicates / axioms** (`:derived`, static / stratified) — closed into
  the initial state via a datalog fixpoint.
- PDDL3 soft-goal preferences (incl. `forall`-quantified and precondition
  preferences) with anytime branch-and-bound metric optimization. IPC-5 coverage
  on par with SGPlan6 (39/48).
- **PDDL2.1 temporal**: durative actions with `at start`/`over all`/`at end`
  conditions & effects, constant or parameter-dependent durations, required
  concurrency, and ε-separation; decision-epoch search; IPC temporal plan output
  with makespan. Plans validated against VAL on real IPC domains (44/45 valid);
  an independent in-crate validator (`temporal::validate`).
- SGPlan-style partition-and-resolve mode.
- **ESPC penalty-resolution loop** (`FF_ESPC`, opt-in) — SGPlan's Extended
  Saddle-Point Condition adaptive penalty coordination, applied to the PDDL3
  preference metric path. It penalizes, on the *concrete* state, once-only
  conditional achievements that fire without delivering (openstacks: a product
  made while its orders still wait — a permanently lost preference the
  delete-relaxed heuristic is blind to), and adapts a **per-trigger** penalty
  across an outer loop, keeping the best plan as an anytime incumbent. Iteration 0
  runs the penalty-free B&B as a floor, so the loop can only improve, never
  regress. Narrows the metric-quality gap on openstacks at the default budget
  (p01 63→42, p02 66→43, p05 138→81, p06 129→90, p08 608→227); a larger
  `FF_ESPC_TIME_MS` / more threads improves the hardest instances further
  (e.g. p07 278→142). The loop is wall-clock-bounded (default 15 s, tunable) and
  always returns its incumbent inside that budget, so it never loses coverage
  under a harness timeout. Inert on every domain without the make-deadline
  structure — including the whole numeric/temporal RPG corpus — and bit-identical
  to the prior default when off. Auto-tunes per instance (no manual weight); never
  claims optimality. See `docs/espc-preferences-spec.md`.
- **Temporal converging-resource demand guidance** (`FF_TDEMAND`, opt-in) — the
  ESPC concrete-state idea ported to the durative/numeric (RPG) search. It regresses
  the numeric goal down the recipe DAG to a TOTAL per-resource demand (`steel ≥ 2` ⇒
  ingots/coal/ore ≥ 2, logs ≥ 4 — bridging snap-compiled start/end the way the
  landmark extractor does) and guides on cumulative availability (init + produced,
  clamped), which survives consumption across rounds. This is the gradient the
  delete-relaxed heuristic lacks once ≥2 contributions converge on a goal quantity
  (see `examples/BORDERS.md`). Phase-1 key only — phase 2 stays byte-identical, so
  completeness holds. Measured on the RPG corpus: **+8 instances solved (26→34/39),
  all plans validated, no regressions**, cracking three shapes the relaxation went
  flat on — multi-round converging DAGs (tech-steel/bronze), cyclic resource regen
  (farmstead `grain≥10`), and multi-path numeric goals (mint-fortune/trade `coin≥N`).
  Off by default (heap key bit-identical when unset).
- **Temporal partition-and-resolve decomposer** (`FF_TDECOMP`, opt-in) — the SGPlan
  partition loop (`resolve.rs`) brought to the durative/numeric path for the
  conjunctive/structural goals the demand term can't crack. A reusable
  `temporal::solve_from(start, goal, forbidden)` subplanner (the temporal analog of
  `solve_subgoal_avoiding`) lets the decomposer partition the world goal into
  contracts, solve each from the running composed state, splice the timed subplans
  strictly sequentially (each offset past the prior makespan + an ε seam), and MERGE
  groups on conflict down to a monolithic `temporal::solve` — so it is solvable
  EXACTLY when the monolithic search is (completeness preserved). Same-epoch
  happenings order on an ε-grid-rounded key (ends before starts) so the offset
  concatenation validates without re-separation. Measured: solves the large mixed
  conjunctive goals `order-8`/`order-12` (RPG temporal 34→36/39), every composed
  plan validated, zero regressions, default path byte-identical. Remaining fails
  (`found-village`, `gather-build`) reduce correctly to a *pre-existing* predicate-
  build (`build-house`/village-shape) search blowup — the next target, separate from
  the decomposer. Groundwork for it (predicate-goal demand seeding; predicate-
  precondition contract regression) is in place behind the same flag.
- **Temporal goal-relevance pruning** (rides `FF_TDEMAND`; `FF_NOREL` disables) — a
  backward closure from the goal marks every op that can contribute (adds/deletes a
  relevant fact or increases a relevant resource, transitively pulling in its
  preconditions and consumed resources); non-contributing ops are pruned from BOTH
  search phases. Fixes the predicate-build blowup: the diagnosis showed phase 1
  (helpful actions) gets stuck under delete-relaxation (the agent is relaxed-
  omnipresent, so travel is never "helpful"), and the COMPLETE phase 2 then drowns in
  goal-irrelevant unbounded accumulators (`forage-food`/`gather-herbs` → food=1,2,3,…).
  Pruning to the relevant subspace lets the search solve instead of exploding. Two
  masks drive three passes — helpful(sound) → full(TIGHT) → full(sound): the SOUND
  mask keeps every producer of a relevant resource (completeness-preserving, the final
  backstop); the TIGHT mask keeps only each resource's single best-yield producer, so
  marking `planks` relevant pulls in `saw-planks` but NOT the alternative producer
  `haul-cargo` (which would otherwise drag the whole logistics subsystem in and
  re-explode). Off by default (empty masks ⇒ op set bit-identical, original two-pass
  behavior). Solves `gather-build` AND `found-village` (RPG temporal 36→38/39); every
  plan validated, no regressions, full suite green. The lone remaining miss,
  `bread-line`, is a pre-existing relaxed dead-end unrelated to relevance.
- **Concurrent temporal scheduling** (`FF_TCONC`, opt-in) — a scheduling phase
  (`tsched`) for durative plans. The decision-epoch search is action-count-guided, so
  it lays actions out sequentially and more workers never shortened the makespan; this
  repacks the found plan onto the domain's actor-objects — one job per worker at a
  time, each action starting as early as its consumed resources and prerequisite
  predicates allow — to minimize makespan. The multi-actor search is flaky, so it
  searches a single-actor reduction and reassigns the plan across the real crew. Every
  rescheduled plan is run through `temporal::validate` and kept only if shorter, so it
  can only improve a plan, never produce a wrong one; default path byte-identical.
  Showcase (`examples/cabin`): a durative crew build where 1→2→3 workers cut makespan
  109→63→47 on the same job.
- **Worker skills** — a task's actor-referencing precondition (e.g. `(smith ?w)`) is
  read by the scheduler as a required capability, so skill-gated tasks are assigned
  only to workers who have them (location is handled the same way); the single-actor
  reduction becomes a super-worker (union of all skills) so the search still finds the
  plan, and a task needing a skill no worker has is correctly unsolvable. Shown in
  `examples/cabin/crew-skilled` (sawyer/smith routing) and a "forge order" where the
  smith is the bottleneck — two extra labourers barely move it (65→62) but a second
  smith at the same crew size cuts ~a third (65→44).
- **WASM feature overrides** (`crate::features`) — the env-gated temporal switches
  (`FF_TDEMAND`/`FF_TDECOMP`/`FF_TCONC`) reachable from non-CLI callers via a process
  override OR'd with the env read (env *writes* panic on `wasm32`), surfaced through
  the WASM `plan(domain, problem, mode, flags)` `flags` arg — so the browser demo runs
  the demand guidance, decomposer, and concurrent scheduler too.
- Library API returning structured, `serde`-serializable results.
- `ff` CLI: drop-in `-o/-f` text, `--json`, `--json-request` job I/O, full
  strategy flags.
- **Robust** against malformed input — pathological/deeply-nested PDDL returns a
  typed error, never a panic.
- **SAS+ / mutex groups** — Helmert-style multi-predicate invariant synthesis,
  feeding SGPlan-style subgoal partitioning + resolution.
- **General metric terms** — the metric optimizer folds monotone numeric fluent
  terms (e.g. rovers' `(sum-traverse-cost)`) into total-cost, so all six IPC-5
  simple-preferences domains are scored, rovers included.
- **Bindings (reach)** — `ferroplan-wasm`: run the planner in the browser via
  WebAssembly with a self-contained "try it" demo (no server/install);
  `ferroplan-py`: a pyo3 **abi3** wheel (`import ferroplan; ferroplan.plan(domain,
  problem)`), one wheel for CPython 3.8+. The core stays pure Rust.
- mdBook documentation site; cross-planner comparison harness (`compare.py`),
  temporal+VAL harness (`bench_temporal.py`), and benchmark results vs
  Metric-FF / SGPlan6 / VAL.
- **Worked-domain corpus + coverage borders** (`examples/`) — a ~120-action
  crafting/economy domain (`rpg-world`) with validated contracts, a flavor-×-scale
  `suite/`, an adversarial `hard/` batch, and an `industrial-city` decomposition
  showcase; plus `logistics` (transshipment) and `jobshop` (machine-scheduling,
  scales to 100 jobs) domains. `examples/BORDERS.md` is a measured map of where
  one-shot planning solves vs. where a goal must be decomposed into contracts. Also
  `villagers` — the generic, data-driven recipe model a live game embeds (3 actions:
  walk/gather/craft, recipes as `:init` data; the abstract counterpart to rpg-world) —
  and `cabin`, a deep linear build (fell→mill→smith→glaze→raise, ~52 steps) with a
  durative "parallel crew" twin showing makespan vs. crew size and worker skills.
- **Claude Code skill** (`.claude/skills/ferroplan`) — PDDL-authoring guidance, a
  CLI/feature reference, and six per-feature examples each re-verified to solve,
  enforcing an author → run → read-the-plan loop.
- **GUI / web** — per-type procedural icons (incl. a machine icon for scheduling
  domains) and relation-colored edges (rail vs road vs stage routing). The in-browser
  WASM demo is a **two-level picker** (choose a domain, then a problem graded
  simplest→most-complex), with an execution toggle (**Web Worker** — responsive +
  cancelable — or main thread, for environments that block workers), solve-on-button
  so a heavy problem never auto-freezes the tab, and per-example **feature flags** that
  enable the demand guidance / decomposer / concurrent scheduler in-browser. Includes a
  `border` example that shows where one-shot planning gives out.

### Performance
- **Grounding** — restrict each parameter's domain by its static unary
  preconditions before enumerating; fixes untyped cartesian-product blowup
  (gripper p02 658µs→247µs, 2.65×; large untyped grounding 1.56s→~0). See
  `docs/perf-notes.md`.
- **EHC** — work cap scaled by op count so large-but-easy instances finish in
  EHC's near-greedy arm instead of unpruned best-first (gripper-250 `--mode ff`
  2.16M evals/33s → 32k/0.86s, 38×).
- **Temporal search** — a weighted-`g` heap key plus two-phase helpful-action
  pruning (a pruned `g+h` phase, then the original complete pure-`h` phase) takes
  multi-step long-chain contracts from timeout to instant. A numeric-threshold
  landmark term (phase-1 key only, so the complete pass is byte-identical) then
  restores the heuristic gradient on converging recipe DAGs — a from-scratch ingot
  and the metallurgy benchmark go from no-plan to instant, and deep accumulations
  get 10–60× faster. No regression on the existing temporal suite.

### Known limitations
- Numeric domains trail Metric-FF (EHC falls back to best-first on some).
- IPC-5 preference metric *quality* on the hardest instances still trails SGPlan6;
  retroactively, ferroplan places ~2nd in the field (SGPlan5 swept). The opt-in
  ESPC penalty-resolution loop (`FF_ESPC`, see above and
  `docs/espc-preferences-spec.md`) narrows the openstacks gap substantially
  (~11–63% per instance) but does not close it — reaching SGPlan's level needs a
  dedicated minimum-open-stacks scheduler, not a relaxation-guided search. ESPC is
  off by default while the cross-domain sweep matures.
- The metric branch-and-bound does not scale to instances with hundreds of
  preferences (e.g. storage p05+) — the Keyder–Geffner compilation grows large.
- Temporal coverage is search-limited on the largest *monolithic* instances; the
  intended path past the border is decomposition into contracts (see
  `examples/BORDERS.md`).
- Not supported: duration inequalities, timed initial literals, continuous (`#t`)
  effects, and *dynamic* derived predicates (static / stratified axioms are
  supported).
