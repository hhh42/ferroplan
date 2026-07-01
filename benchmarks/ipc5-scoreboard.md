# IPC-5 (2006) simple-preferences scoreboard — ferroplan vs the field

Vendored suite: `benchmarks/ipc/pref/{openstacks,pathways,rovers,storage,tpp,trucks}`
— the IPC-5 *simple-preferences* (soft-goal) domains. The metric is each problem's
`(:metric minimize …)` over violated preferences (rovers adds a numeric
`sum-traverse-cost` term); **lower is better**.

Run one: `ff -o pref/<domain>/domain.pddl -f pref/<domain>/pNN.pddl` (the PDDL3
metric optimizer reports `metric value N, K preferences`).

## Reference & scoring (verified from the official archive)

The IPC-5 field for this subtrack was **SGPlan5** (the winner — 1st in all 6 SP
domains, 6/0), **MIPS-XXL**, **MIPS-BDD**, and **YochanPS**. Their per-instance
metrics were read directly from the `; MetricValue` headers in the official
`IPC5-results.tgz`; instance `pNN` is the same physical problem across all
planners (our p01–p08 = the competition's p01–p08).

IPC-5 ranked **per domain by place** (IPC-4 style): **coverage first, then plan
quality, then CPU** — *not* the IPC-2008 normalized ratio. SGPlan5 has full
coverage in every domain, so it is the natural quality benchmark below.
(MIPS-XXL's openstacks headers are a known `0.00` reporting artifact — coverage
only; MIPS-BDD is optimal-but-very-low-coverage.)

## ferroplan vs SGPlan5, p01–p08 (lower is better; **bold** = ferroplan ≤ SGPlan5)

**openstacks** — satisfaction guidance broke the all-forgo floor (70→63), and the
opt-in ESPC penalty loop (`FF_ESPC`, see `docs/espc-preferences-spec.md`) narrows
the gap further — but the shared `stacks-avail` resource scheduling is still
unsolved (the remaining ~3× gap to SGPlan):

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan | 63 | 66 | 62 | 66 | 138 | 129 | 278 | 608 |
| + `FF_ESPC`¹ | 42 | 43 | 55 | 66 | 81 | 90 | 151 | 227 |
| SGPlan5 | 13 | 16 | 12 | 26 | 36 | 33 | 67 | 123 |

¹ `FF_ESPC=1 FF_ESPC_TIME_MS=90000`, 4 cores (2026-07). The loop is
wall-clock-bounded and anytime, so quality scales with budget/cores: at the
default 15 s on the same box only p01/p02/p06 improve (42/43/100).

**tpp** (the whole field ties SGPlan at 16/24/29/35 on p01–p04 — ferroplan trails):

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan | 21 | 28 | 35 | 42 | 105 | 120 | 135 | 150 |
| SGPlan5 | 16 | 24 | 29 | 35 | 79 | 101 | 100 | 105 |

**storage** — ferroplan's metric B&B does not scale past ~hundreds of preferences
(the Keyder–Geffner compilation grows large), so p03+ time out:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan | 8 | 12 | — | — | — | — | — | — |
| SGPlan5 | 5 | 8 | 14 | 17 | 87 | 124 | 160 | 132 |

**trucks** — ferroplan **wins p01** and ties p05; SGPlan5 dominates the rest:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan | **0** | 1 | 6 | 1 | **0** | 10 | 67 | 133 |
| SGPlan5 | 1 | 0 | 0 | 0 | 0 | 0 | 24 | 6 |

**rovers** (MetricSimplePreferences — numeric metric, now optimized via numeric-term
folding) — ferroplan is competitive and **edges p07/p08**:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan | 935.3 | 659.3 | 1018.2 | 559.9 | 649.9 | 664.6 | **402.2** | **979.9** |
| SGPlan5 | 811.3 | 473.2 | 811.3 | 485.4 | 483.6 | 656.7 | 403.4 | 1007.6 |

**pathways** — ties p01; SGPlan5 better after:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan | **2** | 6 | 5.7 | 6.7 | 10.2 | 12.9 | 12.5 | 20.2 |
| SGPlan5 | 2 | 3 | 3 | 2 | 6.5 | 10 | 8 | 12.9 |

## Verdict

ferroplan does **not** beat SGPlan5 on any domain — SGPlan5 swept the subtrack
(6/0) for real, and on our shared p01–p08 it leads on quality everywhere
(ferroplan wins only scattered instances: trucks p01, rovers p07/p08, ties
pathways p01 / trucks p05).

But under the IPC-5 **coverage-first** rule, ferroplan would place **~2nd in the
field**: it solves the subset with full coverage and real, second-best quality,
ahead of the coverage- and quality-limited MIPS-XXL (bogus openstacks metrics,
low coverage elsewhere), MIPS-BDD (very low coverage), and YochanPS (no
openstacks, low coverage). A credible retroactive 2nd — not a win.

## Path to climb

1. **openstacks resource loop** — the headline quality gap (42 → ~13 with
   `FF_ESPC` on); needs `stacks-avail` resource coordination the satisfaction
   term can't see. The mutex-group synthesis (`invariants.rs`) now recovers
   `stacks-avail` as a guidance variable on every instance, so the next step is
   coupling the ESPC penalty loop to the partitioned search ("increment 2" —
   see the 2026-07 revisit in `docs/espc-preferences-spec.md`).
2. **tpp/storage quality** — ferroplan trails the field even on *small* instances
   (tpp p01 21 vs 16); a metric-B&B convergence / guidance fix.
3. **B&B scalability** — make the soft-goal compilation + B&B handle hundreds of
   preferences so storage p03+ (and large instances generally) are covered.

> Reproduce: `for p in p01..p08; do ff -o pref/<domain>/domain.pddl -f pref/<domain>/$p.pddl; done`
