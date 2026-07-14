# Roadmap — the road to v0.6 ("Selection")

> Successor to the executed [0.5 roadmap](roadmap-0.5.md). Ground truth:
> [`benchmarks/ipc5-scoreboard.md`](../benchmarks/ipc5-scoreboard.md) (post-
> 0.5.1 barrier flip) and the forensics that set this direction:
> [`forensics-tpp.md`](forensics-tpp.md).

0.5 closed on first place: three domain leads under both quality conventions
(openstacks, storage — now an 8/8 sweep — and rovers), trucks ahead on totals,
19W/15T/14L across the suite. The forensics then identified what the remaining
tpp/pathways tail *is*: on zero-action-cost domains, plan quality is decided
entirely by **which jointly-satisfiable preference subset the end state lands
in** — a small combinatorial selection problem that h-guided search
structurally cannot coordinate (SGPlan5's tpp p05 79 is the closed-form
selection optimum; our residual 89 forgoes coordinated choices like
goods5@L2-so-goods6-can-match that no open-list ordering can express).

**0.6 has one headline: solve the selection exactly, then plan to it.**

## The ledger 0.6 starts from (defaults, post-barrier)

| domain | W/T/L vs SGPlan5 | totals | status |
|---|---|---|---|
| openstacks | 5/0/3 | 271 vs 326 | led (both) |
| storage | **8/0/0** | 234 vs 547 | led (both, sweep) |
| rovers | 4/2/2 | 5301.6 vs 5632.5 | led (both) |
| trucks | 2/4/2 | 23 vs 31 | totals only |
| tpp | 0/4/4 | 536* vs 489 | SGPlan5 |
| pathways | 0/5/3 | 60.2 vs 47.4 | SGPlan5 |

*post-barrier tail 89/104/110/129.
First place under both conventions needs **one more domain** — tpp, pathways,
or the trucks instance draw broken (p06 1→0 or p08 10→≤6).

## Phase 1 — the selection solver (`selection.rs`)

Model, built from what compile() already produces:

- **Variables**: every mutex group (invariant synthesis) touched by a
  preference phi, with domain = {each relaxed-REACHABLE member fact, ⊥}.
  Facts outside any group are boolean variables.
- **Preferences**: satisfied iff SOME DNF disjunct (the collect-op precond
  facts, exactly what `SatGuidance`/`compose_pref_seed` already extract) has
  all its facts chosen.
- **Objective**: minimize violated weight. **Exact DFS branch-and-bound**
  (deterministic node cap; most-constrained-group-first ordering; greedy
  descent fallback on cap). The spaces are tiny where it matters (tpp p05:
  5^7) but storage p08 has thousands of instances post-simplification — the
  cap and fallback are load-bearing, not decoration.
- Output: the chosen fact set + the **optimistic bound** (per-variable
  reachability is admissible: the true optimum can never beat it).

**Acceptance**: on tpp p05 the solver returns bound = 79 with the assignment
the forensics derived, in negligible time.

## Phase 2 — selection as the closure loop's target

- Plan to the selection as HARD goals: one P3-masked, sat-guided bounded
  search from init (the `compose_pref_seed` skeleton — stages or monolithic,
  whichever measures), close with the exact tail, take the min with the
  existing incumbent, and run the normal anytime+ladder tightening from that
  bound.
- **Repair**: joint reachability is NOT guaranteed by per-variable
  reachability (storage: you cannot clear every cell — crates must sit
  somewhere). On a failed/partial attempt, drop the least-valuable selected
  preference and re-solve the selection (bounded retries), then fall back to
  the current path entirely. The selection layer must never cost more than a
  bounded slice of `FF_PREF_EVAL_BUDGET`.
- **Proven optimality for free**: when the final metric equals the
  selection bound, the result is optimal — report `proven` (tpp p05 at 79
  would be *provably* optimal, something the B&B alone could never certify
  within budget).
- Hatch: `FF_PREF_NO_SELECT=1` restores 0.5.1 behavior.

**Acceptance**: tpp p05 89 → 79 (proven); tpp/pathways tails move; the
storage sweep and rovers/openstacks/trucks rows hold bit-for-bit or better;
full 48-instance sweep before any default flip; t1≡t8.

## Phase 3 — weight-aware barrier (recover pathways p05)

The 0.5.1 blunt barrier's one casualty: pathways p05, 6 → 6.5 (win → tie).
Try including init-satisfied preferences at reduced guidance weight (half, or
thresholded by weight rank) so cheap dip-freedom returns without re-blinding
the search to 16-weight traps. Measure p05 AND the storage sweep AND the tpp
tail for give-back; keep only a strict improvement, else document.

## Phase 4 — trucks instance draw (probe, gated)

trucks p06 (1 vs 0) / p08 (10 vs 6) are time-window preferences over
delivery ordering — selection-flavored but temporal: WHICH windows to hit is
a selection; HITTING them is ordering. Probe whether the Phase-2 machinery
(windows as selected hard goals) moves either instance; gate like every
stretch item — measured win or documented dead end.

## Phase 5 — measure everything, ship 0.6.0

Defaults-only 48-instance sweep, both-conventions ledger, scoreboard/README/
CHANGELOG, release prep. The first-place claim ships ONLY if ≥4 of 6 domains
are led under both conventions; otherwise 0.6 ships its honest ledger like
0.5 did.

## Explicitly NOT in 0.6

- PDDL3 trajectory-constraint ENFORCEMENT (the qualitative-preferences
  track) — still the natural 0.7+ headline, unchanged from the 0.5 list.
- General profiling/latency work — decoupled from the quality ledger
  (measured: the tails were never budget-bound).
