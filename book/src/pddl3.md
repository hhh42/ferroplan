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

For resource-coupled domains an **opt-in ESPC penalty loop** (`FF_ESPC`, after
Hsu–Wah's extended-saddle-point method) prices a shared resource as a global
constraint across a partitioned search — the lever that puts openstacks ahead of
SGPlan5 on its larger instances. Every knob has a restore hatch (`FF_PREF_COMPILED`,
`FF_PREF_NO_STATIC`, `FF_PREF_BARRIER`, `FF_PREF_NO_ESCALATE`, `FF_ESPC_MONO`); see
the [tuning reference](./tuning.md).

On the largest instances exact optimization may return a best-found plan (flagged
*not proven optimal*) within the budget. Full per-instance results vs SGPlan5:
[`benchmarks/ipc5-scoreboard.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md).
