# `rpg-world` — the universal RPG planning domain

`domain.pddl` is the canonical low-level planning domain for a survival /
village-building multiplayer RPG. It is **broad, not deep**: **~120 durative
actions** over **~80 numeric resource stockpiles** and ~100 predicates, spanning a
whole economy —

> gathering · woodline · **metallurgy tiers** (copper/tin→bronze, iron→steel,
> precious metals) · **farming** (till/plant/irrigate/harvest→flour) · **animal
> husbandry** (graze/milk/butcher) · **hunting & fishing** · **leatherworking** ·
> **glass & pottery** · **carpentry** · masonry · **weapons & armor** · **combat /
> defense** (clear threats, train guards) · alchemy & **enchanting** · cooking
> recipes · **transport** · construction (build your own workstations, towers,
> temple) · **civic / skill-training** · trade —

with worker **roles**, a **reachable** map axiom, and a forall-gated village square.
You never plan the whole thing at once: a scheduler hands out **contract-sized
sub-tasks** (see below).

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

**23 self-contained sub-tasks, all verified to solve** against `domain.pddl` —
spanning every subsystem (gathering, smithing, masonry, textiles, cooking,
alchemy, farming, husbandry, glass/pottery, carpentry, weapons, leatherworking,
hunting, transport, defense, civic, trade, and full village builds). Below is a
representative slice; see the directory for all of them. (Several multi-step
crafting chains — `weapons`, `leatherworking`, `hunting`, `transport` — only became
solvable after the [#45](../../) temporal-search improvement: weighted-`g` +
two-phase helpful-action pruning, which raised the per-contract plan ceiling.)

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
| `farming.pddl` | till → plant → irrigate → harvest → mill flour | 15.0 |
| `animal-husbandry.pddl` | tend livestock → produce | 3.0 |
| `glass-pottery.pddl` | sand → glass / clay → fired pottery | 15.0 |
| `carpentry-furniture.pddl` | planks → furniture / cart-parts | 11.0 |
| `cooking-recipes.pddl` | ingredients → a cooked recipe | 9.0 |
| `enchanting-magic.pddl` | meditate → enchant a tool with a potion | 12.0 |
| `defense-combat.pddl` | arm a guard → clear a threat | 10.0 |
| `construction-tiers.pddl` | bootstrap a workstation from materials | 35.0 |
| `civic-skills.pddl` | train an apprentice into a role | 23.0 |

```sh
ff -o examples/rpg-world/domain.pddl -f examples/rpg-world/contracts/smithing.pddl
```

**Long chains need decomposing.** A few subsystems (metallurgy, weapons, leather,
hunting, transport) have crafting chains long enough that a *single* contract
trying to do the whole chain from raw inputs exceeds the temporal search — exactly
the signal to split it: one contract gathers/refines the intermediate, another
consumes it. The actions are all in the domain and work; keep each contract short.

## Authoring your own contracts — tips that keep them fast

- Keep each contract **small** (a few goal units); let the scheduler chain them.
- **Cluster** the workstations a contract needs at one location to avoid travel.
- Give the worker its tools/role in `:init` (`has-axe`, `smith`, …).
- Initialize every numeric fluent you reference (`(= (logs) 0)`).
- You only need to declare the **objects/types you actually use** — empty types
  are tolerated (a smithing contract needn't declare building `slot`s).
