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

## Budgets — the wall and the memory

A budget is declared, never imposed: ferroplan does not kill itself. It
reads what the caller (or the caller's runner) will enforce and arranges to be
finished first. Per call, in code, the same two ideas are `Options::wall_ms`
and `Options::should_continue`.

| variable | default | effect |
|---|---|---|
| `FF_TIME_LIMIT` | unset | the process's REAL wall budget in seconds — see [Classical search](#classical-search-experiments--restore-hatches) for the ladder gate it arms. Since 0.28 it also bounds everything done to a plan ALREADY in hand. |
| `FF_REPORT_RESERVE_SECS` | 3 % of the wall (0.5–3 s) + a size term | how far short of the wall optional work on a found plan stops — the quality chases, the preference scorer, the PDDL3 optimizer over its seed — so the plan is reported before a runner that kills AT the wall fires (0.28). |
| `FF_MEM_BUDGET_GB` | unset | the retained-memory budget in GiB (fractional ok) the caller will enforce. It sizes the modelled node caps, and since 0.28 it is also MEASURED: the engine reads its own resident size (`/proc/self/status`; `task_info` on macOS). |
| `FF_NO_MEM_WALL` | wall on | disable the 0.28 measured memory wall. With it, *bounded* work — a chase over a banked plan, the grounding that would price or score a plan already found, a rung's bet — stops at 75 % of `FF_MEM_BUDGET_GB` and returns what was banked; a trip is sticky for that scope. A first search, with nothing to fall back on, is never cut by it. |
| `FF_MEM_TRIP_FRAC` | `0.75` | move that line — for measuring it (0.85 was measured: nothing solves or scores better, and one board instance peaks at 5.97 GB of 6). |
| `FF_EHC_WALL_FRAC` | `0.25` | the fraction of the remaining wall EHC may spend before handing down the ladder. `FF_NO_EHC_WALLCAP=1` removes the slice. |
| `FF_NO_RUNG_WALLCAP` | checkpoints on | turn OFF the clock checkpoints inside the bounded rungs, best-first batches and grounding (the 0.21 shapes — a run can then overrun its wall; kept as the permanent RED record). |
| `FF_WALL_DEBUG` / `FF_GROUND_PHASES` | off | narrate every wall and memory decision / every grounding phase with its elapsed time on stderr. Probe eyes; never affect the search. |

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
| `FF_NO_TSYMM` | symmetry on | disable the 0.13 pending-interval **symmetry reduction** (canonical agenda order + redundant identical-interval elimination). The restore hatch for its one named corpus casualty (parking-2011 #16). |
| `FF_TEMPORAL_NODE_CAP` | byte model | override the deterministic temporal stored-node cap directly (`0` disables; default derives from the memory target and static task dims). |
| `FF_TEMPORAL_ABS_KEY` | relative | restore ABSOLUTE agenda times in the visited key on TIL-free tasks (the pre-0.10 behavior; relative keying dedups retimed permutations). |
| `FF_TLAMA` | off | **experimental**: the temporal LAMA rung (fact-landmark-dominant key). Measured negative — snap-task landmarks are RUNNING-token chains with no branching signal. |
| `FF_LAX_HELPFUL` | off | **experimental**: re-arm empty temporal helpful sets with a lax fallback instead of a full scan. Measured zero new solves. |
| `FF_TAGENDA_W` | `0` | **experimental**: complete-pass agenda-size ordering term (de-prioritize interval hoarding). Measured negative on parc-printer-t. |
| `FF_NO_FIXPOINT_GROUND` | fixpoint (Session) | make the `Session`'s temporal entry use stratified grounding instead of reached-restricted **fixpoint grounding** (the ~117× transient-memory win on sparse-reachable worlds). Corpus/CLI paths always use stratified — their tie-breaks pin the scoreboard baselines. |
| `FF_NO_ORBIT` | orbits on | disable **object-symmetry orbit** detection (0.14): goal-respecting member permutations canonicalize the temporal visited key, collapsing states that differ only by relabeling interchangeable objects/goal pairs (machine-shop's wall; rescues turn-and-open under the sound invariant semantics). |
| `FF_ORBIT_DEBUG` | off | narrate orbit detection (candidate groups, bail reasons) and per-solve eval counts on stderr — the probe eyes, never affects the search. |
| `FF_ORBIT_GEN` | off | **experimental**: generation-side stabilizer skipping (0.15): with orbits detected, an op whose state-fixing member swap maps it to an already-generated sibling is never generated (TMS: 2.4× real evals at equal budget; duplicates 81% → 53% — but zero new solves, since TMS's wall is the start-credit plateau, not throughput). The 0.15 sweep referee flipped it opt-in: the per-expansion stabilizer scan cost match-cellar 9 solved instances. The canonical-key pre-dedup stays default-on. |
| `FF_TLIFO` | off | **experimental**: LIFO tie-breaking at equal temporal keys. Measured WORSE on the TMS plateau (best_h 150 vs 110) — dives a high-h corridor. |
| `FF_TB_FREE_G` | off | **experimental**: don't charge g for time-advance successors. Measured WORSE on TMS (best_h 196) — rides time forward, wasting windows. |
| `FF_TAGENDA_W_PRUNE` | `0` | **experimental**: the agenda-size term on the PRUNED pass (start-credit counter-account). Measured worse on TMS (best_h 173) though it does push the search deep enough for window blocking to engage. |
| `FF_NO_TCOMPRESS` | rung on | disable the 0.28 **compression rung** (plan a concurrency-free temporal task as a classical one, left-shift it onto the clock, validate against the original). Restores the 0.27 route. |
| `FF_TCOMPRESS_WALL_FRAC` / `FF_TCOMPRESS_CHASE_FRAC` | `0.25` / `0.25` | the share of the remaining wall the compression rung may bet, and the share the decision-epoch makespan chase gets after it banks. Unwalled, the bet is capped at 2,000 evaluations per classical rung instead. |
| `FF_NO_TSUCC` | generator on | restore the full operator scan in temporal expansion. 0.28 wired the anchored successor generator (0.27's classical lever) into the temporal rung; it is byte-identical by construction and, measured, removes a 0.01–7.7 % term — the relaxed planning graph is the other 92–99 %. Kept for exactly that kind of measurement. |
| `FF_TEVAL_BUDGET` | unlimited | cap CLI temporal search **evaluations** — the deterministic measuring stick for A/B probes (eval budgets, never wall clock). |

## Classical search (experiments & restore hatches)

| var | default | effect |
|---|---|---|
| `FF_NOVELTY` | auto |  the width-1 BFWS-style novelty rung (0.17) — a third bounded classical rung after EHC and LAMA fail, open list ordered by state novelty then h. 0.19: DEFAULT-ON when `FF_TIME_LIMIT` is declared and >40% of the budget remains (the 0.18 gated referee measured +4/−0, reversing 0.17's ungated +7/−51 tax); `FF_NO_NOVELTY=1` opts out, `FF_NOVELTY=1` forces it without a budget. No budget declared ⇒ rung off, byte-identical ladder. `FF_NOVELTY_ONLY=1` probes the rung alone. |
| `FF_TIME_LIMIT` | unset | the process's REAL wall budget in seconds (0.18 budget-aware ladder). The clock arms at solve entry (grounding counts as spent budget); a bounded rung (LAMA, novelty) is entered only while **more than 40% of the budget remains**, so late-ladder rungs stop starving the complete fallback near the budget edge — the mechanism behind the novelty referee's −51. Unset = all-rungs behavior, byte-identical to 0.17. `benchmarks/ipc67.py` passes its per-instance `--timeout` automatically. Informational, never kills the process. `FF_WALL_DEBUG=1` narrates the gate's verdict on stderr (the probe eyes, never affects the search). |
| `FF_NO_NEED_DIRS` | direction-aware | restore the 0.27 relaxed-graph fixpoint test, which counted ANY numeric bound movement as progress — a consumable's un-read lower bound drifting down sent every dead-end evaluation to the 2,000-layer cap. Heuristic values are identical either way; only the cost of proving a dead end changes (~78× evaluations per second on rovers-metric-time shapes). |
| `FF_HTRACE` | off | print the classical best-first `best_h` trace (evaluation count and seconds at each improvement) on stderr. |
| `FF_NO_DNF_STATIC` | resolve | disable static resolution inside precondition DNF expansion (restore the 2^k `imply` blowup the 0.10 fix removed — openstacks-ADL 6/30 → 30/30). |
| `FF_NO_TRAJ_END` | end-action | restore the exponential goal-DNF construction for hard trajectory monitors (pre-0.8). |
| `FF_CLM` | off | **experimental**: classical landmark-count guidance term. Measured negative (transport unchanged, floor-tile worse). |
| `FF_LEN_ANYTIME` | off | **experimental**: keep draining the open list after the first plan, returning the shortest found. Measured −9 coverage at 60 s; opt-in only. |
| `FF_LEN_SWEEP_EVALS` | off | **experimental**: iterated-weight anytime re-sweeps for unit-cost plan length (~1.8% shorter at ~28× evals). |

## PDDL3 preference optimizer

The default path is the exact-closure metric optimizer; these restore its
predecessor pieces or tune its budget.

| var | default | effect |
|---|---|---|
| `FF_PREF_EVAL_BUDGET` | `2000000` | deterministic per-solve eval budget — **the real quality dial**. Higher = more optimization time on hard instances. |
| `FF_PREF_NO_ESCALATE` | escalate | disable the budget-escalating retry (abandon a probe on its first capped iteration). |
| `FF_PREF_GREEDY` | anytime | restore first-improvement sweeps: return at the first plan under the bound and restart, instead of tightening the bound in place and draining the sweep. |
| `FF_PREF_NO_RESTARTS` | ladder on | disable the diversified restart ladder (rotated open-list weight profiles on a capped no-improvement sweep) — the lever behind the storage p06/p07 and pathways p05 wins. |
| `FF_PREF_NO_SEED` | seed on | disable 0.28's **incumbent zero** — the hard-goal plan handed to the optimizer as a floor, and returned (with a note) when nothing cheaper was found. Restores 0.27, where such a run reported `solved: false`. |
| `FF_PREF_SEED_BOUND` | floor only | also OPEN the branch-and-bound with the seed's metric as its first bound (off by default: the seed is a floor, not a bound, so the optimizer's own search is byte-identical to 0.27's). |
| `FF_PREF_SEED_WALL_FRAC` / `FF_PREF_SEED_EHC_FRAC` | `1.0` / `0.6` | the share of the remaining wall the hard-goal search may use, and EHC's share of THAT. |
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
| `FF_CONSTRAINTS_REJECT` | enforce | restore the 0.4.1–0.6 blanket **rejection** of every `(:constraints ...)` block, instead of compiling the untimed operators into enforced monitor automata (0.7) — hard constraints as goal conjuncts, soft `(preference ...)` constraints priced through the PDDL3 metric machinery. The hatch restores *rejection*, not ignoring — no setting makes ferroplan silently drop a constraint. |

## Reproducing a specific benchmark

The [IPC-5 scoreboard](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md)
records the exact env for each domain — e.g. openstacks's domain lead is
`FF_ESPC=1 FF_ESPC_TIME_MS=90000`.
