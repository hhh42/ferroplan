# ESPC preference optimization — implementation spec (groundwork)

> **Status update — ESPC is now implemented** (opt-in `FF_ESPC`), see `crate::espc`
> and the CHANGELOG. The realization differs from the original sketch below in one
> key way: rather than a soft *occupancy* penalty (which §"Conclusion" correctly
> found inert), the loop penalizes — on the **concrete** state — once-only
> conditional achievements that fire *without delivering* (a product made while its
> orders still wait), and **adapts a per-trigger penalty** across the outer loop
> with iteration 0 as a penalty-free floor. Measured: it narrows the openstacks gap
> substantially (p01 63→42 … p08 608→227, ~11–63%) without regressing any instance,
> but does **not** reach SGPlan's level — consistent with the conclusion below that
> closing it fully needs a real min-open-stacks scheduler. The notes below are kept
> as the original design record.

How SGPlan5 (Hsu, Wah, Huang & Chen, IPC-2006) gets good metrics on hard PDDL3
**preference** problems, distilled from deep research (primary sources: IJCAI-2007
#310; AIJ-2006 Wah & Chen; IPC-2006 booklet; ICAPS-06 workshop). This is the
*genuine* fix for ferroplan's IPC-5 **quality** gap (coverage is already on par;
metric quality trails, e.g. openstacks 70 vs 13). Not yet implemented — the
`forbidden`/`plan_avoiding` plumbing in `search.rs` + `Compiled.forgos` in
`pddl3.rs` are the groundwork.

## Why our current approach can't close the gap (measured)

- Monolithic anytime B&B: 10× the eval budget left openstacks/p01 at metric 70 —
  it's a *search-direction* problem (length-first can't reach the longer plans
  that satisfy more preferences), not a budget problem.
- Per-preference "force-collect" (forbid a forgo, re-solve): helps marginally on
  few-preference problems (rovers 698→646) but **does not** help openstacks (stays
  70) and times out many-preference instances. Reverted.
- Root cause (research): in openstacks the *subproblems are trivial* — the whole
  difficulty is **joint global-constraint coordination across preferences**, which
  neither monolithic B&B nor per-preference forcing performs.

## The SGPlan method

1. **Partition by guidance variables.** Nodes = multi-valued (SAS+/MDF) state
   variables appearing in goal-state constraints. Edges = constraints over them; a
   soft goal/preference is an **edge weight = its violation cost**, never its own
   node. METIS min-cut. #partitions = `min(#guidance-vars, #bottleneck-vars)`;
   grain chosen to minimize shared variables (→ fewer global constraints).
   Constraints spanning partitions become **global**; localized ones are **local**.

2. **Per-subproblem objective.** Subproblem *t* keeps only its stage's local
   constraints hard and folds **all global constraints into the objective** as
   penalty-weighted violations:
   `min_z(t)  J(z) + γᵀ|H(z)| + ηᵀ max(0, G(z))   s.t. local h(t)=0, g(t)≤0`.
   Solved by the modified-FF subplanner whose heuristic minimizes
   `Π(z) + τ·eᵀ + Σ_{k≠t} γ_{t,k}·em_{t,k}` (Π = Metric-FF heuristic for the
   subgoal; em = estimated active mutexes between subplans).

3. **ESPC resolution loop** (CPOPT partition-and-resolve):
   ```
   automated_partition()
   γ ← γ0 ; η ← η0
   repeat (OUTER):
     for t in 1..N:
       solve P_t with CURRENT FIXED γ, η          # modified-FF subplanner
       update_penalty()                            # raise penalties on violated
                                                   #   global constraints, INSIDE the loop
     recompute global metric; keep best plan as incumbent   # anytime
   until no global constraint is violated (an extended saddle point)
   ```

4. **Penalty update (reimplementable).** `γ ← γ + ρᵀ|H(z)|`, `η ← η + ϱᵀ max(0,G)`.
   Rate `ρ` is per-constraint and adapted multiplicatively by a **consecutive-
   violation counter**: when constraint *i* has been violated for *K* consecutive
   subproblem evaluations, increase its rate. Penalty multipliers are **separate
   from preference weights** (weights compute the metric; multipliers drive
   violations to zero).

5. **Preference classes.** Class 1 = final-state / `always` preferences → enforced
   as **local** constraints, solved by enumerating reachable values of each
   involved variable + backtracking on reachability. Class 2 = the rest →
   **relax-and-tighten** (ignore first, then penalize unsatisfied).

## Mapping to our Keyder–Geffner compilation (the actionable plan)

IPC-5 "simple-preferences" metrics depend only on the **final state**, so (per the
research) the optimal collect/forgo assignment is computable from final-state
constraints + reachability:

1. Treat each `collect_i / forgo_i` decision as a binary guidance variable.
2. Build the interaction graph over the **objects/predicates** the preferences'
   `phi_i` share; partition into loosely-coupled groups (union-find or METIS).
3. Per group, find the max-weight jointly-satisfiable subset of its preferences
   (force those collects, via the existing `plan_avoiding` forbidden-forgo
   mechanism) — small groups make this tractable where the monolithic problem
   isn't.
4. Resolve cross-group conflicts (satisfying group A's prefs forbids group B's)
   with the penalty loop: penalize the shared/global constraints, re-solve groups,
   iterate to an ESP; keep the best metric as an anytime incumbent.

### Open / ambiguous (flagged by the research)
- Exact `update_penalty` schedule + the consecutive-violation threshold *K*.
- Outer-loop termination beyond "no global violation".
- Grain-size selection is a stated objective, not a precise algorithm.

## Conclusion of the implementation study (decision: deferred)

A 4-design / adversarial-critique / synthesis study was run on top of this spec,
plus six measured implementation attempts. The honest finding:

**A general ESPC-style preference optimizer is NOT tractable in ferroplan as it
stands.** Every approach that fits the architecture (the Keyder–Geffner
collect/forgo representation + delete-relaxation heuristic) reduces to a
"force-collect" lever — forbid `forgo_i` so the search must achieve `phi_i` — and
*all variants were built and measured to NOT improve the metric*:

| variant | result |
|---|---|
| cost-aware heuristic + cost-first A*/WA* (×2) | suboptimal + timeouts; `h` blind |
| 10× B&B budget | openstacks unchanged (70) — search *direction*, not budget |
| per-preference greedy force-collect | no gain; timed out many-pref instances |
| all-forgo coverage floor | slow base search; hollow metrics |
| batch force-collect (top-{100/50/25}%) | no gain on pathways/rovers; regressed openstacks coverage via latency |

Two root causes, both architectural:
1. Under delete-relaxation the free `forgo_i` makes every `collected_i` trivially
   reachable, so the heuristic is **blind to which preferences a plan satisfies**.
2. The hardest gap (openstacks, 70 vs 13) is the **minimum-open-stacks scheduling**
   problem; its coupling lives in the `stacks-avail` resource, which appears in no
   preference `phi` and is **invisible to any phi-based partitioning**. A faithful
   ESPC needs SAS+/mutex-group guidance variables over `stacks-avail`.

**Two future paths (neither pursued now):**
- *General:* build a SAS+/mutex-group translation layer, then the real partition +
  penalty-resolution loop. Multi-week; the right architecture but a large build.
- *Scoreboard-only:* a bespoke `openstacks` min-open-stacks oracle (detect the
  structure, schedule outside the relaxation-blinded search, inject as a
  fail-closed incumbent). ~3 days; reaches ~20 not 13; domain-specific code, **not
  a general planner advance** — explicitly a scoreboard fix.

Decision: **coverage is already on par with SGPlan6 (39/48); the remaining gap is
metric quality on solved instances.** Neither future path is justified for the
current milestone, so ESPC is deferred. The `forbidden`/`plan_avoiding` plumbing
and `Compiled.forgos` are retained as groundwork for the general path.

## Revisit (2026-07) — the general path's blocker has since been built

Two facts have changed since the "deferred" decision above, re-verified live:

1. **Root cause 2 no longer holds.** The multi-predicate (Helmert-style)
   monotonicity-invariant synthesis in `crates/ferroplan/src/invariants.rs`
   (see `docs/invariants-measurement.md`) recovers **exactly one mutex group on
   every openstacks instance: `(STACKS-AVAIL n)`** — the precise guidance
   variable this study said a faithful ESPC needs and phi-based partitioning
   can't see (verified: `cargo run --release -p ferroplan --example
   invariants_coverage -- benchmarks/ipc/pref/openstacks`). The groups are
   already consumed by classical partitioning
   (`partition::interaction_partition` → `resolve::solve`).

2. **The penalty loop exists** (`crate::espc`, opt-in `FF_ESPC`) but is still
   coupled to the bespoke make-deadline trigger on the *monolithic* search, and
   its win is **budget-bound**: on a 4-core box at the default 15 s budget only
   p01/p02/p06 improve (42/43/100); at `FF_ESPC_TIME_MS=90000` the loop
   reproduces the recorded quality (e.g. p05 135→81).

So the "multi-week translation layer" half of the general path is done and wired;
what remains is **increment 2** (named at the end of
`docs/invariants-measurement.md`): couple the `espc.rs` penalty schedule to the
partitioned search — subproblems from the goal-interaction components, global
constraints = cross-partition transitions of shared mutex variables
(openstacks: `stacks-avail`), λ raised per the existing per-trigger schedule.
That is also the fix the classical measurement predicts for the
resource-coupled partition regressions (gripper/logistics re-traversal).

## Closed (2026-07) — increment 2 built and measured

Increment 2 shipped on the PDDL3 metric path (opt-in `FF_ESPC`, default path
untouched). Each λ iteration now runs a **partitioned composition** instead of
the monolithic tightening B&B:

- Subproblems: interaction components over the real (non-`P3*`) goal
  (`partition::interaction_partition_of`), with the detected renewable-resource
  variables (`stacks-avail`) **excluded from edge formation** — priced as
  global constraints by the per-trigger λ schedule, exactly as prescribed
  above. On openstacks: one component per order.
- Per-stage quality pressure: stage goals are **enriched with the component's
  own preference deliverables** (a goal claims a deliverable when one of its
  achiever ops requires the deliverable's conditional-achievement condition —
  `ship-order(o)` requires `started(o)`, the condition under which
  `delivered(o,p)` fires), skipping deliverables already locked out. This
  replaces the monolithic B&B's cost bound, which cannot prune cost-flat stage
  plans; infeasible enrichment degrades to the bare goal, never a conflict.
- The `P3*` bookkeeping is closed by an exact phase tail (`P3END`, then
  collect-iff-applicable-else-forgo per preference), and leftover budget goes
  to a monolithic polish B&B bounded by the incumbent (restores the plain-B&B
  floor). `FF_ESPC_MONO=1` reproduces the pre-increment monolithic loop.

Measured (release, 4 threads, `FF_ESPC_TIME_MS=90000`, 3 identical runs per
instance, stall/saddle-terminated well inside budget): openstacks p01–p08
42/43/55/66/81/90/151/227 → **19/23/17/16/21/22/66/87** — ahead of SGPlan5
(13/16/12/26/36/33/67/123) on p04–p08. The other five preference domains carry
no deadline pairs, so `FF_ESPC=1` remains a verified no-op there. See
`benchmarks/ipc5-scoreboard.md`.

## Follow-on (2026-07) — the closure optimizer generalizes the tail to the default path

The phase-tail machinery built for increment 2 became the core of the DEFAULT
preference-metric optimizer (see CHANGELOG "exact-closure metric optimizer"):
static preference simplification at compile, real-state search with
metric-bounded acceptance (`cost + closure(state) < bound`), the exact tail as
closure, and barrier-free full-DNF satisfaction guidance. Effects on the other
IPC-5 preference domains: storage 2/8 coverage → 8/8 and ahead of SGPlan5 on
p01–p05; tpp/pathways parity with SGPlan5 on their small instances; trucks
lifted across the row. The `FF_ESPC` openstacks path is unchanged (verified:
locked results, t1≡t8); the openstacks DEFAULT dropped 63 → 49 by riding the
same closure optimizer. Remaining gaps and the next levers are tracked in
`benchmarks/ipc5-scoreboard.md` ("Path to climb" items 4–5).
