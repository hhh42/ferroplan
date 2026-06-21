# Benchmark corpus attribution

The PDDL instances under `benchmarks/ipc/` are a subset of the International
Planning Competition (IPC) benchmark suites, redistributed for research/testing
with attribution.

- Source mirror: **potassco/pddl-instances** (https://github.com/potassco/pddl-instances)
- Original competitions: IPC 1998–2006 (the respective domain authors).
  - `strips/gripper`, `adl/gripper` — IPC-1998 (Gripper)
  - `strips/blocks` — IPC-2000 (Blocks)
  - `numeric/rovers`, `numeric/satellite` — IPC-2002 numeric track
  - `pref/{openstacks,pathways,rovers,storage,tpp,trucks}` — IPC-2006 (IPC-5)
    simple-preferences / soft-goal track, 8 instances each. This is the suite the
    metric/quality work and the SGPlan/ESPC effort are measured against
    (openstacks-soft authored by Patrik Haslum).

These files retain their original licensing from the IPC / potassco mirror. They
are included only as runnable benchmark examples; ferroplan itself is dual MIT /
Apache-2.0.
