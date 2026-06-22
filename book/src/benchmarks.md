# Benchmarks

ferroplan is differentially tested against the C **Metric-FF** and **SGPlan6**
reference binaries over a curated subset of the IPC contest suites — classical
(gripper, blocks, logistics, …), numeric (rovers, satellite, depots, …), ADL,
and IPC-5 simple-preferences.

The harness lives in [`benchmarks/`](https://github.com/hhh42/ferroplan/tree/master/benchmarks):
solvability agreement, plan validity, plan length, and (for preferences) metric
comparison, plus `criterion` micro-benchmarks for parse/ground/search.

Results tables are regenerated in CI and published here.
