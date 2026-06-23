# Changelog

All notable changes to this project are documented here.

## [0.1.0] - unreleased

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
  one-shot planning solves vs. where a goal must be decomposed into contracts.
- **Claude Code skill** (`.claude/skills/ferroplan`) — PDDL-authoring guidance, a
  CLI/feature reference, and six per-feature examples each re-verified to solve,
  enforcing an author → run → read-the-plan loop.
- **GUI / web** — per-type procedural icons (incl. a machine icon for scheduling
  domains) and relation-colored edges (rail vs road vs stage routing); the
  in-browser WASM demo gains a selectable example picker, including a `border`
  example that shows where one-shot planning gives out.

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
