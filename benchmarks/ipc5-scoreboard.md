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
| ferroplan² | 23 | 24 | 29 | 39 | 66 | 65 | 126 | 370 |
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
contract. Second pass (2026-07): **anytime in-sweep tightening** (each sweep
tightens its bound in place on every acceptance instead of restarting per
improvement — a restart now happens once per cap, not per improvement;
`FF_PREF_GREEDY=1` restores first-improvement sweeps) and a **diversified
restart ladder** — a capped no-improvement sweep says the current h-ordering
can't reach a better plan (measured: same-direction retries re-tread the same
prefix and change nothing), so the loop rotates the open-list weights through
a fixed half-cap profile ladder (h-greedy → h-heavy → g-heavy → pure-h) under
the same bound before the final all-remaining escalation
(`FF_PREF_NO_RESTARTS=1` disables the ladder). Fully deterministic. Most
instances finish in ≤ 65 s wall at 4 cores; the trucks tail is the slowest
(p07 ~104 s, p08 ~154 s) because the escalated retries actually spend the
budget. `FF_PREF_COMPILED=1` / `FF_PREF_NO_STATIC=1` / `FF_PREF_BARRIER=1` /
`FF_PREF_NO_ESCALATE=1` restore the pre-2026-07 pieces. rovers routes to the
legacy compiled-goal B&B by design (folded numeric metric) — it shares the
budget/escalation/anytime/ladder machinery but not the closure search.

**tpp** — the exact-closure optimizer² **ties SGPlan5 on p01–p04** (the whole
field ties SGPlan there); the restart ladder cut the tail (97/116/131 →
93/104/117) but SGPlan5 keeps it:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan² | **16** | **24** | **29** | **35** | 93 | 104 | 117 | 147 |
| SGPlan5 | 16 | 24 | 29 | 35 | 79 | 101 | 100 | 105 |

**storage** — full coverage (was 2/8: the quadratic forall-preference compiled
to 1601–62191 instances and walled the search). Static simplification drops the
statically-satisfied ~90–97%, the exact-closure optimizer² searches real
states only, and the restart ladder² broke the large-instance plateau
(46/145/200/263 → 31/121/124/148) — **ferroplan now beats SGPlan5 on p01–p07**
(7 of 8) **and on the domain total** (447 vs 547):

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan² | **3** | **5** | **6** | **9** | **31** | **121** | **124** | 148 |
| SGPlan5 | 5 | 8 | 14 | 17 | 87 | 124 | 160 | 132 |

**trucks** — the closure optimizer² lifted the whole row (p08: 133 → 10, p07:
67 → 12) and the ladder² finished p03 (1 → 0) and p06 (6 → 1); ferroplan
**wins p01 and p07**, ties p02–p05, and is **ahead on the domain total**
(23 vs 31):

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan² | **0** | **0** | **0** | **0** | **0** | 1 | **12** | 10 |
| SGPlan5 | 1 | 0 | 0 | 0 | 0 | 0 | 24 | 6 |

**rovers** (MetricSimplePreferences — numeric metric, optimized via numeric-term
folding on the legacy B&B; the ladder² bought p04, 559.9 → 485.5, a whisker
from SGPlan's 485.4, at the price of p02, 596.7 → 653.5 — net −17.6 on the
total) — ferroplan is competitive and **edges p07/p08**:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan | 935.3 | 653.5 | 1018.2 | 485.5 | 523.3 | 664.6 | **402.2** | **979.9** |
| SGPlan5 | 811.3 | 473.2 | 811.3 | 485.4 | 483.6 | 656.7 | 403.4 | 1007.6 |

**pathways** — **ties SGPlan5 on p01–p04** (was p01 only) and the ladder²
**wins p05 outright** (8.5 → 6 vs SGPlan's 6.5); SGPlan5 better after:

| inst | p01 | p02 | p03 | p04 | p05 | p06 | p07 | p08 |
|---|---|---|---|---|---|---|---|---|
| ferroplan² | **2** | **3** | **3** | **2** | **6** | 12.9 | 12.5 | 20.2 |
| SGPlan5 | 2 | 3 | 3 | 2 | 6.5 | 10 | 8 | 12.9 |

## Verdict

The picture flipped in 2026-07, and the second pass (anytime tightening + the
diversified restart ladder) moved it again. On p01–p08 quality with full
coverage everywhere:

- **ferroplan LEADS SGPlan5 on two domains, now decisively**: **openstacks**
  (with the opt-in `FF_ESPC` partitioned penalty loop: wins p04–p08, totals
  271 vs 326) and **storage** (default path: wins **p01–p07**, 7 of 8, and
  the domain total, 447 vs 547; SGPlan5 keeps only p08, 148 vs 132 — down
  from 263).
- **Parity band**: **trucks** (wins p01/p07, ties p02–p05; totals 23 vs 31 —
  ahead on total), **pathways** (ties p01–p04, **wins p05**, 6 vs 6.5),
  **tpp** (ties p01–p04 — the whole field does; the ladder cut the tail to
  93/104/117/147), and **rovers** (wins p07/p08, p04 within 0.1 of a tie) —
  SGPlan5 still ahead on each domain's largest instances.
- Instance tally across the 48: **17 wins / 12 ties / 19 losses** (was
  14/11/23 before the ladder). SGPlan5's real 6/0 sweep is now, on this
  p01–p08 slice, roughly a **4/2** carried almost entirely by the p05–p08
  tails of tpp/pathways and rovers' numeric metric.

Under the IPC-5 **coverage-first** rule the placement is a strong **2nd**:
full 48/48 coverage (storage was 2/8 before), two domain-level quality wins,
and parity on small instances nearly everywhere — clearly ahead of MIPS-XXL
(bogus openstacks metrics, low coverage elsewhere), MIPS-BDD (very low
coverage), and YochanPS (no openstacks, low coverage). What separates 2nd from
1st is now concentrated in the tpp/pathways p05–p08 tails and rovers'
numeric metric.

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
4. ~~**Large-instance tails**~~ — **largely closed** (2026-07, second pass):
   the anytime+ladder combination. Measured in two halves: **anytime in-sweep
   tightening alone changed nothing** (identical metrics, fewer iterations) —
   the plateau was never restart churn but a GUIDANCE limit: at the plateau
   bound the h-ordering exhausts its budget without reaching a better plan,
   in any amount of contiguous budget. The **diversified restart ladder** is
   what broke it (same bound, rotated open-list weights, half-size rungs so
   the final full-budget escalation stays strong — full-size rungs starved it
   and gave back tpp p04 / trucks p07): storage p05–p08 46/145/200/263 →
   31/121/124/148 (p06/p07 flipped to wins), pathways p05 8.5 → 6 (win), tpp
   p05–p07 −4/−12/−14, trucks p03 1→0 / p06 6→1, openstacks default p01
   42→23. Cost: tpp p08 +1, openstacks p03 +1, rovers p02 +56.8 (all
   already-losing instances). What remains (tpp/pathways p05–p08) plateaus
   under every profile in the ladder; the next lever is partitioned closure
   search (ESPC-style composition on the closure path).
5. **rovers numeric metric** — the residual gap is a subset-selection problem
   (which preferences are worth their forced traverse cost). TWO measured
   levers: (a) cost-aware open-list ordering (`SearchCfg::w_c`,
   `FF_PREF_COST_WEIGHT`) — DEAD END, collapses quality at every weight (cost
   only grows along a path, so cost-ordering buries goal-reaching prefixes);
   (b) **forgo-aware seeding** (`FF_PREF_SEED=1`, `heuristic::
   relaxed_plan_cost` prices each preference's completion from init and
   pre-forgoes those priced over their weight) — NEUTRAL: the estimates fire
   correctly (p01: est 157 vs weight 76.5) but the EHC seed already lands at
   the same incumbent cost; final metrics identical on/off across p01–p08.
   The restart ladder (item 4) is what actually moved rovers: p04 559.9 →
   485.5, within 0.1 of SGPlan5. Machinery for both stays, both default-off.
   The open question is completion pricing *inside* the search (a cost-aware
   relaxed-plan heuristic), not at the seed.

> Reproduce: `for p in p01..p08; do ff -o pref/<domain>/domain.pddl -f pref/<domain>/$p.pddl; done`
