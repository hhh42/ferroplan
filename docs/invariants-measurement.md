# Mutex-group synthesis — coverage measurement (increment 1)

"Measure before commit" for the SAS+ slice (`crates/ferroplan/src/invariants.rs`),
to decide whether the cheap **single-predicate** monotonicity-invariant synthesis
is strong enough to feed SGPlan/ESPC subgoal partitioning (Plan B).

Tool: `cargo run -p ferroplan --example invariants_coverage -- <domain-dir>...`

## Results (in-repo + parent IPC benchmarks)

| domain | groups | coverage | biggest group |
|---|---|---|---|
| blocks | **0** | 0% | — |
| logistics | **0** | 0% | — |
| gripper | 1 | 2–7% | `(at-robby …)` |
| elevator | 1 | 22–29% | `(lift-at …)` |
| rovers | 1 | 5% | `(at rover0 …)` |
| satellite | 1 | 35% | `(pointing sat0 …)` |
| trucks | 1 | 2–5% | `(time-now …)` |

## Finding

Single-predicate synthesis finds **only** variables whose values are all the same
predicate — the clean "X is at exactly one Y" position variables (`at-robby`,
`lift-at`, `pointing`, truck/rover `at`, `time-now`). Coverage is 2–35% (usually
<10%), and **blocks/logistics yield nothing**.

Everything it misses is **multi-predicate** — a variable whose values span several
predicates:
- blocks: block support `{on(b,·), ontable(b), holding(b)}`; hand `{handempty, holding(·)}`
- logistics: package location `{at(pkg,·), in(pkg,·)}`
- gripper: ball location `{at(ball,·), carry(ball)}`; gripper `{free(g), carry(·,g)}`

The balance check rejects these because an action adds into the variable via one
predicate while deleting via another (e.g. `unload` adds `(at pkg loc)`, deletes
`(in pkg truck)`), so a single-predicate candidate looks unbalanced. These
"where is X / what is held" variables are exactly the guidance variables ESPC
partitioning needs.

## Implication for Plan B (ESPC)

The cheap slice is **insufficient** to feed ESPC. To make Plan B viable the
synthesis must be upgraded to **multi-predicate monotonicity invariants** — i.e.
the real Helmert refinement: when an action unbalances a candidate, extend the
candidate with the facts that action deletes (so `{holding(?x)}` grows to
`{holding(?x), handempty}`, which `pickup` balances), then re-verify. That is the
documented "weeks" core the SAS+ investigation flagged.

The single-predicate pass stays as a correct, useful base (it already nails the
position variables); the multi-predicate refinement is the next increment if we
commit to Plan B.

## Update — multi-predicate refinement implemented

`synthesize` now does Helmert-style branch-and-verify: when an action's add is
unbalanced, it extends the candidate with a deleted-**and-required** fact (the
precondition guarantees the removed unit was the true one — this is what keeps it
sound) and re-verifies to a fixpoint. Coverage on the same instances:

| domain | single-pred | multi-pred | biggest group |
|---|---|---|---|
| blocks | 0%, 0 grp | **100%, 9 grp** | block support `{on,ontable,holding}` |
| logistics | 0%, 0 grp | **93%, 9 grp** | object location `{at,in}` |
| gripper | 7%, 1 grp | **71%, 7 grp** | gripper hand `{free,carry}` |
| rovers | 5% | 20% | `(at rover0 …)` |
| trucks | 5% | 12% | `(time-now …)` |
| elevator / satellite | 29% / 35% | 29% / 35% | (already single-pred) |

The multi-predicate variables (block support, package/object location, the
gripper hand) — exactly the guidance variables ESPC partitions on — are now
recovered. **Verdict: Plan B is fed.** Next: consume these groups in the
SGPlan/ESPC partitioning (`resolve`/`partition`).
