# Performance notes

How ferroplan's hot paths were profiled, what was fixed, and the ranked backlog of
remaining optimizations. Measure with `cargo bench -p ferroplan --bench planning`
(criterion — the only noise-robust harness on a loaded machine) and the
deterministic `benchmarks/perf.py` (evaluated-states; a *constant-factor* win
leaves these bit-identical, a *search-strategy* win changes them and must be
re-baselined).

## Methodology caveats (learned the hard way)

- **Wall-clock here is noise-dominated** for sub-~15% deltas (the same binary
  ranged 11.5–14s under background load). Use criterion, or `min` of many runs.
- **`atos` symbolication on optimized builds mis-attributes** inlined hot code to
  adjacent symbols. A profile showing "22% `core::fmt::Display`" / "12% clap" in
  the *search* was an artifact — there is no `format!`/`Display` in `search.rs` or
  `heuristic.rs`. Trust the de-noised picture (heuristic + successor-gen), not the
  raw top symbols.

## Shipped wins

| fix | instance | before | after | guarantee |
|---|---|---|---|---|
| **Grounding: static-precondition param-domain restriction** (`ground.rs`) | gripper p02 | 658 µs | 247 µs (2.65×) | identical ground ops |
| | 150-ball untyped, 1-step | 1.56 s | ~0 | |
| | gripper-250 (partition) | 11.9 s | 3.96 s (3×) | |
| **EHC: op-count-scaled work cap** (`search.rs`) | gripper-250 `--mode ff` | 2.16M evals / 33 s | 32k evals / 0.86 s (38×) | plan-valid; h untouched |

Root causes: (1) untyped domains enumerated the full cartesian product of every
parameter (gripper `pick`: 154³ ≈ 3.6M bindings/action) and string-matched 99.98%
away — fixed by restricting each param's domain by its static unary preconditions
first. (2) EHC's fixed `TOTAL_CAP=30_000` made large-but-easy instances bail into
the *unpruned* best-first (2.16M evals); the cap now scales as `(200·n_ops).max(30k)`
so EHC's near-greedy arm finishes them. Both leave small/typed instances unchanged.

## Ranked backlog (from the ultracode analysis workflow)

Each is correctness-preserving; the "preserves" column says how to verify.

1. **Generation-counter `Scratch::reset`** (h-identity) — replace the per-eval
   `op_layer`/`selected`/`need_fact` `.fill()`s (`2·n_ops + n_facts` writes) with a
   `gen` bump + per-access stamp check. ~4% on heuristic-bound instances; ~10
   fragile gate sites (notably `select`'s `op_layer == 0`). Verify: gripper-250
   stays exactly 32,123 evals + 40 tests.
2. **Preferred-operator (helpful-action) best-first**, behind a flag (plan-valid) —
   the FF-parity fix for instances that *genuinely* fall back (deep plateaus, which
   the cap fix doesn't help). Variant A (heap-key bonus, stays complete) is safest.
   Higher ceiling; needs a flag + evaluated-count re-baseline.
3. **`apply_into` clone-on-survival** (h-identity) — `apply` clones a full `State`
   per applicable op *before* the cost-bound + visited dedup discard most of them;
   apply into a reusable buffer, materialize only survivors.
4. **Pre-size `visited` / static `op_has_relevant_neff`** (h-identity) — small,
   low-risk allocator/scan trims.

**Do NOT** add an applicable-action index or a scattered `build_rpg` precondition
trigger index: tried, reverted — the scattered loads lose to the sequential CSR
scan's cache locality on shallow graphs.
