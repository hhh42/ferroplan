# Benchmark results

`ferroplan` vs the reference planners, over a subset of the IPC contest suites:
classical STRIPS / numeric / ADL are measured against a native arm64 **Metric-FF**;
the IPC-5 simple-preferences quality is measured against **SGPlan5** (the IPC-5
winner), read from the official `IPC5-results.tgz` archive. The oracles are **not
bundled** (GPL / non-commercial licences); see [COMPARING.md](https://github.com/hhh42/ferroplan/blob/main/benchmarks/COMPARING.md) to reproduce.

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

## IPC-5 simple-preferences (vs SGPlan5)

ferroplan compiles preferences away (Keyder & Geffner) and, as of 0.4.0, optimizes
them with an **exact-closure metric optimizer** (the default) plus a
budget-escalating branch-and-bound, and — opt-in via `FF_ESPC` — an ESPC-style
partitioned penalty loop. Metric-FF is PDDL2.1-only and errors on every preference
problem, so the benchmark is **SGPlan5, the IPC-5 winner** (per-instance metrics
from the official archive; lower is better).

The 0.4.0 headline: **full 48/48 coverage** (storage was 2/8) and ferroplan now
**leads SGPlan5 on two of the six domains** —

- **openstacks** (with `FF_ESPC`): wins p04–p08, totals 271 vs 326.
- **storage** (default path): wins p01–p05.

…with a parity band on the rest — **trucks** (ahead on the total, wins p01/p07),
**pathways** and **tpp** (tie on p01–p04), **rovers** (edges p07/p08) — where
SGPlan5 keeps each domain's larger instances. Under the IPC-5 coverage-first rule
this is a strong **2nd**, with the remaining gap concentrated in the
tpp/pathways/storage p05–p08 tails and rovers' numeric metric.

**The full per-instance tables, the ESPC method, and the reproduction commands
live in the scoreboard:**
[`benchmarks/ipc5-scoreboard.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md).

## Reproduce

Vendored micro-suite: `cargo bench` (criterion, ferroplan-internal). Cross-planner
comparison: [`compare.py`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/compare.py) with the oracles, per
[COMPARING.md](https://github.com/hhh42/ferroplan/blob/main/benchmarks/COMPARING.md).
