# ferroplan examples

Worked PDDL domains and problems, each a self-contained lesson in one part of the
engine. Every directory has its own README with the command to run it and the
expected result. Run any with the `ff` CLI:

```sh
ff -o examples/<dir>/domain.pddl -f examples/<dir>/problem.pddl
```

(Some temporal examples need feature flags — each README states them; see the
[tuning reference](../book/src/tuning.md).)

## By feature — what to read to learn X

| you want to learn… | start here |
|---|---|
| STRIPS + typing, plans & goals | [`logistics`](logistics) — trucks, capacity, a train line |
| numeric fluents (a quantity gates actions) | [`cabin`](cabin) — a deep numeric build |
| **derived axioms** (`:derived`) | [`reachability`](reachability) — static transitive-closure reachability |
| full ADL (`when`, `forall`, `or`, negation) | [`village`](village) — the ADL stress test |
| **PDDL3 preferences / metric** optimization | [`villagers`](villagers) — a data-driven recipe planner scored by a metric |
| durative / temporal + resources | [`rpg`](rpg) — gather → craft → build with workers & materials |
| scheduling with machine exclusion | [`jobshop`](jobshop) — scales to 100 concurrent jobs |
| everything at once + **decomposition** | [`rpg-world`](rpg-world) — a ~120-action economy, solved as contracts |

## Suggested reading order

1. **[`rpg`](rpg)** — the gentlest complete example: durative actions with renewable
   (workers) and consumable (materials) resources. The mental model for the rest.
2. **[`logistics`](logistics)** / **[`cabin`](cabin)** — classical STRIPS/typing and
   numeric fluents, the two classical foundations. `cabin` also has a durative "crew"
   twin showing makespan vs. crew size.
3. **[`reachability`](reachability)** — how a fact can be a *consequence* of static
   state via a `:derived` axiom (closed into the initial state).
4. **[`village`](village)** — the full ADL surface (`when`, `forall`+`when`, `or`,
   negation) over durative + numeric state, with honest notes on where the temporal
   heuristic runs out.
5. **[`villagers`](villagers)** — PDDL3 metric optimization (`--mode pddl3`); the
   "embed the planner in a game" model.
6. **[`jobshop`](jobshop)** — durative scheduling with a machine-exclusion token.
7. **[`rpg-world`](rpg-world)** — the flagship: everything above at once, plus the
   decomposition workflow (`ff --decompose`) that splits a too-big goal into ordered
   contracts. See its [`suite/`](rpg-world/suite), [`hard/`](rpg-world/hard), and
   [`industrial-city/`](rpg-world/industrial-city).

## See also

- **[`BORDERS.md`](BORDERS.md)** — the measured map of where one-shot planning
  solves vs. where a goal must be decomposed into contracts.
- **Rust library examples** — [`crates/ferroplan/examples/`](../crates/ferroplan/examples):
  `solve`, `parse`, `json_api`, `replan` (the `Session` API), `decompose`, and
  `validate_plan`.
- **Minimal per-feature snippets** — the [ferroplan skill](../.claude/skills/ferroplan)
  ships one tiny domain+problem per feature under its own `examples/`.
