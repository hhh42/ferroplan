# ferroplan 0.24 roadmap — the SAT wing (the cycle ten negatives asked for)

Scoped 2026-08-14, by direct request and by conversation — this
cycle's shape was CHOSEN, question by question, and the decision
trail is part of the record:

- **Centerpiece: the SAT compilation wing.** Picked over classical
  PDBs (whose design read deflated them to a side-dish: +2–8
  centered ~4, openstacks probe-NEGATIVE — the read is banked below,
  the centerpiece it is not) and over a game cycle (a small game
  phase rides instead).
- **The solver: ABSORB AND OWN.** The costing read flipped the
  first call (vendor) to hand-roll — no pure-Rust crate is better
  than MiniSat-class, vendoring buys only calendar, and in-tree
  ownership unlocks planning-specific branching (Madagascar's
  bespoke CDCL beat generic solvers on planning CNFs). The owner
  then sharpened it further: start FROM varisat's code and make it
  ferroplan's — "I want it to become our code, and us carry it
  forward." varisat is MIT OR Apache-2.0, the same dual license as
  this repo: it absorbs cleanly, attribution preserved, and from
  the absorption commit onward it is ferroplan code with a
  ferroplan roadmap. Reference checkout at `.solver-refs/varisat`
  (gitignored).
- **Stage c: taken.** The 0.23 sizing memo priced it phase-shaped
  (70 rows, `hold-*` proven absent from the corpus).
- **A small game phase rides** — the first since 0.18.

Standing dependency, stated before any number: **the 0.23 cut has
not run.** Every board figure below is the 0.22 table plus 0.23's
solo receipts; the cut (22 boards, the tier move, the re-entries)
and the backfill columns re-price the dockets when the sweep session
runs them. Phase 0 exists to absorb that re-pricing honestly.

The receipts that chose the centerpiece:

- Ten mechanism-precise temporal negatives across five cycles,
  ending in 0.23's verdict: the remaining walls are
  **choreography/serialization, which no per-state relaxation payout
  can price** — and TRPG-lite's post-mortem showed the two window
  facts a relaxation CAN learn are provably absent where the
  plateaus live.
- The field file: ITSAT (SAT-based) solves TMS 18/20 and storage
  20/20 at budgets where every non-SAT entrant scores 0;
  parc-printer is double-receipted SAT territory (ITSAT 20/20,
  orbit scan zero groups).
- The scoping probes: TMS-2011 i1 grounds to **15,384 snap events,
  ~5.2k structural facts, ZERO fluents, ZERO TILs** — the perfect
  first target for a layered causal encoding with STN scheduling;
  slitherlink's CNF is trivial at any plausible horizon (L=30 →
  0.07M vars) — the puzzle face costs nothing to carry as a smoke
  test.
- And the house already owns two of ITSAT's three thirds: the snap
  compilation (0.23 Phase 2) and the STN/ε/emission machinery
  (0.18–0.22). The wing builds the third.

## Phase 0 — the standing dependencies (the sweep session's, absorbed here)

Not this build's to run; this roadmap's to absorb:

- **The 0.23 cut** (22 boards, the tier move with the registry flip,
  the six re-entries) — re-prices the constraints band, the tier
  arithmetic, and every "provisional" marker below.
- **The backfill columns** (`v0.21.0`, the v0.19 completion) — the
  open bills read against them: onlycraft's hatch-invisible −6, the
  damping bill's three rows, the casualty adjudications
  (org-synth-split i15 confirmed, hiking-agile i11 owed).
- When they land, the affected Recorded blocks get their board
  columns and this file's bands get re-anchored — a half-day sitting,
  not a phase.

### Recorded — the dependencies landed; the bills are read

The 0.23 cut promoted (62% / 22 boards, the complete table) and the
v0.21.0 backfill column staged — Phase 0's sitting is discharged:

| bill | row | v0.19 | v0.21.0 (this box) | 0.22 | 0.23 fresh | verdict |
|---|---|---|---|---|---|---|
| onlycraft −6 | sat i4 / opt i4 | 19.3 / 19.8 s | 24.2 / 24.9 s | dead / dead | dead 58.7 s ×2 | **STANDS-ENGINE** |
| onlycraft −6 | sat i5 / opt i5 | dead / dead | 19.4 / 20.2 s | dead / dead | dead 58.4 s ×2 | **STANDS-ENGINE** |
| onlycraft −6 | sat i6 / opt i6 | dead / dead | 49.9 / 48.7 s | dead / dead | dead 58.0 s ×2 | **STANDS-ENGINE** |
| damping | ext-plant i10 | 5.4 s | 58.4 s (1.6 s margin) | dead 59.0 s | dead 58.9 s | **STANDS-ENGINE** (wall-margin flag) |
| damping | ext-plant i16 | 5.6 s | 52.8 s | dead 59.1 s | dead 59.0 s | **STANDS-ENGINE** |
| damping | sugar i18 | 38.7 s | 47.0 s | dead 57.0 s | dead 57.0 s | **STANDS-ENGINE** |
| casualty | org-synth-split i15 | 15.9 s | 20.4 s | dead | **SOLVED 59.99 s** m=882 VAL✓ | **PAID** (at the buzzer) |
| casualty | hiking-agile i11 | 28.9 s | 36.0 s | dead | **SOLVED 34.54 s** VAL✓ | **PAID** |
| casualty | floor-tile-2011 i11 | 7.0 s | 7.9 s m=170 | dead | dead 59.1 s | **STANDS-ENGINE** (carried) |
| casualty | nurikabe i12 | 56.5 s | 46.9 s | dead | dead 60 s | **STANDS-ENGINE** (5A loop; 0.24 fix = measured negative) |
| ramp | openstacks-2014 i11+i12 | dead both | dead both | dead both | dead both | **STANDS-ENVIRONMENTAL** (acquitted) |

- **onlycraft's −6 is REAL and UNPAID:** all six rows solve under
  the v0.21.0 tag on this box (19.4–49.9 s) and stay dead at 0.22
  AND 0.23 — a genuine 0.22 reallocation cost that no hatch reaches.
  It is now THE open engine docket of this cycle, with the column
  receipt attached. (The fold's proof-side win on the same domain —
  i2's cert 16.07 s → 1.00 s at board conditions — does not reach
  these six coverage rows.)
- **PAID:** org-synth-split i15 (on the board, 59.99 s — joins the
  fragile watchlist) and hiking-agile i11 (34.5 s, clean).
  **STANDS-ENGINE, carried:** floor-tile-2011 i11 (five releases of
  solves, killed by the driver, guard declined); the damping three
  (with i10's 1.6 s v0.21-margin flagged); nurikabe i12 (docket
  CLOSED against P6.2's measured negative).
  **STANDS-ENVIRONMENTAL, corroborated:** the openstacks-2014 ramp
  — the v0.21.0 re-run reads 10/20 beside the promoted board's
  receipt-less 12/20; the 0.21 board was the outlier.
- **The trend line** on the 10-board subset every column supports:
  v0.19 1,459 → v0.21 1,639 → 0.22 1,740 → 0.23 1,758 — and the
  backfill validates the promoted 0.21 boards wholesale (within 8
  rows on 3,186).
- Every provisional band in this file is now re-anchored on the
  0.23 numbers above; the constraints referee for stage c is the
  70-row timed class the cut isolated.

## Phase 1 — the absorption (varisat becomes ferroplan-sat)

The largest single decision of the cycle, executed as surgery, not
as a dependency bump:

- **Vendor the solver core IN-TREE** (a `ferroplan-sat` workspace
  crate or `crates/ferroplan/src/sat/` — decided at absorption time
  by what the strip leaves): CDCL with watched literals, 1UIP
  learning, VSIDS/phase-saving, restarts, clause-DB reduction, and
  the incremental-assumptions interface if it survives the strip
  audit. Attribution preserved in file headers and ATTRIBUTION.md;
  the dual license carries through unchanged.
- **The DIMACS differential FIRST:** before any strip, the absorbed
  solver must reproduce reference verdicts on a fixture battery of
  planning-shaped CNFs (SAT instances with known models, UNSAT with
  known cores) — the gate that makes every later strip and every
  future ferroplan-side change safe. This is the wing's version of
  fixtures-first.
- **Then the strip:** proof logging, the checker, anything the wing
  does not need — each removal behind the differential. What
  remains is ours: the named first ferroplan-side improvement is
  planning-specific branching (variable ordering from the encoding's
  layer structure), explicitly NOT taken this cycle — the wing must
  prove out on stock heuristics first, so the solver's contribution
  stays separable from the encoding's.
- Cost note from the read, kept honest: absorption ≈ the vendored
  estimate (0.10–0.15 of the cycle for a working, differential-gated
  core) rather than the 0.25–0.35 hand-roll — the calendar saved is
  what pays for stage c and the game phase riding.

### Recorded — the absorption (varisat is ours now)

- **crates/ferroplan-sat exists**: 5,012 lines absorbed from
  upstream's 7,414 (core + lit/cnf), ZERO external dependencies
  dev-deps included (upstream pulled ten crates; the strip and two
  small rewrites removed every one). Attribution in every file
  header; root ATTRIBUTION.md created; the dual license carries
  through unchanged.
- **The differential came first and never trusted the solver:** 22
  fixture CNFs (pigeonhole, coloring, planning-shaped layered
  chains — including the pre-goal-horizon UNSAT shape) plus three
  ~100k-clause in-code mediums; every SAT model verified by DIRECT
  CLAUSE EVALUATION. The battery went RED against a deliberately
  lying always-UNSAT stub before it ever went green against the
  real core, and re-green after every strip commit.
- **Kept**: the assumptions interface (~200 clean lines; the CEGAR
  loop may want failed cores). **Added, one thing only**: a
  per-solve conflict budget (Interrupted as an honest third
  verdict) — the horizon ramp's requirement, pinned. **Named
  not-taken**: planning-specific branching — the wing proves out on
  stock heuristics so the solver's contribution stays separable.
- Battery timing on the loaded sweep box: 100k-clause instances
  decide in 8–38 ms — the constituency's 0.3–5M-clause estimates
  are comfortably inside a 60 s wall's budget.

## Phase 2 — the classical core (encoder, decoder, round trip)

- **∃-step bounded-layer encoding** over the existing grounded task:
  Rintanen-style disabling-chain interference clauses (linear-size,
  no quadratic mutex blowup), explanatory frame axioms,
  invariants.rs mutex groups as seed clauses, empty layers allowed.
- **The decoder and THE fixture:** a SAT-decoded plan must
  VAL-validate against the ORIGINAL problem (the round-trip
  discipline the orbit witness set). Plan soundness is structurally
  free on this route — that is the beauty of compilation, and the
  referee harness (VAL + the internal fold-oracle + the
  crash-unavailable booking) already exists.
- **Horizon scheduling:** geometric layer ramp with per-horizon
  CONFLICT budgets — ramp on UNSAT or budget exhaustion, never wait
  for full UNSAT proofs at pre-goal horizons (the classic SATPLAN
  sink, named and avoided). An UNSAT-at-horizon-1 fixture pins that
  the ramp escape names itself loud.
- The classical/puzzle face ships behind `FF_SAT_CLASSICAL` as the
  smoke test it is — band honestly ~0 (+0–6 on the 2023 puzzles,
  unpriced: no SAT planner was ever fielded there, and the encoder
  prices every instance in milliseconds and may simply decline).

## Phase 3 — the temporal face (the priced pot)

- **The ITSAT move on machinery we own:** encode the snap-compiled
  task's start/end events as layered actions with pairing clauses
  (a start implies a later end inside the horizon; over-all facts
  held between); durations NEVER enter the CNF. Decode the causal
  event sequence; hand it to tsched's STN; on a negative cycle,
  assert the refutation clause and re-solve inside the horizon —
  CEGAR with the existing scheduler as the teacher.
- **Pre-registered thrash read:** STN-refutation loops per horizon
  on TMS i1, written down before the loop exists — a thrashing
  CEGAR is a measured negative, not a mystery.
- **The required-concurrency detector** (~100 LOC): an op whose
  over-all needs a fact that exists only DURING another op — TMS's
  fire-kiln shape, match-cellar's light. It promotes the SAT rung
  early on families where decision-epoch search is provably
  hopeless; everywhere else the rung arms only at ladder exhaustion
  with wall remaining AND an encoder size check — the encoder can
  decline honestly in milliseconds.
- **Arming policy, per the house law** (no sweep arms = no
  evidence): router-armed DEFAULT-ON for the temporal rung at the
  0.24 sweep, `FF_NO_SAT` as the byte-identity restore. The 486
  solved temporal rows are protected structurally (exhaustion-only
  arming) plus the standing canaries; the load-bearing fixture is a
  match-cellar-style required-concurrency micro-task RED on every
  existing mode and GREEN only via SAT+STN — the proof the wing adds
  EXPRESSIVENESS, not speed.
- **Bands, priced against 30× budget-gapped field receipts and
  carried humbly:** TMS +4–14 (ANY solve beats every non-SAT entrant
  ever fielded), storage-t +6–16, parc-printer-t +4–12, floor-tile-t
  +2–8 (mixed evidence — LPG-td also 20/20 there, not pure-SAT
  territory). Wing total **+16–50**, on walls where every other
  lever in this codebase has a recorded death.

### Recorded — the wing flies: the zero block has its first nonzero row

- **TMS-2011 i2: SOLVED.** Mode::Sat, horizon 16, 242 steps,
  makespan 40.031, ONE STN refutation, internal oracle green, VAL
  "Plan valid", ~1 second — **the first temporal-machine-shop solve
  in this planner's history**, on the family where every non-SAT
  entrant ever fielded scored 0 valid. Ten negatives said the walls
  needed different machinery; the machinery arrived and the wall
  moved on its second instance.
- **And the smoke face drew blood too:** slitherlink p01 SOLVED by
  the classical ∃-step encoder (horizon 16, replay-verified, VAL
  green) — the puzzle constituency's first row, from the face
  priced at ~0.
- **The honest no-solves, exactly as the discipline demands:** TMS
  i1 exhausts honestly at 46 s (h1–8 PROVEN UNSAT, the >100-
  refutation thrash bail firing per the pre-registered read — 404
  refutations, cores cut from 150+ events to 4–18 by the
  duration-endpoint reduction); storage-t i1 proves UNSAT through
  h32 and walls honestly; parc-printer i12 exhausts at h128. The
  CEGAR loop's layer-specific refutations are the named 0.25
  residue, alongside planning-specific branching and ITSAT-style
  in-CNF timing.
- **The fixtures carried the phase:** the required-concurrency
  micro-task RED on every pre-wing mode and GREEN only via SAT+STN
  (the expressiveness proof, pinned forever under FF_NO_SAT); the
  round-trip fixture RED before Mode::Sat existed; the loud ramp,
  the honest decline, the no-plan-within-horizon wording, the TIL
  decline — all pinned. A real bug caught RED by the final batch:
  dual-typed IPC objects ground TWIN ops that broke the pairing
  decode (TMS's own kiln0, again) — fixed, pinned.
- **For the sweep owner:** the detector promotes SAT on
  match-cellar-family rows and burns 2–13 s thrash-bailing before
  the ladder solves (canaries green at wall 60, receipts recorded)
  — watch match-cellar 40/40 at the cut; FF_NO_SAT restores
  byte-identity per row if it regresses. Integration caught the
  same cost in miniature: the wing's wall spend ate tground_wall's
  1 s pre-expiry margin, and the battery now hatches FF_NO_SAT off
  exactly as it hatches MCV — orthogonal machinery that merely
  SPENDS the wall stays out of a timing fixture's frame.

## Phase 4 — stage c (the timed operators, the last locked door)

Per the 0.23 sizing memo, receipts attached: constituency 70 rows
(`within` + `always-within` ONLY — `hold-*` grepped absent from the
whole 2006 corpus); encoding = a search-maintained clock fluent
lowering timed operators to ordinary monitor transitions with
numeric conditions — the machinery stages a+b built. Fixtures per
operator, VAL the oracle, the constraints board the referee
(provisional band +20–45 of the 70, re-priced when the 0.23 cut
lands a+b's board column). The complex-preferences unlock rides the
same lowering; the rankings doc's two "last of 3" rows are the
long-game referee.

### Recorded — stage c lands; the 2006 gate names nothing (board at the cut)

- **Six timed solves banked solo, oracle-green:**
  trucks-time-constraints i1–i3 (0.1–0.8 s) and
  storage-time-constraints i11–i13 — i13's VAL SIGBUS booked
  unavailable exactly as the 0.23 runner fix intends, fold-oracle
  green. The negative control has teeth: a hand-shifted late drop
  is REJECTED by name ("trajectory constraint (within) violated").
- The lowering is the sizing memo's clock made real: within +
  always-within become monitor transitions with numeric conditions
  on a search-stamped clock; the 0.23 fixture
  timed_operators_stay_rejected_by_name pivots to hold_* (the
  contract survives, the constituency moved); a degenerate class
  (negative bounds) rejects by name in one place.
- One honest flag from the receipts, carried to the basket's wall
  item: an always-within probe ran ~116 s past a 60 s limit on the
  monitor-compiled path — the same eval-denominated-search gap the
  basket's P6.1 closes; verified covered at integration.
- Provisional band +20–45 re-prices at the sweep; the
  complex-preferences composition is one paragraph in the phase
  report, entry scoped for 0.25.

### Recorded — the basket: five landed, one measured negative (Phase 6)

- **The temporal search pays the wall** (P6.1): sokoban-t's third
  re-referee — 2008 i21/i2 and 2011 i8 all exit honestly at ~30.1 s
  (first pass trips at ~29 s, the resume pass in milliseconds).
- **The 5A convergence fix is a MEASURED NEGATIVE for a default
  flip** (P6.2): nurikabe's drip and spider's wall-edge conversion
  are locally irreconcilable — every rule that frees one amputates
  the other. Both traces recorded; the loop keeps its 0.23 name.
- **The a2 chain converts its RED fixture** (P6.3): pathwaysmetric
  i2 — 948,388 evals dead → 54 steps at 173 evals in 0.08 s.
- **The hash-join gate CLEARS where the 0.23 refusal predicted it
  might** (P6.4): slitherlink p03 grounding >60 s → 1.3–2.2 s under
  per-predicate candidate lists, and p01 now SOLVES whole (0.54 s).
  folding p01 honestly not cleared (a different mechanism — the
  or-aware hoist is sized, not taken); org-synth not chased.
- P6.5: the 2023-legit compiled formulation is what the engine
  ALREADY runs (verified at the op level) — recorded, no work
  owed. P6.6 took two lines (engine narration + runner suffix,
  because a SIGKILLed child leaves no JSON). P6.7's h-economy read
  is delivered as a report section for the deferred list.

## Phase 5 — the game phase (small, and finally)

The first game-side phase since 0.18, scoped to EXPOSING what five
contest cycles built rather than building anew:

- Session/MCP surface the new engine: budget-stamped thinks (the
  honest-wall discipline as a think contract), orbit-aware replans,
  the capped-vs-proven honesty in think verdicts — and `Mode::Sat`
  over the wire, because game puzzles are exactly the
  constraint-shaped tasks the wing serves (a village lock, a
  logistics riddle: bounded-horizon, small, SAT-trivial).
- The village/bazaar demos re-measured on the 0.24 engine (the tick
  loop's think budgets were last priced at 0.21-era heuristics; the
  5A slices and the driver changed that arithmetic unobserved).
- Explicitly bounded: one phase, no new game systems — the 0.25
  direction question (a full game cycle) is on the record from this
  scoping's Q&A and gets decided at the 0.24 cut with this phase's
  receipts in hand.

### Recorded — the game phase: what five contest cycles built, on the wire

- Budget-stamped thinks with the capped-vs-proven honesty verbatim
  on the MCP wire (protocol-pinned); orbit-aware thinks inside the
  session contract (the standing_replans_stay_orbit_free pin
  documents the safe boundary); the Mode::Sat seam turned out to be
  ALREADY WIRED by construction — the typed options schema gained
  the "sat" variant the moment the enum did, no new tool, no new
  field.
- **The village re-measure, and a receipt worth the phase alone:**
  the pair contract at 200k/500k evals now returns HONEST CAPPED
  verdicts (never "unplannable"); 1M sails as before — and the tick
  loop runs byte-identical events and think evals (1,510,921) at
  **15.61 s → 10.57 s wall**. The bazaar tables: every
  eval-denominated cell identical, think milliseconds roughly
  HALVED (k=11 max-B 495.6 → 280.9 ms). Three cycles of engine work
  reached the game surface without moving a single eval.
- Provenance honesty: the inherited work was audited against the
  spec, transplanted, and squashed to one phase commit (the WIP
  branch preserved for diffing); the fixtures that arrived with the
  feature are marked as pins, not RED-first — said plainly in their
  doc comments.

## Phase 6 — the follow-through basket (every item carries its receipt)

- **Search-side temporal wall discipline** — the 0.23 finding with
  the receipt (sokoban-t grounds in seconds and then searches past
  60 s: the decision-epoch search has NO wall checkpoints,
  eval-budget-denominated only). Clock checkpoints in the temporal
  search loops, the 0.22 Phase 2 idiom; sokoban-t's 33 re-referee a
  third time, now search-side.
- **The h-economy design read** (one day, NO code unless it exits as
  a byte-identical-off probe): deferred evaluation — the blockers
  decode's cross-cutting candidate for the 2018/2023 residue, to be
  0.17-history-proofed (old-binary column mandatory) before any
  pitch.
- **Hash-joins, new gates:** the 0.23 refusal stands for org-synth
  (i11 remains the honest stretch gate), but the blockers decode
  found CHEAPER constituencies: pre-registered reads — slitherlink
  p03 and folding p01 ground <5 s solo. If the same lever clears
  those, the 2023 puzzle wall's grounding half moves without
  touching the refused case.
- **The a2 chained charge** with the pathwaysmetric-i2 RED fixture
  (the 0.21 charge that cracked i1 at 4,710 evals does not reach
  i2's chain), inheriting the SUM/first-wins design constraints.
- **The 5A convergence fix:** the "progress 0.00 s ago" extension
  loop (nurikabe i12's mechanism, named at 0.23) — with spider p01
  as the do-not-give-back canary, since the same loop converts it.
- **Slitherlink's negative-goal compile** (the 2023 organizers
  scored the better of original/compiled formulations — an
  in-engine compile is IPC-legit), and the label-hygiene rider
  (node-raise re-entries booking self-inflicted MEM).

## Phase 7 — cut 0.24.0 (a new wing meets the whole table)

The standing template on the post-0.23 table (22 boards, one box,
no ghosts — assuming Phase 0's dependency lands first), plus:

- The wing's referee constituencies read AGAINST THE FIELD FILE, not
  just the boards: any TMS/storage solve is a first for this
  planner's entire style class, and the record will say so with the
  budget gap named.
- The solver absorption gets its own line in the release record —
  the first adopted subsystem in the project's history, the license
  trail, and the differential that keeps it honest.
- Bands, summed with the usual discipline: the wing +16–50, stage c
  +20–45 provisional, basket +5–15, game phase measured in demo
  receipts not coverage. The headline question at this cut is not
  the percentage — it is whether the zero block finally has a
  nonzero row in it.

## Anti-pots, and the banked reads

- **Classical PDBs: the read is banked, the centerpiece declined.**
  Parking's counted case (+1–4 via the sprint slot, i2–i4 named)
  waits as a 0.25 side-dish; openstacks-opt is probe-NEGATIVE with
  its mechanism named — nobody re-pitches it without new evidence.
- **org-synth stands refused** (twice, the second time with a
  lower-bound simulation). **ricochet is none-known** (symbolic
  style-mates own it; not our machinery, said plainly).
  **model-train stays unpromised** — the zero block's SAT-facing
  corner is TMS/storage; model-train's last-mile numeric shape is
  not obviously CNF-friendly and is priced at 0 until an encoder
  probe says otherwise.
- **No solver-tuning rabbit holes:** the wing proves out on stock
  CDCL heuristics; planning-specific branching is 0.25's candidate
  WITH the wing's receipts, never this cycle's tinker.

## Deferred, on the record (carried forward)

- The in-tree solver's planning-specific branching; incremental
  assumptions (only if horizon-ramp profiling demands them).
- Parking's PDB counted case; caldera's selectivity-aware route
  gate; block-grouping's search residue.
- The 0.25 direction question: game cycle vs contest continuation —
  decided at the 0.24 cut, with the game phase's receipts and the
  wing's board verdict on the table.
- IPC-5 complex preferences (unlocked by stage c's lowering — entry
  scoped when stage c's board column exists); cross-mind planning;
  continuous `#t`; dynamic derived predicates — the standing lists.
