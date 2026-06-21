# IPC-5 (2006) preference-track scoreboard

Vendored suite: `benchmarks/ipc/pref/{openstacks,pathways,rovers,storage,tpp,trucks}`
— the IPC-5 simple-preferences / soft-goal domains, 8 instances each. The metric
is each problem's `(:metric minimize …)` over violated preferences; **lower is
better**. This is the suite the SGPlan/ESPC effort is measured against.

Run one: `ff -o pref/<domain>/domain.pddl -f pref/<domain>/pNN.pddl`
(the PDDL3 metric optimizer reports `metric value N, K preferences`).

## openstacks-soft — the SGPlan/ESPC quality-gap target

The original baseline was the **all-forgo floor**: ferroplan delivered nothing
(metric 70 on p01) because under delete-relaxation the free Keyder-Geffner forgo
makes every preference look reachable, so the metric search had no gradient toward
delivering. **Satisfaction guidance** (a heap penalty counting preferences forgone
in the *concrete* state, `search::SatGuidance`, built from each `P3COLLECT-i` phi)
breaks that floor:

| instance | before (all-forgo) | + satisfaction guidance | sgplan6 (ref) |
|---|---|---|---|
| p01 | 70 | **63** | ~13 |
| p02 | 70 | **66** | — |
| p03 | 90 | **62** | — |
| p04 | 100 | **66** | — |
| p05 | 140 | **138** | — |

Monotone by construction (guidance changes node *ordering* only; B&B keeps the
best plan), so it never regresses. Guarded by `tests/ipc5_pref_metric.rs`.

The residual gap to ~13 is the *scheduling* of the shared `stacks-avail` resource,
which the satisfaction term cannot see (it appears in no preference). Closing it
needs the SAS+ mutex-group partition + a resource penalty loop — the next ESPC
increment.

## Other IPC-5 pref domains (p01, with satisfaction guidance)

| openstacks | tpp | storage | trucks | rovers | pathways |
|---|---|---|---|---|---|
| 63 | 21 | 8 | 0 | 0 | 2 |

(trucks/rovers reach metric 0 — all preferences satisfied.)

> Reproduce: `for p in p01..p08; do ff -o pref/openstacks/domain.pddl -f pref/openstacks/$p.pddl; done`
