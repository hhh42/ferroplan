# villagers — the generic, recipe-driven model

This is the planning model a **live game embeds** (ferroplan's `sim_core` villager
planner): build a PDDL problem from world state, `solve`, map steps back to game
verbs (walk / gather / craft), replan when done. It's the counterpart to
[`../rpg-world`](../rpg-world) — the same crafting/economy idea, encoded the
*opposite* way.

## Two models, side by side

| | **villagers** (this dir) | **rpg-world** |
|---|---|---|
| Encoding | **generic / data-driven**: 3 actions (`walk`/`gather`/`craft`); recipes, sources, workshops, walk-times are all data in `:init` | **specific**: ~100 hand-written actions, one per recipe |
| Time | **non-durative** — duration is a `(total-time)` cost the `:metric` minimizes (action-costs) | **`:durative-actions`** — real makespan, concurrency, ε-separation |
| ferroplan path | numeric **metric optimization** (`--mode pddl3`) | the **temporal** decision-epoch search (+ the `FF_TDEMAND`/`FF_TDECOMP` decomposer) |
| Recipes | single-input (`recipe ?out ?in`, one `recipe-qty`) | multi-input DAGs (smelt = ore + charcoal; house = planks + bricks) |
| Lines | ~50 | ~1000 |
| Best for | embedding in an app; many agents replanning fast (satisfice) | showcasing deep temporal crafting + goal decomposition |

Neither is "better" — they're different trade-offs. villagers is compact and is
what a real consumer writes (recipes as data, not code); rpg-world is more
expressive (multi-input, real time) and exercises the temporal/decomposition
features. They're the two ends of the encoding A/B in `benchmarks/encoding-ab`.

## Run it

```sh
# the township scenario: 2 crafting chains over a 7-node map, minimize total-time
ff -o examples/villagers/domain.pddl -f examples/villagers/township.pddl --mode pddl3
# -> a 19-step optimized plan, total-time 38

# the minimal blacksmith errand
ff -o examples/villagers/domain.pddl -f examples/villagers/errand.pddl --mode pddl3
```

`--mode pddl3` runs the metric optimizer (minimize `total-time`); the default
`auto`/`ff` route returns a faster *satisficing* plan — which is exactly what the
game uses for ambient NPCs (`Options { optimize: false, .. }`).

## Files

- `domain.pddl` — the generic 3-action domain (adapted from `sim_core`'s `village`
  planner; domain renamed `village`→`villagers` so it doesn't clash with the
  durative [`../village`](../village) survival-builder).
- `errand.pddl` — minimal: gather 2 ore, forge 1 nail.
- `township.pddl` — the interesting one: chains `wood→plank` and `ore→bar→tool`
  over a real map, with a laden-walk penalty, optimizing total travel+work time.
