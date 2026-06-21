# RPG crafting example — durative actions + resource allocation

A worked example of using ferroplan as the **low-level operational planner** for a
game agent: gather → craft → build, scheduled around a limited crew.

It demonstrates the primitives a live game needs:

| primitive | how it's modeled |
|---|---|
| **Durative actions** | `:durative-action` with `:duration` — chopping/sawing/mining/building take in-world time |
| **Renewable resource** (workers) | a numeric `(workers)` fluent, `decrease` at-start + `increase` at-end, guarded by `(at start (>= (workers) 1))` — held over the action's interval |
| **Consumable resources** (materials) | `(wood)`, `(planks)`, `(stone)` — produced and consumed by the chain |
| **Crafting chain** | chop → saw → build; mine → build |

## Run it

```sh
ff -o examples/rpg/domain.pddl -f examples/rpg/build-1worker.pddl    # serialized, makespan ~19
ff -o examples/rpg/domain.pddl -f examples/rpg/build-3workers.pddl   # parallel,  makespan ~13
```

The only difference between the two problems is `(= (workers) N)`. ferroplan holds
a worker for each in-progress action and releases it at the action's end, so the
crew size sets how much of the chain can run concurrently — exactly the
resource-allocation behavior a workforce/contract system needs.

## Notes / current limits

- Plans are **satisficing**, not makespan-optimal (a good plan fast, not a proven
  shortest one) — the right trade-off for a live game where an agent can plan,
  act, and replan as the world changes.
- The same pattern models tools (`(axes)`), mana/cooldowns (a fluent that
  regenerates over time), and machines — any renewable or consumable resource.
