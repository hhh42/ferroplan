# ferroplan 0.15 roadmap — the research cycle opens

Scope settled 2026-07-22, as 0.14.0 cuts. The game track's arc is
COMPLETE: 0.12 gave one mind bounded thinks, 0.13 made minds a
population, 0.14 made the population live together — loop, contention,
schedules, in-flight intervals, all measured and shipped. The records
have said for three cycles that the fenced research work waits "for
when the corpus matters again"; with the game track at a natural rest,
that time is now. 0.15 spends its risk budget on the two named
research targets, pays two CORRECTNESS debts found in the records, and
publishes the bindings that have waited since 0.5.

The full open-thread harvest behind this scope (141 items swept from
every cycle record, audited for staleness) informed what follows; the
discipline is unchanged: measured win or recorded negative, fixtures
first, scoreboards defend themselves, research phases severable.

## Phase 1 — correctness debts (small, first, non-negotiable)

Two diagnosed soundness bugs from the records, each with a named fix:

- **ε-separation misses conditional/numeric mutexes**: the mutex model
  covers unconditional add/del/pre only — openstacks-time i2 produced
  a VAL-INVALID plan on the record. Extend the footprint to
  conditional effects and numeric read/write sets (the 0.10 numeric
  write-write fix pointed the way).
- **The temporal printer under-separates ε at same-timestamp
  produce-at-end / consume-at-start boundaries** (rpg-world fixtures
  flagged by selfcheck.py; logged in perf-notes, never applied).
- Bar: both repros VAL-green; full temporal corpus re-validated (the
  388 baseline must not lose a VAL column); suite pins both.

## Phase 2 — object-symmetry orbits (research target #1)

The 0.13 diagnosis practically wrote the spec: temporal-machine-shop
is 0/20 because interchangeable pieces — distinguished ONLY by which
`(baked-structure p q)` goal pair they serve — make every
subset-assignment of "which identical piece is baking" a distinct
visited state. parc-printer-t (18/30 + 7/20) is the same family.

- Fixtures first: an orbit-detection probe (objects with identical
  init profiles modulo renaming and symmetric goal treatment) run
  across the temporal corpus — MEASURE how much symmetry exists per
  domain before building the reducer.
- Lever: canonicalize the visited key under goal-respecting object
  permutations (orbit-canonical renaming at key time — the state
  itself and the plans stay concrete). Determinism preserved: the
  canonical form is a function of the state, t1 ≡ t8 holds.
- Bar: TMS off 0/20 or the negative recorded with mechanism precision;
  parc-printer-t is the second witness; zero corpus regressions (the
  scoreboard defends itself — parking #16 taught us to name every
  casualty and check it solo).
- Severable: ships behind a default gate only if the sweep is clean.

## Phase 3 — the different heuristic, first rung (research target #2)

Three 0.11 guidance transfers measured negative with the standing
conclusion: the walls (storage, transport11, model-train,
turn-and-open, the elevator-11 tail, floor-tile/visit-all quality)
need a GENUINELY different heuristic — red-black planning or semantic
landmarks over numeric structure. The fence comes down for ONE
scoped rung, not a rewrite:

- Candidate, cheapest first: numeric/resource semantic landmarks
  (transport-style "the truck must return at least ⌈demand/capacity⌉
  times" lower bounds folded as a landmark-count term), measured on
  transport11 + storage first — they are the purest diagnosed
  guidance walls. Red-black stays second unless the landmark rung
  measures dead.
- The 0.11 discipline verbatim: any ONE wall domain moves off zero at
  the standard budget, or the negative gets the same precision as the
  three before it (`FF_*` hatch either way; defaults bit-identical
  unless the corpus sweep is clean).
- Deliberately capped: one rung, one cycle. This phase is severable
  and its failure is a legitimate, recordable outcome.

## Phase 4 — corpus and platform debts within reach

- **transport-sequential-satisficing-2011 is the ONLY 0/N classical
  variant** (0/20 at 60 s) — diagnose whether it is the transport11
  guidance wall verbatim or something cheaper (openstacks-adl-11 6/30
  and floor-tile 5/20 ride along as second looks).
- **Publish the bindings**: ferroplan-py has been "pip-installable"
  and unpublished since 0.5 — version it with the workspace, build
  the wheel (maturin), and stage a PyPI release for the user to
  publish alongside crates.io; flip ferroplan-mcp's `publish = false`
  and add it to the publish set (RELEASING.md gains both steps).
- **The mdBook is stale against 0.10+** (still documents ?duration
  effects as unsupported and the June-era temporal table) — refresh
  it against the current record.
- Housekeeping fold-ins: single-source the version pins RELEASING.md
  complains about if cheap; vendored-corpus tidbits as encountered.

## Phase 5 — temporal follow-biased rethinks (the severable keeper)

The recorded 0.13 limit — `replan_following` delegates temporal
sessions to a plain rethink because a timed prefix ends mid-interval,
"not the at-rest state a session may stand in" — was UNLOCKED by 0.14:
sessions now hold in-flight intervals natively (`apply_start`, carried
root agendas). The bias becomes implementable: replay the surviving
prefix's happenings into an in-flight state, search only the tail.

- Measured against the same churn metric as 0.13 Phase 4, on a
  temporal drift fixture; the claims+following combination — recorded
  as never actually exercised — gets a fixture where breaks DO happen.
- Honest exit pre-authorized: if the prefix-replay semantics don't
  close cleanly over concurrent intervals, record why with a worked
  example and keep the delegation.

## Phase 6 — 0.15.0 release mechanics

CHANGELOG `[0.15.0]`, workspace bump + cli/mcp pins, README refresh,
`rustup update stable` first, the FULL pre-flight per RELEASING.md
(now including the wheel build and the mcp publish step if Phase 4
lands them), scoreboards refreshed where phases moved them —
binary-A/B attribution for any wall-clock delta, solo re-runs for any
30 s-edge casualty. **Finish in main** (CLAUDE.md): the cycle is done
when `main` holds it, publish.sh-ready.

## Deferred, on the record

- **Cross-mind planning** (negotiating/cooperating NPCs): still "a
  different engine and a different year."
- **Partial observability / belief state**: still the game's business.
- **Session PDDL3 constraints** and the four timed modal operators
  (`within`, `hold-during`, `hold-after`, `always-within`): still
  rejected by name; the 0.7/0.8-era gated designs remain on file.
- **Continuous `#t` effects and dynamic derived predicates**: still
  out, tracked in README limitations.
- **Fixpoint/stratified grounding unification**: still blocked on the
  interning-order tie-break lottery (sokoban-t 4/10 → 1/10); waits on
  tie-break-robustness work, where parking #16 also folds in.
- **The in-page Session UI** for the browser bazaar: still the
  recorded Phase 4 (0.14) follow-up, needs an environment where it
  can be visually verified.
- **IPC-5 official reference graft-in** and vendored best-known
  reference costs (real IPC quality scoring): still open, still
  documented on the scoreboards themselves.
- **Perf micro-debts** (Scratch generation-counter reset,
  apply-into-buffer clone-on-survival, preferred-operator best-first
  variant A): parked in perf-notes with their measured ceilings;
  worth a half-day sweep some cycle, not this one's risk budget.
