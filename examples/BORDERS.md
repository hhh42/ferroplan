# Coverage borders & the decomposition ruleset

The map a **subproblem-maker** needs: exactly how big/complex a single contract may
be before ferroplan's one-shot temporal search stops solving it — so the decomposer
knows what to hand over whole and what to break up. All numbers below are measured
(see `rpg-world/suite/`, `rpg-world/hard/`, `logistics/`, `jobshop/`).

## The unifying law

> ferroplan's delete-relaxed temporal heuristic keeps a clean gradient on **linear /
> accumulative** work, and goes **flat the instant ≥2 contributions must converge
> onto one goal quantity.**

Every failure is an instance of *converging-contributions ≥ 2*: the relaxation
counts the first contribution as already satisfying a `>=` goal and stops guiding
toward the rest. A contract is whole-able iff **(i)** it is one accumulating/
processing chain within the op budget, **or (ii)** every goal quantity in it
receives **at most one** converging contribution.

> **Update (temporal landmark term).** The temporal search now adds a
> numeric-threshold *landmark deficit* to its phase-1 key, which restores the
> gradient for **single-round** converging DAGs — a join whose inputs are each
> produced *once* from cold now solves (e.g. a from-scratch ingot, the metallurgy
> benchmark), and big linear accumulations got much faster too. The border that
> remains is **multi-round** convergence (a goal needing N of a product, so each
> intermediate is needed N times — e.g. `steel ≥ 2` from cold) and the other shapes
> below; those still want decomposition. So the rule for a subproblem-maker is
> unchanged as a *safe* default (stage inputs), but the engine is now more forgiving
> of a single converging step left in a contract.
>
> **Update 2 (`FF_TDEMAND` converging-resource demand term).** Opt-in, the temporal
> search now regresses the numeric goal down the recipe DAG to a total per-resource
> demand and guides on cumulative availability — the gradient the relaxation lacks
> for **multi-round** convergence. Measured: it lifts RPG coverage 26→34/39 (all
> validated), now solving multi-round converging DAGs (`steel≥2` from cold), cyclic
> regen (`grain≥10`), and multi-path numeric goals (`coin≥15`). So with `FF_TDEMAND`
> the **converging-contributions ceiling is no longer 1** for numeric goals — a
> contract may carry a full multi-round numeric chain. What still wants the
> decomposer: **predicate/structural conjunctions** (the monolithic "village shape"
> `built-wall`, multi-structure `found-village`, big mixed `order-8/12`), which the
> demand term — numeric-only — does not address.

## Border table (measured)

| shape | safe to hand whole | first fail | split unit |
|---|---|---|---|
| linear single-resource accumulate | ≤ **2000** primitive ops | 2001 (ore 3999→4000) | `ceil(ops/2000)` |
| deep **travel** (corridor) | ~**100** hops | ~200 (agent-location goes relaxed-"everywhere") | route in ≤100-hop legs |
| shallow (depth-1) **conjunction** | ≥ **10** independent parts | not the bottleneck | groups of ~10 |
| a single **depth≥2 chain**, alone | yes (sole goal) | **arity 2** — the moment any sibling is added | 1 chain per contract |
| **converging join** (2-input recipe) | ≤ **1** fresh sub-chain (others pre-staged); may accumulate N | ≥2 fresh sub-chains converging | 1 fresh input per contract |
| **farming harvest** (cyclic regen) | **1** harvest (≤3 grain) | the 2nd harvest — same whether cyclic *or* parallel fields | `ceil(N/3)` single-harvest |
| **multi-source numeric**, from scratch | **1** unit | 2 units (coin 2) | 1 unit / 1 chain per contract |
| multi-source numeric, **inputs in stock** | ≥ 30 units (makespan-linear) | none seen | stage inputs first |
| **logistics** leg (per-location goods) | **1** unit · 1 vehicle · any #hops | 2 units OR a 2nd vehicle/transshipment | per-unit, per-leg, per-package |
| **jobshop** (independent jobs) | ≤ ~**40k** operate groundings (100 jobs×20×20, 45s) | ~90k (100×30×30) | partition by jobs (never by machine/stage) |

Two universal anchors: **(1)** op-count ceiling ≈ 2000 for a clean linear chain;
**(2)** converging-contributions ceiling = **1**.

## HAND WHOLE — a contract may contain

- one linear single-resource accumulation, ≤2000 ops;
- a wide-but-shallow conjunction (≥10 independent depth-1 deliverables), inputs staged;
- a single depth≥2 chain as the **sole** goal;
- a converging recipe where **all but one** input is pre-staged inventory (accumulate that one N times if needed);
- one farming harvest (≤3 grain);
- a multi-source numeric goal whose inputs are **already in stock** (scales to 30+);
- a logistics single-unit / single-vehicle leg over any number of hops;
- a whole job-shop under ~40k route-table tuples.

## MUST SPLIT — boundary → rule

- **>2000 ops** → `ceil(ops/2000)` contracts, sum the partials.
- **any depth≥2 chain conjoined with ≥1 sibling** → pull each multi-step deliverable into its own single-deliverable contract; depth-1 siblings may stay grouped (≤10).
- **a join needing ≥2 fresh sub-chains** → stage all-but-one input as inventory; one fresh input per contract; sequence stagers, then a final join contract.
- **≥2 harvests (grain≥4)** → `ceil(N/3)` single-harvest contracts (extra fields do *not* help).
- **from-scratch multi-source numeric >1 unit** → 1 unit / 1 chain per contract, or pre-stage inputs.
- **logistics beyond 1 unit / 1 vehicle** → split per-unit, per-leg at each transshipment, per-package.
- **jobshop over the tuple budget** → partition by **jobs** (jobs are independent; never slice by machine or stage).

## How the rules generalize across domains

- **rpg-world** (crafting/economy): the source of the numbers above.
- **logistics** (per-location goods, trucks/trains, capacity): *same failure family,
  bites earlier* — the per-location stock model is relaxation-hostile, so almost
  every non-trivial delivery is a converging-flow problem. Qualitative rules carry
  over verbatim; the quantitative allowances collapse to **multiplicity 1** (1 unit,
  1 vehicle, 1 leg). Deep travel stays free, exactly as in rpg-world.
- **jobshop** (scheduling, machine-exclusion): the heuristic-shaped thresholds **do
  not apply** — jobs are independent linear chains that never converge, the engine's
  strong suit (it schedules **100 jobs** fine). The only limit is grounding-table
  size; slice by jobs.

So for the subproblem-maker: the **converging-contributions = 1** invariant is the
master rule across all three domains; the op-count and travel-depth ceilings are
secondary; and "is this domain like logistics (collapse to 1) or like jobshop
(slice by independent units)?" tells you which quantitative budget applies.
