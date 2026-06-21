# Village builder — full-feature stress test

A survival/village-builder used to stress ferroplan's **feature surface** at once:
durative actions, numeric fluents (graph distances + material counts), and the full
**ADL** set. It deliberately mixes everything a rich game domain would:

| feature | where in `domain.pddl` |
|---|---|
| Durative actions | every action (`travel`, `chop-wood`, `build-house`, …) |
| Numeric fluents | `(dist ?a ?b)` durations, `(wood)/(stone)/(sticks)` stockpiles, `(chops-left ?l)` |
| Disjunctive precondition | `travel`: `(or (road …) (trail …))` |
| Conditional effect (`when`) | `chop-wood`: skilled chopper gets `+1 wood` |
| Universal effect (`forall`+`when`) | `make-fire`: warms every agent at camp |
| Quantified precondition (`forall`) | `build-square`: needs **all** house slots built |
| Negation | `build-house`: `(not (built ?s))` |

## What this validated ✅

Parsing, grounding, and solving all handle the **combination** of ADL + durative +
numeric. `onesite.pddl` (a single-site village — gather, craft, build the square,
light the fire) solves end to end:

```sh
ff -o examples/village/domain.pddl -f examples/village/onesite.pddl   # solves
```

## What it surfaced ❌ (the honest limits)

1. **No axioms / derived predicates.** `:derived` is rejected at parse time (this is
   a Metric-FF-class engine). Derived predicates (`reachable`, `village-complete`
   from its parts) would be a genuine engine addition — flagged for implementation.

2. **Temporal search is the bottleneck on a *graph map*.** `graph.pddl` (the same
   goal across a 3-node forest/quarry/camp map, so the agent must **travel**) is
   **not solved** within the node budget:

   ```sh
   ff -o examples/village/domain.pddl -f examples/village/graph.pddl   # exceeds the temporal search
   ```

   The cause is the temporal relaxed heuristic, not the features: delete-relaxation
   is blind to agent *location* and weak on *numeric accumulation*, so on a rich,
   many-action domain it gives little guidance for the realistic "travel → gather →
   return → build" loop, and the decision-epoch search exhausts. A stripped 3-action
   version (travel + gather + fire) on the same map solves fine — it's the
   *combination of richness + transport + accumulation* that needs a stronger
   temporal heuristic (and helpful-action pruning that depends on it).

This is the key result for using ferroplan as a game's planner: the **classical**
engine is strong, and durative + resources work, but **temporal search strength on
rich transport/crafting domains is the next engine investment** (the RPG blocker).
