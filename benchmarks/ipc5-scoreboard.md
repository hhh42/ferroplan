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

**openstacks** — satisfaction guidance broke the all-forgo floor (70→63), the
exact-closure optimizer² pushed the default further (63→42 on p01), and the
opt-in ESPC penalty loop (`FF_ESPC`, see `docs/espc-preferences-spec.md`)
couples its λ schedule to a **partitioned search** ("increment 2": one
subproblem per order-interaction component, the shared `stacks-avail` variable
priced as a global constraint instead of being solved inside any one stage) —
**ferroplan beats SGPlan5 on p04–p08**, the first domain where it leads the
IPC-5 winner on the larger half of the suite:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan² | 42 | 34 | 28 | 39 | 67 | 85 | 153 | 370 |
| + `FF_ESPC`¹ | 19 | 23 | 17 | **16** | **21** | **22** | **66** | **87** |
| SGPlan5 | 13 | 16 | 12 | 26 | 36 | 33 | 67 | 123 |

¹ `FF_ESPC=1 FF_ESPC_TIME_MS=90000`, 4 cores (2026-07, partitioned coupling).
Deterministic across runs (3/3 identical per instance) and terminates by
stall/saddle well inside the budget — worst case p04 at ~58 s wall, p01 in ~4 s.
`FF_ESPC_MONO=1` reproduces the earlier monolithic loop
(42/43/55/66/81/90/151/227 at the same budget).

² Default path since 2026-07: the **exact-closure metric optimizer** (static
preference simplification at compile + real-state search with metric-bounded
acceptance + the exact `P3END`/collect/forgo phase tail + barrier-free DNF
guidance), with a **budget-escalating retry**: a tightening probe that hits its
300k per-iteration eval cap without improvement retries the same bound with all
remaining budget instead of ending the optimization — `FF_PREF_EVAL_BUDGET`
(default 2M evals, deterministic, thread-count independent) is the real
contract. Most instances finish in ≤ 65 s wall at 4 cores; the trucks tail is
the slowest (p07 ~108 s, p08 ~163 s) because the escalated retries actually
spend the budget. `FF_PREF_COMPILED=1` / `FF_PREF_NO_STATIC=1` /
`FF_PREF_BARRIER=1` / `FF_PREF_NO_ESCALATE=1` restore the pre-2026-07 pieces.
rovers routes to the legacy compiled-goal B&B by design (folded numeric
metric) — it shares the budget/escalation but not the closure search.

**tpp** — the exact-closure optimizer² **ties SGPlan5 on p01–p04** (the whole
field ties SGPlan there); SGPlan5 keeps the tail:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan² | **16** | **24** | **29** | **35** | 97 | 116 | 131 | 146 |
| SGPlan5 | 16 | 24 | 29 | 35 | 79 | 101 | 100 | 105 |

**storage** — full coverage (was 2/8: the quadratic forall-preference compiled
to 1601–62191 instances and walled the search). Static simplification drops the
statically-satisfied ~90–97%, and the exact-closure optimizer² searches real
states only — **ferroplan now beats SGPlan5 on p01–p05**:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan² | **3** | **5** | **6** | **9** | **46** | 145 | 200 | 263 |
| SGPlan5 | 5 | 8 | 14 | 17 | 87 | 124 | 160 | 132 |

**trucks** — the closure optimizer² lifted the whole row (p08: 133 → 10, p07:
67 → 12); ferroplan **wins p01 and p07**, ties p02/p04/p05:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan² | **0** | 0 | 1 | 0 | 0 | 6 | **12** | 10 |
| SGPlan5 | 1 | 0 | 0 | 0 | 0 | 0 | 24 | 6 |

**rovers** (MetricSimplePreferences — numeric metric, optimized via numeric-term
folding on the legacy B&B; the escalating retry bought p02/p05) — ferroplan is
competitive and **edges p07/p08**:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan | 935.3 | 596.7 | 1018.2 | 559.9 | 523.3 | 664.6 | **402.2** | **979.9** |
| SGPlan5 | 811.3 | 473.2 | 811.3 | 485.4 | 483.6 | 656.7 | 403.4 | 1007.6 |

**pathways** — **ties SGPlan5 on p01–p04** (was p01 only); SGPlan5 better after:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan² | **2** | **3** | **3** | **2** | 8.5 | 12.9 | 12.5 | 20.2 |
| SGPlan5 | 2 | 3 | 3 | 2 | 6.5 | 10 | 8 | 12.9 |

## Verdict

The picture flipped in 2026-07. On p01–p08 quality with full coverage
everywhere:

- **ferroplan LEADS SGPlan5 on two domains**: **openstacks** (with the opt-in
  `FF_ESPC` partitioned penalty loop: wins p04–p08, totals 271 vs 326) and
  **storage** (default path: wins p01–p05, 5 of 8 instances; SGPlan5 keeps the
  three largest).
- **Parity band**: **trucks** (wins p01/p07, ties p02/p04/p05; totals 29 vs
  31 — now AHEAD on total too), **pathways** (ties p01–p04), **tpp** (ties
  p01–p04 — the whole field does), and **rovers** (wins p07/p08) — SGPlan5
  still ahead on each domain's larger instances.
- SGPlan5's real 6/0 sweep is now, on this p01–p08 slice, roughly a **4/2**
  with three of its four domain leads carried by the p05–p08 tail.

Under the IPC-5 **coverage-first** rule the placement is a strong **2nd**:
full 48/48 coverage (storage was 2/8 before), two domain-level quality wins,
and parity on small instances nearly everywhere — clearly ahead of MIPS-XXL
(bogus openstacks metrics, low coverage elsewhere), MIPS-BDD (very low
coverage), and YochanPS (no openstacks, low coverage). What separates 2nd from
1st is now concentrated in the large-instance tails (tpp/pathways/storage
p05–p08) and rovers' numeric metric.

## Path to climb

1. ~~**openstacks resource loop**~~ — **done** ("increment 2", 2026-07): the
   ESPC λ schedule now drives a partitioned composition (one stage per order
   component, `stacks-avail` excluded from edges and priced as a global
   constraint, per-stage goals enriched with their own deliverables), taking
   p01–p08 from 42/43/55/66/81/90/151/227 to 19/23/17/16/21/22/66/87 at the
   same budget — ahead of SGPlan5 on p04–p08. Remaining gap: p01–p03 (small
   instances, 19/23/17 vs 13/16/12) where the per-order grain is too coarse to
   matter and the polish B&B is the binding mechanism.
2. ~~**tpp/storage quality**~~ — **done** (2026-07): the exact-closure metric
   optimizer (real-state search + metric-bounded acceptance + exact phase
   tail) with barrier-free DNF guidance ties SGPlan5 on tpp p01–p03 /
   pathways p01–p04 and beats it on storage p01–p05 and trucks p01/p07.
3. ~~**B&B scalability**~~ — **done** (2026-07): static preference
   simplification (statically-satisfied instances dropped at compile) + the
   closure optimizer's instant init-tail incumbent give storage full 8/8
   coverage (62k raw instances on p08) with every instance ≤ 60 s.
4. **Large-instance tails** — partially closed (2026-07): the budget-escalating
   retry converted the "abandon on first capped probe" plateau into real budget
   consumption — tpp p04 35 (tie), p05 101→97, trucks p07 19→12, storage
   p05/p06/p08 46/145/263, openstacks default 49→42, rovers p02/p05
   596.7/523.3. What remains (tpp/pathways p05–p08, storage p06–p08) plateaus
   at the 2M default budget with greedy first-improvement tightening; next
   levers are a better-than-first-improvement iteration (e.g. exhaust-then-pick
   or restarts) and partitioned closure search.
5. **rovers numeric metric** — the residual p01–p06 gap is a subset-selection
   problem (which preferences are worth their forced traverse cost). MEASURED
   DEAD END (2026-07): cost-aware open-list ordering (`SearchCfg::w_c`,
   `FF_PREF_COST_WEIGHT`) collapses quality at every weight tried — cost only
   grows along a path, so cost-ordering buries goal-reaching prefixes; the
   machinery stays for experiments, default 0. A working lever must price the
   *completion* (cost-aware heuristic or forgo-aware seeding), not the prefix.

> Reproduce: `for p in p01..p08; do ff -o pref/<domain>/domain.pddl -f pref/<domain>/$p.pddl; done`
