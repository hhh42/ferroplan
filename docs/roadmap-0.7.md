# Roadmap — the road to v0.7 ("Trajectories")

> **Status: Phases 1–2 executed** (hard untimed constraints enforced, soft
> constraint-preferences priced, qualitative suite vendored + scored — see
> the *Recorded* blocks in each phase). Successor to the executed
> [0.6 roadmap](roadmap-0.6.md). Ground truth: the 0.4.1 rejection gate
> (`pddl3::unsupported_constraints`, called from all five entrypoints), the
> [`benchmarks/ipc5-scoreboard.md`](../benchmarks/ipc5-scoreboard.md) ledger
> 0.6 shipped, the 0.6 Phase-4 verdict (the trucks draw needs TEMPORAL
> selection — that work moves here), and a fresh survey of the IPC-2006
> qualitative-preferences track on the potassco mirror. One negative result
> is already recorded by that survey: **pathways has no qualitative
> formulation** — the qualitative track is five domains (openstacks, rovers,
> storage, tpp, trucks), not the six the simple-preferences suite has.

0.4.1 made PDDL3 trajectory constraints a clean *rejection*: every operator
parses into the AST, and any non-empty `(:constraints ...)` block is refused
with an explanation at every entrypoint — the "never silently ignore"
contract. 0.5 and 0.6 then banked the simple-preferences quality story on
top of that fence. **0.7 has one headline: enforce trajectory constraints**
— the qualitative-preferences track promised in the 0.5 and 0.6 NOT-lists —
via the standard monitor-automaton compilation (Gerevini & Long / Edelkamp):
each constraint instance becomes a small automaton over state trajectories,
synchronized by conditional effects on every action, with acceptance folded
into the goal (hard constraints) or into preference facts priced by the
existing metric stack (soft constraints). The fence is narrowed
operator-by-operator, never deleted: anything 0.7 does not enforce keeps
the 0.4.1 rejection, now naming the specific operator.

Two contracts are sacred throughout: **reported == verified** (verify.rs is
the sole conformance oracle — there is no VAL fallback for PDDL3, and the
repo's standing stance is VAL-as-advisory anyway), and **determinism** (same
problem, same plan, any thread count; every default change keeps a restore
hatch; negative results get recorded in this document).

---

## The ground 0.7 starts from

- **Parser: complete.** All thirteen `Constraint` variants parse
  (`parse_constraint`, parser.rs): `And`/`Forall`/`Pref` wrappers plus
  `always`, `sometime`, `at-most-once`, `sometime-after`, `sometime-before`,
  `at end`, `within`, `always-within`, `hold-during`, `hold-after`. Bodies
  are ordinary `Formula`s; the time bounds are raw `f64` with no
  interpretation attached yet. The AST is permissive — non-BNF nestings
  (pref-in-pref, pref inside a modal body) parse; validation is deferred.
- **Enforcement: none.** `pddl3::unsupported_constraints` is presence-only
  (any non-empty constraints vector → one catch-all message) and is called
  from five gates: `api::solve`, `api::decompose`, `Session::new`,
  `planner::run_planner`, `planner::run_ff`. It does not distinguish hard
  from soft, and `pref_weights`/`preferences` enumerate goal preferences
  only — constraint-preference names are invisible to the metric machinery.
- **Benchmarks: zero.** No `.pddl` file in the repo contains a
  `(:constraints` block. The vendored `benchmarks/ipc/pref/` suite is the
  *simple*-preferences track. The qualitative track
  (`<domain>-preferences-qualitative` on potassco/pddl-instances, the same
  mirror ATTRIBUTION.md already cites) is confirmed fetchable via raw
  URLs and uses exactly four modal operators in the wild: `always`,
  `sometime`, `at-most-once`, `sometime-before` — all soft
  (preference-wrapped, forall-quantified, scored by `(is-violated ...)`
  metric weights). The timed operators appear only in the temporal
  constraint sets.
- **Oracle: verify.rs replays every intermediate state already** but keeps
  only the current one, checks the goal in the final state only, and
  re-derives preferences from `problem.goal` only. `validate_plan` maps
  `hard_goal_met → Valid` — today a plan violating `(always ...)` would
  print "Plan valid". That is the first thing 0.7 must fix.
- **Engine assets that carry over unchanged:** the exact-closure metric
  optimizer + anytime/ladder B&B, the ESPC penalty loop, the selection
  layer (`selection.rs`), the Keyder–Geffner collect/forgo compilation, the
  snap-action temporal compiler, and grounder/heuristic support for
  conditional effects (`Effect::When` → `CondEff`, relaxed-reachability and
  RPG back-chaining through conditional adds, negative when-conditions
  checked natively at apply time).

---

## The compilation (one design, shared by every phase)

Each ground constraint instance *i* (after `Forall` expansion, exactly the
`combos`/`subst_formula` machinery goal-forall preferences already use)
becomes a few fresh 0-ary **monitor facts** plus `Effect::When` transitions
appended to every action's effect — the pattern `pddl3::compile` already
uses. No synthetic actions are needed on the classical path; monitors ride
the real operators.

**The observation offset is the load-bearing design note.** `apply`
evaluates conditional-effect conditions against the SOURCE state, so a
monitor riding action a_k observes S_{k−1}. The trajectory S_0..S_n is
therefore covered three ways: **S_0** by compile-time evaluation against
init; **S_0..S_{n−1}** by the per-action `When` conditions; **S_n** by a
goal-side formula. For `sometime-before` the one-step lag exactly
implements "strictly earlier" for free. All transition conditions are kept
mutually exclusive per fact so the add-wins conflict rule can never co-fire
a set and a clear of the same monitor bit.

| operator | monitor bits | per-action `When`s (read source state) | goal-side formula |
|---|---|---|---|
| `always φ` | VIOL-i | ¬φ → +VIOL-i | ¬VIOL-i ∧ φ |
| `sometime φ` | SEEN-i | φ → +SEEN-i | SEEN-i ∨ φ |
| `at-most-once φ` | HOLD-i, SEEN-i, VIOL-i | φ → +HOLD; ¬φ → −HOLD; φ∧¬HOLD → +SEEN; φ∧¬HOLD∧SEEN → +VIOL | ¬VIOL-i ∧ ¬(φ ∧ ¬HOLD-i ∧ SEEN-i) |
| `sometime-after φ ψ` | PEND-i | φ∧¬ψ → +PEND; ψ → −PEND | ψ ∨ (¬PEND-i ∧ ¬φ) |
| `sometime-before φ ψ` | SAFE-i, VIOL-i | ψ → +SAFE; φ∧¬SAFE → +VIOL | ¬VIOL-i ∧ (¬φ ∨ SAFE-i); init: φ(S_0) → VIOL |
| `(at end φ)` | — | — | φ as a plain goal conjunct |
| `within / always-within / hold-during / hold-after` | — | **not compilable to state monitors** — they need the search clock | Phase 3 (temporal path) or rejection |

- **Hard** constraint: the goal-side formula is conjoined into
  `problem.goal` (negative literals and disjuncts route through the
  existing NOT-fact and disjunctive-goal compilation in the grounder).
- **Soft** (`Constraint::Pref`) constraint: the SAME monitor is built, but
  the goal-side formula is wrapped as `(preference name ...)` in the goal —
  `split_goal` then turns it into an ordinary Keyder–Geffner preference and
  the entire 0.5/0.6 metric stack (closure B&B, anytime ladder, ESPC,
  selection) applies with zero new optimizer code.
- **Naming discipline:** monitor facts must NOT use the `P3` prefix — the
  PDDL3-path filters treat `P3*` facts as synthetic, and hard monitor goals
  must count as *real* goals for the closure layer. Instance indices are
  assigned deterministically (lexicographic by preference name, then
  binding tuple) so grounding is stable across thread counts.
- `always` could alternatively be enforced by search-level pruning; the
  compilation is the default because it flows through all three modes and
  the verifier identically. If Phase-1 measurement shows the ¬φ-monitor DNF
  cost binding on real instances, pruning is the recorded fallback lever —
  measured, not assumed.

The pass — `compile_constraints(&Domain, &Problem)` in a new
`constraints.rs` — runs immediately after `derived::compile` and BEFORE
mode routing / `pddl3::compile` / grounding, replacing the blanket
rejection at each gate. It consumes what it can enforce, clears those
entries, and returns a **selective** rejection for everything else. On an
empty constraints vector it is a provable no-op: the entire existing
regression surface must stay byte-identical.

---

## Phases

```
Phase 1: hard untimed constraints ──► Phase 2: soft constraints + the ──► Phase 5: measure
         (classical path, verifier,             qualitative suite               everything,
          the contract rewrite)                 (the headline ledger)           ship 0.7.0
                                                        │                          ▲
                      Phase 3: temporal path ◄──────────┤ (gated)                  │
                      Phase 4: temporal selection ◄─────┘ (gated, from 0.6) ───────┘
```

Ordering rationale: Phase 1 builds the compilation and — critically — the
oracle, on the path where semantics are simplest and every test is
hand-checkable. Phase 2 is the headline (the wild qualitative corpus is
soft-only, four operators, all untimed). Phases 3 and 4 are gated stretch
work in the 0.5/0.6 tradition: measured win or documented dead end, neither
on the minimum shipping path.

---

## Phase 1 — Hard untimed constraints on the classical path

**Why:** the six untimed operators (`always`, `sometime`, `at-most-once`,
`sometime-after`, `sometime-before`, `at end`) compile to pure
state-transition monitors — no clock, no new search machinery — and the
grounder/heuristic already handle everything the compilation emits.
Building the verifier extension FIRST (in the same phase, same PR series)
is non-negotiable: verify.rs is the only oracle, and enforcement without an
independent trajectory check would be self-grading.

**Scope:**
- `constraints.rs`: forall expansion (reusing `combos`/`subst_formula` +
  `peval_static` static simplification — the storage-forall mitigation),
  well-formedness validation (non-BNF nestings become compile-time errors,
  not support), monitor emission per the table, init-state evaluation,
  goal conjunction. Wired at all five gates in place of the blanket call.
- `unsupported_constraints` becomes selective: it walks the tree and
  rejects only what this build cannot enforce — the four timed operators
  (both paths, Phase 1), any constraint on the temporal path (until
  Phase 3), soft constraints (until Phase 2) — with messages that name the
  operator. `run_ff` (no PDDL3 path) and `Session` (already rejects
  preferences) KEEP their rejections for anything they don't gain; tests
  pin all five gates explicitly so no entrypoint can silently diverge.
- verify.rs: fold constraint semantics over the replay incrementally
  (`always` = running AND, `sometime` = running OR, `at-most-once` =
  rising-edge count ≤ 1, `sometime-after`/`-before` = the obvious folds,
  `at end` = final state), against the ORIGINAL constraints — not the
  compiled monitors, so the oracle stays independent of the compilation.
  `Verified` grows a `constraints_met` verdict (per-constraint list);
  `validate_plan` requires it for `Valid`; plans whose problems carry
  timed operators are REJECTED by verify, never skipped.
- tests/constraints.rs rewritten to the new contract: per operator, a
  bite/no-bite pair of inline hand-checkable instances (e.g.
  `(always (not (on)))` + goal `(on)` → unsolvable; `(sometime (on))` from
  init `(off)` with goal `(off)` → forces flip-on;flip-off over the empty
  plan; `(at-most-once (on))` blocking a re-flip route;
  `(sometime-before ...)` with φ true in init → unsolvable), every solved
  plan cross-checked through `verify::verify`, `threads: 1` where order
  matters plus a t1≡t8 sweep. Both entrypoint styles exercised
  (`solve` and `run_planner` exit codes/messages), per house convention.
  A handful of IPC-5 hard-`constraints` instances (storage/trucks) ride as
  `#[ignore = "heavy"]` fixtures for grounding-cost measurement.
- Doc updates in the same change: types.rs / pddl3.rs / tests module
  headers stop saying "later phase".

**Acceptance:** every untimed operator has passing bite/no-bite tests with
verify agreement; an unsatisfiable hard constraint yields unsolvable, never
a wrong plan; all five gates pinned; rejection messages name operators; the
full existing regression surface (simple-pref 48, classical/ADL/numeric,
temporal locks) is **byte-identical** — the pass is a measured no-op on
constraint-free input; t1≡t8 on everything new; grounding cost on the hard
fixtures measured and recorded (conditional-effect count and wall time vs.
the unconstrained domain).

**Hatch:** `FF_CONSTRAINTS_REJECT=1` restores the 0.4.1 blanket rejection
byte-for-byte at every gate. Note the hatch restores *rejection*, not
ignoring — a hatch that silently dropped constraints would itself violate
the contract.

**Recorded (Phase 1 shipped).** Grounding cost on the hard-overlay fixtures
(`constraints::grounding_cost`, release build, `--ignored --nocapture`):

| fixture | monitors | ops (REACH-GOAL) | cond. effects | ground wall |
|---|---|---|---|---|
| trucks p03 unconstrained | 0 | 1,065 (0) | 0 | 8 ms |
| trucks p03 + `(forall (?t ?l) (at-most-once (at ?t ?l)))` | 3 | 1,083 (18) | 12,780 | ~50 ms |
| storage p05 unconstrained | 0 | 920 (0) | 0 | ~80 ms |
| storage p05 + `(forall (?h ?c) (at-most-once (lifting ?h ?c)))` | 10 | 59,969 (59,049) | 36,800 | ~1.2 s |

The storage blow-up is not conditional-effect volume — it's the predicted
**goal-DNF risk, now quantified**: each monitor's S_n acceptance check is a
goal conjunct (`at-most-once` contributes a 3-way disjunction,
`sometime`/`sometime-after`/`sometime-before` 2-way, `always` literals
only), and the grounder compiles a disjunctive goal into one synthetic
REACH-GOAL operator per DNF disjunct — **exponential in the monitor count**
(storage: 3^10 = 59,049 exactly; verified, not a pruning artifact —
`FF_NOREL` is a no-op here). Constraint-free inputs pay nothing (the gate
is a no-op), and Phase 2's qualitative-track constraints are all SOFT
(different machinery), so real exposure is user-authored hard constraint
sets with many `at-most-once`/`sometime*` instances. The known fix if it
bites: the standard END-action construction (move the S_n checks into a
forced-terminal collect action's transitions, leaving a literal-only goal)
— deferred because moving `problem.goal` into an action precondition
interacts with the goal-preference metric machinery; take it as a measured
increment, not a side effect.

**Touches:** `constraints.rs` (new), `pddl3.rs`, `api.rs`, `planner.rs`,
`session.rs`, `verify.rs`, `plan.rs`, `types.rs`, `tests/constraints.rs`.

**Risks:** DNF blowup of `When` conditions for quantified φ bodies (each
disjunct becomes a separate ground conditional effect — same profile as
preconditions; mitigated by static simplification, measured on the hard
fixtures); heuristic blindness — delete relaxation cannot see
VIOL-traps (`always` violations are negative information), so guidance on
always-heavy problems is expected weak in this phase; that is a *quality*
gap to record, not a soundness gap, and the constraint-aware-guidance lever
is explicitly deferred (see NOT-list).

---

## Phase 2 — Soft constraints + the qualitative-preferences suite

**Why:** this is the headline. Every `(:constraints ...)` block in the wild
qualitative corpus is preference-wrapped and untimed — Phase 1's monitors
plus a preference wrapper cover 100% of the operators the track actually
uses. And the pricing machinery already exists: `pref_weights`' metric-side
extraction already picks up `(is-violated p)` coefficients regardless of
where `p` lives — only the name *enumeration* is goal-only today.

**Scope:**
- A constraint-side analogue of `preferences()`: walk
  `And`/`Forall`/`Pref`, expand forall instances sharing one name (the
  instance-count semantics of `(is-violated name)`), and merge into the
  same weight lookup — the existing defaults (no metric → 1.0 per
  preference, metric-unreferenced → 0.0) fall out unchanged and get pinned
  by tests. Anonymous prefs (`Pref(None, ...)`) get a deterministic
  generated name, matching goal-preference handling.
- Lower `Pref(name, C)` to Phase-1 monitors + a `(preference name
  <goal-side formula>)` goal wrapper; `split_goal` → collect/forgo → the
  whole closure/ladder/ESPC/selection stack applies with no optimizer
  changes.
- verify.rs metric scoring extended to constraint preferences over the
  trajectory (the fold from Phase 1, weighted) — mandatory for
  reported==verified before any benchmark lands.
- Vendor the qualitative suite: 5 domains × 8 instances from
  `potassco/pddl-instances` `ipc-2006/domains/<d>-preferences-qualitative/`
  (openstacks, rovers, storage, tpp, trucks — pathways recorded as
  nonexistent), under `benchmarks/ipc/qualpref/`; ATTRIBUTION.md updated
  with exact paths, same licensing note as the existing suite.
- Scoreboard: `benchmarks/ipc5-qualitative-scoreboard.md` in the
  house format. Reference numbers: the official IPC-5 qualitative results
  for SGPlan5 if obtainable from the competition archive; failing that,
  SGPlan6 via the existing `compare.py` Docker path as advisory. If
  neither is obtainable, the scoreboard ships self-scored with the gap
  stated honestly — *scoring honestly is the gate; leading is not.*
- Heavy metric locks in the ipc5_pref_metric.rs style
  (`#[ignore = "heavy"]`, oracle cross-check per instance, dated re-lock
  comments).

**Acceptance:** all 40 qualitative instances parse + compile with no
rejection; coverage measured and recorded, every non-covered instance gets
a named reason; reported==verified exact on every produced plan (the
oracle test extends, not forks); weight-default semantics pinned; the
simple-preferences 48-instance ledger byte-identical on defaults; t1≡t8
across the new suite.

**Hatch:** `FF_CONSTRAINTS_REJECT=1` (same hatch — soft constraints are
inside the fence it restores).

**Touches:** `constraints.rs`, `pddl3.rs` (`pref_weights`/enumeration),
`verify.rs`, `benchmarks/ipc/qualpref/` (new), `benchmarks/ATTRIBUTION.md`,
`benchmarks/run.py`/`compare.py`, `tests/constraints.rs`, new heavy test
file, scoreboard.

**Risks:** quadratic forall-preference blowup (storage p3A quantifies over
crates × the exists-body — the same shape 0.5's `peval_static` pass made
tractable; re-measure, extend if binding); initial metric quality expected
to trail SGPlan5 where guidance is monitor-blind — record the tails, they
define the 0.8 heuristic agenda rather than blocking 0.7.

**Recorded (Phase 2 shipped, 2026-07-17).** The lowering worked exactly as
designed — zero optimizer changes; the constraints suite grew 18 → 23 tests
(weight defaults, anonymous naming, forall instance-count, mixed hard+soft,
hatch-covers-soft), all asserting reported == verified. Beyond the plan,
three things were forced by measurement:

- **The quantified-body gap was real and two-sided**: the qualitative
  bodies nest `exists`/`forall` inside modal operators, so
  `constraints::expand` grounds formula-level quantifiers (monitors stay
  ground for the grounder), and the storage p01 oracle mismatch (reported
  0 vs verified 6) exposed that the VERIFIER's best-effort quantifier
  evaluation was the wrong side — `verify` now grounds goal-preference
  bodies too, making the oracle exact on every qualitative domain AND
  tightening the simple-preferences oracle to exact on 5 of 6 domains.
- **The predicted quadratic-forall risk bound**: storage p03+ OOM'd a
  15 GB container until constraint-side static simplification (the
  `peval_static` extension this section predicted; `FF_PREF_NO_STATIC`
  hatch) — p03 drops 1,548 of 1,554 instances and solves at metric 60.
- **Two memory walls remain on the storage tail, both named on the
  board**: the ESPC monolithic tightening pass blows memory on
  wide-monitor states (p05/p06 ship with a documented `FF_NO_ESPC=1` row —
  memory-bounding ESPC is 0.8 work), and p07/p08 exceed memory in
  grounding itself (1,147+ survivors × ground actions — the Phase-1
  END-action construction is the recorded lever).

Coverage: **36/40 instances produce a plan+metric**, with reported ==
verified exact on all 11 oracle-checked instances (every gap named: 2 × memory, 2 × trucks search budget — the trucks tail is
the same shared-timeline draw 0.6 Phase 4 recorded, doubled down by
`sometime-before` ordering; Phase 4 here remains its gate). Scoreboard:
[`benchmarks/ipc5-qualitative-scoreboard.md`](../benchmarks/ipc5-qualitative-scoreboard.md)
— self-scored (the official archive is network-blocked from the dev
container; both reference graft-in paths documented). t1 ≡ t8 wherever both
complete; the largest instances are budget-bound at t1, never divergent.

---

## Phase 3 — The temporal path (gated)

Two sub-phases, independently gated; either may ship as still-rejected with
rationale recorded here — the 0.6 Phase-3/4 precedent that a measured
negative is an acceptable outcome.

**3a — untimed operators on temporal problems.** Mechanically close:
`temporal::compile` copies at-start/at-end effect trees verbatim into snap
actions, and the decision-epoch search applies every happening through the
same `apply` — so Phase-1 `When` monitors work IF the pass injects them
into both `domain.actions` and every `DurativeAction.effects` list, plus a
hook for the TIL-applier actions synthesized inside `temporal::compile`
(they are created after the pass runs, so exogenous-event observation needs
a compile-side hook, not just the front pass). Acceptance: per-operator
durative hand tests; the temporal benchmark locks hold byte-identical on
constraint-free input; verify grows a temporal replay check at every
happening. Until 3a lands, temporal + constraints keeps the rejection.

**3b — timed operators (`within`, `hold-during`, `hold-after`,
`always-within`).** These cannot be state-transition monitors at all — the
bound is against plan time, which only the search clock knows. The design
is a `SnapInfo`-style side table plus in-search checks against
`TNode.time`, and a timestamped trajectory fold in verify. `always-within`
(sometime-after with a deadline) is the hardest and is expected to stay
rejected even if the other three land. Acceptance if shipped: hand tests
where the deadline forces a schedule; verify agreement; temporal locks
hold. Acceptance if NOT shipped: the rejection message and this document
state precisely why, and the classical-path rejection for timed operators
is re-affirmed (see NOT-list for why the degenerate step-index reading is
deliberately excluded).

**Hatch:** rejections are the default until each sub-phase's gate is met;
`FF_CONSTRAINTS_REJECT=1` covers whatever lands.

**Touches:** `constraints.rs`, `temporal.rs`, `tresolve.rs`, `verify.rs`,
`tests/temporal.rs`, `benchmarks/temporal-results.md`.

---

## Phase 4 — Temporal selection: the trucks draw (gated, inherited from 0.6)

**Why:** 0.6 Phase 4 closed as a measured dead end *for selection-shaped
levers*: trucks' window preferences select delivery SLOTS competing for one
shared timeline — WHICH windows is a selection, HITTING them is ordering,
and the 0.6 model could not express the coupling. The qualitative trucks
domain doubles down on the same structure (`sometime-before` over
deliveries is literally an ordering constraint), so 0.7 is where this
machinery pays twice if it pays at all.

**Scope:** extend `selection.rs` with a **schedule-feasibility relation**:
a candidate assignment is admissible only if its selected slots admit a
consistent ordering (a deterministic precedence/deadline feasibility check
over the slot lattice — EDF-style test, no search). The B&B prunes
infeasible joint selections instead of discovering infeasibility by failed
planning attempts (the 0.6 failure mode: probes pass individually, joint
target infeasible). Feed feasible selections to the existing Phase-2-of-0.6
plan-to-selection loop; on the qualitative side, reuse the same relation to
seed ordering-constrained preference subsets.

**Acceptance (the 0.6 gate, unchanged):** trucks p06 1→0 or p08 10→≤6 on
the simple-preferences board, no won instance regresses, t1≡t8 — or a
documented dead end extending the 0.6 Phase-4 record with what the
feasibility relation did and did not change. Secondary: measured effect on
qualitative-trucks metrics.

**Hatch:** rides `FF_PREF_NO_SELECT=1` (restores the pre-selection path
entirely); a narrower `FF_PREF_NO_SCHED=1` disables only the feasibility
relation if it ships as a default.

**Touches:** `selection.rs`, `pddl3.rs`, scoreboards.

**Risk:** highest research content in the plan; explicitly off the shipping
path — 0.7.0 ships without it.

---

## Phase 5 — Measure everything, ship 0.7.0

**Scope:**
- Defaults-only sweeps: qualitative suite 5 domains × 8 instances × 3 runs
  × {1, 8} threads; simple-preferences 48 re-run for the byte-identical
  claim; full classical/ADL/numeric/temporal regression; perf harness
  compare (the constraint pass must be measured-zero on constraint-free
  input, not assumed-zero).
- Scoreboards finalized under both conventions where reference data exists;
  every gated phase's outcome — shipped or dead end — recorded in this
  document's status header, in the 0.6 style.
- Surface honesty: the CLI/JSON output states what was enforced ("N hard
  trajectory constraints enforced, M constraint preferences priced") in
  `Solution.notes`; the satisficing fallback keeps its never-claim-an-
  unoptimized-metric rule for constraint preferences too.
- README/book/CHANGELOG retell the story; Limitations section rewritten:
  the trajectory-constraints line moves from "rejected" to the precise
  enforced/rejected split. Release mechanics per `RELEASING.md`.

**Acceptance:** all suite locks green; reported==verified across every
plan-producing benchmark; both hatches (`FF_CONSTRAINTS_REJECT`,
`FF_PREF_NO_SCHED` if applicable) verified to restore prior behavior
byte-for-byte; t1≡t8 suite-wide.

---

## Version story: why 0.7.0

A minor bump, not 0.6.1, for the same reason 0.4.0/0.5.0/0.6.0 were minor:
the default behavioral surface changes. Inputs previously rejected now
solve; the public `Verified` struct grows a constraints verdict;
`--validate`'s meaning strengthens (a constraint-violating plan flips from
"Plan valid" to invalid — a *correctness* strengthening, but a visible
one). Every one of those changes has a restore hatch
(`FF_CONSTRAINTS_REJECT=1` recovers the full 0.4.1 fence). Not 1.0: the
timed operators may still be rejected, and the qualitative quality ledger
is expected to open with honest gaps.

---

## Explicitly NOT in 0.7

- **Step-index classical semantics for `within`/`hold-during`/
  `hold-after`/`always-within`.** PDDL3 does define t_i = i for sequential
  plans, so a step-bounded reading is *legitimate* — but shipping it means
  the same constraint text validates differently on the classical and
  temporal paths, and a "valid" verdict that silently changes meaning
  between modes is exactly what the never-silently-ignore contract exists
  to prevent. The rejection stays until the temporal implementation
  (Phase 3b) exists to define the reference semantics; a step-index
  opt-in is 0.8 material at most.
- **Constraint-aware search guidance.** Delete relaxation is blind to
  ALWAYS-violation traps and monitor negative information; a
  SatGuidance-style monitor penalty or automaton-distance heuristic is the
  obvious next lever — but the discipline is compile-and-measure first.
  It rides only if the Phase-2/5 ledger shows guidance-bound tails, and
  then as the 0.8 headline candidate.
- **The IPC-5 hard-`constraints` benchmark sets as a scored suite**
  (pipesworld/storage/tpp/trucks hard variants). A handful of instances
  ride as Phase-1 grounding fixtures; vendoring and scoring the full sets
  is separate work with no reference ledger.
- **Non-BNF constraint nestings** (preference-inside-preference,
  preference inside a modal body). The parser is permissive by design;
  `compile_constraints` validates and rejects these with a named error —
  that is a bug fence, not a feature gap.
- **VAL as authority.** verify.rs is the oracle. VAL's PDDL3 checking may
  be wired into a bench harness as *advisory* cross-validation in the
  bench_temporal.py style, unvendored, but no verdict ever depends on it
  (the repo's recorded stance: VAL rejects valid ferroplan temporal plans).
- **pathways-qualitative.** Recorded negative: it does not exist in the
  IPC-2006 corpus — the track is five domains. No amount of vendoring
  effort changes that; the scoreboard says 5, not 6, on purpose.
- **Anything beyond the thirteen parsed variants** (PDDL3.1 action costs
  interactions, general LTL, dynamic derived predicates, continuous
  effects) — all tracked in README Limitations, none of them move this
  release.

---

## The 0.7 story

> 0.4 built the fence: trajectory constraints were refused loudly instead
> of ignored silently. 0.5 and 0.6 won on what was inside the fence. **0.7
> moves the fence**: the untimed PDDL3 modalities compile to monitor
> automata that ride every action, hard constraints become goals the
> existing search must honor, soft constraints become preferences the
> existing metric stack already knows how to price — and the independent
> verifier learns to check every claim over the whole trajectory before
> any of it is called done. What 0.7 cannot enforce, it still refuses, by
> name. Same plan on any machine at any thread count; every new default
> has a way back; every gated bet ends in a number or a written dead end.
