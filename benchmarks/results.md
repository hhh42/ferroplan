# Benchmark results

`ferroplan` vs the C reference planners **Metric-FF** and **SGPlan6**, over a
subset of the IPC contest suites (classical STRIPS, numeric, ADL, and IPC-5
simple-preferences). Generated locally with a native arm64 Metric-FF build and
SGPlan6 under Docker — the oracles are **not bundled** (GPL / non-commercial
licences); see [COMPARING.md](https://github.com/hhh42/ferroplan/blob/master/benchmarks/COMPARING.md) to reproduce.

> Absolute times are machine- and load-dependent; only *ratios within a single
> run* are meaningful. Default ferroplan search is enforced hill-climbing (EHC)
> with a weighted-best-first fallback — the FF/Metric-FF default.

## Speed & coverage vs Metric-FF (native arm64)

| category | ferroplan solved | speed (geomean vs Metric-FF) |
|---|---:|---|
| STRIPS   | 40/40 | **0.71×** (~1.4× slower) |
| ADL      | 23/24 | **0.77×** (~1.3× slower) |
| numeric  | 36/40 | 0.22× (~4.5× slower) |

On **classical + ADL**, ferroplan — a from-scratch, memory-safe Rust planner — is
within ~1.4× of the heavily-optimized C reference. EHC is the reason (below). On
**numeric** it still trails; of the 4 unsolved numeric tasks, 3 time out under
Metric-FF too (genuinely hard), and 1 (`satellite/p06`) Metric-FF solves but
ferroplan does not — its EHC lookahead stalls on some numeric domains and falls
back to (slower, complete) best-first. Closing the numeric EHC gap is future work.

## Why EHC matters — states evaluated

EHC reaches the goal in *dozens* of evaluations where plain best-first needs
*thousands*, matching Metric-FF's order of magnitude:

| problem | ferroplan EHC | ferroplan best-first | Metric-FF |
|---|---:|---:|---:|
| strips/gripper/p08 | **223** | 17 446 | 158 |
| numeric/depots/p01 | **20** | 403 | 21 |
| strips/blocks/p02  | **16** | 69 | 22 |

(`--search best-first` selects the old behaviour; EHC is the default.)

## IPC-5 simple-preferences

ferroplan compiles preferences away (Keyder & Geffner) and runs anytime
branch-and-bound on the metric. Metric-FF is PDDL2.1-only and errors on every
preference problem, so the real comparison is **SGPlan6, the IPC-5 winner**
(measured head-to-head, 15 s budget, 48 problems):

**Coverage: on par with SGPlan6.** Adopting SGPlan's modified-FF idea — an
EHC-then-best-first subplanner for the first incumbent, then a budget-capped
branch-and-bound refinement — took ferroplan from **11/48 to 39/48** within 15 s,
matching SGPlan6's ~38/48. By domain (ferroplan, after): openstacks 8/8, tpp 8/8,
pathways 8/8, rovers 8/8, trucks 6/8, storage 1/8.

**Quality: SGPlan6 still leads on the hardest.** The capped refinement satisfices
rather than fully optimizes, so metric values trail on the large instances:

| | ferroplan | SGPlan6 | |
|---|---:|---:|---|
| trucks/p01 | **0** | 1 | win |
| trucks (others) / pathways/p01 | 0 / 2 | 0 / 2 | tie |
| pathways/p03, p04 | 5.7 / 6.7 | 3 / 2 | loss |
| rovers/p02, p04, p05 | 725 / 699 / 1052 | 473 / 419 / 499 | loss |
| openstacks/p01 | 70 | 13 | loss |

So coverage is now on par (we even solve tpp, which SGPlan6 errors on here), and
we tie/beat it on small instances — but it remains the stronger preference
planner on metric *quality* for the hard cases. Narrowing that quality gap is
where the rest of the SGPlan-class work (constraint partitioning + penalty
resolution) — or simply a longer/smarter metric optimizer — would help.

## Reproduce

Vendored micro-suite: `cargo bench` (criterion, ferroplan-internal). Cross-planner
comparison: [`compare.py`](https://github.com/hhh42/ferroplan/blob/master/benchmarks/compare.py) with the oracles, per
[COMPARING.md](https://github.com/hhh42/ferroplan/blob/master/benchmarks/COMPARING.md).
