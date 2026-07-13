# Tuning & environment knobs

ferroplan's defaults are chosen so that a plain `ff -o domain -f problem` is the
measured-best path — you should rarely need any of these. When you do, every knob is
an **environment variable**, read at solve time. Reads are panic-free on
`wasm32`; the boolean features additionally have in-process overrides in
[`ferroplan::features`](https://docs.rs/ferroplan) (`set_overrides`,
`set_escalate_override`, `set_espc_override`) for WASM/embedded callers, where
`std::env::set_var` panics.

Almost every knob is a **restore hatch**: it exists to reproduce an earlier
behavior or to run an experiment, and the default is the recommended setting. All
results are deterministic and thread-count independent.

## Temporal

| var | default | effect |
|---|---|---|
| `FF_TDEMAND` | numeric-only | force the **Full** demand tier (also seed demand from predicate-goal thresholds) *first*, for conjunctive/structural builds. |
| `FF_NO_TDEMAND` | — | master switch to the pristine pre-v0.2 path: no demand guidance, no relevance pruning, no escalation. |
| `FF_NOREL` | — | disable goal-relevance pruning alone (keep demand guidance). |
| `FF_NO_ESCALATE` | ladder on | disable the on-failure escalation ladder (retry Full tier, then decomposer). Only affects would-be failures. |
| `FF_TDECOMP` | off | route the temporal path through the partition-and-resolve decomposer first (the `decompose` API always does, regardless). |
| `FF_TCONC` | off | run the concurrent scheduling phase — repack a plan onto actor objects to minimize makespan. |
| `FF_TDEMAND_W` | `3` | weight of the temporal demand seed. |

## PDDL3 preference optimizer

The default path is the exact-closure metric optimizer; these restore its
predecessor pieces or tune its budget.

| var | default | effect |
|---|---|---|
| `FF_PREF_EVAL_BUDGET` | `2000000` | deterministic per-solve eval budget — **the real quality dial**. Higher = more optimization time on hard instances. |
| `FF_PREF_NO_ESCALATE` | escalate | disable the budget-escalating retry (abandon a probe on its first capped iteration). |
| `FF_PREF_GREEDY` | anytime | restore first-improvement sweeps: return at the first plan under the bound and restart, instead of tightening the bound in place and draining the sweep. |
| `FF_PREF_NO_RESTARTS` | ladder on | disable the diversified restart ladder (rotated open-list weight profiles on a capped no-improvement sweep) — the lever behind the storage p06/p07 and pathways p05 wins. |
| `FF_PREF_SEED` | off | **experimental**: forgo-aware second seed — price each preference's completion with a cost-aware relaxed plan and pre-forgo those priced over their weight. Measured neutral on rovers (the EHC seed already lands there). |
| `FF_PREF_NO_STATIC` | simplify | disable static preference simplification at compile (keep statically-satisfied instances). |
| `FF_PREF_BARRIER` | barrier-free | restore the compilation barrier in DNF guidance. |
| `FF_PREF_COMPILED` | closure | route through the legacy compiled-goal B&B instead of the exact-closure optimizer. |
| `FF_PREF_COST_WEIGHT` | domain-dependent | cost-aware open-list weight (`SearchCfg::w_c`). **Experimental** — a measured dead end on rovers; default 0 there. |
| `FF_RES_WEIGHT` / `FF_RES_THRESH` | tuned / `0` | satisfaction-guidance resource penalty weight / threshold. |
| `FF_DEADLINE_WEIGHT` | `0` | extra penalty on deadline-pair triggers in satisfaction guidance. |
| `FF_RES_DEBUG` | — | print resource/preference simplification diagnostics. |

## ESPC penalty loop (opt-in)

The extended-saddle-point penalty loop for resource-coupled preference domains
(openstacks-shaped). Off by default; engages only when the compiled task carries
once-only conditional-achievement deadline pairs.

| var | default | effect |
|---|---|---|
| `FF_ESPC` | off | **enable** the ESPC partitioned penalty loop. |
| `FF_ESPC_MONO` | partitioned | reproduce the earlier monolithic (pre-0.4.0) loop. |
| `FF_ESPC_TIME_MS` | `15000` | wall-clock budget for the loop, in ms. |

Advanced ESPC schedule tuning (rarely needed): `FF_ESPC_OUTER` (outer iterations),
`FF_ESPC_RATE` (initial penalty rate, `20`), `FF_ESPC_K` (consecutive-violation
rate bump, `2`), `FF_ESPC_LAMBDA0` (initial λ, `0`), `FF_ESPC_STALL` (stall limit
before termination, `4`).

## Reproducing a specific benchmark

The [IPC-5 scoreboard](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md)
records the exact env for each domain — e.g. openstacks's domain lead is
`FF_ESPC=1 FF_ESPC_TIME_MS=90000`.
