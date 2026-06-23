# Encoding A/B/C results — specific vs data-table vs forall-numeric

_ff @ `dbb9bb9` · threads=1 · max_evaluated=2000000 · timeout=45.0s · lower is better (fewer node expansions / shorter makespan)._

**specific** = one `:action` per recipe · **data-table** = one `craft ?rec ?in ?out` + static `(recipe …)` table over `:constants` · **forall** = one `craft ?rec` quantifying over all resources via `(need/make ?rec ?res)`.


## chain — inst  ·  metric = evaluated_states (node expansions)

coverage — specific: 12/12 · data-table: 12/12 · forall: 12/12

| problem | specific | data-table | forall | winner |
|---|---|---|---|---|
| k08/p_n01.pddl | 9 | 9 | 9 | all tie |
| k08/p_n02.pddl | 33 | 33 | 33 | all tie |
| k08/p_n04.pddl | 125 | 125 | 125 | all tie |
| k16/p_n01.pddl | 17 | 17 | 17 | all tie |
| k16/p_n02.pddl | 97 | 97 | 97 | all tie |
| k16/p_n04.pddl | 658 | 658 | 658 | all tie |
| k24/p_n01.pddl | 25 | 25 | 25 | all tie |
| k24/p_n02.pddl | 193 | 193 | 193 | all tie |
| k24/p_n04.pddl | 2058 | 2058 | 2058 | all tie |
| k32/p_n01.pddl | 33 | 33 | 33 | all tie |
| k32/p_n02.pddl | 321 | 321 | 321 | all tie |
| k32/p_n04.pddl | 4954 | 4954 | 4954 | all tie |

aggregate — **specific** total_eval=8523, geomean_eval=128.9, geomean_ms=3.6 · **data-table** total_eval=8523, geomean_eval=128.9, geomean_ms=6.1 · **forall** total_eval=8523, geomean_eval=128.9, geomean_ms=13.0


## chain — temporal  ·  metric = makespan

coverage — specific: 11/12 · data-table: 11/12 · forall: 10/12

| problem | specific | data-table | forall | winner |
|---|---|---|---|---|
| k08/p_n01.pddl | 16 | 16 | 16 | all tie |
| k08/p_n02.pddl | 32 | 32 | 32 | all tie |
| k08/p_n04.pddl | 64.001 | 64.001 | 64 | forall |
| k16/p_n01.pddl | 32 | 32 | 32 | all tie |
| k16/p_n02.pddl | 64 | 64 | 64 | all tie |
| k16/p_n04.pddl | 122 | 122 | 122 | all tie |
| k24/p_n01.pddl | 48 | 48 | 48 | all tie |
| k24/p_n02.pddl | 96 | 96 | 96 | all tie |
| k24/p_n04.pddl | 186 | 186 | × | specific+data-table |
| k32/p_n01.pddl | 64 | 64 | 64 | all tie |
| k32/p_n02.pddl | 128 | 128 | 128 | all tie |
| k32/p_n04.pddl | × | × | × | - |

aggregate — **specific** geomean_makespan=62.6, geomean_ms=29.3 · **data-table** geomean_makespan=62.6, geomean_ms=99.8 · **forall** geomean_makespan=56.1, geomean_ms=52.3


## converge — inst  ·  metric = evaluated_states (node expansions)

coverage — specific: 8/9 · data-table: 8/9 · forall: 8/9

| problem | specific | data-table | forall | winner |
|---|---|---|---|---|
| d2/p_n01.pddl | 5 | 5 | 5 | all tie |
| d2/p_n02.pddl | 9 | 9 | 12 | specific+data-table |
| d2/p_n04.pddl | 17 | 17 | 28 | specific+data-table |
| d3/p_n01.pddl | 25 | 25 | 26 | specific+data-table |
| d3/p_n02.pddl | 144 | 144 | 171 | specific+data-table |
| d3/p_n04.pddl | 648 | 648 | 808 | specific+data-table |
| d4/p_n01.pddl | 676 | 676 | 677 | specific+data-table |
| d4/p_n02.pddl | 49941 | 49941 | 49941 | all tie |
| d4/p_n04.pddl | × | × | × | - |

aggregate — **specific** total_eval=51465, geomean_eval=125.2, geomean_ms=4.3 · **data-table** total_eval=51465, geomean_eval=125.2, geomean_ms=12.2 · **forall** total_eval=51668, geomean_eval=145.8, geomean_ms=9.8


## converge — temporal  ·  metric = makespan

coverage — specific: 5/9 · data-table: 5/9 · forall: 5/9

| problem | specific | data-table | forall | winner |
|---|---|---|---|---|
| d2/p_n01.pddl | 6 | 6 | 4 | forall |
| d2/p_n02.pddl | 12 | 12 | 8 | forall |
| d2/p_n04.pddl | 20.001 | 20.001 | 16 | forall |
| d3/p_n01.pddl | 14 | 14 | 6 | forall |
| d3/p_n02.pddl | 10 | 10 | 12 | specific+data-table |
| d3/p_n04.pddl | × | × | × | - |
| d4/p_n01.pddl | × | × | × | - |
| d4/p_n02.pddl | × | × | × | - |
| d4/p_n04.pddl | × | × | × | - |

aggregate — **specific** geomean_makespan=11.5, geomean_ms=13.0 · **data-table** geomean_makespan=11.5, geomean_ms=58.0 · **forall** geomean_makespan=8.2, geomean_ms=13.6


## techtree — inst  ·  metric = evaluated_states (node expansions)

coverage — specific: 1/1 · data-table: 1/1 · forall: 1/1

| problem | specific | data-table | forall | winner |
|---|---|---|---|---|
| tree/p_n01.pddl | 187261 | 187261 | 187261 | all tie |

aggregate — **specific** total_eval=187261, geomean_eval=187261.0, geomean_ms=17177.3 · **data-table** total_eval=187261, geomean_eval=187261.0, geomean_ms=14213.4 · **forall** total_eval=187261, geomean_eval=187261.0, geomean_ms=132868.4


## techtree — temporal  ·  metric = makespan

coverage — specific: 0/1 · data-table: 0/1 · forall: 0/1

| problem | specific | data-table | forall | winner |
|---|---|---|---|---|
| tree/p_n01.pddl | × | × | × | - |

aggregate — **specific** geomean_makespan=0.0, geomean_ms=0.0 · **data-table** geomean_makespan=0.0, geomean_ms=0.0 · **forall** geomean_makespan=0.0, geomean_ms=0.0


## Pairwise (instantaneous, via perf.py compare)

```
$ perf.py compare chain-specific-inst chain-data-table-inst
problem                                                eval Δ      len Δ  verdict
--------------------------------------------------------------------------------------

=== aggregate (deterministic) ===
  coverage:        12 -> 12
  total_evaluated: 8523 -> 8523 (+0.0%)
  geomean_eval:    128.9 -> 128.9 (+0.0%)
  geomean_ms:      3.6 -> 6.1 (+69.4%)  [machine-dependent, informational]

  improvements: 0   regressions: 0
  no deterministic regressions ✓

$ perf.py compare chain-specific-inst chain-forall-inst
problem                                                eval Δ      len Δ  verdict
--------------------------------------------------------------------------------------

=== aggregate (deterministic) ===
  coverage:        12 -> 12
  total_evaluated: 8523 -> 8523 (+0.0%)
  geomean_eval:    128.9 -> 128.9 (+0.0%)
  geomean_ms:      3.6 -> 13.0 (+261.1%)  [machine-dependent, informational]

  improvements: 0   regressions: 0
  no deterministic regressions ✓

$ perf.py compare converge-specific-inst converge-data-table-inst
problem                                                eval Δ      len Δ  verdict
--------------------------------------------------------------------------------------

=== aggregate (deterministic) ===
  coverage:        8 -> 8
  total_evaluated: 51465 -> 51465 (+0.0%)
  geomean_eval:    125.2 -> 125.2 (+0.0%)
  geomean_ms:      4.3 -> 12.2 (+183.7%)  [machine-dependent, informational]

  improvements: 0   regressions: 0
  no deterministic regressions ✓

$ perf.py compare converge-specific-inst converge-forall-inst
problem                                                eval Δ      len Δ  verdict
--------------------------------------------------------------------------------------
d2/p_n02.pddl                                          +3 +33.3%         +0  regression (more states)
d2/p_n04.pddl                                         +11 +64.7%         +0  regression (more states)
d3/p_n01.pddl                                          +1  +4.0%         +0  regression (more states)
d3/p_n02.pddl                                         +27 +18.8%         +0  regression (more states)
d3/p_n04.pddl                                        +160 +24.7%         +0  regression (more states)
d4/p_n01.pddl                                          +1  +0.1%         +0  

=== aggregate (deterministic) ===
  coverage:        8 -> 8
  total_evaluated: 51465 -> 51668 (+0.4%)
  geomean_eval:    125.2 -> 145.8 (+16.5%)
  geomean_ms:      4.3 -> 9.8 (+127.9%)  [machine-dependent, informational]

  improvements: 0   regressions: 5
  REGRESSED: d2/p_n02.pddl, d2/p_n04.pddl, d3/p_n01.pddl, d3/p_n02.pddl, d3/p_n04.pddl

$ perf.py compare techtree-specific-inst techtree-data-table-inst
problem                                                eval Δ      len Δ  verdict
--------------------------------------------------------------------------------------

=== aggregate (deterministic) ===
  coverage:        1 -> 1
  total_evaluated: 187261 -> 187261 (+0.0%)
  geomean_eval:    187261.0 -> 187261.0 (+0.0%)
  geomean_ms:      17177.3 -> 14213.4 (-17.3%)  [machine-dependent, informational]

  improvements: 0   regressions: 0
  no deterministic regressions ✓

$ perf.py compare techtree-specific-inst techtree-forall-inst
problem                                                eval Δ      len Δ  verdict
--------------------------------------------------------------------------------------

=== aggregate (deterministic) ===
  coverage:        1 -> 1
  total_evaluated: 187261 -> 187261 (+0.0%)
  geomean_eval:    187261.0 -> 187261.0 (+0.0%)
  geomean_ms:      17177.3 -> 132868.4 (+673.5%)  [machine-dependent, informational]

  improvements: 0   regressions: 0
  no deterministic regressions ✓
```

