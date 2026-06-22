# Industrial city — a functioning metal/stone/wood industry on the base domain

A whole working industrial town, built and supplied entirely with the **base
`rpg-world` domain** — raw extraction → smelting/processing → manufacturing →
construction. `city.py` runs it through the real planner.

## The point, in one run

The same city, two ways:

```
$ python3 city.py monolith        # the whole city as ONE plan, cold start
  -> NO PLAN  (60s)               # every border failure at once (converging DAGs,
                                  #  big conjunction, the village shape)

$ python3 city.py                 # the city as a pipeline of in-border contracts
  -> 26/26 contracts solved, the town stands
```

A monolithic industrial city is unplannable in one shot. Run it the way the game
will — a sequence of small contracts sharing one **city stockpile** — and the base
domain handles the entire industry. `city.py` builds one world (a full-kit
**workforce stationed at each site + the central hub with every workshop**), then
for each contract emits a problem from the current stockpile, runs ferroplan, and
**replays the returned plan** (through effects auto-extracted from the domain) to
carry the stockpile forward. The engine does the planning; the harness only carries
state. Every contract stays inside the [`BORDERS.md`](../../BORDERS.md) limits.

## The production run (26 contracts, 392 operations)

| stage | contracts | output |
|---|---|---|
| **Raw extraction** | forestry · iron · coal · quarry · clay · copper · tin | 140 logs, 60 ore, 30 coal, 70 stone, 30 clay, 16 copper-ore, 16 tin-ore |
| **Primary processing** | charcoal-kiln · sawmill · smeltery · masonry · brickworks · copper/tin smelt | 40 charcoal, 60 planks, 30 ingots, 40 blocks, 20 bricks, 12+12 alloy ingots |
| **Manufacturing** | steelworks · bronze-foundry · axe/pick forge · carpentry · fittings | 8 steel, 8 bronze, 6 axes, 6 picks, 8 timber-frames, 12 cut-fittings |
| **Construction** | 3 houses · town wall · town **square** · well | the town itself |

**City status:** 26/26 contracts solved · 392 operations · sequential makespan
~1260 (most stages run in parallel in reality — different crews, different sites) ·
structures `built-house ×3, built-wall, built-square, well-dug` · a stocked
warehouse of metal, stone, timber, tools, and alloys.

## Why each contract fits

Every contract is one border-safe shape: extraction is **linear accumulation**
(≤2000 ops); each processing/manufacturing step is a **converging recipe with its
inputs already staged** by the prior stage (so ≤1 fresh sub-chain — the rule from
`BORDERS.md`); construction consumes pre-built materials. The shared stockpile is
what lets a deep converging industry (ore+charcoal→ingots→steel→tools;
planks+bricks→houses→square) run as a chain of shallow, solvable pieces — exactly
the decomposition a subproblem-maker would produce.

```sh
python3 examples/rpg-world/industrial-city/city.py            # build the city
python3 examples/rpg-world/industrial-city/city.py monolith   # watch the one-shot fail
```
