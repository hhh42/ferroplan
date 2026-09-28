# cut28 -- the 0.28.0 cut's reads (receipts)

- `compare-0.27.0-vs-0.28.0.txt` -- `crucible compare --set cut28 --a 86302e06d81b --b 89cfdc5f06ed`,
  the banked boards side by side, per-cell flips among cells both engines banked.
- `lost.rows` -- the 24 cells 0.27.0 banked solved and 0.28.0 did not (`--rows` format).
- `attempts-estimator-cut28.txt` -- `benchmarks/attempts-estimator.py --a 86302e06d81b --b 89cfdc5f06ed --per-board`:
  the PRE-REGISTERED read (docs/roadmap-0.28.md "THE CUT"): first attempt vs first attempt, equal-N beside it.
- `regress-v0.27.1.log`, `run.log` -- the differential: v0.27.1 (`d812c2328305`) re-run over `lost.rows`
  through the crucible (`backfill --set cut27 --tag v0.27.1 --rows lost.rows --name cut28-regress`),
  staged at benchmarks/probes/cut28-regress/ in the operator's checkout.
