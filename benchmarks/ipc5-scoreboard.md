# IPC-5 (2006) preference-track scoreboard

Vendored suite: `benchmarks/ipc/pref/{openstacks,pathways,rovers,storage,tpp,trucks}`
— the IPC-5 simple-preferences / soft-goal domains, 8 instances each. The metric
is each problem's `(:metric minimize …)` over violated preferences; **lower is
better**. This is the suite the SGPlan/ESPC effort is measured against.

Run one: `ff -o pref/<domain>/domain.pddl -f pref/<domain>/pNN.pddl`
(the PDDL3 metric optimizer reports `metric value N, K preferences`).

## openstacks-soft — the SGPlan/ESPC quality-gap target

ferroplan baseline (current: a single plan, ~1 branch-and-bound iteration — the
metric search can't reorder to reduce open stacks):

| instance | ferroplan metric | sgplan6 (ref) |
|---|---|---|
| p01 | 70 | ~13 |
| p02 | 70 | — |
| p03 | 90 | — |
| p04 | 100 | — |
| p05 | 140 | — |

The gap is large because minimizing open stacks is a *scheduling/ordering* problem
over the shared `stacks-avail` resource — exactly the coupling the ESPC penalty
loop (#63) is meant to coordinate across partitioned subgoals. The mutex-group /
partitioning groundwork (committed) is the prerequisite; this table is the target
to move.

> Reproduce: `for p in p01..p08; do ff -o pref/openstacks/domain.pddl -f pref/openstacks/$p.pddl; done`
