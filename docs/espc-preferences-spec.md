# ESPC preference optimization — implementation spec (groundwork)

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
