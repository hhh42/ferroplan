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
  - `qualpref/{openstacks,rovers,storage,tpp,trucks}` — IPC-2006 (IPC-5)
    *qualitative-preferences* track, 8 instances each, vendored from the mirror's
    `ipc-2006/domains/<domain>-preferences-qualitative/` (`domain.pddl` +
    `instances/instance-{1..8}.pddl` → `p01..p08.pddl`). These add PDDL3
    `(:constraints ...)` trajectory preferences (always / sometime /
    at-most-once / sometime-before) on top of soft goals — the 0.7 target
    suite. The mirror has no pathways-preferences-qualitative directory (the
    track ran 5 domains).

These files retain their original licensing from the IPC / potassco mirror. They
are included only as runnable benchmark examples; ferroplan itself is dual MIT /
Apache-2.0.

## Official results archive

`benchmarks/IPC5-results.tgz` is the official IPC-5 (2006) competition
results archive (`RESULTS/<planner>/<domain>/<track>/pNN.soln`, per-instance
plans with `; MetricValue` headers), produced by the IPC-5 organizers
(Gerevini, Long et al., hosted at the University of Brescia). Vendored
2026-07-24, hand-retrieved from the original site's live redirect — the
Wayback Machine holds only a 301 for this file, so the vendored copy also
serves as preservation. It is the reference source for both preference
scoreboards (`ipc5-scoreboard.md`, `ipc5-qualitative-scoreboard.md`) and
the IPC-5 quality columns of the standings audit.
