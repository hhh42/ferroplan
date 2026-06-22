# `rpg-world` — the universal RPG planning domain

`domain.pddl` is the canonical low-level planning domain for a survival /
village-building multiplayer RPG. It is **broad, not deep**: ~28 durative actions
spanning gathering, processing, toolsmithing, alchemy/magic, construction, and
trade, over ~18 numeric resource stockpiles, with worker **roles**, a **reachable**
map axiom, and a forall-gated village square.

It exercises essentially the whole engine at once:

| feature | in the domain |
|---|---|
| Durative actions | every action has a `:duration` |
| Numeric fluents | `(dist …)` travel times + ~18 resource stockpiles |
| Conditional effects (`when`) | role bonuses (a `woodcutter` chops 2 logs, a `smith` smelts 2 ingots) |
| `forall` effects | `hold-feast` feeds every agent at the hearth |
| Quantified precondition | `build-square` requires **all** house slots built |
| Disjunctive precondition | `forage-food` works in a field **or** a forest |
| Negation | `equip-axe` needs `(not (has-axe ?a))` |
| Derived predicate (axiom) | `reachable` = transitive closure of the map's `link` |

## The decomposition model (how it's meant to be used)

A live game does **not** ask the planner for one monolithic "build the whole
village" plan. A higher-level AI/scheduler carves the world goal into
**contract-sized sub-tasks** — "make 8 planks", "forge 2 axes", "raise the
square" — and hands each to an available worker for a time window. ferroplan plans
each contract: an easy, **non-optimal, fast** plan.

This isn't just convenient — it's load-bearing. The full gather→process→build
chain in one shot exceeds the current temporal search ([#45](../../)): a smithing
contract that must *also* synthesize all its own gathering does not solve, but the
same contract with raw materials **pre-delivered** solves in 5 actions. So the
decomposition is the architecture: **gathering is one contract, processing
another, building another** — connected through the shared stockpile.

## Validated contracts (`contracts/`)

Each is a self-contained sub-task, verified to solve against `domain.pddl`:

| contract | what it shows | makespan |
|---|---|---|
| `woodline.pddl` | chop → saw planks + burn charcoal (role bonus) | 16.0 |
| `smithing.pddl` | (raws in) saw + smelt → forge 2 axes | 13.0 |
| `masonry.pddl` | quarry → blocks → build a wall **and** a well | 31.0 |
| `village-square.pddl` | build 2 houses then the square (forall precond) | 29.0 |
| `textiles.pddl` | shear → weave cloth → tailor clothing | 14.0 |
| `feast.pddl` | cook meals → feast (forall feeds everyone) | 6.0 |
| `alchemy.pddl` | mage meditates for mana → brews potions | 8.0 |
| `travel-gather.pddl` | travel a multi-hop map (reachable axiom) → mine | 8.0 |
| `team-build.pddl` | 2 workers split one contract concurrently | 16.0 |
| `trade.pddl` | turn surplus goods into coin at the market | 11.0 |

```sh
ff -o examples/rpg-world/domain.pddl -f examples/rpg-world/contracts/smithing.pddl
```

## Authoring your own contracts — tips that keep them fast

- Keep each contract **small** (a few goal units); let the scheduler chain them.
- **Cluster** the workstations a contract needs at one location to avoid travel.
- Give the worker its tools/role in `:init` (`has-axe`, `smith`, …).
- Initialize every numeric fluent you reference (`(= (logs) 0)`).
- You only need to declare the **objects/types you actually use** — empty types
  are tolerated (a smithing contract needn't declare building `slot`s).
