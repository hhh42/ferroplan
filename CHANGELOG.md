# Changelog

All notable changes to this project are documented here.

## [0.1.0] - unreleased

Initial public release.

### Added
- Data-parallel FF planner core (bitset / CSR, parallel grounding + heuristic).
- **Enforced hill-climbing (EHC)** with helpful actions and a weighted-best-first
  fallback — the default, ~3× faster than best-first and metric-ff-class on
  classical/ADL (geomean 0.21× → 0.66× Metric-FF).
- **Configurable `Options`** (library-first; CLI flags + JSON map to the same
  fields): `mode`, `search`, `helpful_actions`, `weight_g/weight_h`, `threads`,
  `max_evaluated`, `optimize`.
- ADL: conditional effects, `forall`/`exists`, object equality.
- Numeric fluents (Metric-FF style).
- PDDL3 soft-goal preferences (incl. `forall`-quantified and precondition
  preferences) with anytime branch-and-bound metric optimization. IPC-5 coverage
  on par with SGPlan6 (39/48).
- **PDDL2.1 temporal**: durative actions with `at start`/`over all`/`at end`
  conditions & effects, constant or parameter-dependent durations, required
  concurrency, and ε-separation; decision-epoch search; IPC temporal plan output
  with makespan. Plans validated against VAL on real IPC domains (44/45 valid);
  an independent in-crate validator (`temporal::validate`).
- SGPlan-style partition-and-resolve mode.
- Library API returning structured, `serde`-serializable results.
- `ff` CLI: drop-in `-o/-f` text, `--json`, `--json-request` job I/O, full
  strategy flags.
- **Robust** against malformed input — pathological/deeply-nested PDDL returns a
  typed error, never a panic.
- mdBook documentation site; cross-planner comparison harness (`compare.py`),
  temporal+VAL harness (`bench_temporal.py`), and benchmark results vs
  Metric-FF / SGPlan6 / VAL.

### Known limitations
- Numeric domains trail Metric-FF (EHC falls back to best-first on some).
- IPC-5 preference metric *quality* on the hardest instances trails SGPlan6 — the
  general fix needs a SAS+ layer (see `docs/espc-preferences-spec.md`).
- Temporal coverage is search-limited on large instances.
- Not supported: duration inequalities, timed initial literals, continuous (`#t`)
  effects, derived predicates.
