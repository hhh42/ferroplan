# ferroplan 0.15 roadmap — the seen-and-scheduled cycle

Scope settled 2026-07-23, the day the extended 0.14.0 cut. The
extension left exactly one hard problem with a *fresh, precise*
mechanism attached and one game capability the deferred list has
named twice. 0.15 takes both, plus the correctness close-out and the
platform piece that ride along naturally. One hard bet, two
near-certain ships, everything traceable to a recorded debt or
diagnosis — no fresh guesses.

The recorded design answers this cycle serves:

- **The TMS wall moved and we watched it move**: not symmetry
  (orbits collapsed it), not soundness (the transition guard closed
  it), but the invariant-BLIND relaxation — h^FF has no concept of
  "this kiln window closes at t=8 and what you just started will not
  fit." That is the most actionable temporal-guidance diagnosis on
  file, and storage-t / model-train are feasibility-window shaped in
  their own ways.
- **A mind's Session already IS a belief state** — a world copy that
  drifts from the authoritative one. The bazaar loop has been
  simulating perfect information over an architecture built for
  imperfect information; formalizing observation is cheap relative
  to what it delivers, and the deferred list has called belief "the
  game's business" two cycles running.
- **Soundness debts get closed while the machinery is warm**: the
  numeric-invariant endpoint limit was recorded in Phase 10 of the
  extension; its fix reuses the same InvMap plumbing.

## Phase 1 — window-aware temporal guidance (research target, the hard bet)

The TMS follow-through. Fixtures first, mechanism first:

- **Fixtures first**: a minimal windows fixture family
  (`bench/kiln-pack-*.pddl`) — k jobs of mixed durations, one
  resource whose invariant-bearing windows open and close on a
  schedule (kiln firings), instance sizes that step from trivially
  packable to tight. The MEASUREMENT before the lever: on each size,
  where do the evals go — window-infeasible starts (already
  doom-pruned at birth), window-WASTING starts (fit now, strand the
  remainder), or plateau ordering among equally-h states? The probe
  decides which lever gets built; do not build first.
- Candidate levers, cheapest first, ONE gets a real swing:
  (a) an ordering term from LIVE window slack — for each pending
  invariant-deleting end at `te`, how much of `[now, te]` do running
  intervals actually use vs. what still must fit (a concrete-state
  read, like the demand and trip-bound terms before it); (b) seeding
  the relaxation with pending-end DELETES (the dual of 0.14's
  `seed_til_h` adds) so h at least sees that the window CLOSES —
  riskier: it can make reachable goals look dead, so it must be an
  ordering signal or a pruned-pass-only bias, never a completeness
  gate.
- Bar, the house rule verbatim: TMS off 0/20 at 30 s — or the
  negative recorded with the same mechanism precision as the four
  guidance negatives before it. Second witnesses: storage-t,
  model-train. Zero corpus regressions; casualties named and
  solo-checked; `FF_*` hatch either way.
- Severable: ships behind a default gate unless the tempo-sat sweep
  is clean.

### Recorded — the probe rewrote the phase, and shipped the win it found

**The fixture disproved the framing before anything got built.**
kiln-pack (8-long kiln windows, 2 slots, two-stage jobs, mixed
durations, no symmetry) solves near-LINEARLY — 29→539 evals for
N=2..12. Clean window packing is not a wall for this engine at all;
doom-pruning already kills every overrun at birth. Whatever
"window-aware guidance" would have ordered, the search wasn't
suffering from it.

**What TMS actually suffers from, measured** (TStats probe, per-pass
`doomed / deduped / evaluated / dead_end / b_blocked / tie_rescue /
best_h` under FF_RES_DEBUG): at a 60 k candidate budget, 81% of
generated successors were orbit-permutation DUPLICATES — generated,
canonically keyed, and thrown away one at a time. The shipped lever:
**generation-side stabilizer skipping** (`stabilizer_classes` +
`gen_key`): an op is never generated when a state-fixing member swap
— verified against cross-member facts, fluents, and the pending
agenda, then extended class-wide (transpositions compose) — maps it
to an already-generated sibling. Real evals at the same budget:
11,113 → 26,562 (2.4×); duplicate share 81% → 53% (the remainder is
cross-node convergence, which is exactly what the visited key is
for). Deterministic, t1 ≡ t8; match-cellar / turn-and-open stay
solved VAL-green; suite green; kiln-pack byte-identical (no orbits
there). Default-on going in, Phase 6's sweep is the referee — **and
the referee ruled against it**: 9 match-cellar instances lost to the
per-expansion stabilizer scan, zero instances gained anywhere (TMS's
2.4× throughput bought no solves — its wall is the start-credit
plateau, not evaluation rate). Shipped as the opt-in hatch
`FF_ORBIT_GEN=1`; the canonical-key pre-dedup stays default-on
(pay-per-duplicate, not pay-per-expansion). Full attribution in the
Phase 6 record.

**And the wall, at last, with full precision: the start-credit
plateau.** best_h = 110 at budgets 15 k / 30 k / 60 k / 120 k /
300 k — an 8.9 k → 114 k real-eval range (13×) finds h=110 almost
immediately and NEVER beats it. The mechanism: h^FF pays out for a
START the moment it fires (the snap-start leaves the relaxed plan,
h−1) while the interval delivers nothing until its end lands inside
a legal window — so the pruned pass floods the start-subset lattice
(the 110 floor is root-h minus the free starts) and never completes
even one structure. Four ordering schemes measured against it, four
negatives: FIFO ties (default) 110; `FF_TLIFO` 150 (dives a high-h
corridor); `FF_TB_FREE_G` 196 (rides time forward, wasting
windows); `FF_TAGENDA_W_PRUNE=3` — the exact start-credit
counter-account in the key — 173, though it alone pushes deep
enough for window blocking to finally engage (b_blocked 26). The
conclusion the four negatives force: **the credit misallocation is
inside the relaxation, and no reweighting of its output restores
the lost discrimination. The named fence: end-gated interval credit
in h itself (count a snap pair as one unit, paid when the end is
relaxed-feasible) — h surgery, a future cycle's bet.** TMS stays
0/20 at 30 s, now for a reason stated to the decimal.

**The second witnesses each have a DIFFERENT wall — "window-aware
guidance" was never one lever:** storage-t i1 reaches best_h 20
(guidance is fine!) with 3,494 invariant-BLOCKED agenda heads and
zero rescues — its wall is spatial `clear`-chain blocking, a
feasibility shape; model-train i1 reaches best_h 6 with nothing
blocked and nothing doomed — a last-mile numeric shape. Three walls,
three mechanisms, all now on file with probe numbers.

## Phase 2 — numeric invariant conjuncts in the transition guard (correctness sibling)

Phase 10's guard covers conjunctive PROPOSITIONAL invariants; numeric
conjuncts (`over all (>= (fuel ?v) 0)`) still get endpoint-only
checking — the recorded limit, and the same delete+re-add shape can
slip through numerically (drain past the floor, refuel before the
end).

- Fixtures first: a numeric kiln-gap (`bench/fuel-gap-*.pddl`) the
  current engine provably takes the bait on, VAL-red, before any fix.
- The fix reuses the InvMap plumbing: per pending interval, the
  grounded numeric conjuncts; a happening that writes a fluent read
  by one re-evaluates the comparison on the post-happening state and
  is refused on violation. Exact, diff-driven, like the
  propositional guard.
- The named risk: over-blocking legitimate concurrent fluent writes
  (a fuel decrease that STAYS above the floor must pass — only
  actual violations block). The tempo-sat sweep is the referee;
  transport-t/model-train/crew are the watch domains.
- Doom-pruning extension is explicitly OUT of scope for numerics
  (whether a comparison can "never recover" needs value reasoning —
  a different animal than fact monotonicity).

### Recorded — SHIPPED (bait proven, then killed; watch domains clean)

The fixture earned its keep twice before the fix existed: v1's drain
was schedulable BEFORE the run (the engine innocently found the sound
order — no bait), so `dip` became hot-gated (only possible inside the
interval) with `topup` as the pre-run escape. On that shape the
0.14 engine takes the bait exactly as predicted: `run@0, dip@0.001`
— level 2 → 0 mid-interval, both endpoint checks pass, **VAL-red**.

The fix rides the InvMap plumbing as spec'd: `ground_inv` grounds
`Comp` conjuncts to `NumPre` + read-fluent ids (via the duration
grounder; a conjunct over never-written fluents is dropped — the
endpoints already check it; a conjunct that fails to ground reverts
the op to endpoint-only). `inv_ok` re-evaluates a pending interval's
comparisons only when a read fluent actually moved, and only an
actual true→false FLIP blocks — the named over-blocking risk
(a fuel decrease that stays above its floor) never triggers by
construction. Start self-checks now include the numeric conjuncts on
the post-effect state. Numeric doom-pruning stays fenced out as
spec'd. On the fixed engine the fuel-gap plan is `topup → run →
dip(4→2) → refill` — **VAL-green**, suite-pinned
(`numeric_over_all_guard_forces_the_topup`).

Watch domains, all VAL-green solves or baseline-consistent: crew-08
i1, crew-11 i1, transport-t-08 i1, openstacks-t-08 i1 solved
VAL-green; model-train i1 no-plan — its baseline is 0/30 and its
wall (Phase 1's probe: best_h 6, nothing blocked) is untouched by
this guard. The full tempo-sat referee runs at the Phase 6 cut as
spec'd.

## Phase 3 — orbits where they don't yet reach (cheap revisit)

The recorded "pass None for now" decisions, re-taken deliberately:

- `tresolve` (the decomposer) and the Session's temporal thinks
  currently pass no orbit map. Wire detection in where the
  soundness conditions hold (a Session with `restrict_ops` masks or
  asymmetric drift must keep None — the mask must be σ-invariant,
  same rule the CLI already documents).
- Bar: byte-identical where detection declines; measured think-cost
  or coverage delta where it engages (the bazaar fixtures and the
  decomposer's crew/order corpus are the probes). If nothing
  engages anywhere honest, record that and keep None — this phase
  is allowed to be a two-line negative.

### Recorded — the two-line negative, with the two lines stated precisely

**tresolve keeps None, structurally:** orbit soundness requires the
WHOLE goal to be σ-invariant, and a contract's subgoal is by
construction a member-naming subset of it (the decomposer exists to
split goals apart); its sibling-protection `forbidden` masks likewise
name members' goal facts. Both disqualifiers are the decomposer's
identity, not an implementation gap.

**The Session keeps None, structurally + economically:** detection
reads the LIFTED problem — a session's runtime world (set_fact drift,
`set_timed_fact` schedules, `apply_start` running intervals, and the
post-grounding TILSET ops those ride on) is invisible to it, and any
of them can distinguish members an init-profile check calls
identical; `restrict_ops` (every bazaar mind) names an actor.
Detection would be sound only for an unrestricted, event-free,
drift-free think — the CLI one-shot case, already served — and would
bill its detection cost against think budgets that are themselves
milliseconds. Both Nones stay; the CLI documents the σ-invariant-mask
rule either way.

## Phase 4 — belief and observation (the game capability)

Partial-observability lite, on the machinery that already exists:

- **`Session::observe(facts)`** — the mind's belief surface: a set
  of facts (a visibility mask over the fact space) the loop refreshes
  from the authoritative world each tick; everything outside it
  stays as believed. `set_fact` drift becomes the OBSERVED channel;
  belief diverges silently until observation contradicts it.
- **Surprise semantics**: a replay/validity check against belief is
  free (0.12's machinery); a SURPRISE is an observation that breaks
  the current plan or falsifies a believed precondition of an
  in-flight step. Surprises — not wall-clock paranoia — trigger
  rethinks.
- **Fog in the bazaar**: `bazaar_live` gains a visibility policy
  (own stall + current trading partner); the measured questions:
  theft-discovery latency (ticks between rival's take and the
  victim's surprise), wasted follows on stale belief, churn and
  conflict rate vs. the full-vision baseline rows that already
  exist in `benchmarks/bazaar-thinks.md`.
- Determinism unchanged: observation is a pure function of
  (authoritative state, visibility policy, tick order); the whole
  fogged simulation replays byte-identical at any thread count.
- Explicitly OUT: belief over OTHER MINDS' plans or goals (that is
  cross-mind modeling — still "a different engine and a different
  year"), probabilistic belief, and any engine change: belief lives
  in the Session/loop layer or it doesn't ship this cycle.

### Recorded — SHIPPED, and the fog taught more than it was asked to

**`Session::observe(&[(fact, value)]) -> surprises`** — sighted facts
snap to observed truth (same writability fences as `set_fact`; a bad
batch errors with belief intact), unsighted facts stay believed, and
the return is exactly the news: facts whose observed value differed
from belief. Suite-pinned (matching observations are not news; the
failed batch moves nothing). Zero engine changes, as fenced.

**Fog in the bazaar** (`Policy::ClaimsFogged`): truth lives in a
separate authoritative session; per-stall change ledgers (latest value
per fact, deterministic order) carry what happened to whoever LOOKS.
A mind observes its own stall at turn start and its partner's stall on
arrival — a surprise on arrival can invalidate the step it walked over
for, and the turn is honestly spent discovering. Claims stay public
(intentions are posted on the board; stalls are fogged). Everything
replays byte-identical.

Measured, against the standing full-vision rows:

- **Fog overhead (no theft)**: 2/2 goals still met; the fogged mind
  whose lane crosses the rival's pays 7 conflicts / 8 thinks / churn
  22 where full vision paid one think — stale third-party belief
  produces plans that die on contact, and rediscovery is bounded.
- **Theft discovery**: +1 tick latency, 1 stale follow — the victim
  was already en route, so the arrival observation is an efficient
  discovery channel.
- **The inversion nobody scripted**: under full vision the theft's
  victim (a0) burns 10 thinks against claim-blocked recovery routes
  and gives up while a1 sails; under fog a0 discovers late and
  RECOVERS (4 thinks, MET) — because fog-a1's stale-belief struggles
  dropped its plan, releasing the claims that had fenced a0's
  recovery route. Information asymmetry reshuffles the winners,
  deterministically.
- **The named pathology: FALSE DORMANCY.** Fog-a1 gave up on a goal
  full-vision-a1 achieves — under fog, a failed think can mean "my
  belief is wrong," not "no plan exists," and the dormancy counter
  can't tell the difference. The fix class (belief-aware dormancy:
  look — or wander — before quitting) is the game's next policy
  layer, recorded and deliberately OUT alongside cross-mind modeling.

Docs: the Session chapter gains the belief/fog section;
`bazaar-thinks.md` carries the two fog rows with surprise /
stale-follow / discovery-latency columns.

## Phase 5 — the in-page Session UI (platform, visible)

The deferred browser piece: the bazaar demo page grows a live
Session panel — inspect a mind's belief vs. the world, poke a fact
(`set_fact` from the page), watch the surprise → rethink → follow
cycle run client-side in WASM. The fog work in Phase 4 is what makes
this panel worth looking at; without it the panel is a state dump.

- Deliverable: `bazaar-live.html` upgraded from canned-trace replay
  to a LIVE loop driven by the wasm Session bindings; module-graph
  check stays green; no server, no install, same as every demo page.
- Scope fence: presentation only — any capability the panel needs
  that the Session API lacks goes on the record as a Phase 4 gap,
  not a page-side hack.

### Recorded — SHIPPED, live and headless-verified (and it flushed a real wasm bug)

**`WasmSession`**: the browser gets the real mind surface —
constructor/fork/set_goal, `restrict_prefix_claims` (the actor scope +
public claim board as one mask), `think` (stores the plan; `valid()` is
the free suffix replay; `step_json`/`suffix_json`/`advance` walk it),
`set_fact`, `observe` (JSON batch → surprises), `goal_met`, `fact`.
`bazaar-live.html` is no longer a canned replay: two real forked minds
run the crossed-chain bazaar in WASM with policy toggles
(naive / claims / claims+fog), a mid-run **steal** button (the world
poke), per-mind belief-drift badges, and the surprise → rethink →
follow cycle visible in the lanes.

**Verified headlessly** (Playwright + the pre-installed Chromium
against a local static serve): claims reproduces the native trace
exactly — both minds MET, 7 follows / 0 conflicts / 1 think each —
and the fog+steal run shows the surprises on arrival, the discovery
inversion from Phase 4, and a live drift badge. One page-side
calibration: the web x2m fixture needs a deeper think budget than the
native fixture (4,000 evals vs 400 — its restricted first think needs
~413), which is the Phase 4 gap ledger working as designed: a page
number, not a page-side hack.

**The bug the page flushed out**: `std::time::Instant::now()` PANICS
on `wasm32-unknown-unknown`, and the engine timed itself
UNCONDITIONALLY in `search_from` (phase attribution), `heuristic`
(probe counters), `temporal_search`, and the constraints monitor —
only the dbg PRINTS were gated. The deployed demo had survived by
luck: its showcase inputs solve inside EHC, which never reaches the
timer; any harder input (and every budgeted think) died. Fixed
lib-wide with `clock::Clock` — a monotonic timestamp that freezes at
zero on wasm (every timing read is measurement, never behavior), plus
a panic hook in the wasm crate so future panics surface as readable
console errors instead of `unreachable`. Native byte-identity: the
shim IS `Instant` off-wasm.

## Phase 6 — 0.15.0 cut

The 0.14 extended-cut mechanics, now the standing template: CHANGELOG
/ README / book refresh, both scoreboards against the final binary
(binary-A/B attribution, casualties named and solo-checked,
`mem-cap` environmental deaths tracked separately from engine
deltas), bazaar-thinks re-emitted, full pre-flight per RELEASING.md —
`--all-targets` clippy included, the lesson of the one failed
publish.sh iteration — wheel build, finish in main; the user
publishes.

### Recorded — the sweep referee earned its title

**The first tempo sweep of the cut candidate came back 390/630 —
DOWN 13 from 0.14's 403 — and the cut stopped until the regression
was attributed and fixed.** Per-domain A/B against the 0.14
scoreboard: match-cellar 20→11 (i12–i20 lost), elevators-08 23→21
(i22, i23), elevators-11 3→1 (i2, i3). Every loss a 30 s timeout;
zero VAL-red, zero mem-cap — pure slowdown, no soundness cost.

**Attribution — one structural mechanism, one dissolved suspect,
solo-proven both directions:**

1. **Gen-skip's per-expansion price** (match-cellar, STRUCTURAL):
   the lost instances solved in 7.75–15.27 s at 0.14 — a 2–4×
   slowdown in the one corpus domain that is orbit-RICH but not
   plateau-walled. `stabilizer_classes` runs per expansion
   (pairwise swap verification against the full state + agenda);
   on match-cellar's deep searches that scan costs more than the
   skipped duplicates ever cost to dedup. Solo proof, both
   directions: i12 solves in 8.9 s with gen-skip off and TIMES OUT
   at 30 s with `FF_ORBIT_GEN=1` — same binary, same box, quiet.
   The referee arithmetic: **9 instances lost, 0 gained** — TMS's
   2.4× eval throughput bought no solves because its wall is the
   start-credit plateau, not evaluation rate. Verdict executed:
   gen-skip is now the OPT-IN hatch `FF_ORBIT_GEN=1` (default
   off); the canonical-key pre-dedup — pay-per-duplicate, not
   pay-per-expansion — stays default-on, exactly as 0.14 shipped.
2. **The elevator flips were BUDGET-EDGE, not engine** — and the
   first-draft suspect (the numeric guard's per-happening cost)
   is formally DEAD: both lost variants (elevators-08-strips,
   elevators-11) are PROPOSITIONAL — there is no numeric conjunct
   loop to pay — and the A/B is conclusive: the pre-fix binary
   solves i22 solo in 26.3 s, the fixed binary in 26.0 s, the
   `FF_ORBIT_GEN=1` counter-probe in 27.1 s. Nothing structural
   moved. All four lost instances (i22 24.5 s, i23 22.4 s at
   0.14; i2 25.3 s, i3 22.1 s) live within ~20% of the 30 s wall,
   where jobs-3 contention jitter flips coins. Named as
   budget-edge, solo-checked, kept on the casualty list.

**What still shipped from the elevator investigation:** `InvTouch`,
a per-op write-set pre-filter for the guard (built once per pass):
an op whose unconditional + conditional + shared-monitor effects
can't touch any invariant-watched fact or read fluent exits
`inv_ok`/`doomed` in constant time. Recorded honestly as COST
HYGIENE — conservative by construction (write sets
over-approximate, no real threat skipped; fuel-gap and kiln-gap
fixtures still pin the semantics), motivated by a suspect that
dissolved under the probe. The guard is now pay-per-threat by
design rather than by luck of the domain.

**Seq-sat: 441/580 vs 0.14's 442** — the lone casualty is parking
i12, which solved at 59.93 s (of 60) in the 0.14 sweep and solves
solo at 58.5 s today with classical code untouched between the two
binaries: budget-edge, named, solo-checked. The seq sweep retains
its validity for the cut (both fixes are temporal-only); tempo
re-sweeps against the final binary.

**Final scoreboards (the cut's numbers):** tempo-sat **399/630, 0
VAL-red, 35 mem-cap** (0.14: 403, same 35 mem-cap class) —
match-cellar fully recovered at 20/20; the four-instance gap is
exactly the elevator budget-edge class, and the paired solo A/B on
the same quiet box settles it as ENVIRONMENT, not engine: the
0.14.0 binary solves i22 in 26.2 s, the 0.15 final binary in
23.2 s — the shipping binary is FASTER on the very instance the
sweep flipped, with ±3 s run-to-run jitter on a 30 s wall.
Seq-sat **441/580, 20 mem-cap** (0.14: 442, 22 mem-cap; parking
i12 the named budget-edge casualty). Both scoreboards from the
final binary; raw JSONLs promoted to the standing baseline names
for the next cut's A/B.

## Deferred, on the record (carried forward)

- **transport-class classical guidance**: the fence is named
  (drive-level route structure / red-black) after FOUR precise
  negatives; it stays deferred rather than earning a fifth
  reweighting swing. Nothing cheaper is hiding underneath.
- **Cross-mind planning** (negotiation, cooperation, plan exchange):
  belief (Phase 4) is deliberately the LAST stop before this line;
  crossing it is a different engine and a different year.
- **Session PDDL3 constraints + the four timed modal operators**:
  still rejected by name; the 0.7/0.8-era gated designs remain on
  file.
- **Continuous `#t` effects and dynamic derived predicates**: still
  out, tracked in README limitations.
- **Fixpoint/stratified grounding unification**: still blocked on the
  interning-order tie-break lottery (sokoban-t 4/10 → 1/10);
  parking #16 folds into the same tie-break-robustness work.
- **Numeric doom-pruning** (the "can this comparison ever recover"
  analysis Phase 2 fences off): waits until the numeric guard has
  corpus mileage.
- **Memory-frugal classical grounding for the woodworking-class
  `mem-cap` deaths**: recorded as environmental in the 0.14 sweep;
  becomes engine work only if a target box makes it matter.
