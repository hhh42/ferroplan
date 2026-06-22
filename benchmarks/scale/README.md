# Scale & complexity stress test

Pushes ferroplan along the three axes that matter for using it as a live game's
planner, to find *where you have to reduce the problem*. Generators:

- `gen.py N M [E]` — an `rpg-world` problem with **N locations** (a connected map:
  chain + E extra edges each, with `dist` + the `reachable` axiom), **M agents**,
  every resource fluent initialized, a 1-action goal so **grounding dominates**.
- `gen_domain.py K` — a domain with **K procedural craft action schemas** (+ a
  trivial matching problem), to stress raw domain breadth.

```sh
python3 benchmarks/scale/gen.py 200 4 > /tmp/p.pddl
ff -o examples/rpg-world/domain.pddl -f /tmp/p.pddl
python3 benchmarks/scale/gen_domain.py 1000 > /tmp/d.pddl
```

## Results (M4, contended; grounding-dominated)

**Domain complexity — action-schema count.** Free. Linear and negligible.

| K schemas | 100 | 300 | 1000 | 3000 |
|---|---|---|---|---|
| parse+ground+solve | 0.03s | 0.03s | 0.04s | 0.06s |

**Agents — per-agent action grounding** (fixed 60-location map). Linear, gentle.

| M agents | 2 | 5 | 10 | 20 | 40 |
|---|---|---|---|---|---|
| wall | 0.25s | 0.36s | 0.56s | 1.10s | 2.40s |

**Static content — map size + the `reachable` transitive-closure axiom.** The wall.

| N locations | 20 | 50 | 100 | 150 | 200 | 300 | 500 |
|---|---|---|---|---|---|---|---|
| wall | 0.09s | 0.16s | 0.76s | 2.4s | 5.5s | **22s** | **97s** |

Growth is ~**O(N³·⁵)** — dominated by the reachability closure computed at grounding
(`crate::derived::compile`, a naïve datalog fixpoint that re-evaluates every
binding each round, with an `exists` that scans all nodes).

## Verdict — where you reduce

The engine handles **arbitrarily wide domains** (3000 actions ≈ free) and **many
agents** (linear) without trouble. The **only** scaling wall is the **reachability
axiom on large maps** (≈300+ POIs). Practical reductions, in order of preference:

1. **Don't derive reachability for huge maps — precompute it.** The game already
   owns the navigation graph; feed `(reachable a b)` (or just the edges the planner
   needs) as init facts instead of as a `:derived` rule. Zero closure cost.
2. **Plan per region.** A contract operates over a *local* sub-map (tens of POIs),
   not the whole world — the same decomposition that keeps crafting chains short.
3. **Engine optimization (identified, not yet done):** make the derived closure
   *semi-naïve* (derive only from the previous round's new facts) and/or
   join-aware (index `link` by source so the `exists` scans neighbours, not all
   nodes). That turns the closure from ~O(N⁴) toward **O(N·E)**, pushing the map
   limit from hundreds to many thousands of POIs.

Everything else — domain breadth, resource variety, agent count — is not a
bottleneck. Make the domain as massive as you like; reduce *map scope per plan*.
