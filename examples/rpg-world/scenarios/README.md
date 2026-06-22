# Scenarios — problem spaces that stress different mechanics

Beyond the per-subsystem `contracts/`, these instances each push a *different*
dimension of the engine on the same `rpg-world` domain. All solve quickly.

| scenario | stresses | makespan |
|---|---|---|
| `bootstrap-a-workshop` | self-built workstations + a build-then-use dependency: no sawmill exists, so the plan must `build-sawmill` (from pre-cut components) **before** it can `saw-planks` | 8 |
| `logistics-run` | durative **travel** + the derived **reachability axiom** + multi-site accumulation: the quarry is two hops from camp (via a junction), so the agent chops at the forest and mines at the quarry | 25 |
| `mana-cycle` | a **renewable resource consume/regenerate loop**: brewing costs mana, meditation restores it, so three potions force `meditate`→`brew` interleaving | 11 |
| `guild-order` | a **multi-part (conjunctive) goal** — ingots + blocks + meals in one order — the temporal search's known weak spot; solves because each part is short | 6 |

```sh
ff -o examples/rpg-world/domain.pddl -f examples/rpg-world/scenarios/mana-cycle.pddl
```

## A modelling note surfaced by `guild-order`

Roles (`smith`, `mason-skill`, `cook`, …) are **yield bonuses, not gates**, and the
domain has **no agent-exclusion** (a durative action doesn't reserve its agent), so
the planner happily fills a whole order with one craftsman rather than dividing the
work. If you want NPCs to be one-task-at-a-time (and thus *force* division of
labour), add a per-agent `(free ?a)` token: consume it `at start`, restore it
`at end` on every action. That single change makes the model agent-exclusive — and
turns `guild-order` into a real parallel-crew problem.
