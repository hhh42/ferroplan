# jobshop — a scheduling domain (machine-exclusion)

Each **job** is a fixed sequence of operations (s1→s2→…); each operation runs on a
**machine** that does **one op at a time** — machine-exclusion modeled with a
`(free ?m)` token consumed at-start / restored at-end (the resource-exclusion
pattern rpg-world deliberately omits). Operations have per-(job,stage) durations.
Goal = all jobs complete; the planner overlaps different jobs on free machines.

Regenerate the ladder with `../../benchmarks/scale/gen_jobshop.py`.

## The ladder & the border

| problem | size (jobs×stages×machines ≈ groundings) | result |
|---|---|---|
| `p1`–`p5` | 1×2×2 … 5×5×5 | ✅ (makespan 6–19) |
| `s10` | 10×10×10 ≈ 1k | ✅ 40 |
| `s20` | 20×10×10 ≈ 2k | ✅ 84 |
| `s50` | 50×10×10 ≈ 5k | ✅ 177 (1.2s) |
| `s50w` | 50×20×20 ≈ 20k | ✅ 210 (6s) |
| `s100` | 100×20×20 ≈ 40k | ✅ **382 (45s)** |
| `s100g` | 100×30×30 ≈ 90k | ❌ grounding wall |

**The surprise: jobshop scales hugely.** 100 jobs × 20 stages on 20 machines with
full machine-exclusion schedules to makespan 382 in 45s. The reason: jobs are
**independent linear chains that never converge** — the engine's strong suit (see
the *unifying law* in [`BORDERS.md`](../BORDERS.md)). None of rpg-world's
heuristic killers appear here. The **only** limit is grounding-table size (~40k
operate instances solves, ~90k walls). For a subproblem-maker: a whole job-shop is
safe under ~40k tuples; beyond that, **partition by jobs** (never by machine or
stage — those are what tie the schedule together).

```sh
ff -o examples/jobshop/domain.pddl -f examples/jobshop/s100.pddl   # 100 jobs, 45s
```
