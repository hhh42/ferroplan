# Metric quality & invariants

Two pieces of the IPC-5 / SGPlan-class work: a satisfaction-guided metric
optimizer for the preference (soft-goal) track, and the Helmert-style mutex-group
synthesis that feeds the partition-and-resolve mode.

## IPC-5 / metric quality

ferroplan handles all six IPC-5 (2006) *simple-preferences* soft-goal domains —
openstacks, tpp, storage, trucks, rovers, pathways — where the `:metric` charges
for violated preferences (and, in rovers, a numeric traverse cost) and **lower is
better**. Preferences are compiled away (Keyder & Geffner) and the metric is driven
by an exact-closure optimizer with budget-escalating branch-and-bound (see
[PDDL3 preferences](./pddl3.md)).

rovers was the last to fall in: its metric also charges a **monotone numeric
quantity** (`sum-traverse-cost`), which the optimizer used to ignore — scoring a
bogus `0`. Folding monotone numeric terms into total-cost lets it optimize the
*full* metric (a real **935.3**); see [Performance](./performance.md).

The hard part is that delete-relaxation hides the cost of *forgoing* a soft goal:
the free Keyder–Geffner forgo makes every preference look reachable, so on
`openstacks-soft` the metric search had no gradient toward actually delivering —
it sat on the **all-forgo floor** (metric 70 on p01). Two engine steps closed most
of that gap:

1. **Satisfaction-guided ordering** (`search::SatGuidance`) — a heap penalty
   counting the preferences forgone in the *concrete* state, giving the search a
   reason to deliver. It broke the floor (70 → 63 on openstacks p01) without ever
   regressing, since it only changes node ordering (branch-and-bound keeps the best
   plan found).
2. **The exact-closure metric optimizer** (0.4.0, now the default) — real-state
   search with metric-bounded acceptance plus an exact `collect`/`forgo` phase
   tail, static preference simplification at compile, barrier-free full-DNF
   guidance, and a budget-escalating branch-and-bound. This pushed openstacks p01
   further (63 → 42) and lifted whole domains (storage to full 8/8 coverage,
   trucks p08 133 → 10).

The last piece for openstacks was the *scheduling* of the shared `stacks-avail`
resource, invisible to the satisfaction term because it appears in no preference.
That is exactly what the opt-in **ESPC penalty loop** (`FF_ESPC`) now does: its λ
schedule drives a partitioned composition that prices `stacks-avail` as a global
constraint — taking openstacks **ahead of SGPlan5 on p04–p08**.

**Standing (vs SGPlan5, the IPC-5 winner):** full 48/48 coverage and a domain-level
lead on two of six (openstacks with `FF_ESPC`; storage on defaults), parity on the
small instances nearly everywhere else — a strong 2nd under the coverage-first
rule. The full per-instance tables, the ESPC method, and reproduction commands:
[`benchmarks/ipc5-scoreboard.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md).

## Mutex groups & SAS+

For the SGPlan-style [partition mode](./architecture.md), ferroplan synthesizes
**mutex-group "guidance variables"** — Helmert-style monotonicity invariants
that recover the SAS+ multi-valued variables of a task (the "where is X / what is
held" facts that are mutually exclusive).

A cheap single-predicate pass only finds variables whose values are all the same
predicate — the clean position variables (`at-robby`, `lift-at`, `pointing`) —
and yields **nothing** on blocks or logistics. The real work is the
**multi-predicate** refinement: when an action unbalances a candidate (it adds
into the variable via one predicate but deletes via another), the candidate is
extended with the deleted-*and-required* fact and re-verified to a fixpoint
(`crates/ferroplan/src/invariants.rs`). That recovers exactly the variables the
partitioner needs:

| domain | single-pred | multi-pred | biggest group |
|---|---|---|---|
| blocks | 0%, 0 grp | **100%, 9 grp** | block support `{on, ontable, holding}` |
| logistics | 0%, 0 grp | **93%, 9 grp** | object location `{at, in}` |
| gripper | 7%, 1 grp | **71%, 7 grp** | gripper hand `{free, carry}` |

These groups feed SGPlan-style partitioning: the initial partition is seeded from
a goal-interaction graph over the mutex variables, and on a conflict the resolver
merges the actual conflicting pair. The result shortens **blocks plans ~25%**
where goals share structure but aren't resource-coupled; on resource-coupled
domains naive decomposition still re-traverses the shared resource — which the
opt-in ESPC penalty loop (`FF_ESPC`) now prices as a global constraint. Method,
coverage numbers, and findings:
[`docs/invariants-measurement.md`](https://github.com/hhh42/ferroplan/blob/main/docs/invariants-measurement.md).
