# Benchmark results

`ferroplan` vs the C reference planners **Metric-FF** and **SGPlan6**, over a
subset of the IPC contest suites (classical STRIPS, numeric, ADL, and IPC-5
simple-preferences). Generated locally with a native arm64 Metric-FF build and
SGPlan6 under Docker — the oracles are **not bundled** (GPL / non-commercial
licences); see [COMPARING.md](COMPARING.md) to reproduce.

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
branch-and-bound on the metric. Of the three planners, **only ferroplan handles
PDDL3 here**: Metric-FF is PDDL2.1-only and errors on every preference problem,
and SGPlan6 (the IPC-5 winner) is the relevant comparison but is not bundled.

Over the 48-problem set (20 s budget): ferroplan **solves ~16** with sensible
metrics — `pathways/p01` = **2** (matching SGPlan6's optimum), `trucks/p01–p05`
= **0** (all preferences satisfied) — and **times out on the hardest ~2/3**
(`openstacks`, `tpp`, `storage`). So: competitive and optimal on small/medium
preference tasks, but the anytime B&B does **not** beat SGPlan6's specialised
penalty / constraint-partitioning search on the largest — ferroplan is *not* the
IPC-5 winner.

## Reproduce

Vendored micro-suite: `cargo bench` (criterion, ferroplan-internal). Cross-planner
comparison: [`compare.py`](compare.py) with the oracles, per
[COMPARING.md](COMPARING.md).
