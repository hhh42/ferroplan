# Changelog

All notable changes to this project are documented here.

## [Unreleased]

### Added

- **PDDL3 trajectory constraints — the hard untimed operators are now
  ENFORCED on the classical path** (0.7 Phase 1, `docs/roadmap-0.7.md`).
  `always`, `sometime`, `at-most-once`, `sometime-after`, `sometime-before`,
  and `at end` compile into monitor automata (new `constraints` module):
  0-ary monitor facts plus conditional-effect transitions on every action,
  with the goal conjoined on the automaton's accepting condition. `forall`
  constraints expand at the constraint level (bodies stay ground); the init
  state S_0 counts for the trajectory (evaluated at compile time), and
  `sometime-before` is strict ("strictly earlier"). Wired at every gate:
  `solve`, `decompose`, `run_planner`, `run_ff`.
- **Independent trajectory oracle**: `verify::verify` folds the ORIGINAL
  constraint semantics over its replay (never the compiled monitors) —
  `Verified` gains `constraints_met` + `constraint_failures`, and
  `plan::validate_plan` now requires `constraints_met` for `Valid`: a plan
  that reaches the goal but breaks a constraint is `Invalid`, with the
  violated operators named.
- Heavy `#[ignore]` grounding-cost fixtures (hard overlays on vendored
  IPC-5 storage/trucks instances). Recorded (release build): trucks p03,
  3 monitors — 1,065→1,083 ops, +12,780 conditional effects, ground
  8→~50 ms; storage p05, 10 monitors — 920→59,969 ops, +36,800 conditional
  effects, ground ~80 ms→~1.2 s. The storage blow-up quantifies the
  roadmap's predicted goal-DNF risk: the monitors' end-state acceptance
  checks make the compiled goal's DNF exponential in the monitor count
  (3^10 = 59,049 synthetic REACH-GOAL disjunct ops, exactly), with the
  END-action construction recorded as the known fix if real workloads bite
  (`docs/roadmap-0.7.md`). Constraint-free inputs are untouched (the gate
  is a no-op), so this cost is opt-in with the feature.

- **Soft constraint-preferences are ENFORCED and PRICED** (0.7 Phase 2).
  `(preference name <constraint>)` inside `(:constraints ...)` lowers to the
  same monitor automata plus a goal-side `(preference name <acceptance>)`,
  so the whole PDDL3 metric stack (Keyder–Geffner collect/forgo pricing, the
  exact-closure optimizer, the selection layer) scores trajectory
  preferences with **no optimizer changes**. `forall` preferences expand to
  instances sharing the name (`(is-violated name)` counts violated
  instances); anonymous preferences get deterministic `TRAJPREF{n}` names;
  weight defaults match goal preferences exactly (no metric → 1 each,
  metric-unreferenced → 0), pinned by tests.
- **The IPC-5 qualitative-preferences suite is vendored and scored**
  (`benchmarks/ipc/qualpref/{openstacks,rovers,storage,tpp,trucks}`, 8
  instances each, from the potassco mirror — the track ran 5 domains; there
  is no qualitative pathways). All 40 instances parse, gate, and compile
  with no rejection; **36 of 40 produce an independently verified plan +
  metric** (32 on pure defaults, 3 more within a 600 s budget, storage
  p05/p06 under a documented `FF_NO_ESPC=1` env), and every gap has a
  named reason on the board — storage p07/p08 exceed 15 GB during
  grounding, trucks p07/p08 exceed the search budget. See
  `benchmarks/ipc5-qualitative-scoreboard.md` (self-scored: the official
  reference archive is unreachable from the dev container; the board
  documents both graft-in paths) and the heavy locks in
  `tests/ipc5_qual_metric.rs`.
- **Constraint-side static simplification** (in `constraints::compile`,
  same `FF_PREF_NO_STATIC=1` hatch as the 0.5 goal-preference pass):
  constraint bodies are partially evaluated against static predicates +
  init, and instances whose fold verdict is statically ACCEPTED are
  dropped before monitor compilation — statically VIOLATED instances are
  never dropped. This is what makes the qualitative storage instances
  compile at all (p03: 1,548 of 1,554 instances dropped; without it,
  quadratic `forall` preferences OOM grounding). Planner-side only: the
  verifier folds the unsimplified semantics, so the oracle stays
  independent.
- **The independent verifier is now authoritative on quantified preference
  bodies**: `verify::verify` grounds formula-level `forall`/`exists` (both
  in constraint bodies and in goal-preference bodies) before scoring, folds
  every soft constraint-preference over the replayed trajectory, and reports
  per-instance verdicts (`Verified::constraint_prefs`). reported ==
  verified is asserted exactly on every qualitative domain and on 5 of 6
  simple-preferences domains (rovers stays validity-only: its metric folds
  a numeric term the preference verifier doesn't recompute).

### Changed

- What stays rejected is now rejected **by name**: the four timed operators
  (`within`, `always-within`, `hold-during`, `hold-after`), any constraint
  on the temporal path (Phase 3), and `Session` (grounds once and replans
  from mutated states — a compiled monitor's S_0 baking would go stale).
  The 0.4.1 blanket rejection survives behind `FF_CONSTRAINTS_REJECT=1` (a
  restore hatch that restores *rejection*, not ignoring — no setting
  silently drops a constraint).

## [0.6.0] - 2026-07-15 — Selection: solve the choice, then plan to it

The forensics release. `docs/forensics-tpp.md` proved the remaining
tpp/pathways tail was never a search problem — on zero-action-cost domains,
plan quality is decided by WHICH jointly-satisfiable preference subset the
end state lands in, and SGPlan5's tpp p05 score is the closed-form optimum of
that selection. 0.6 answers it twice over: the guidance BARRIER flip (keep
init-satisfied preferences visible — the storage 8/8 domain sweep vs SGPlan5,
totals 234 vs 547) and the SELECTION layer (solve the subset choice exactly,
then plan to it — tpp p06 ties SGPlan5 exactly, and rovers' totals lead
widens to 4862 vs 5632 because selection also picks which samples are worth
their traverse cost). Suite tally vs the IPC-5 winner: **19W / 16T / 13L**
(0.4.0 shipped at 14/11/23), with three domains led under both quality
conventions (openstacks, storage, rovers) and trucks ahead on totals — all
on pure defaults, deterministic at any thread count. Two levers were measured
and honestly retired along the way (a weight-aware barrier variant; three
selection-shaped attempts at trucks' shared-timeline scheduling, which moves
to 0.7 as temporal selection). Every default change keeps a restore hatch
(`FF_PREF_NO_BARRIER`, `FF_PREF_NO_SELECT`).

### Added

- **The selection layer** (`selection.rs` + the closure loop's selection
  seed; `FF_PREF_NO_SELECT=1` restores 0.5.1) — the 0.6 headline, built
  from the tpp forensics: on preference domains, plan quality is largely
  decided by WHICH jointly-satisfiable preference subset the end state
  lands in, so ferroplan now solves that selection EXACTLY (a variable per
  invariant mutex group, Eq/Neq atoms coupling compiled `(NOT p)` facts to
  their groups, DFS branch-and-bound with a deterministic node cap) and
  plans to the chosen facts as a hard-goal target: singleton pre-probes ban
  supply-capped facts (on tpp they re-discover the market caps exactly),
  at most two joint attempts (per-fact bans cannot repair counting
  infeasibility), the exact tail closes, and the incumbent feeds the normal
  tightening loop. The seed's bounded evals stay OUTSIDE the tightening
  budget, like the legacy EHC seed (charging them starved storage p08,
  83 → 104 → fixed). The selection bound is admissible, so `final == bound`
  can prove optimality. Measured (defaults, deterministic, t1≡t8):
  **tpp p05 89 → 80** (the solver's bound reproduces the forensics'
  79 optimum; the +1 is one `p-drive` application, outside end-state
  selection), **p06 104 → 101 — an exact tie with SGPlan5**, p07 110 → 103;
  **rovers p02 596.7 → 502.2, p03 935.3 → 847.4, p08 998.1 → 740.9**
  (selection picks which samples are worth their traverse cost; the rovers
  totals lead widens to 4862.0 vs 5632.5). Storage's 8/8 sweep, pathways,
  trucks, and openstacks hold exactly. Suite tally vs SGPlan5:
  **19W / 16T / 13L**.

### Changed

- **Init-satisfied preferences are kept in the satisfaction guidance** (was:
  excluded since 0.4.0's barrier-free change). Plan forensics on tpp p05
  (`docs/forensics-tpp.md`) showed the exclusion makes the search blind to
  high-weight TRAP preferences — `not (stored goods1 level3)` is satisfied at
  init, so the guidance rewarded trampling it for a cheaper positive
  preference, and every restart-ladder profile inherited the blindness; the
  entire 93-vs-79 gap on that instance was this one decision. Re-measured on
  the 0.5 engine: keeping them takes **storage p05–p08 from 31/121/124/148 to
  25/43/60/83 — an 8/8 domain sweep vs SGPlan5** (totals 234 vs 547) — plus
  tpp 89/104/110/129 and pathways p06 11, at the cost of pathways p05 alone
  (6 → 6.5, a win becoming an exact tie). Suite tally vs SGPlan5:
  19W/15T/14L. `FF_PREF_NO_BARRIER=1` restores the 0.4–0.5.0 exclusion.

### Added

- `docs/forensics-tpp.md` — the tail-gap forensics: on zero-action-cost
  domains quality is pure end-state selection; SGPlan5's tpp p05 79 is
  derived as the closed-form selection optimum (per-goods stored level under
  supply caps + the 16-weight coupling constraints); the identified 0.6
  lever is exact selection planned as hard goals.

## [0.5.0] - 2026-07-14 — Closing on first: three IPC-5 domains on the defaults

The 0.5 roadmap ("First Place") executed end-to-end, shipped with its honest
verdict. On the vendored IPC-5 simple-preferences suite, **pure defaults** —
one configuration, no env vars, deterministic at any thread count — ferroplan
now **leads SGPlan5 under BOTH quality conventions (per-instance wins AND
domain totals) on three of the six domains**: openstacks (wins p04–p08, 271
vs 326), storage (wins p01–p07, 447 vs 547), and rovers (wins p04/p06/p07/
p08, exact ties p01/p05, 5301.6 vs 5632.5). trucks leads on totals (23 vs 31)
with instances drawn; tpp and pathways stay with the IPC-5 winner. Suite-wide
the instance tally is **19W / 14T / 15L** — more wins than losses against the
contest winner for the first time (0.4.0: 14/11/23). The 4-of-6 bar this
release aimed at was not met, so the claim is "closing on first," not first —
the remaining gap is exactly the tpp/pathways p05–p08 tails, measured
direction-bound (identical at 4× budget) and resistant to every lever below.
Full ledger: `benchmarks/ipc5-scoreboard.md`; the executed plan:
`docs/roadmap-0.5.md`.

### Changed

- **ESPC graduated: deterministic eval budget, default-on where it bites.**
  The penalty loop's outer budget converts from wall-clock to an evaluated-
  state pool (`FF_ESPC_EVAL_BUDGET`, default 6M) threaded through every inner
  search — thread-count and machine independent, exactly the contract
  `FF_PREF_EVAL_BUDGET` set for the B&B. `features::espc()` defaults ON (it
  engages only on deadline-pair structure — a verified no-op elsewhere);
  `FF_NO_ESPC=1` opts out; `FF_ESPC_TIME_MS` is demoted to an optional
  additional wall cap that applies only when set. The graduated default
  openstacks row reproduces the old opt-in row exactly (19/23/17/16/21/22/
  66/87; worst wall ~63 s on p04).
- **Folded numeric metrics route through the exact-closure optimizer** (was:
  legacy compiled-goal B&B). The 0.4.0 verdict that the closure path measures
  worse on rovers ("tiny-epsilon tightening churn") was an artifact of
  first-improvement restarts, which the anytime sweeps removed; with the
  routing flipped, rovers goes 935.3/653.5/1018.2/485.5/523.3/664.6/402.2/
  979.9 → **811.3/596.7/935.3/418.7/483.6/655.7/402.2/998.1** — a full
  domain lead. `FF_PREF_NUMLEGACY=1` restores the pre-0.5 split.
- **Anytime sweeps + a diversified restart ladder in both preference B&B
  loops** — the two remaining scoreboard levers, measured and landed. Each
  bounded metric sweep now tightens its bound **in place** on every acceptance
  and keeps draining (a restart happens once per eval cap, not once per
  improvement; `FF_PREF_GREEDY=1` restores first-improvement sweeps). Measured
  alone this changed no metric — the large-instance plateau was never restart
  churn but a **guidance limit** — so a capped no-improvement sweep now
  rotates the open-list weights through a fixed half-cap **profile ladder**
  (h-greedy → h-heavy → g-heavy → pure-h) under the same bound before the
  final all-remaining escalation (`FF_PREF_NO_RESTARTS=1` disables). Fully
  deterministic and thread-count independent. On the IPC-5 suite
  (`benchmarks/ipc5-scoreboard.md`): **storage now beats SGPlan5 on p01–p07
  and on the domain total** (46/145/200/263 → 31/121/124/148 on p05–p08),
  **pathways p05 flips to a win** (8.5 → 6 vs 6.5), tpp p05–p07 −4/−12/−14,
  trucks p03 1→0 and p06 6→1, openstacks default-path p01 42→23, rovers p04
  559.9→485.5 (0.1 from a tie). Cost, recorded honestly: tpp p08 +1,
  openstacks p03 +1, rovers p02 +56.8 — all already-losing instances.
  Instance tally vs SGPlan5: 14W/11T/23L → **17W/12T/19L**. The opt-in
  `FF_ESPC` openstacks path is untouched (spot-checked identical).

### Added

- `heuristic::relaxed_plan_cost` — a cost-aware relaxed plan (sums the
  selected ops' `increase` effects on a cost fluent), and an experimental
  **forgo-aware seed** built on it (`FF_PREF_SEED=1`): price each
  preference's completion from the initial state and pre-forgo those priced
  over their weight in one extra seeded solve. Measured **neutral** on rovers
  (the estimates fire correctly, but the EHC seed already lands at the same
  incumbent; identical metrics on/off across p01–p08) — default off, kept as
  the substrate for completion pricing inside the search.
- **Partitioned closure seed** (`FF_PREF_SEED3=1`, experimental, default
  off): ESPC increment 3 generalized past deadline pairs — mutex-conflict-
  pruned preference components composed into an incumbent by P3-masked,
  sibling-protected stages before the tightening loop. The composition
  genuinely works (tpp p05 composes 99 vs the 105 init-tail) but measured
  **neutral on finals**: the anytime+ladder loop reaches the same metric from
  either starting bound. Kept as the substrate for per-stage λ pricing (0.6).
- The 0.5 roadmap (`docs/roadmap-0.5.md`), now annotated with the executed
  outcome per phase.

## [0.4.1] - 2026-07-06 — Trajectory-constraint safety and a docs correctness pass

A correctness point release. It closes one silent-correctness footgun — PDDL3
trajectory `(:constraints ...)` were parsed but enforced by nothing, so a hard
constraint was accepted and dropped — and runs a documentation once-over that
retires the pre-0.4.0 "we trail SGPlan6" story the docs still told in places. No
engine or plan-quality change to any solve that succeeds today; the only behavior
change is that a domain declaring trajectory constraints now errors instead of
being silently mis-solved.

### Changed

- **PDDL3 trajectory constraints are now rejected instead of silently ignored.**
  The modal `(:constraints ...)` operators (`always`, `sometime`, `at-most-once`,
  `sometime-after`/`-before`, `within`, `hold-during`/`-after`) were parsed into
  the AST but enforced by no solving path, so a hard constraint was accepted and
  dropped. Every public entrypoint (`solve`, `decompose`, `Session::new`, the `ff`
  CLI) now returns a clear error (new `SolveError::Unsupported`) when a domain or
  problem carries one. Goal `(preference ...)` soft goals are unaffected — they
  live in the goal formula, not in `:constraints`, and the PDDL3 metric path still
  handles them.

### Added

- `ferroplan-py`: `temporal` mode, for parity with the `ferroplan-wasm` binding.
- Library examples `decompose.rs` and `validate_plan.rs` (the two advertised
  public APIs that had no runnable Rust example).
- An `examples/README.md` index (feature-by-feature map + reading order) and a
  `book/src/tuning.md` reference collecting the full `FF_*` env-knob family.

### Docs

- Corrected stale/contradictory documentation left over from before 0.4.0: the
  README's ESPC "not yet built" limitation (it shipped), the SGPlan5/SGPlan6
  baseline mix, the book's `results`/`metric-quality`/`pddl3`/`temporal` pages
  (which still told the pre-0.4.0 "we trail SGPlan6" story and marked timed
  initial literals / duration inequalities unsupported), the non-compiling
  `library.md` example, and the `village` example's false "`:derived` is rejected"
  claim. Archived the 0.2.1 roadmap.

## [0.4.0] - 2026-07-03 — Preference metrics: ferroplan takes on SGPlan5

The PDDL3 preference-metric release. On the vendored IPC-5 simple-preferences
suite (p01–p08, six domains, vs the official SGPlan5 results — see
`benchmarks/ipc5-scoreboard.md`), ferroplan goes from a distant quality 2nd to
**leading the IPC-5 winner on two domains** (openstacks via the opt-in
`FF_ESPC` partitioned penalty loop; storage on the plain defaults), **ahead on
the trucks total**, at **small-instance parity on tpp and pathways**, with
**full 48/48 coverage** (storage was 2/8) — every result deterministic and
thread-count independent.

Bumped to 0.4.0, not 0.3.1: the preference-metric default path changed (the
exact-closure optimizer replaces the compiled-goal B&B; wall time now scales
with the eval budget instead of stopping at the first failed probe) and the
public API grew (`SearchCfg::w_c`, `features::espc()` /
`set_espc_override`). Every behavior change has a restore hatch:
`FF_PREF_COMPILED`, `FF_PREF_NO_STATIC`, `FF_PREF_BARRIER`,
`FF_PREF_NO_ESCALATE`, `FF_ESPC_MONO`; budget via `FF_PREF_EVAL_BUDGET`.

### Added
- **Budget-escalating B&B retry — the eval budget becomes a real contract,
  lifting five of six IPC-5 preference domains at the default settings.**
  Both preference-metric optimizers (closure and legacy) treated one capped
  300k-eval tightening probe that found no cheaper plan as terminal, abandoning
  the optimization with most of `FF_PREF_EVAL_BUDGET` unspent — and the
  per-iteration cap was pinned at 300k, so raising the budget changed nothing
  (measured: 16x budget, identical results). A capped failure now retries the
  same bound with ALL remaining budget (deterministic eval counts, so plans
  stay thread-independent; `FF_PREF_NO_ESCALATE=1` restores the old behavior;
  the legacy loop also gains the budget accounting it never had). Measured at
  defaults: tpp p04 36 -> 35 (SGPlan5 tie, completing p01-p04 parity), tpp
  p05/p07/p08 97/131/146; trucks p07 19 -> 12 (now ahead of SGPlan5's 24 by
  half); storage p05/p06/p08 46/145/263; openstacks default p01 49 -> 42;
  rovers p02 659.3 -> 596.7 and p05 649.9 -> 523.3. Wall time now scales with
  the budget (trucks p08 ~163 s at 4 threads; lower `FF_PREF_EVAL_BUDGET` to
  trade quality for speed).
- **`SearchCfg::w_c` — experimental metric-cost open-list ordering** (default
  0.0 = priority key bit-identical), settable via `FF_PREF_COST_WEIGHT`. Built
  as the designed rovers lever and measured to be a dead end there: every
  non-zero weight collapsed rovers to the all-forgo floor (accumulated cost
  ordering buries deep goal-reaching prefixes), so the default stays 0
  everywhere and the field is documented as experimental. Additive public-API
  change to `SearchCfg` (constructors default it).
- **Exact-closure metric optimizer (new default for preference metrics) —
  storage flips from 2/8 coverage to beating SGPlan5 on p01–p05; tpp and
  pathways reach SGPlan5 parity on their small instances; trucks p08 drops
  133 → 10.** Three coupled changes to the PDDL3 path, each with a restore
  hatch:
  - *Static preference simplification* (compile): a preference whose phi is
    statically true (e.g. an `imply` over a static relation that never holds
    for that binding) can never be violated, so it is dropped before the
    Keyder–Geffner expansion — storage's quadratic forall-preference shrinks
    ~90–97% (p03: 1601 → 53 live instances; p08: 62k raw). Reported metrics
    are unaffected (the verifier scores from the original goal).
    `FF_PREF_NO_STATIC=1` restores blind expansion.
  - *Exact-closure metric search* (optimize): the anytime B&B now searches
    REAL states only, accepting a state iff the real hard goal holds and
    `cost-so-far + closure(state) < bound` — `closure` being the exact weight
    the deterministic `P3END`/collect/forgo phase tail pays from that state —
    instead of searching a compiled goal of hundreds/thousands of bookkeeping
    facts with a satisfaction-blind heuristic. The first incumbent is the tail
    applied to the initial state (instant coverage on any pure-preference
    instance); the tightening budget is a deterministic evaluated-state count
    (`FF_PREF_EVAL_BUDGET`, default 2M), so plans are thread-count
    independent, and un-capped exhaustion still proves optimality. Folded
    numeric metrics (rovers) deliberately stay on the legacy compiled-goal
    B&B; `FF_PREF_COMPILED=1` forces it everywhere. Multi-disjunct phis
    (`imply`/`exists`) now close correctly (the collect-op map kept one
    arbitrary disjunct before).
  - *Barrier-free DNF guidance*: the open-list satisfaction penalty now
    evaluates each preference's full DNF (so `imply`/`exists` preferences
    guide at all) and skips preferences already satisfied in the initial
    state — penalizing their transient dips walled off every improving
    trajectory (tpp's weight-16 `p4A` made metric 16 unreachable from 21).
    `FF_PREF_BARRIER=1` restores the old shape.

  IPC-5 defaults (release, 4 threads, all ≤ 60 s): tpp 16/24/29/36/101/116/
  133/148 (ties SGPlan5 p01–p03), storage 3/5/6/9/48/148/200/272 (beats
  SGPlan5 p01–p05; was 8/12 then nothing), trucks 0/0/1/0/0/6/19/10 (wins
  p01/p07), pathways 2/3/3/2/8.5/12.9/12.5/20.2 (ties p01–p04), openstacks
  default 49/40/29/41/67/86/153/370 (`FF_ESPC` row unchanged at 19/…/87),
  rovers unchanged. See `benchmarks/ipc5-scoreboard.md`.
- **Partitioned ESPC (opt-in `FF_ESPC`) — ferroplan now beats SGPlan5 on
  openstacks p04–p08.** The PDDL3 preference-metric penalty loop
  ("increment 2" of `docs/espc-preferences-spec.md`) couples its per-trigger λ
  schedule to a partitioned search instead of one monolithic B&B per penalty
  setting: subproblems come from the goal-interaction components of the real
  (non-compiled) goal, the shared renewable-resource variable (openstacks'
  `stacks-avail`) is excluded from component formation and priced as a global
  constraint by λ, each stage's goal is enriched with its own preference
  deliverables (the per-stage quality pressure a cost bound can't provide on
  cost-flat stage plans), the compiled `P3*` bookkeeping is closed by an exact
  phase tail, and leftover budget runs an incumbent-bounded monolithic polish
  (the "never worse than the plain B&B" floor). IPC-5 openstacks p01–p08 at
  the same 90 s budget: 42/43/55/66/81/90/151/227 →
  **19/23/17/16/21/22/66/87**, ahead of the IPC-5 winner SGPlan5 on p04–p08
  (26/36/33/67/123) — deterministic (3/3 identical runs, thread-count
  independent) and typically stall-terminated in 4–60 s. The default path is
  untouched (`FF_ESPC` stays opt-in; the other five IPC-5 preference domains
  are verified no-ops); `FF_ESPC_MONO=1` reproduces the previous monolithic
  loop. New WASM-safe toggle: `features::espc()` / `set_espc_override`.

### Fixed
- **Bevy Animator: "Animate this plan" always showed the embedded demo.** The
  Solver web page writes the domain, problem, and already-solved plan to
  `localStorage['ferroplan.handoff']` before navigating to the Animator — but no
  Rust code ever read it back, so the Animator always loaded its embedded demo
  regardless of what was actually solved and selected. `webhandoff.rs` now reads,
  parses, and applies the handoff at startup (scene + the pre-solved plan,
  autoplaying immediately — no re-solve, so it can't disagree with what the
  Solver page reported); falls back to the embedded demo if there is no handoff
  or it fails to parse. Verified in headless Chromium: no handoff → embedded
  demo; a real handoff → the handed-off domain/problem with its plan already
  playing; a corrupted handoff → clean fallback, no panic.

## [0.3.0] - 2026-07-02 — Solver depth: escalation, parallelism, sessions

A temporal goal that used to fail in ~45 s can now solve in ~30 ms (default-on
goal-relevance pruning); a search that used to just fail now escalates through two
more rungs before giving up (the Full demand tier, then the decomposer); and a
caller embedding the planner in a live loop gets a proper `Session` API instead of
re-grounding every tick. Measured on the 75-instance RPG temporal corpus:
**65 → 73 solved, zero regressions on anything that already solved.**

Bumped to 0.3.0, not 0.2.3: this release adds a new public API (`Session`) and
changes default `solve()`/`ff` behavior for temporal domains — an instance that
previously failed fast can now take substantially longer before returning
`solved: false`, because the escalation ladder tries harder before giving up
(`FF_NO_ESCALATE` restores the single-pass pre-ladder behavior). Two correctness
fixes are included too: a validator/replay bug on `:derived`-axiom domains, and a
domain-authoring bug in the `rpg-world` example (`bread-line` was unsolvable by
construction).

### Added
- **`Session` — ground once, replan many.** The embedding API for callers that
  re-solve the same world every tick (a game's villagers, a simulation loop):
  `Session::new` parses, compiles `:derived` axioms, and grounds ONCE; the session
  then holds the *current world state* — mutate it with `set_fact`/`set_fluent`
  (plus `fact`/`fluent` readbacks) as the world evolves and `replan()` solves from
  wherever it stands, paying only the search. Measured on `villagers`: a
  tick-sized contract (`errand`) drops **223 µs → 22 µs per replan (~10×)**; a
  search-dominated instance (`township`) is break-even, as expected — size
  per-agent contracts small (the decomposer's whole job) and the tax vanishes.
  Static facts are rejected with an explanatory error (grounding bakes them in;
  flipping one could require never-enumerated operators), as are temporal and
  PDDL3-preference inputs (v1 scope). See `examples/replan.rs`.

### Solver
- **Goal-relevance pruning graduated to the default tier.** Previously it rode the
  opt-in `FF_TDEMAND` Full tier only; the default search could exhaust its node
  budget in goal-irrelevant unbounded accumulators (`food=1,2,3,…`) on
  feature-rich domains. Measured trigger: on the rpg-world bread-line hub,
  `flour >= 2` — a 5-step till→plant→irrigate→harvest→mill chain — **failed after
  ~45 s; it now solves in ~30 ms** under defaults. The pass structure gains an
  **unmasked complete backstop** (helpful/sound → full/tight → full/sound →
  full/unmasked), so completeness is now *unconditional* — a hypothetical mask bug
  can cost time, never coverage. `FF_NOREL` disables pruning alone;
  `FF_NO_TDEMAND` still restores the pristine pre-v0.2 path.
- **Static unproducibility check — fail unsolvable goals in microseconds.** If a
  positive goal fact has no adder anywhere in the grounded task, or a `>=`/`>`
  numeric goal's fluent has no effect that could ever raise it, the temporal
  search (and every decomposer contract) now reports unsolvable immediately
  instead of exhausting every pass — bread-line's unproducible goal went from a
  **~45 s** exhaustive failure to **~9 ms**. Sound and conservative: an effect
  counts as a potential raiser unless it provably never raises; the check never
  changes a found plan.
- **Validator/replay fix: `:derived` domains.** Every solve path compiles derived
  axioms into init facts before grounding — but `plan::validate_plan` (the CLI
  `--validate`), `verify::verify`, and `trace::trace` replayed against the **raw**
  problem, so on axiom-using domains (e.g. rpg-world's `(:derived (reachable …))`)
  they wrongly rejected valid plans ("problem grounds to unsolvable" / "unknown
  action") and the GUI animator couldn't trace them. All three now run
  `derived::compile` first (identity when a domain has no axioms).
- **rpg-world domain fix: the bread economy.** `bake-bread` produced `meals`
  directly, leaving the `bread` fluent with **no producer** — so `hard/bread-line`
  was unsolvable-by-construction (violating the hard-set's "solvable in principle"
  contract) and `plate-spread` was dead code. `bake-bread` now yields bread
  (cook bonus included); meals keep their direct path via `cook-meal`, and the
  bread→plate-spread→meals chain is live. `bread-line` now solves and validates
  under default options.
- **On-failure escalation ladder.** When the default-tier monolithic temporal
  search fails, `temporal::solve` now retries at the **Full demand tier**
  (predicate-goal seeding), then hands the goal to the **decomposer** — each rung
  runs only after the previous one failed, so no instance that solves today can
  change its plan; the ladder spends extra time on would-be failures to convert
  them into solves. Ladder gains (all plans independently `--validate`d):
  `crew-solo`/`crew-pair`/`skilled-specialists` at the Full rung (makespans
  109/152/198 — matching their documented flagged solves, now flag-free),
  `order-8`/`order-12` and `found-village` at the decomposer rung. The tier is
  now threaded explicitly through the search (no racy global overrides), the
  decomposer's own monolithic fallbacks are **skipped on the ladder path** (the
  ladder already exhausted that exact search at both tiers — and this is also
  what makes the ladder recursion-free), and TIL-bearing compositions stay safe
  (the decomposer hard-validates before returning). `FF_NO_ESCALATE` — or
  `features::set_escalate_override(false)` in-process (WASM) — disables the
  ladder alone; `FF_NO_TDEMAND` still restores the pristine pre-v0.2 path.
- **Parallel temporal search.** The decision-epoch search now evaluates successor
  heuristics **in parallel** (the `threads` option previously only parallelized
  grounding on the temporal path). Successors are generated serially, batch-
  evaluated across workers (one relaxation `Scratch` per worker; frontiers under
  128 stay on the serial path with zero new allocation — per-round fan-out has to
  amortize against a full unpruned op scan to win), then enqueued serially in
  input order — so the heap and visited-set evolve exactly as before and **plans
  are byte-identical for any thread count**, verified by a corpus-wide
  determinism sweep at `--threads 1/2/4/8` (65 instances, 0 mismatches).
  Measured honestly: the win is modest (~4% on exhaustion-bound searches, ~0 on
  typical solves) — the temporal search is dominated by its serial successor-gen
  / dedup / heap machinery, so this lays the plumbing without changing the
  performance story; the corpus-visible speed lever remains the ladder + pruning.

**Measured** on the full temporal corpus (rpg suite + hard + contracts, cabin,
villagers — 75 instances): **65 → 73 solved, zero losses, zero makespan changes
on previously-solving instances** (pruning graduation +2, escalation ladder +6).
The hard set is now 12/12 — 10 under plain defaults (was 3/12 when authored) and
the two big conjunctive orders via the ladder's decomposer rung. The remaining
corpus misses are `crew-trio` and `skilled-crosstrained`, which resist every
rung — the honest border.

### Benchmarks & docs
- **IPC-5 openstacks: the opt-in `FF_ESPC` gap to SGPlan5 re-measured, ~5× → ~3×.**
  A fresh measurement (`FF_ESPC=1 FF_ESPC_TIME_MS=90000`, 4 cores) narrows the
  scoreboard's headline quality gap: 42/43/55/66/81/90/151/227 vs. the prior
  default row 63/66/62/66/138/129/278/608 across p01–p08, no instance regresses.
  The loop is budget-sensitive — at the *default* 15 s only p01/p02/p06 improve
  on the same box.
- **`docs/espc-preferences-spec.md`: the general-path ESPC blocker has been
  built.** A 2026-07 revisit found that the multi-predicate mutex-group
  synthesis added since the original "deferred" decision (`invariants.rs`) now
  recovers exactly the `(STACKS-AVAIL n)` guidance variable a faithful
  cross-domain ESPC needs — the specific gap the deferred design cited as
  blocking. What remains is "increment 2": coupling the `espc.rs` penalty
  schedule to the partitioned search (subproblems from goal-interaction
  components, global constraints on shared mutex variables). Not yet
  implemented; recorded as the concrete next step.

## [0.2.2] - 2026-06-30 — GUI & tooling

A GUI- and tooling-focused release: the web surfaces and the native Bevy app get a
shared "forge" visual identity, the animator gains a real timeline UI (a scrubbable
transport bar) plus a temporal timescale (Gantt) view, the engine is brought up to
current dependencies, and the publish pre-flight is fast again. No solver/library
API or behavior changes — `ferroplan` / `ferroplan-cli` are functionally identical
to 0.2.1 (dependency refresh only).

### Added
- **Animator transport bar** (native Bevy GUI) — a play/pause button, a scrubbable
  timeline (click or drag to seek, one notch per step), a molten progress fill +
  playhead, and a step/time readout. Mirrors the keyboard controls so the animator is
  usable with the mouse alone.
- **Temporal timescale (Gantt) view** — temporal plans (overlapping durative actions
  the graph can't tween) are now legible: each durative action is a bar on a shared
  plan-time axis, greedily lane-packed so non-overlapping actions share a row, coloured
  by the acting object, with a cyan "now" line swept by the transport playhead. Toggle
  with **T**.
- **Duration-aware playback + active-edge highlight** — classic plans dwell on each
  step in proportion to its `duration`; temporal plans sweep their whole makespan in a
  fixed wall-clock time (relative durations preserved); the edge a mobile is traversing
  at the current timeline position is recoloured molten and thickened.

### Changed
- **"Forge" visual identity** across all three surfaces — the Solver web demo, the
  Bevy visualizer/animator web shell, and the native GUI are restyled to a shared
  dark / molten / cyan palette, and the logo is retinted to match (cyan start, molten
  target).
- **Bevy 0.15 → 0.19** — the GUI is migrated to current Bevy (rendering split into
  `*_render` feature crates, the `Projection` enum, and the `BorderColor` /
  `BorderRadius` / `FontSize` / `ScrollPosition` API changes). Building the GUI now
  needs Rust ≥ 1.95; the published library keeps its 1.74 MSRV (it has no Bevy
  dependency).
- **Dependencies modernized** — `thiserror` 1 → 2, `criterion` 0.5 → 0.8, `pyo3`
  0.24 → 0.29, `wasm-bindgen` pinned to 0.2.126, and the rest brought current.

### Fixed
- **Fast publish pre-flight / `cargo test`** — two IPC-benchmark regression guards
  (`espc` ~346 s, `ipc5_pref_metric` ~175 s) are now `#[ignore]`d, so the default test
  run (and `publish.sh`) finishes in seconds. They remain gated: CI runs them in
  release (`cargo test --release -p ferroplan -- --ignored`), and `RUN_HEAVY=1
  ./publish.sh` (or `cargo test -- --include-ignored`) includes them on demand. No
  assertions changed — only when they run.
- **Bevy GUI black screen on launch** — the 0.19 render features (`bevy_ui_render`,
  `bevy_gizmos_render`, `bevy_sprite_render`) weren't enabled, so the ECS data was
  there but nothing drew.

## [0.2.1] - 2026-06-26 — "The Bridge"

The engine release (0.1) made ferroplan fast and correct; 0.2 makes the README's
bet real and inspectable: the proven temporal heuristics are on by default, temporal
coverage goes deeper (duration inequalities + timed initial literals), and a goal too
big for the one-shot search is **automatically decomposed** into solvable,
individually-verified contracts.

### Added
- **`parse` API — syntax-check PDDL without solving.** `ferroplan::parse(src)`
  auto-detects domain vs problem, validates syntax, and returns a serde-serializable
  `ParseReport` (ok/error-with-line, name, requirements, and a structure summary:
  types/predicates/functions/actions or objects/init/goal/metric/TIL counts) — fast
  feedback for an authoring loop or editor tooling, no grounding or solving. Exposed
  as a **`parse` MCP tool** too.
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
