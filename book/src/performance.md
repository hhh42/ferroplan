# Performance

ferroplan is [data-oriented by design](./architecture.md), but a fast layout
only pays off if the hot paths don't do redundant work. This session landed
three optimizations that turn instances which were previously *un-finishable* or
*un-scoreable* into routine ones — each measured, each correctness-preserving.

## Grounding — static-precondition parameter-domain restriction

Untyped domains used to enumerate the full cartesian product of every parameter
and string-match almost all of it away. gripper's `pick(?obj ?room ?gripper)`
was generating 154³ ≈ 3.6M bindings *per action* and discarding 99.98% of them.

The fix restricts each parameter's domain by its **static unary preconditions**
*before* enumerating — so `?gripper` only ever ranges over the grippers, not
every object — collapsing the blowup at the source. The produced ground ops are
bit-identical; only the work to find them shrinks.

| instance | before | after |
|---|---|---|
| gripper p02 | 658 µs | 247 µs (2.65×) |
| 150-ball untyped, 1-step | 1.56 s | ~0 |
| gripper-250 (partition mode) | 11.9 s | 3.96 s (3×) |

(`crates/ferroplan/src/ground.rs`)

## EHC work cap — scaled by operator count

Enforced hill-climbing carried a fixed work cap. Large-but-easy instances would
exhaust it and bail into the *unpruned* best-first arm, doing millions of
evaluations on a problem EHC's near-greedy descent would have walked straight
through. Scaling the cap by operator count lets those instances finish in the
cheap arm; the heuristic is untouched and the plan stays valid.

| instance | before | after |
|---|---|---|
| gripper-250 `--mode ff` | 2.16M evals / 33 s | 32k evals / 0.86 s (38×) |

Small and genuinely-hard instances are unchanged — they never hit the old cap,
or they legitimately need the fallback (deep plateaus are still on the backlog;
see [perf-notes](#how-the-wins-are-measured)).
(`crates/ferroplan/src/search.rs`)

## Metric optimizer — monotone numeric-term folding

The [metric optimizer](./metric-quality.md) drives an anytime branch-and-bound
over the `:metric`. Previously it could only see the preference-violation terms,
so on domains whose metric also charges a **monotone numeric quantity** — like
rovers' `(sum-traverse-cost)` — it scored a bogus `0`: the numeric part was
invisible and the search had nothing to optimize.

It now folds monotone numeric metric terms into total-cost, so it optimizes the
**full** metric. rovers went from un-scoreable to a real metric of **935.3** —
which is what unlocked the sixth IPC-5 domain (see
[Metric quality](./metric-quality.md)).
(`crates/ferroplan/src/pddl3.rs`)

## How the wins are measured

Two harnesses, with a deliberate division of labor:

- **`cargo bench -p ferroplan --bench planning`** (criterion) is the reference
  for wall-time deltas. Its `solve/` group covers small typed/numeric instances
  (gripper, blocks, rovers) and `solve_large/` covers the scale-sensitive
  grounding- and search-dominated cases. Criterion is the *only* noise-robust
  timer on a loaded machine.
- **`benchmarks/perf.py`** reports deterministic evaluated-state counts. A
  constant-factor win leaves these bit-identical (proof the work, not the
  strategy, shrank); a search-strategy win changes them and must be re-baselined.

> Raw wall-clock here is noise-dominated below ~15% — the same binary has ranged
> 11.5–14 s under background load — so treat any single timed run with suspicion
> and let criterion arbitrate.

The ranked backlog of remaining optimizations (generation-counter `Scratch`
reset, preferred-operator best-first, `apply_into` clone-on-survival) lives in
[`docs/perf-notes.md`](https://github.com/hhh42/ferroplan/blob/main/docs/perf-notes.md),
along with the methodology caveats learned the hard way (notably: `atos`
mis-attributes inlined hot code on optimized builds — trust the de-noised
profile, not the raw top symbols).
