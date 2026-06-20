# Temporal benchmark results (real IPC domains, validated with VAL)

ferroplan run on **19 real IPC temporal domains** (IPC-2002 → IPC-2014, durative
actions), with **every produced plan validated by [VAL]**, the standard IPC plan
validator, under PDDL2.1 continuous-time semantics (`-t 0.001` ε-tolerance).

Harness: [`bench_temporal.py`](bench_temporal.py). Budget: 20 instances/domain,
**10 s** each, single-threaded. VAL and the benchmark instances are not vendored
(licence/size) — see [`COMPARING.md`](COMPARING.md).

## Headline

- **Soundness: 44 / 45 produced plans are VAL-valid (98%).** ferroplan's temporal
  semantics — snap-action compilation, `over all` invariants, required
  concurrency, and **ε-separation** — are correct on real benchmarks. (Testing
  against VAL is what surfaced the ε-separation gap, now fixed.)
- **Coverage is search-limited.** At a 10 s budget, 45 of ~368 instances solve;
  the rest time out *or the decision-epoch search exhausts without a plan*. This
  is the clear next target — temporal search performance/quality, not parsing or
  validity.

## Per-domain

| domain | solved | VAL-valid | of total | note |
|---|---:|---:|---:|---|
| ipc2002 driverlog-time | 13 | 13 | /20 | rest time out |
| ipc2011 parking | 11 | 11 | /20 | rest time out |
| ipc2006 storage-time | 6 | 6 | /20 | rest time out |
| ipc2011 match-cellar | 6 | 6 | /20 | required concurrency ✓ |
| ipc2002 satellite-time | 4 | 4 | /20 | rest time out |
| ipc2006 openstacks-time | 2 | 1 | /20 | 1 invalid (ADL cond-effect mutex) |
| ipc2002 rovers-time | 1 | 1 | /20 | rest time out |
| ipc2002 zenotravel-time | 1 | 1 | /20 | rest time out |
| ipc2014 satellite | 1 | 1 | /20 | rest time out |
| ipc2002 depots-time | 0 | – | /20 | search exhausts / times out |
| ipc2006 trucks-time | 0 | – | /20 | search exhausts / times out |
| ipc2011 elevator | 0 | – | /20 | times out |
| ipc2011 floor-tile | 0 | – | /20 | times out |
| ipc2011 storage | 0 | – | /20 | times out |
| ipc2011 turn-and-open | 0 | – | /8 | times out |
| ipc2014 driver-log | 0 | – | /20 | times out |
| ipc2014 floor-tile | 0 | – | /20 | times out |
| ipc2014 match-cellar | 0 | – | /20 | search exhausts (~2.7 s) on solvable inst |
| ipc2006 rovers-metric-time | 0 | – | /20 | **parse gap** (see below) |

## Findings

1. **ε-separation works (the main result).** Before the STN re-timing pass, *all*
   non-trivial plans were VAL-rejected (a start coinciding with the at-end effect
   it depends on). After it, 44/45 validate. Required-concurrency (match-cellar)
   and back-to-back durative chains (driverlog) are VAL-valid.

2. **Coverage is bottlenecked by the temporal search, not budget.** A 60 s probe
   still fails: depots/inst-1 searches 54 s and gives up; match-cellar-2014/inst-1
   *exhausts in 2.7 s* on a solvable instance. The decision-epoch search +
   relaxed heuristic doesn't scale or guide well on many temporal instances
   (no temporal EHC; the relaxed-plan heuristic ignores time/concurrency). This is
   the next temporal work item.

3. **One parse gap:** `rovers-metric-time` uses `?duration` inside a numeric
   effect (`(increase (energy ?x) (* ?duration (recharge-rate ?x)))`) — a
   duration-dependent (continuous-style) effect ferroplan does not parse. 1 of 19
   domains.

4. **One invalid plan:** `openstacks-time/instance-2` — the ε-separation pass
   models mutexes over unconditional add/del/precondition only, so an ADL
   *conditional*-effect mutex (ship-order's conditional delete) slips through.
   Rare (1/45); fixing it means extending mutex detection to conditional/numeric
   effects.

[VAL]: https://github.com/KCL-Planning/VAL
