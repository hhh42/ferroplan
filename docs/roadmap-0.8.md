# Roadmap — the road to v0.8 ("Pay the Costs")

> **Status: in execution.** Successor to the executed
> [0.7 roadmap](roadmap-0.7.md). Ground truth: the 0.7 *Recorded* blocks
> (the goal-DNF blow-up quantified at 3^10 = 59,049 REACH-GOAL ops on the
> storage hard fixture; the two storage-tail memory walls named on the
> [qualitative scoreboard](../benchmarks/ipc5-qualitative-scoreboard.md)),
> the 0.7 NOT-list (constraint-aware guidance named "the 0.8 headline
> candidate", gated on the ledger showing guidance-bound tails — it does:
> trucks p07/p08 exceed the search budget, and four more instances need a
> doubled wall budget), and the twice-punted temporal gates (0.7 Phase 3
> constraints-on-temporal, 0.6→0.7 Phase 4 temporal selection).

0.7 moved the fence: the six untimed PDDL3 modalities are enforced, soft
trajectory preferences are priced, and the qualitative track is scored. But
the enforcement came with two measured prices and one measured blindness,
all recorded rather than fixed:

- **Exponential goals.** Each monitor's S_n acceptance check is a goal
  conjunct, several operators contribute disjunctions, and the grounder
  compiles a disjunctive goal into one synthetic REACH-GOAL operator per
  DNF disjunct — exponential in the monitor count (storage p05 hard
  fixture: 3^10 = 59,049 ops, measured again on this box at 59,969 total /
  2.16 s against 920 unconstrained).
- **A monitor tax on every action.** Every monitor's `When` transitions
  are appended to every action and *re-grounded and re-stored per ground
  op* — although the transition block is fully ground and byte-identical
  across all of them — with the conditional-effect payload held in four
  simultaneously-resident copies through `ground_v`. On storage qualpref
  p07/p08 (1,147+ surviving monitors) that product exceeds 15 GB before
  any search starts.
- **A search that cannot read its own monitors.** Delete relaxation drops
  negative conditions and never re-adds deleted complements, so `always`
  violation traps cost zero in the heuristic and a set `TRAJi-VIOL` bit —
  provably permanent — steers nothing. The ESPC pass engages on
  monitor-compiled tasks via deadline pairs that are monitor artifacts,
  then blows its frontier memory inside a single pass on the widened
  states (storage p05/p06 ship with a documented `FF_NO_ESPC=1` row).

**0.8 pays those costs.** The compilation becomes linear where it was
exponential, the grounding becomes shared where it was multiplied, the
ESPC pass becomes bounded where it was unbounded, and the search learns to
read the automata it already carries. The twice-deferred temporal work
(constraints on the temporal path; temporal selection) rides again as
gated phases, in the 0.5/0.6/0.7 tradition: measured win or documented
dead end, neither on the minimum shipping path.

Two contracts are sacred throughout, unchanged: **reported == verified**
(verify.rs folds the ORIGINAL constraint semantics over its replay and
never sees the compiled monitors — every encoding change in this plan is
invisible to the oracle by construction, which is exactly what makes the
oracle the regression net for it), and **determinism** (same problem, same
plan, any thread count; every default change keeps a restore hatch; every
budget is an eval count, never a wall clock; negative results get recorded
in this document).

---

## The ground 0.8 starts from

- **The compiled artifacts.** `constraints::compile` (constraints.rs:511)
  emits per-monitor 0-ary facts (`TRAJ{i}-VIOL/SEEN/HOLD/PEND/SAFE`,
  reserved-namespace-enforced), appends the accumulated ground `When`
  transitions to every action (:674-680), and conjoins S_n acceptance into
  the goal — hard conjuncts directly (:652), soft as
  `(preference name ...)` wrappers (:670). The observation offset
  (conditions read the SOURCE state, so a monitor riding a_k observes
  S_{k-1}; S_0 at compile time; S_n goal-side) is the load-bearing design
  note this plan relocates *once more* — onto a terminal action that
  observes S_n as its source state.
- **The blow-up sites, located.** Goal side: `to_dnf(problem.goal)`
  (ground.rs:811) cartesian-multiplies the per-monitor acceptance
  disjunctions (`at-most-once` contributes 3 disjuncts, `sometime`/
  `sometime-after`/`sometime-before` 2) and REACH-GOAL synthesis
  (ground.rs:900-933) mints one op per disjunct. Product side:
  `ground_action` emits one RawOp per precondition-DNF disjunct *cloning
  the entire grounded effect* (ground.rs:439-453), `ground_effect` stores
  the byte-identical monitor RCondEff block per op (:318-324), and raws /
  interned mids + `cond_atoms` string clones / FinalOps / CSR are all
  live at once to the end of `ground_v`. Soft acceptance never widens the
  goal DNF (`Formula::Pref` → True in the classical grounder,
  ground.rs:197): the exponential is a HARD-constraint cost; the product
  is everyone's.
- **The ESPC failure, located.** The exit-137 happens inside ONE
  monolithic tightening pass (`solve_under_penalties`, espc.rs:291 →
  `search_from`): `nodes` (a full State clone per inserted successor,
  append-only), `visited` (a full-bitset StateKey per entry), and the
  per-batch `cand_chunks` spike all grow per *insertion*, while
  `FF_ESPC_EVAL_BUDGET` caps *popped* nodes only — and the first pass runs
  with bound0 = ∞, so the successor cost-prune (search.rs:450) never
  fires. Monitor compilation inflates both factors (branching and state
  width). The closure loop survives the same instances because it starts
  from a finite init-tail bound, masks the P3 ops, and caps per-iteration
  evals. One engagement quirk matters: `build_deadline_guidance`
  (pddl3.rs:2491) pattern-matches ordinary single-add ops whose
  monitor-`When` effects add TRAJ facts appearing in priced-preference
  collect preconditions — so ESPC engages on storage qualpref through
  *monitor artifacts*, not openstacks-shaped achievement structure.
- **The blindness, located.** hFF fires conditional adds when `cond_pos`
  is relaxed-reached and drops `cond_neg` entirely (heuristic.rs:262-270);
  complementary `(NOT (TRAJi-VIOL))` facts are init-true and never
  re-added, so acceptance is always relaxed-satisfied and VIOL adds are
  goal-irrelevant — the heuristic is provably blind to monitor state in
  both directions. Meanwhile no transition ever *deletes* a VIOL bit
  (only HOLD has a delete), so a set hard-VIOL bit is a *provable dead
  end* the search never uses. Soft trajectory preferences already get
  partial concrete-state guidance for free (their acceptance formulas
  reach `PrefPhi` through the P3COLLECT preconditions — TRAJ facts
  survive the `(P3` filter); hard constraints get none: every classical
  path passes `sat=None`.
- **No monitor table survives.** `emit`'s metadata (index → operator →
  fact names → hard/soft → bodies) is discarded after `compile`; the gate
  returns only the rewritten `(Domain, Problem)`. Both the guidance phase
  and any future temporal work want that table as a structured artifact.
- **The temporal fence, unchanged.** `constraints::gate` rejects any
  constraints on durative domains (constraints.rs:490-497). The 0.7
  Phase-3 design stands: transitions must reach snap actions (they copy
  effect trees verbatim, so AST-level injection into
  `DurativeAction.effects` flows through for free) AND the TIL appliers
  synthesized *inside* `temporal::compile` after the gate ran. The
  temporal goal test consumes literal-only `goal_pos`/`goal_num` — the
  goal shape Phase 1 produces is the one the temporal path already
  assumes.

---

## Phases

```
Phase 1: the END action ──► Phase 2: the grounding ──► Phase 3: ESPC bounded ──► Phase 6: measure
         (linear goals)              product (shared            (p05/p06 on               everything,
                                     monitor block)             defaults)                 ship 0.8.0
                                                                                             ▲
                      Phase 4: constraint-aware guidance (headline) ─────────────────────────┤
                      Phase 5a: temporal-path constraints ◄── (gated, needs Phase 1) ────────┤
                      Phase 5b: temporal selection ◄────────── (gated, from 0.6/0.7) ────────┘
```

Ordering rationale: Phases 1–2 are correctness-preserving encoding fixes
with exact regression oracles (the grounding fixture counts, the
byte-identical suite) — cheapest risk first, and Phase 5a *depends* on
Phase 1's literal goal. Phase 3 turns the scoreboard's two documented-env
rows into defaults rows. Phase 4 is the headline bet on the remaining
tails. Phases 5a/5b are the inherited gated stretch work; 0.8.0 ships
without them if their gates fail.

---

## Phase 1 — The END-action construction: linear goals for hard monitors

**Why:** the standard construction (recorded twice in 0.7 as "the known
fix if it bites" — it bit) moves each hard monitor's S_n acceptance check
off the goal and onto a forced-terminal synthetic action, leaving a
literal-only goal. The grounder's DNF product then never fires: cost is
linear in monitors (k conditional latches on one op) instead of
exponential (2^k/3^k REACH-GOAL ops). The 0.7 deferral reason — "moving
`problem.goal` into an action precondition interacts with the
goal-preference metric machinery" — dissolves under one design decision:
**nothing moves into a precondition, and soft acceptance does not move at
all.**

**The construction** (all inside `constraints::compile`, emitted only
when hard monitors exist — the constraint-free path stays a provable
no-op):

- Two fresh 0-ary phase facts: `TRAJ-PLANNING` (init-true), `TRAJ-ENDED`.
  Every real action's precondition gains `TRAJ-PLANNING` (the exact
  pddl3.rs:749-754 pattern).
- One synthetic 0-ary action `TRAJ-END`: precondition `TRAJ-PLANNING`;
  effect deletes `TRAJ-PLANNING`, adds `TRAJ-ENDED`, and carries one
  `Effect::When` latch per hard monitor: condition = that monitor's
  acceptance formula (monitor bits + the S_n body), add = a fresh
  `TRAJ{i}-ACC` fact. `When` conditions read the SOURCE state, so
  `TRAJ-END` applied after the last real action observes exactly S_n —
  the observation-offset contract's third leg, relocated intact. The
  latch conditions DNF-expand *additively* (2–3 conditional effects per
  monitor on one op), never multiplicatively.
- The compiled goal becomes original goal ∧ `TRAJ-ENDED` ∧ the
  `TRAJ{i}-ACC` literals — all positive conjuncts. No REACH-GOAL
  synthesis, no goal-side complementary NOT-facts, no per-CondEff
  complement toggles for acceptance. `at end` (which today contributes
  its bare φ, possibly disjunctive, to the goal) gets the same latch and
  becomes literal too.
- **Soft acceptance is untouched.** `(preference name <acc>)` wrappers
  stay in the goal with their S_n bodies intact: they are invisible to
  the classical grounder's DNF, they are what routes soft tasks into
  Pddl3 mode, `split_goal`/`pref_weights`/`preferences` keep walking
  them, P3COLLECT preconditions keep evaluating them in the frozen
  post-P3END state (which IS S_n — `TRAJ-END` only touches its own
  bookkeeping facts), and `ClosureCost`'s pre-end exactness assumption
  holds unchanged. This is the entire reason the 0.7 deferral risk
  evaporates: the metric machinery never sees a difference.
- Mixed hard+soft: `TRAJ-END` lands in `d.actions` before
  `pddl3::compile` runs, so it is a "real" action to the P3 machinery
  (gains `P3PLANNING`, is search-visible); required plan shape becomes
  real\* → `TRAJ-END` → P3END → collect/forgo. Pinned by test.
- Reporting: `TRAJ-END` is stripped from every reported surface exactly
  as REACH-GOAL is — `steps_of`, `render_plan`, `output::plan_block`,
  and the plan-text parsers gain the reserved name — so the verifier
  (which grounds the ORIGINAL problem and would hard-fail on an unknown
  op) replays exactly the real-action trajectory. The reserved-namespace
  fence (`reject_reserved_names`) grows the new fact/op names.

**Acceptance:** the storage p05 hard fixture drops from 59,969 ops
(59,049 REACH-GOAL) to ~921 (0 REACH-GOAL) with grounding wall ≪ the
2.16 s baseline, asserted by the `grounding_cost` harness extended into a
locked count test; trucks p03 overlay likewise (1,083 → ~1,066); every
bite/no-bite pair, all six entrypoint-parity tests, and the full 23-test
constraints suite hold with zero semantic change; the qualitative and
simple-preferences metric locks hold exactly (soft path untouched —
byte-identical is the target, asserted by the locks); constraint-free
input remains byte-identical; t1≡t8; reported plans contain no synthetic
step and reported == verified on every oracle-checked instance.

**Hatch:** `FF_NO_TRAJ_END=1` restores the 0.7 goal-side acceptance
byte-for-byte (the fixture then re-measures 59,969 — the hatch keeps the
old exponential *reachable*, per house convention).

**Touches:** `constraints.rs`, `ground.rs` (nothing — that is the point),
`planner.rs`/`api.rs`/`output.rs`/`plan.rs` (strip surfaces),
`tests/constraints.rs`, the grounding fixtures.

**Risks:** search must fire `TRAJ-END` exactly once, last (goal requires
`TRAJ-ENDED`; after it no real action applies — a premature `TRAJ-END` is
an immediately-recognized dead end; measured, not assumed, on the EHC
path); the empty-plan edge (goal init-true + constraints) now produces a
one-step compiled / zero-step reported plan — pinned by test; `Session`
keeps its bespoke rejection (S_0 staleness is unaffected by this
construction — see NOT-list).

---

## Phase 2 — The grounding product: ground the monitor block once

**Why:** Phase 1 removes the exponential but NOT the p07/p08 wall — the
monitor transition block is byte-identical across every ground op, yet is
re-derived and re-stored per op in four simultaneously-live copies. With
1,147+ survivors × thousands of ops that duplication IS the 15 GB. The
fix is to make the shared thing actually shared.

**Scope:**

- **Measure first** (the open question the map left): instrument
  grounding on storage qualpref p07 to rank the four resident copies
  (string-form raws vs interned mids + `cond_atoms` vs FinalOps vs CSR)
  and confirm `expand` itself fits before `simplify_static` drops
  instances. The numbers go in this document; the mechanism below is
  adjusted to what they say.
- Ground the monitor transition block ONCE (it is fully ground before
  grounding begins) into a shared conditional-effect table; every op
  grounded from a transition-carrying action references the shared block
  instead of owning a copy. Ops created after the gate (P3 synthetic ops,
  `TRAJ-END`) do NOT reference it — the re-observation of a frozen state
  by phase-toggling ops must stay impossible by construction, exactly as
  today.
- Collapse the resident copies regardless of sharing: drop `raws` after
  interning, stop cloning `cond_atoms` for complement toggling where the
  shared block makes it redundant, move rather than clone into the CSR.
- Consumers that iterate `task.cond.slice(oi)` (apply, heuristic
  cond_ops, reachability, achiever buckets) learn the shared block behind
  the same iteration surface — behaviorally invisible, asserted by the
  byte-identical suite.

**Acceptance:** storage qualpref p07/p08 ground inside the 15 GB box with
the monitor payload linear in (monitors + ops); whether they then SOLVE
within budget is measured and recorded either way (a grounding fix is not
oversold as a coverage fix — if search is the next wall, the board says
so and Phase 4 owns it); the full regression surface is byte-identical
(same plans, same metrics, same eval counts); grounding wall on the
existing green suite does not regress; t1≡t8.

**Hatch:** `FF_NO_COND_SHARE=1` restores per-op ownership.

**Touches:** `ground.rs`, `packed.rs`, `heuristic.rs` (iteration surface
only), the grounding fixtures, scoreboard.

**Risks:** the per-op-ownership assumption is pervasive — the change
rides behind the existing iteration interface or it does not ship;
per-precondition-disjunct RawOp fan-out must reference, never clone, the
shared suffix; a measured negative (copies don't dominate; sharing wins
too little) is an acceptable outcome recorded here with the profile
numbers.

---

## Phase 3 — ESPC on wide-monitor states: bounded, or not engaged

**Why:** storage qualpref p05/p06 complete only under a documented
`FF_NO_ESPC=1` row — the scoreboard's own text calls memory-bounding the
pass 0.8 work. The map found something better than a bound: the
engagement itself is an artifact. Both get fixed, cheapest first.

**Scope:**

- **Measure first:** confirm (FF_RES_DEBUG + a counter at the cap check)
  that storage qualpref's deadline pairs are monitor artifacts
  (TRAJ-named deliverables), and which structure trips 137 (retained
  nodes/visited inside `solve_under_penalties`' first pass vs the
  `cand_chunks` spike vs the un-pooled 1.5M EHC seed).
- **The engagement fix:** `build_deadline_guidance` stops emitting pairs
  whose deliverable is a reserved-namespace monitor fact — per-pair, not
  per-task, so a genuine achievement domain that also carries monitors
  keeps its real pairs. Monitor-artifact-only tasks then take the
  closure path on pure defaults — exactly the behavior the
  `FF_NO_ESPC=1` rows document as known-good. Openstacks (no TRAJ facts)
  is untouched by construction.
- **The bound:** a deterministic insertion cap in `search_from`
  (`max_nodes` alongside `max_eval`, checked at the existing cap point,
  returning the anytime incumbent with `capped:true` — which every
  caller's ladder already handles). The default derives from a
  documented byte model over static task dimensions (words, fluent
  widths) — never RSS, never wall clock — and is set so no currently
  green fixture trips it (instrumented and verified, not assumed). This
  bounds any future wide-state pass, not just this suite's.
- The `proven` flag stays honest: a capped pass can never report
  exhaustion.

**Acceptance:** storage qualpref p05 and p06 produce their plans and
metrics on PURE DEFAULTS (no env), metrics ≤ the current 47/90, and the
scoreboard rows lose their footnote; the simple-preferences ESPC locks
(openstacks 19.0 chain, storage 3.0) hold byte-identical; `tests/espc.rs`
determinism holds; t1≡t8 suite-wide; the measured attribution (which
structure blew, which fix retired it) is recorded here.

**Hatches:** `FF_ESPC_TRAJ_PAIRS=1` restores monitor-artifact engagement;
`FF_SEARCH_NODE_CAP` overrides the cap (0 disables).

**Touches:** `pddl3.rs` (`build_deadline_guidance`), `espc.rs`,
`search.rs` (cap), `features.rs` docs, `tests/espc.rs` style single-test
binaries for the env-sensitive cases, scoreboard.

**Risks:** the cap changes which plan is found *when it binds* — the
default must bind only beyond today's green envelope; if p05/p06's blow
turns out to be the un-pooled EHC seed, the seed inherits the same cap
(same discipline, same hatch).

---

## Phase 4 — Constraint-aware search guidance (the headline)

**Why:** the 0.7 NOT-list nominated this as the 0.8 headline "if the
Phase-2/5 ledger shows guidance-bound tails." It does: trucks p07/p08
produce nothing inside 600 s, four more instances need the doubled
budget, and the blindness is structural (delete relaxation cannot see
VIOL traps or pending obligations). The automata are already in every
state — the search just never reads them.

**Scope:**

- **The monitor table.** `constraints::compile` exports what `emit`
  currently discards: per monitor — operator kind, hard/soft, weight
  (for soft members, via the instance name), and the monitor fact names;
  the gate returns it alongside the rewritten pair (the four call sites
  thread it; `None` on constraint-free input) and it re-interns to fact
  ids post-ground. String re-derivation from `fact_names` is the
  documented fallback, not the design.
- **Hard-VIOL dead ends (soundness-backed, default-on candidate):** a
  set hard-monitor VIOL bit is a provable dead end (no transition
  deletes VIOL; the goal requires its absence — asserted against the
  table when built, so a future resettable monitor kind cannot silently
  break the invariant). Implemented h-side: `relaxed_to` returns None
  when a hard VIOL bit is set — one bit-scan per eval, covering EHC,
  best-first, and the init dead-end check uniformly.
- **Soft locked-loss term (measured, weight-gated):** a soft member's
  VIOL set means its instance's weight is permanently forfeit — the
  exact `SatGuidance` deadline "locked loss" shape. The term must be the
  *marginal* signal against the existing forgone-pref penalty (a
  violated instance already fails `PrefPhi`) — the design splits
  "not yet satisfied (recoverable)" from "locked (unrecoverable)" and
  prices only the lock-in delta. Built unconditionally, weight-gated to
  zero by default until measured (the res_weight/deadline_weight
  precedent keeps the default key bit-identical).
- **Obligation terms (measured, highest risk):** outstanding PEND bits
  (sometime-after obligations) and unSEEN sometime monitors as an
  additive distance term, optionally priced by a static per-monitor
  relaxed cost from init (the FF_PREF_SEED pattern). PEND *dips* — the
  0.6 BARRIER lesson applies verbatim — so this term rides only through
  the full measurement discipline: the five qualitative domains as the
  target, the entire simple-preferences 48 as the no-regression bar.
- Weights integer-calibrated against the documented key units; all terms
  serial, ordering-only (the hard-VIOL prune is the one legality change,
  and it rests on a machine-checked monotonicity invariant).
- Ship shape per the graduation template: measured wins graduate
  default-on with `FF_NO_*` hatches and dated re-locks of every affected
  ceiling; anything that doesn't win stays opt-in or is recorded here as
  a dead end with its numbers.

**Acceptance (the gate):** trucks qualpref p07 or p08 produce a
plan+metric inside the 600 s budget at defaults, or a measured negative
is recorded with the eval/expansion evidence; at least one of the
600-s-tail instances (openstacks p06–p08, storage p05/p06 t1, trucks p06
t1) moves inside the default budget; NO regression on any current
scoreboard row or heavy lock under both quality conventions; t1≡t8 on
everything; reported == verified everywhere (the oracle is
guidance-blind by construction).

**Hatches:** `FF_NO_VIOL_DEADEND=1`; weight knobs for the soft/obligation
terms (`FF_MONITOR_LOCK_W`, `FF_MONITOR_PEND_W`) defaulting to the
measured graduation outcome.

**Touches:** `constraints.rs` (table export), `api.rs`/`planner.rs`
(gate signature), `heuristic.rs`, `search.rs` (`SatGuidance`),
`pddl3.rs` (builders), `features.rs`, tests + heavy locks, scoreboards.

**Risks:** highest measurement content on the shipping path; every
default heap-key change re-derives ceilings (dated re-lock chain, the
house rule: quality ceilings may move with justification, coverage may
not regress); hard-VIOL pruning helps user-authored hard sets, not the
all-soft qualitative suite — the honest framing is that the suite's
tails are carried by the soft/obligation terms or not at all.

---

## Phase 5a — Constraints on the temporal path (gated; inherited 0.7 Phase 3)

The fence at constraints.rs:490-497 comes down for the untimed operators
if and only if the full 0.7 Phase-3a design lands with its oracle:
transitions injected into every `DurativeAction.effects` (both
time-specs, so every happening observes — the per-happening analogue of
per-action observation), the TIL appliers synthesized inside
`temporal::compile` gain the transitions through an explicit compile-side
hook (they are created after the gate runs; unhooked, exogenous flips are
invisible — the silent-ignore the contract forbids), and
`temporal::validate` grows the constraint fold at every happening — the
verifier-side counterpart ships WITH enforcement, 0.7 Phase-1 style,
including a decision recorded here for same-epoch fold granularity.
Dependencies and stances: Phase 1 first (the temporal goal test consumes
the literal `goal_pos` the END construction produces; `TRAJ-END` rides
the temporal path as an ordinary classical happening); the decomposer
forces monolithic when monitors exist (monitor state crosses contract
seams unchecked — validate-at-the-end is not an enforcement story);
`tsched` reordering stays safe solely through its validate gate, measured
for schedule-rejection cost. The timed operators (3b: `within`,
`hold-during`, `hold-after`) stay rejected unless their three named walls
fall inside the phase budget — the time-less visited key
(`tkey` excludes `TNode.time`), the later-only STN with no deadline
edges, and duration-range divergence between search and validator —
each a recorded reason if 3b ships rejected again. `always-within`
stays rejected regardless (unchanged 0.7 verdict).

**Acceptance if shipped:** per-operator durative bite/no-bite pairs with
temporal-validate agreement; constraint-free temporal locks byte-identical;
the gate rejection narrows by name. **If not:** this section records
precisely which wall held, and the rejection message keeps naming it.

**Hatch:** rejections are the default until the gate is met;
`FF_CONSTRAINTS_REJECT=1` covers whatever lands.

---

## Phase 5b — Temporal selection (gated; inherited 0.6→0.7 Phase 4)

The trucks draw's third appearance, with a cheaper first move than the
relation: **measure the value-ordering ceiling first** — the 0.6 dead end
was recorded with the DFS demanding earliest (maximal-weight) slots;
whether latest-feasible slot preference alone moves p06/p08 is a one-line
probe that bounds what the full EDF relation can win before it is built.
If the probe justifies it: the schedule-feasibility relation prunes joint
selections whose selected slots admit no consistent ordering
(deterministic prefix check over the slot lattice — the only trivially
admissible bound is max-of-individual-earliest, and the `proven` flag's
soundness gates how sharp the relation may be: a sharper bound decouples
from `bound_out` rather than corrupting the round-0 admissible record).
Slot variables and the clock are synthesized from what the model can
actually see (windows are extensional in the collect-op DNFs; the
`time-now` group is synthesizable; earliest-achievement bounds need a
clock-projected relaxed layer count that does not exist today — that
gap is the phase's real cost).

**Acceptance (the 0.6/0.7 gate, unchanged):** trucks p06 1→0 or p08
10→≤6 on the simple-preferences board, no won instance regresses, t1≡t8
— or a third, final dead-end record stating what the relation changed
and did not, closing this line of attack. Secondary, not gating:
measured effect on qualitative trucks (whose `sometime-before` edges the
compilation currently erases before selection can see them — restoring
them is new plumbing, recorded as out of scope unless 5b's primary gate
opens cheaply).

**Hatches:** `FF_PREF_NO_SELECT` (existing), `FF_PREF_NO_SCHED` (reserved
since 0.7) if the relation ships as default.

---

## Phase 6 — Measure everything, ship 0.8.0

- Defaults-only sweeps: qualitative 5×8 at {1,8} threads (the two
  ex-`FF_NO_ESPC` rows and any newly-covered p07/p08 rows on pure
  defaults); simple-preferences 48 re-run for the no-regression claim
  under both conventions; full classical/ADL/numeric/temporal regression;
  the constraint-free byte-identical claim re-proven (the Phase 1/2
  passes must be measured no-ops without constraints, not assumed).
- Every hatch verified to restore prior behavior byte-for-byte
  (`FF_NO_TRAJ_END`, `FF_NO_COND_SHARE`, `FF_ESPC_TRAJ_PAIRS`,
  `FF_SEARCH_NODE_CAP=0`, `FF_NO_VIOL_DEADEND`, weight knobs at 0).
- Every gated phase's outcome — shipped or dead end — recorded in this
  document's status header, 0.6/0.7 style; the scoreboard's "two scaling
  findings" section rewritten to name which walls fell and which moved.
- Heavy-lock hygiene: new locks for everything newly covered (p05/p06 on
  defaults; p07/p08 as grounding-count assertions if a full solve cannot
  fit CI's ~16 GB runners — a documented first for the suite); dated
  re-locks for any guidance-moved ceiling; CI heavy-tier wall budget
  re-checked.
- Surface honesty: `Solution.notes` and README Limitations rewritten to
  the new enforced/bounded/rejected split; CHANGELOG tells the
  pay-the-costs story; release mechanics per `RELEASING.md` (workspace
  version, both dep pins, py lockfile, publish library-then-CLI).

**Acceptance:** all suite locks green; reported == verified across every
plan-producing benchmark; t1≡t8 suite-wide; `FF_*` hatch matrix verified;
CHANGELOG/README/book consistent with the measured ledger.

---

## Version story: why 0.8.0

A minor bump, for the 0.4–0.7 reason: the default behavioral surface
changes. The default compilation emits a different (equivalent) task
shape; inputs that OOM'd now ground and solve; ESPC's engagement
narrows on monitor tasks; guidance defaults may move plan shapes within
equal-validity bounds. Every change keeps a restore hatch. Not 1.0: the
timed temporal operators are still expected to end 0.8 rejected, and the
scoreboard still self-scores against an unreachable reference archive.

---

## Explicitly NOT in 0.8

- **Session + constraints.** The bespoke rejection stands. The staleness
  argument is unchanged by the END construction: `Session` grounds once
  and replans from mutated states, and a compiled monitor's baked-S_0
  bits would be stale from the second `step` on. Lifting it needs
  monitor re-seeding from arbitrary states — designable, not scheduled.
- **Step-index classical semantics for the timed operators** — same
  verdict, third release running: a "valid" verdict that changes meaning
  between modes violates the never-silently-ignore contract. Phase 5a's
  temporal reference semantics remain the prerequisite.
- **Visited-key hashing** (the B6 option from the ESPC map): a 128-bit
  key collision silently merges distinct states — rejected on the
  exactness culture, recorded here so it is not re-derived.
- **The IPC-5 hard-`constraints` sets as a scored suite** — the hard
  fixtures remain grounding-cost instruments; a scored suite still has
  no reference ledger.
- **REACH-GOAL synthesis for genuinely disjunctive USER goals** — stays
  exactly as is; Phase 1 removes *monitor* acceptance from the goal, not
  the general mechanism. (The latent verify quirk for disjunctive user
  hard goals — the oracle's own grounding collapses them to a
  GOAL-REACHED fact no replayed plan sets — is recorded as a known
  pre-existing issue, fixed only if a real input hits it.)
- **VAL as authority; continuous `#t` effects; dynamic derived
  predicates; general LTL** — all unchanged from the 0.7 list.

---

## The 0.8 story

> 0.7 moved the fence and wrote down the bill: goals exponential in
> monitors, a monitor tax multiplied across every ground action, a
> penalty pass that drowned in the states it widened, a heuristic blind
> to the automata it carried. **0.8 pays the costs.** The acceptance
> checks ride one terminal action instead of an exponential goal; the
> monitor block is grounded once and shared; the pass that couldn't be
> budgeted is bounded or not engaged; and the search finally reads the
> monitor bits it has been dragging through every state — dead ends
> pruned by proof, locked losses priced, obligations counted. The
> temporal debts come up for payment too, gated as ever: measured win or
> written dead end. Same plan on any machine at any thread count; every
> default keeps a way back; reported == verified, still, on everything.
