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

## Recorded — Phase 4 (2026-07-22): SHIPPED — the canned-trace route, as pre-authorized

The browser demo (ferroplan-wasm's pages site) gained the bazaar on
both of its surfaces, choosing the roadmap's pre-authorized cheap
route (a full in-page `Session` UI could not be visually verified
from this environment — recorded as the follow-up, not half-shipped):

- **`bazaar-live.html`** — a self-contained replay page (matching the
  demo's design system) that animates a REAL deterministic run of the
  0.14 tick loop: `bazaar_live --trace` emits the event feed (thinks,
  free follows, trades, conflicts, waits, verdicts) for the x2m
  crossed-chain fixture under BOTH policies, embedded verbatim. The
  naive/claims toggle is the Phase 2 story made visible: red
  conflicts and repeated rethinks vs zero conflicts and one think
  each. Nav-linked from the solver page; regeneration is one command.
- **The solver picker** gained the wants-gated bazaar domain (solo
  11-hop chain + the 22-trade crossed-chain joint goal), so the
  fixture is solvable in-page like every other demo domain. The
  pages workflow's module-graph check passes (18 domains).

No new engine claims — this phase sells the existing ones. Trace
JSON and page script machine-verified (parse + syntax); suite 170/0.

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

## Recorded — Phase 5 (2026-07-22): SHIPPED — the fence lifted, with ZERO engine changes

The severable stretch landed whole, and the honest-exit clause went
unused for the best reason: Phase 3 had already built the mechanism.
A running interval is just a root-agenda happening — `(remaining,
end_op)` enters the think through the same parameter as scheduled
events, where the time-advance block already fires `Kind::End`
entries with real applicability checks and the termination condition
already refuses a goal while any action end is pending. The at-rest
fence came down without touching the search.

The surface that replaced it:

- **`Session::apply_start("(fire urn)")`** — the world begins a
  durative action NOW: start effects (including the `RUNNING-*`
  token) apply immediately, the duration resolves against current
  fluents, and the end joins the in-flight set. Thinks happen
  MID-INTERVAL: the plan covers what remains (never restarts the
  running action) and is valid THROUGH every pending end —
  conservative and sound (the search verifies the goal holds once no
  end is pending). A think can even be PURE WAITING: goal `(power)`
  with the grid cycle in flight returns the zero-step plan whose
  makespan is the pending end's moment.
- **`Session::elapse(dt)`** now fires due interval ends alongside due
  scheduled events, in merged time order — the end applies its own
  at-end effects, RETIRING the mirror-the-end-effects idiom 0.12
  prescribed. An end whose preconditions drift broke (an over-all
  condition killed mid-flight) is REPORTED in the return value with
  its effects dropped — the game decides what a ruined firing means.
- Validity replays inject running ends as real checked happenings
  (drift that breaks an interval breaks every plan living through
  it), sorted by the search's own events-before-starts convention.

Suite 175/0 (5 new: mid-interval thinks, mirror-idiom retirement,
wait-is-the-plan, broken-interval honesty incl. over-all kills,
in-flight t1 ≡ t8). The module docs' scope paragraph rewritten — the
0.12 "AT REST between thinks" contract is now historical.

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

## The extension (2026-07-22): 0.15's scope folds in — one release, not two

0.14.0 was one phase from cutting when the decision landed: the
release is unpublished, so the research-cycle scope specced for 0.15
EXTENDS this cycle instead (the 0.15 roadmap is withdrawn; its full
open-thread harvest — 141 items audited across every cycle record —
stands behind these phases). Order: sure things first, research bets
last and severable, mechanics once at the end.

## Phase 7 — correctness debts (small, first, non-negotiable)

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

## Recorded — Phase 7 (2026-07-22): both bugs were ALREADY FIXED — the records were stale

Verification before surgery paid off: neither diagnosed soundness bug
still exists, because the 0.10 ε-separation rewrite replaced the
pairwise mutex-footprint test with TOTAL ε-ordering — every
consecutive happening in execution order is ε apart, so simultaneity
(the only thing any mutex definition can object to) is impossible BY
CONSTRUCTION, and no footprint under-approximation can ever slip a
coincidence through again.

- **openstacks-time/instance-2** (the recorded VAL-invalid
  conditional-effect mutex): re-solved against the fetched ipc-2006
  corpus, VAL-validated GREEN (`-t 0.001`, 30 steps).
- **The produce-at-end / consume-at-start under-separation**
  (rpg-world fixtures): `selfcheck.py` now reports **5 valid, 0
  flagged** — the 0.10 numeric write-read footprint covered it.

Both stale records amended in place (`benchmarks/temporal-results.md`
finding 4, `benchmarks/encoding-ab/README.md` item 2) with dated
resolutions. No engine change was needed; the 388-plan VAL-green
temporal sweep already standing is the corpus-wide witness. The
phase's real lesson feeds Phase 8: records that outlive their bugs
are exactly why the documentation rework exists.

## Phase 8 — the documentation rework + platform debts

Rework ALL the documentation against the current record, and publish
the surfaces that have waited since 0.5:

- The mdBook is stale against 0.10+ (documents `?duration` effects as
  unsupported, carries the June-era 45-of-368 temporal table) — every
  chapter re-checked against the current engine; the Session chapter
  gains the 0.13/0.14 API story (fork, set_goal, restrict_ops,
  scheduled events, in-flight intervals).
- README: limitations section re-audited line by line; stale claims
  fixed everywhere they appear.
- ferroplan-py: version with the workspace, stage the wheel build;
  ferroplan-mcp: flip `publish = false`, join the publish set;
  RELEASING.md gains both steps.
- Corpus debt ride-along: transport-sequential-satisficing-2011, the
  ONLY 0/N classical variant (0/20 at 60 s) — diagnose whether it is
  the transport11 guidance wall verbatim or something cheaper.

## Recorded — Phase 8 (2026-07-22): SHIPPED — the book catches up to the engine

The mdBook was six releases stale (last touched at 0.8.0). Reworked
against the current record, chapter by chapter:

- **A new flagship chapter** — *Game embedding (the `Session`)* — the
  0.11→0.14 API arc in one place: bounded thinks, follow-before-you-
  rethink, retargetable goals, population forks, actor scoping,
  scheduled worlds, in-flight intervals, the fences, and the memory
  accessors; linked from the library chapter, the intro, and the
  living-bazaar demo page.
- Stale claims fixed everywhere found: temporal VAL story updated from
  the 0.9-era "44/45" to the current 388/630-all-VAL-green corpus
  sweep; "ε-separation of conditional mutexes not handled" (fixed in
  0.10 — the Phase 7 verification); duration-dependent effects and
  state-dependent durations documented; ESPC described as default-on
  (it has been since 0.5) in three places; symmetry reduction and the
  portfolio mode surfaced; install chapter gains `cargo install`.
- `tuning.md` gains the 0.9→0.14 knob sections it was missing
  (`FF_NO_TSYMM`, `FF_TEMPORAL_NODE_CAP`, `FF_TEMPORAL_ABS_KEY`,
  `FF_TLAMA`, `FF_LAX_HELPFUL`, `FF_TAGENDA_W`,
  `FF_NO_FIXPOINT_GROUND`, `FF_NO_DNF_STATIC`, `FF_NO_TRAJ_END`,
  `FF_CLM`, `FF_LEN_ANYTIME`, `FF_LEN_SWEEP_EVALS`), each labeled with
  its measured verdict. README limitations re-audited. Book BUILD
  verified with mdbook.

Platform, staged for the user's publish:

- **ferroplan-mcp joins the publish set**: `publish = true`, LICENSE
  symlinks added (they were missing), `publish.sh` publishes it third
  (library → cli → mcp), dry-run branch build-checks it, RELEASING.md
  documents the order. Build-checked green.
- **ferroplan-py versions with the workspace** (0.1.0 → 0.14.0 in both
  manifests, re-locked) and **the wheel builds**:
  `ferroplan-0.14.0-cp38-abi3-manylinux_2_34_x86_64.whl` via maturin —
  the pre-flight gains the wheel build; PyPI publishing stays a
  separate optional step (RELEASING.md).

Ride-along diagnosis closed: **transport-sequential-satisficing-2011
IS the transport11 guidance wall verbatim** — instance-1 (40 nodes, 16
packages) is search-bound past 90 s solo, the recorded avg_helpful→0
family, nothing cheaper hiding underneath. It waits on Phase 11's
landmark rung, which targets exactly this domain.

## Phase 9 — temporal follow-biased rethinks (the severable keeper)

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

## Phase 10 — object-symmetry orbits (research target #1)

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

## Phase 11 — the different heuristic, first rung (research target #2)

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

## Phase 12 — 0.14.0 release mechanics, extended

The Phase 6 mechanics, re-run once at the very end: CHANGELOG
`[0.14.0]` grows the extension sections, README refresh, full
pre-flight per RELEASING.md (now including the wheel build and mcp
publish staging), and BOTH scoreboards re-swept against the FINAL
binary — seq-sat (the standing debt; the mid-extension partial run
was discarded as stale) and tempo-sat (the ε-separation fixes and any
landed research phase touch it). Binary-A/B attribution for any
wall-clock delta; solo re-runs for 30 s-edge casualties. Finish in
main; the user publishes.

## Deferred, on the record (carried from the withdrawn 0.15 spec)

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

## Deferred, on the record (original 0.14 scope)

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
