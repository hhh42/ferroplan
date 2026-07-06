# Benchmarks

ferroplan is differentially tested against reference planners over a curated subset
of the IPC contest suites — classical (gripper, blocks, logistics, …), numeric
(rovers, satellite, depots, …), ADL, and IPC-5 simple-preferences. Coverage, plan
validity, and speed are checked against the C **Metric-FF**; IPC-5 **preference
quality is scored against SGPlan5**, the contest winner (per-instance metrics from
the official archive — see the
[scoreboard](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md)).
A locally-built SGPlan6 is also used for solvability/validity agreement.

The harness lives in [`benchmarks/`](https://github.com/hhh42/ferroplan/tree/main/benchmarks):
solvability agreement, plan validity, plan length, and (for preferences) metric
comparison, plus `criterion` micro-benchmarks for parse/ground/search.

Results tables are regenerated in CI and published here.
