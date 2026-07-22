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
