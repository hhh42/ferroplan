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
| `FF_PREF_SEED3` | off | **experimental**: partitioned closure seed — compose a per-preference-component incumbent (mutex-conflict-pruned, sibling-protected stages) before the tightening loop. Composes genuinely (tpp p05: 99 vs the 105 init-tail) but measured neutral on finals: the anytime+ladder loop reaches the same metric from either bound. |
| `FF_PREF_NO_SELECT` | select on | disable the 0.6 **selection layer** (exact preference-subset selection solved combinatorially, then planned as a hard-goal target; `docs/forensics-tpp.md`). Selection is what ties SGPlan5 on tpp p06 and widened the rovers totals lead; its bounded seed runs outside the tightening budget. |
| `FF_PREF_NO_STATIC` | simplify | disable static preference simplification at compile (keep statically-satisfied instances). |
| `FF_PREF_NO_BARRIER` | barrier on | exclude init-satisfied preferences from the guidance (the 0.4–0.5.0 behavior). Keeping them (the 0.5.1 default) protects high-weight trap preferences — the storage 8/8 sweep — see `docs/forensics-tpp.md`. `FF_PREF_BARRIER` is accepted, now redundant. |
| `FF_PREF_COMPILED` | closure | route through the legacy compiled-goal B&B instead of the exact-closure optimizer. |
| `FF_PREF_NUMLEGACY` | closure | folded **numeric** metrics only (rovers-shaped): restore the pre-0.5 routing to the legacy B&B. The closure path now dominates it (rovers flipped to a domain lead). |
| `FF_PREF_COST_WEIGHT` | domain-dependent | cost-aware open-list weight (`SearchCfg::w_c`). **Experimental** — a measured dead end on rovers; default 0 there. |
| `FF_RES_WEIGHT` / `FF_RES_THRESH` | tuned / `0` | satisfaction-guidance resource penalty weight / threshold. |
| `FF_DEADLINE_WEIGHT` | `0` | extra penalty on deadline-pair triggers in satisfaction guidance. |
| `FF_RES_DEBUG` | — | print resource/preference simplification diagnostics. |

## ESPC penalty loop (default-on where it bites)

The extended-saddle-point penalty loop for resource-coupled preference domains
(openstacks-shaped). **On by default since 0.5**, with a deterministic
evaluated-state budget; it engages only when the compiled task carries once-only
conditional-achievement deadline pairs — on every other task it is a verified
no-op.

| var | default | effect |
|---|---|---|
| `FF_NO_ESPC` | espc on | **opt out** — restore the closure-optimizer-only default path. |
| `FF_ESPC_EVAL_BUDGET` | `6000000` | deterministic eval pool for the loop (λ iterations + polish) — the primary budget contract, thread-count independent. |
| `FF_ESPC_TIME_MS` | unset | optional **additional** wall-clock cap for interactive use. Applies only when set; setting it trades determinism for latency. |
| `FF_ESPC_MONO` | partitioned | reproduce the earlier monolithic (pre-0.4.0) loop. |
| `FF_ESPC` | — | accepted for compatibility (pre-0.5 opt-in); redundant now. |

Advanced ESPC schedule tuning (rarely needed): `FF_ESPC_OUTER` (outer iterations),
`FF_ESPC_RATE` (initial penalty rate, `20`), `FF_ESPC_K` (consecutive-violation
rate bump, `2`), `FF_ESPC_LAMBDA0` (initial λ, `0`), `FF_ESPC_STALL` (stall limit
before termination, `4`).

## PDDL3 trajectory constraints

| var | default | effect |
|---|---|---|
| `FF_CONSTRAINTS_REJECT` | enforce | restore the 0.4.1–0.6 blanket **rejection** of every `(:constraints ...)` block, instead of compiling the hard untimed operators into enforced monitor automata (0.7). The hatch restores *rejection*, not ignoring — no setting makes ferroplan silently drop a constraint. |

## Reproducing a specific benchmark

The [IPC-5 scoreboard](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md)
records the exact env for each domain — e.g. openstacks's domain lead is
`FF_ESPC=1 FF_ESPC_TIME_MS=90000`.
