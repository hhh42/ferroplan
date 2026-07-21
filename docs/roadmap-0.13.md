# ferroplan 0.13 roadmap — the many-minds cycle

Scope settled 2026-07-20, the day 0.12.0 shipped. 0.12 proved out ONE
agent thinking in ONE world: the temporal `Session`, bounded thinks,
suffix replay, fixpoint grounding where the memory matters. But the
game from STATUS.md's recorded design answers is not one agent — it is
a bazaar full of NPCs, each with its own goals, all sharing one world.
0.13 closes the distance between "a session" and "a population":
engine work with clean, measurable deliverables — no research-grade
heuristic gambles (that fence stands; see Deferred).

The recorded design answers this cycle serves:

- **A bazaar full of agents**: many NPCs, one shared world. Today N
  minds cost N groundings (bazaar: 644 MB EACH). That is not a
  population, it is a memory leak with personality.
- **Desires change**: an NPC that wants bread today and iron tomorrow
  must not re-ground the world to want it. A session's goal is fixed
  at construction — the quiet hole in the ground-once story.
- **Follow, don't dither**: `plan_still_valid` made unbroken plans
  free; a BROKEN plan still triggers an unconstrained rethink that can
  thrash to a structurally different plan. Visible NPC dithering is a
  game bug even when both plans are optimal.

## Phase 1 — retargetable goals

`Session::set_goal`: swap the goal set over the already-interned fact
space, keep the task, no regrounding. One world, changing desires.

- Goals are conjunctions over atoms (and numeric conditions where the
  tier allows) that must already exist in the grounded fact space;
  a goal over an atom the world never contained is an honest ERROR,
  not a silent unsolvable.
- Same fences as `set_fact`: statics rejected, compiler-reserved
  `RUNNING-*` tokens rejected.
- Classical and temporal both; `plan_still_valid` must answer against
  the CURRENT goal (a retarget invalidates a suffix that no longer
  ends in the new goal — exactly, no false "still valid").
- Acceptance: a bazaar NPC switches goals across thinks with zero
  regrounding; t1 ≡ t8; suite tests for retarget-then-replay and
  retarget-to-missing-atom.

## Recorded — Phase 1 (2026-07-20): SHIPPED, and it flushed out a latent bug

`Session::set_goal` landed as designed: any ground conjunction (atoms,
negated atoms with grounded mirrors, numeric comparisons) over the
interned fact space, no regrounding, errors-before-mutation on
everything else (unknown atoms, missing mirrors, `RUNNING-*`, ADL
connectives, non-ground terms). `plan_still_valid` answers against the
current goal by construction. Classical + temporal; t1 ≡ t8 across
retargets; suite 154/0.

Two soundness points the feature forced into the open:

- **The visited key must grow with the goal**: `state_key` omits
  fluents no precondition/goal reads (the write-only-accumulator
  dedup), so a retarget onto a formerly-irrelevant fluent re-runs the
  grounding relevance closure. Relevance only ever GROWS in a session —
  keys get finer, never coarser. The pace-counter test pins it.
- **A latent mirror bug, found and fixed**: `set_fact` left a
  `(NOT (p ...))` complementary mirror STALE when flipping its base —
  every op with a negative precondition on a session-set fact was
  silently mis-evaluated (0.11-era bug, invisible until goals could
  point at mirrors). set_fact now syncs both directions, and the
  `RUNNING-*` fence looks through `(NOT ...)` so the mirror display
  cannot dodge it. The lamp fixture pins the end-to-end story.

## Phase 2 — shared world, many sessions

The immutable ground task behind `Arc`; sessions become cheap forks
holding only their own state view (facts, fluents, goal, stats).

- `Session::fork()` (name settled at implementation): N sessions = ONE
  grounding + N small states. The temporal compilation and any
  heuristic precomputation that is state-independent shares too.
- Determinism unchanged per session (t1 ≡ t8); forks are independent —
  no cross-session interference (a fork's `set_fact` must not move a
  sibling's tie-breaks).
- Measured deliverable: memory and world-load numbers for 12 bazaar
  NPCs — the bar is one 644 MB ground + per-mind state measured in
  MB, not GB; load time amortized to ~one grounding.
- Honest exit: if `PackedTask` sharing needs invasive mutability
  surgery, ship the measurement + a recorded design, not a half-landed
  refactor.

## Recorded — Phase 2 (2026-07-21): SHIPPED, no invasive surgery needed

The honest-exit clause went unused: the mutability seam was already
clean. Everything a session mutates after construction was ALREADY
seven small fields (current facts/fluents, goal, fluent relevance);
the rest of `PackedTask` is written once at grounding and only read.
So the payload — CSR operator columns, names, achiever indexes, the
monitor block — moved behind `Arc` INSIDE the same read API (`Csr`
keeps `slice()`, slices deref transparently), `PackedTask` derives a
cheap `Clone`, and NOT ONE line of search code changed. The session's
lookup maps (fact/fluent/op ids, mirrors, the temporal compilation)
share the same way.

`Session::fork()`: an independent mind over the same world, starting
from the parent's CURRENT state and goal, free to diverge — no shared
tie-breaks, no cross-mind writes. Suite 158/0 (4 new: payload-sharing
`Arc::ptr_eq` pins, sibling isolation, temporal population, forked
t1 ≡ t8).

Measured on the vendored bazaar (`many_minds` example; also corrected
the 0.12 "644 MB each" framing — that was load-PEAK RSS; retained is
~40 MB/session, ~16 MB of it payload):

- world load, once: ~1.9 s through a ~516 MB transient peak
- 12 forks + 12 retargets: **~0.0 ms, +0.0 MB RSS** (~0.4 KB private
  state per mind — KB, not the bar's MB)
- 12 divergent thinks: 0.05 s total
- the old way, per mind: ~1.7 s + ~40 MB retained, twelve times over

New `Session::world_bytes()` / `mind_bytes()` accessors give embedders
the shared-vs-private split (documented as flat-bytes floors); Phase 3
reads per-mind retained memory from them.

## Phase 3 — the barter think benchmark

The corpus was the planner's measuring stick; the game track needs its
own scoreboard. Bazaar thinks have never been measured end-to-end.

- Trade-chain goals at depth 1..k (has Y, wants X, the exchange path
  goes through k intermediate trades) on the vendored bazaar fixtures.
- Curves recorded: think latency and plan quality vs eval budget;
  where budget-exhausted verdicts begin; per-mind retained memory.
- Deliverable: `benchmarks/bazaar-thinks.md` (generated by a runner
  script, like the corpus tables) + a `game_think` example beat that
  exercises a multi-hop trade chain.
- This is measurement, not tuning: whatever the curves say, they ship.

## Recorded — Phase 3 (2026-07-21): SHIPPED, and the first fixture confessed

The vendored fixtures grew a chain variant (`bazaar-chain*.pddl`, from
`gen_bazaar_chain.py`): wants-gated barter at bazaar scale (12 holders
× 40 items) where a vendor releases goods only for the item they want,
so depth-k goals force k-hop trade-up chains — every trade crosses one
want-edge, ≥ k trades in ANY plan; junk inventory makes wrong picks
stranding dead ends. `benchmarks/bazaar-thinks.md` is generated by the
`bazaar_thinks` example: one grounding per fixture, every cell a
`fork` + `set_goal` + one bounded think (Phases 1+2 in anger).

What the curves said (they ship as-is):

- **Solo chains are heuristic-transparent** — a finding, not a
  failure. The relaxed plan sees a single chain exactly, so every
  depth ≤ 11 solves at k+1 evals, sub-millisecond. Game-track
  headline: an NPC can chase an 11-hop trade chain EVERY TICK.
- **Contention makes the curves live**: `bazaar-chain-x2` crosses TWO
  chains at every vendor, both fed from one mind's seeds — the
  relaxation can't see that one offered item can't advance both
  chains, so search must dodge stranding cross-picks. Onset marches
  B=1k (k≤4) → 4k (k=6,8) → 16k (k=10,11); quality stays optimal (2k)
  at every solving budget; an 11-hop contended think costs ~4.7k evals
  / ~460 ms — a budget a game genuinely has to ration.
- Per-mind retained: ~0.05 KB private state; ~0.1–0.2 MB shared
  payload, once.

`game_think` gained the multi-hop beat: a forked trader plans a 3-hop
chain, drift makes the desire IMPOSSIBLE (honest unsolved verdict —
grounding even rejects off-want-edge drift facts), and the NPC settles
for the reachable rung via `set_goal`. Suite 158/0.

## Phase 4 — plan churn under drift (follow, don't dither)

When a rethink IS forced, bias it toward the broken plan's structure.

- Mechanism candidates, cheapest first: seed the rethink from the
  broken plan's still-valid prefix state; a tie-break preference for
  ops shared with the old plan. No completeness loss, no heuristic
  surgery.
- Measured deliverable: a churn metric (edit distance between old and
  new plan, steps preserved) on the scripted drift fixture, with and
  without the bias.
- The 0.11 discipline verbatim: measured win or recorded negative;
  the knob ships opt-in unless the win is clean (`FF_*` hatch either
  way).

## Recorded — Phase 4 (2026-07-21): SHIPPED on the cheapest candidate

`Session::replan_following(prior, from_step, budget, mem)`: replay the
still-applicable PREFIX of the broken plan's remaining suffix (pure
replay, zero search), then search only for a tail from where the
prefix ends — the new plan shares the prefix by construction. The
Phase 2 payload sharing made the seeded start state a cheap task view
(Arc bumps + small vectors), so this landed with NO search surgery;
the second candidate (shared-op tie-break preference) was never
needed. Completeness is fenced: goal met mid-prefix cuts the plan
there; no tail found falls back to an unbiased rethink with honest
combined eval counts. Temporal sessions delegate to the plain bounded
think (a timed prefix ends mid-interval, not at-rest). Opt-in by
construction — it is a separate method, so no `FF_*` hatch is needed
(recorded as the hatch decision).

Measured (scoreboard section in `benchmarks/bazaar-thinks.md`;
scripted drift = one of the plan's own vendor-vendor pre-trades
happens off-screen, discovered by replay so the script survives plan
changes):

- **early hole** (step 2 of 16): biased 15 steps / churn 12 / 2,540
  evals vs unbiased 15 / 12 / 2,997 — little prefix to save, mild win.
- **deep hole** (step 13 of 16): biased **churn 1, 3 evals, 0.1 ms**
  (follows 13 steps, patches one) vs unbiased **churn 16, 2,899
  evals, 124 ms** — a completely restructured plan, one step shorter.
  The bias trades that one step of quality for a three-orders-of-
  magnitude cheaper, visibly steadier NPC; the unbiased path remains
  one call away when quality matters.
- The probe run also caught a drift where the unbiased rethink
  BUDGET-EXHAUSTS at 64k evals while the biased one answers in 6 —
  following is sometimes the difference between answering and not.

Suite 161/0 (3 new: verbatim-prefix preservation, goal-met-mid-prefix
cut with zero search, stranded-prefix fallback recovery).

## Phase 5 — TMS symmetry reduction (the corpus stretch)

The one remaining wall with a DIAGNOSED mechanism (0.12 Phase 4
record): the complete pass drowns in start-spam — ~2,076 pending
intervals per node, identical concurrent intervals multiplying the
agenda combinatorially.

- Lever: symmetry-reduce identical pending intervals (same op, same
  remaining duration, interchangeable tokens) — batch their ends or
  canonicalize their order so the agenda counts CLASSES, not copies.
- Bar: temporal-machine-shop moves off 0/20 or the negative gets
  recorded with the same precision as the diagnosis; parc-printer-t
  (same family) is the second witness; zero regressions elsewhere
  (the scoreboard defends itself — 0.12 proved it).
- Deliberately LAST and severable: if it slips, 0.13 ships without it.

## Phase 6 — 0.13.0 release mechanics

CHANGELOG `[0.13.0]`, workspace bump, README refresh, `rustup update
stable` first, the FULL pre-flight (fmt, clippy `--all-targets
--all-features -D warnings`, suite, doc `-D warnings`, bench
`--no-run`, the ferroplan-py re-lock — 0.12 caught it stale since
0.9). Scoreboards refreshed where phases moved them; binary A/B
attribution if the box is having a slow day (0.12 Phase 5 record);
main publish.sh-ready.

## Deferred, on the record

- **Red-black planning / semantic landmarks over numeric structure**:
  still fenced. 0.11 bought three recorded dead ends; the game track
  has more value per week right now. A research cycle for when the
  corpus matters again.
- **Session TILs** (absolute-clock exogenous events — market opens at
  nine): deferred until the game actually asks for scheduled events.
- **Partial observability / belief state**: the planner plans over
  full state; an NPC's limited knowledge is the game's business (feed
  the session what the NPC believes). Recorded so nobody re-asks.
