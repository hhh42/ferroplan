# ferroplan 0.14 roadmap — the living-bazaar cycle

Scope settled 2026-07-22, the day after 0.13.0 shipped. 0.13 built a
population of minds — fork, retarget, follow — but every piece is a
primitive waiting for the actual loop: N minds following plans in ONE
authoritative world that changes under them. Nothing yet makes the
population LIVE together, and the interesting failures (two NPCs
racing for the same item, plans invalidating each other, starvation)
have never been measured. 0.14 closes the distance between "a
population" and "a simulation": engine work with clean, measurable
deliverables, the research fence intact (see Deferred).

The recorded design answers this cycle serves:

- **The bazaar runs**: minds don't think in a vacuum — they act, the
  world moves, rivals interfere. The tick loop is the game's real
  shape and it has never been driven end-to-end.
- **Contention is the norm**: in a market worth simulating, desires
  overlap. Conflict handling must be measured, not assumed.
- **The world has a schedule**: markets open, kilns cool, caravans
  arrive. Sessions rejected TILs because they pin the ABSOLUTE clock;
  the clock-relative form ("in 5 units the market closes") dodges
  exactly that objection.

## Phase 1 — the tick loop, measured (fixtures first)

A scripted bazaar simulation over the vendored chain fixtures: N
forked minds with overlapping trade-chain goals, one authoritative
world session, fixed tick order. Each tick a mind follows its plan's
next step (the world mutates), checks validity for free, rethinks
only when broken — the 0.12/0.13 machinery, finally composed.

- Deliverable: a `bazaar_live` example + a generated section in
  `benchmarks/bazaar-thinks.md`: ticks-to-quiescence (all goals met
  or honestly stuck), thinks spent vs free follows, total evals,
  churn, and — the number nobody knows — the CONFLICT RATE: how often
  a mind's plan is broken by a RIVAL's trade rather than its own act.
- Goal assignment deliberately overlapping (shared vendors, shared
  intermediate items) so contention EXISTS; a disjoint-goals control
  row so the contention cost is attributable.
- Determinism: fixed tick order, budgeted thinks — the whole
  simulation replays byte-identical at any thread count.
- This is measurement, not tuning: whatever the loop shows, it ships.

## Recorded — Phase 1 (2026-07-22): SHIPPED — the loop runs, and the economy is brutal

Two API pieces fell out as correctness needs, not conveniences:

- **`Session::restrict_ops(keep)`** — the actor-scoping primitive,
  promoted from Phase 2: without it a mind freely plans RIVAL moves
  (0.13's solver loved vendor-vendor pre-trades), which a tick loop
  cannot execute. Plumbs to the `forbidden` masks both engines already
  carried (`plan_avoiding` / `solve_from`); replays and
  `replan_following` prefixes reject forbidden steps; forks inherit
  the mask; restricted t1 ≡ t8 suite-pinned.
- **`Session::goal_met()`** — the pure state test. The first loop
  draft probed "done" with a zero-budget think and got silently wrong
  results: a think answers "could I still find a plan," and a
  near-done mind must not confuse the two (a mind was marked MET
  without ever acting). Suite-pinned against exactly that confusion.

The `bazaar_live` example drives the serial tick loop (fixed order —
so conflict attribution is EXACT: a break found at a mind's turn can
only be rival-caused) and emits the live-loop section of
`benchmarks/bazaar-thinks.md`. What it measured, shipping as-is:

- **Disjoint control**: 4/4 met, zero conflicts, one think each,
  quiescent in 3 ticks / 0.5 ms — the loop itself costs nothing.
- **Overlapping goals**: 1/4 met. First-tick trades DESTROY three of
  four goals — in a one-way want-edge economy, a stolen rung cannot
  come back, so the losers' thinks fail honestly (~5 evals to exhaust
  the own-actor reachable space) and they give up. The one survivor
  (v5) adapted THROUGH the hole a rival left (churn 2, shortcut past
  the stolen rung) and met its goal.
- Conflict counted once per break (the dead plan drops at break
  time); the whole simulation replays byte-identical at any thread
  count.

The Phase 2 question is now sharp: naive simultaneous pursuit in a
contended one-way economy is CATASTROPHIC (75% goal destruction), and
no rethink discipline can recover a goal the world made unreachable —
Phase 2's levers must PREVENT the destruction (staggering, masking
claimed exchanges), not just replan after it.

## Phase 2 — contention, handled

Whatever Phase 1 measures, make it livable. Levers cheapest first:

- Rethink discipline: broken-by-rival rethinks go through
  `replan_following` (churn already measured cheap); compare against
  unbiased rethinks on the same script.
- Think staggering: one mind rethinks per tick (round-robin) vs all
  at once — does the thundering herd re-collide?
- Only if starvation appears: an opt-in op-mask on the think (a mind
  plans AROUND a rival's claimed exchange) — the temporal search
  already carries a `forbidden` mask; the classical path would gain
  the same shape. No engine surgery beyond that mask.
- Bar: no starving mind on the scripted fixture (every satisfiable
  goal eventually met), bounded churn, measured overhead; negative
  results recorded with the same precision as wins.

## Recorded — Phase 2 (2026-07-22): SHIPPED — claims prevent, and prevention is cheap

All loop-side, exactly as scoped — the engine's only contribution is
the `restrict_ops` mask it already had. A CLAIM is an item a rival's
active plan still intends to receive (its remaining steps' takes); a
mind about to think masks away trades that would take claimed items,
and a mind that cannot plan under claims WAITS (claims release as the
rival's plan drains) instead of burning toward dormancy — give-up
verdicts come only from claim-FREE failures, so they are honest.

The fixture that made it measurable: `bazaar-chain-x2m` (generator
mode `x2m`) — the crossed chains split across TWO actor minds, a0
climbing chain A and a1 climbing chain B. Jointly satisfiable (each
can stay in its lane), yet contended: every vendor stocks both chains
and will hand a B-rung to an A-offer, so a naive mind raids the other
lane as currency. Measured (`bazaar_live` rows, shipping as-is):

- **Naive**: 2/2 met, but a1 pays the raid tax — 6 conflicts, 7
  thinks, 387 evals, churn 12: a0's plan grabs B-rungs, a1 recovers
  over and over.
- **Claims**: 2/2 met, ZERO conflicts, one think each, a1 at 21 evals
  (~18× cheaper), churn 0 — the second mind simply plans AROUND the
  first's declared route. The first thinker's numbers are identical
  to naive (no claims exist yet when it plans), which is the correct
  first-mover semantics, not an artifact.
- **Claims + follow-biased rethinks**: identical to claims here —
  under claims nothing broke, so `replan_following` never engaged.
  The discipline matters exactly when breaks still happen; recorded,
  not oversold.
- **The zero-sum row stays zero-sum, now with honest verdicts**:
  claims cannot make Phase 1's mutually-destructive goal set
  satisfiable (1/4 met either way — the same deterministic winner),
  but the losers now WAIT while claims exist and give up only after
  claim-free thinks fail. Quiet-tick handling lets waiting minds
  resolve to give-ups instead of ending the run "stalled".

Bar check: no starving mind on the jointly-satisfiable fixture (2/2
met under claims, zero starvation), bounded churn (0 under claims vs
12 naive), overhead measured (the mask costs one op-scan per think).
Suite 164/0 unchanged; the whole simulation stays byte-deterministic.

## Phase 3 — the scheduled world (relative-time events)

`Session` scheduled events: "in `dt` units, fact F flips" — the
market-opens-at-nine shape, clock-RELATIVE so 0.12's TIL rejection
(absolute clocks pin thinks) does not apply. The temporal search's
`til_events` are already think-relative internally; the surface is
new plumbing, not new search.

- Shape settles at implementation (likely
  `set_timed_fact(dt, name, value)` + decay as the game advances the
  world, or per-think event lists on `replan_*`); same honest fences
  as `set_fact` (grounded, dynamic, non-`RUNNING-*`).
- Classical sessions: either gain the temporal-compiled path for
  scheduled worlds or reject with a clear error — no silent wrongness.
- Acceptance: a bazaar beat where a think correctly plans AROUND a
  closing market window; suite tests for events the plan must beat,
  events after the plan completes, and events that make the goal
  unreachable (honest unsolved).

## Recorded — Phase 3 (2026-07-22): SHIPPED — and waiting works better than scoped

`Session::set_timed_fact(dt, name, value)` (temporal sessions): in
`dt` units, the fact flips — clock-RELATIVE, exactly dodging 0.12's
TIL rejection. `Session::elapse(dt)` decays the schedule as the game's
clock moves, firing due events (mirrors synced) in time order. Pending
events ride into every think as think-relative TIL events and into
`plan_still_valid` replays (a suffix replays WITH the events it would
live through; events past the plan's span are the game's future).

The machinery underneath, all fenced:

- Per-dynamic-fact setter ops appended POST-grounding behind a minted
  never-true `TIL-NEVER` fact — invisible to the relaxation (no
  achiever registration, unsatisfiable precondition: zero heuristic
  pollution) and to the start block; only agendas fire them.
  `Kind::Til` now fires UNCONDITIONALLY at time-advance (exogenous
  events don't ask permission) — provably behavior-preserving for the
  CLI path, whose compiled TIL ops carry `True` preconditions
  (spot-checked: crew-2011 i1 and parc-printer-2011 i1 byte-match the
  fresh baseline lengths).
- **The static fence earned its keep twice**: probing showed grounding
  STRIPS static facts from runtime preconditions — flipping one by
  event could not soundly change behavior, so `set_timed_fact` refuses
  statics with the same honesty as `set_fact`. The domain contract:
  an exogenous-changeable fact (power, market-open) must be touched by
  SOME domain action to be schedulable.
- **Waiting works** — better than the roadmap dared scope: pending
  events seed each node's heuristic state with their add-effects
  (session-path only, `seed_til_h`; CLI passes false and stays
  byte-identical), so an outage the agenda will repair no longer reads
  as a relaxed dead end. A think IDLES through a scheduled outage and
  acts when the enabler returns (suite-pinned: fuel out at 2, back at
  9 → the firing starts at 9). Replay ties resolve by the search's own
  convention (events fire before same-instant starts).
- The recorded limit is narrower than feared: a goal whose enabler
  exists ONLY via events never grounds — an honest construction
  error, not a silent unsolvable.

Acceptance delivered: window-beating (fires at t=4 under a t=12
shutdown — `game_think` Act 4), unbeatable windows honestly unsolved,
post-plan events ignored by validity, waiting through outages, fences
(classical/dt/static/unknown), timed t1 ≡ t8. Suite 170/0.

## Phase 4 — the visible bazaar

Fold the live loop into a demo surface people can see: the browser
demo (ferroplan-wasm already solves in-page; the bazaar loop is a
canned deterministic trace if a full in-page Session is too heavy) or
a ferroplan-bevy scene — settle by cost at implementation.

- Acceptance: a runnable/loadable demo showing the 12-mind loop —
  trades happening, a mind's plan breaking, the cheap rethink — with
  think stats visible. No new engine claims; this phase sells the
  existing ones.

## Phase 5 — in-flight intervals (the severable stretch)

The deepest game gap: durative worlds still require AT-REST thinks —
the game mirrors end effects manually because a session cannot hold a
running interval between thinks. The lever: `advance(dt)` /
carried-agenda thinks, so a mind can rethink WHILE its kiln fires.

- The semantics must close cleanly: carried intervals become the
  think's root agenda (the machinery exists — contracts drain
  agendas); `set_fact` fences relax only where soundness is provable;
  determinism and replay stay exact.
- Honest exit, explicitly pre-authorized: if the at-rest fence turns
  out to be the correct permanent design (the mirror-the-ends idiom
  is genuinely sufficient), record WHY with a worked example and ship
  0.14 without this.
- Deliberately LAST and severable: if it slips, 0.14 ships without it.

## Phase 6 — 0.14.0 release mechanics

CHANGELOG `[0.14.0]`, workspace bump + cli/mcp pins, README refresh,
`rustup update stable` first, the FULL pre-flight (fmt, clippy
`--all-targets --all-features -D warnings`, suite, doc `-D warnings`,
bench `--no-run`, ferroplan-py re-lock, `publish -p ferroplan
--dry-run`). Scoreboards refreshed where phases moved them — plus one
standing debt: the classical seq-sat scoreboard, unrefreshed for
several cycles (classical paths untouched since, but the record
should say so with a fresh run, binary-A/B'd if the box wobbles).
Main publish.sh-ready.

## Deferred, on the record

- **Object-symmetry orbits** (TMS / parc-printer family): the 0.13
  Phase 5 diagnosis practically wrote its spec — goal-respecting
  orbit reduction over interchangeable objects. FIRST research target
  when a research cycle opens; not this cycle.
- **h-guidance walls** (storage, transport, model-train,
  turn-and-open): the standing research fence, three cycles old, still
  correct.
- **parking #16**: the named 0.13 tie-break casualty, recoverable
  under `FF_NO_TSYMM=1`; folds into any future tie-break-robustness
  work, not worth a phase.
- **Partial observability / belief state**: the game feeds the
  session what the NPC believes; recorded so nobody re-asks.
- **Cross-mind planning** (minds negotiating/cooperating in-plan):
  the tick loop treats rivals as drift, which is the honest MVP; true
  multi-agent planning is a different engine and a different year.
