# PDDL3 preferences

ferroplan compiles soft-goal preferences away (Keyder & Geffner, JAIR 2009) and
minimizes the `:metric`.

- **Goal preferences**, including `(forall (?x) (preference p phi))`, are
  expanded into one instance per binding; `(is-violated p)` counts violated
  instances.
- **Precondition preferences** become satisfied/violated action variants, so a
  violation is charged exactly once per application.
- The metric must be linear in `(is-violated …)` and `(total-cost)` (the IPC-5
  *simple-preferences* shape) — plus any monotone numeric term (e.g. rovers'
  `sum-traverse-cost`), which is folded into `total-cost`; maximize / negative /
  scaled metrics fall back to a satisficing plan with a clear note.

## The optimizer (0.4.0)

The default is an **exact-closure metric optimizer**: it searches real states with
metric-bounded acceptance and closes the compiled preference bookkeeping with a
provably-optimal `collect`/`forgo` phase tail. Three pieces make it scale:

- **Static preference simplification** at compile — statically-satisfied preference
  instances are dropped before grounding (storage's 62k-instance quadratic `forall`
  collapses ~97%).
- **Barrier-free full-DNF guidance** — the search sees a preference's forgone cost
  directly instead of behind a compilation barrier.
- **A budget-escalating branch-and-bound** — a tightening probe that hits its
  per-iteration eval cap without improvement retries the same bound with the
  remaining budget rather than giving up. The deterministic, thread-count-independent
  budget is `FF_PREF_EVAL_BUDGET` (default 2M evals) — a real quality dial.
- **Anytime sweeps + a diversified restart ladder** — each sweep tightens its
  bound in place on every acceptance (a restart happens once per cap, not once
  per improvement), and a capped sweep that fails to improve rotates the
  open-list weights through a fixed profile ladder before the final
  full-budget escalation — a stuck h-ordering is a direction problem, not a
  budget problem. This is what broke the storage/tpp large-instance plateau
  (storage now beats SGPlan5 on p01–p07).

For resource-coupled domains an **opt-in ESPC penalty loop** (`FF_ESPC`, after
Hsu–Wah's extended-saddle-point method) prices a shared resource as a global
constraint across a partitioned search — the lever that puts openstacks ahead of
SGPlan5 on its larger instances. Every knob has a restore hatch (`FF_PREF_COMPILED`,
`FF_PREF_NO_STATIC`, `FF_PREF_BARRIER`, `FF_PREF_NO_ESCALATE`, `FF_ESPC_MONO`); see
the [tuning reference](./tuning.md).

On the largest instances exact optimization may return a best-found plan (flagged
*not proven optimal*) within the budget. Full per-instance results vs SGPlan5:
[`benchmarks/ipc5-scoreboard.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md).

## Trajectory constraints (`(:constraints ...)`) — enforced since 0.7

The six untimed modal operators — `always`, `sometime`, `at-most-once`,
`sometime-after`, `sometime-before`, `at end` — are **enforced on the
classical path** by compiling each ground constraint instance into a small
monitor automaton: fresh 0-ary monitor facts transitioned by conditional
effects on every action, with acceptance checked at the goal. `forall`
outside a `(preference ...)` multiplies instances (so `(is-violated name)`
counts violated instances); `and`/`forall` *inside* a preference body stay
ONE instance, violated at most once — the PDDL3 instance boundary.

- **Hard** constraints gate the goal through a forced-terminal END action
  (since 0.8): acceptance is latched by conditional effects on one
  synthetic `TRAJ-END` step — stripped from reported plans — so the
  compiled goal stays literal-only and grounding cost is LINEAR in the
  monitor count (the 0.7 goal-conjunct compilation was exponential in the
  worst case; `FF_NO_TRAJ_END=1` restores it). A plan that violates a hard
  constraint is simply not a plan.
- **Soft** `(preference name ...)` constraints lower to ordinary goal
  preferences priced by the metric machinery above — the whole optimizer
  stack applies unchanged, and `(is-violated name)` works across goal and
  constraint preferences in one namespace.
- The monitor transition block is ground ONCE and shared across all ground
  actions (since 0.8, `FF_NO_COND_SHARE=1` restores per-op copies) — the
  storage-scale instances that previously exhausted 15 GB during grounding
  now ground in well under a second at ~100–200 MB.
- The independent verifier (`ferroplan::verify`) replays the ORIGINAL
  constraint semantics over the trajectory — never the compiled monitors —
  so reported metrics are cross-checked by construction, and
  `validate_plan` rejects constraint-violating plans.
- Statically decidable instances are simplified away before grounding
  (quadratic `forall` constraints over static relations stay tractable);
  `FF_PREF_NO_STATIC=1` restores the blind expansion.

The timed operators (`within`, `always-within`, `hold-during`,
`hold-after`) and constraints on durative-action domains are **rejected by
name** — never silently dropped. `FF_CONSTRAINTS_REJECT=1` restores the
pre-0.7 blanket rejection. Measured results on the IPC-5
qualitative-preferences track:
[`benchmarks/ipc5-qualitative-scoreboard.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-qualitative-scoreboard.md).
