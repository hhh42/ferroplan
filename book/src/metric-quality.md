# Metric quality & invariants

Two pieces of the IPC-5 / SGPlan-class work: a satisfaction-guided metric
optimizer for the preference (soft-goal) track, and the Helmert-style mutex-group
synthesis that feeds the partition-and-resolve mode.

## IPC-5 / metric quality

ferroplan handles all six IPC-5 (2006) *simple-preferences* soft-goal domains —
openstacks, tpp, storage, trucks, rovers, pathways — where the `:metric` charges
for violated preferences (and, in rovers, a numeric traverse cost) and **lower is
better**. Preferences are compiled away (Keyder & Geffner) and the metric is
driven by anytime branch-and-bound (see [PDDL3 preferences](./pddl3.md)).

rovers was the last to fall in: its metric also charges a **monotone numeric
quantity** (`sum-traverse-cost`), which the optimizer used to ignore — scoring a
bogus `0`. Folding monotone numeric terms into total-cost lets it optimize the
*full* metric (a real **935.3**); see [Performance](./performance.md).

The hard part is that delete-relaxation hides the cost of *forgoing* a soft goal:
the free Keyder–Geffner forgo makes every preference look reachable, so on
`openstacks-soft` the metric search had no gradient toward actually delivering —
it sat on the **all-forgo floor** (metric 70 on p01). A **satisfaction-guided**
optimizer (`search::SatGuidance`) fixes this: a heap penalty counts the
preferences forgone in the *concrete* state, giving the search a reason to
deliver. It broke the floor — **70 → 63** on openstacks p01 — and, because the
guidance only changes node *ordering* (branch-and-bound still keeps the best
plan found), it is monotone by construction and never regresses.

| openstacks | tpp | storage | trucks | rovers | pathways |
|---|---|---|---|---|---|
| 63 | 21 | 8 | 0 | 935.3 | 2 |

(trucks reaches metric 0 — every preference satisfied; rovers' 935.3 is the
folded numeric metric, not a preference-violation count.) The residual gap to
SGPlan6's ~13 on openstacks is the *scheduling* of the shared `stacks-avail`
resource, which the satisfaction term can't see because it appears in no
preference — closing it needs the SAS+ mutex-group partition plus a resource
penalty loop.

**IPC-5 retroactive ranking (in progress).** We're scoring ferroplan against the
2006 contest entrants to see where it would have placed; the numbers are still
being computed. Full per-instance table, ranking, and reproduction:
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
domains naive decomposition still re-traverses the shared resource (the open
ESPC penalty-loop work). Method, coverage numbers, and findings:
[`docs/invariants-measurement.md`](https://github.com/hhh42/ferroplan/blob/main/docs/invariants-measurement.md).
