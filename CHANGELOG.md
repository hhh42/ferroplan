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
  preferences) with anytime branch-and-bound metric optimization.
- SGPlan-style partition-and-resolve mode.
- Library API returning structured, `serde`-serializable results.
- `ff` CLI: drop-in `-o/-f` text, `--json`, `--json-request` job I/O, full
  strategy flags.
- mdBook documentation site; cross-planner comparison harness (`compare.py`) and
  static benchmark results vs Metric-FF / SGPlan6.

### Known limitations
- Numeric domains trail Metric-FF (EHC falls back to best-first on some).
- IPC-5 preference quality is competitive on small instances, not winning on the
  largest; temporal/durative actions and derived predicates unsupported.
