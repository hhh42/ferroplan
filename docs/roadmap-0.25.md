# ferroplan 0.25 roadmap — Wing II and the table grows

Scoped 2026-08-21, at the 0.24.0 publish, by direct request and by
conversation — the cycle's shape was CHOSEN, question by question,
and the decision trail is part of the record:

- **The goal, in the owner's words: better standings across all the
  tracks on all the years.** That sentence has two irreducible
  halves, and the cycle takes both — "both (big cycle)" was the
  explicit answer to the shape question.
- **Engine centerpiece: the wing's second flight.** The 0.24 cut's
  loudest number is the band that missed: the SAT wing was priced
  +16–50 against field receipts and delivered **+1/+0** on the two
  temporal boards (roadmap-0.24.md Phase 7) — up to +49 sits
  unclaimed, with two live hypotheses already named in the cut
  record and three sanctioned residues waiting. Nothing else on the
  menu has a priced band at all.
- **Co-headline: the table grows.** "All the tracks on all the
  years" is mostly HARNESS work, chosen at "everything fetchable":
  five-plus tracks with corpus on disk or one `get-ipc.sh` stanza
  away — including the standing scandal of **2023-numeric optimal,
  the only track with vendored field receipts
  (`.ipc-corpus/ipc-2023n/results/opt.csv`, eight entrant columns)
  and no board at all**.
- **The field layer gets automated** (chosen explicitly): field
  placement becomes a regenerating column, not a hand-refreshed
  page that drifts two cuts stale (`docs/ipc-rankings.md` is pinned
  at 0.22 as this is written).
- **The 0.24 lesson, named before any band:** a priced band
  delivered +1. This cycle diagnoses before it constructs — the
  wing work opens with an arming audit, not code, and every
  undiagnosed wall gets a design read, not a lever.
- House law, restated: **an opt-in flag no sweep arms produces no
  evidence, and no evidence means no pitch.** Everything shipped
  here is router-armed at the sweep with an `FF_NO_*` restore.

The receipts that chose the shape (all 0.24-cut numbers,
`benchmarks/air24/` + `CHANGELOG.md`):

- The cut: **63% — 3,981/6,366 across 22 boards, 386 certified
  optima, +65 net** — but +50 of the +70 gained came from two
  boards (onlycraft +34 UNATTRIBUTED, stage c +16), all four 2014
  sequential boards LOST (−5, unadjudicated), and eight boards were
  flat including four of the weakest.
- The cross-board bleeds, counted for the first time: **transport
  −211/300 across six-plus boards** (0/20 on ALL three 2014
  sequential tracks simultaneously), **floor-tile −177/220 across
  six**, and the **2006 metric-time family ~−340** (trucks −102/160,
  pipesworld −79/170, tpp −76/150, rovers −44/120, pathways −41/90
  — the same domains score 82%+ propositionally, so the gap is the
  timed/metric layer, not the domains).
- The temporal zero block still stands where the wing's field
  receipts point: storage-t **0/40** (2011+2014), TMS-2014 1/20,
  model-train 0/30 — ITSAT's 18–20/20 rows
  (`benchmarks/ipc2011-temporal-field.md`) remain the style-class
  proof that these walls are SAT territory.
- Integrity debts found in the audit: **71 solved-but-unvalidated
  rows** (`val 0/0` — sailing 20/20, drone 16/20, factory-robot
  10/20, data-network 9/20, markettrader 1/20, plus 10 of
  storage-time-constraints' 15); ~35 early-exits on the
  constraints/metric-time boards never classified by reason.

## Phase 0 — the dockets and integrity sitting (light)

Before anything new: the 0.24 record's own loose ends, adjudicated.

- **The onlycraft re-check, contended.** +34 of the cut's +65 is
  paid-on-paper with no commit targeting it by name
  (roadmap-0.24.md Phase 7: "worth a contended re-check before
  calling it closed for good"). Re-run both numeric-2026 variants
  under deliberate load; the cycle's largest gain either survives
  its conditions or gets its fragility on the record.
- **The 2014 bleed, adjudicated.** All four 2014 sequential boards
  lost rows (−2 sat, −1 agile, −1 opt, −1 mco-t4) with the
  movement column blank. Read the five rows against
  `benchmarks/air24/*.conditions.json` and the 0.23 raws; verdicts
  in the STANDS-ENGINE / STANDS-ENVIRONMENTAL vocabulary.
- **The val 0/0 gap closes.** 71 solved rows carry no external
  validation. Either VAL learns to check them or the standings grow
  an explicit oracle column (VAL / fold-oracle / none) — a solved
  row must say which referee passed it. sailing 20/20 and drone
  16/20 are among the strongest numeric rows on the table; they
  should not also be the least attested.
- Referee: the adjudication table in this phase's Recorded block.

### Recorded (partial) — the 2014 bleed adjudicated: wall-margin churn, no docket

The five lost rows, named from the 0.23-vs-0.24 board diffs (per-
instance raws exist only for the opt board pre-0.24; the other three
adjudicate at domain level plus the current raws' shape):

| board | loss | verdict |
|---|---|---|
| ipc2014-sat −2 | tetris 11→9/20 | **STANDS-ENVIRONMENTAL** — the domain lives AT the wall: 0.24 still solves i4/i5/i12 at 59.4–59.6 s while its misses die at 59.4–60 s; ±1 s of churn flips rows |
| ipc2014-agile −1 | tetris 11→10/20 | same shape (i14 solved 52.9 s; misses at 59.3–60 s) |
| ipc2014-mco-t4 −1 | parking 9→8/20 | same shape (i10/i11 solved 58.7–58.8 s; misses ~59 s) |
| ipc2014-opt −1 | hiking i17 | 0.23 solved at 59.0 s of a 60 s wall; 0.24 node-caps at 59.4 s — the SAME instance the opt-differential gate already flagged contention-borderline (solves ~46–48 s solo) |

Both boards' conditions were CLEAN (median idle 69–75%), so this is
not contamination either — it is the buzzer-beater class doing what
the fragile watchlist says it does. No engine docket opens; tetris
i4/i5/i12 and parking i4/i10/i11 are named as this cycle's watchlist
cohort. Still open in this sitting: the onlycraft contended re-check
and the val-0/0 attestation (both need the box, queued behind the
entries sweep).

## Phase 1 — the table grows (medium, harness)

The breadth half, chosen at "everything fetchable." Entries carry
no bands — a new board is an honest row count, not a win.

- **`benchmarks/get-ipc.sh` learns two stanzas:** the 2018 dataset's
  `opt/` half (the existing L33-51 stanza fetches `sat/` only) and
  the 2023 classical dataset's `sat/` + `opt/` halves (L57 fetches
  `agl/` only — today's "2023 classical" board is the agile corpus
  at 60 s, flagged baseline). Same idempotent-guard shape as the
  stanzas they join.
- **New boards into `benchmarks/ipc67.py` track patterns +
  `benchmarks/standings.py` `SWEEPS`:** 2014 seq-mco **t2 and t8**
  (corpus already on disk; t4 reads 163/280), **2018-opt**,
  **2023 sat + opt** (real tracks, retiring the baseline asterisk),
  **2023n-opt** (the vendored-receipts track), **2026-opt on the
  full 260** (today: 3 pairs / 60 rows), and the 2006
  **simple/qualitative preference tracks swept full-corpus** (today
  they exist only as hand-scored boards on the curated 8-instance
  subset). Exact constituencies sized at entry — instance counts
  are read from the corpus, not promised here.
- **The field layer becomes data:** parse the 2023n field CSVs
  (vendored since their fetch, read by nothing —
  `standings.py:848`); promote the hand-curated temporal field file
  (`benchmarks/ipc2011-temporal-field.md`) and the rankings page's
  per-track entrant numbers into a machine-readable field file
  consumed by `standings.py`; `STANDINGS.md` gains a regenerating
  **"vs field"** placement column; `docs/ipc-rankings.md` is
  regenerated from it (and its three standing caveats — the 30–60×
  budget gap, the hardware confound, coverage ≠ IPC's quality
  formula — carry verbatim).
- **The first run of every new board is a separate ENTRIES sweep**,
  not part of the cut sweep: entries have no before/after, and the
  cut sweep stays the comparable 22-board instrument until the new
  boards have a stable first column.
- Fixtures: idempotent-fetch checks on the new stanzas; a one-board
  canary run per new track before the entries sweep; a regen test
  pinning the field column against a checked-in fixture CSV.

### Recorded — the table grows: built, canaried, waiting only on the sweep

- The two `get-ipc.sh` stanzas landed and fetched (2018-opt 12×20,
  2023 sat/opt 7×20 each; the vendored 2023 bounds.json already
  carries sat/ (140) and opt/ (137) keys — quality columns free).
  Seven track patterns dry-checked variant-by-variant against the
  corpus: opt-2018 240, sat/opt-2023 140+140, opt-2026-full exactly
  the official 13-domain/260 (a `-sat`-excluding lookbehind),
  simple/qual-pref 130/100 (the `$` anchors keep the -grounded
  alternates out), complex-pref 108 waiting on Phase 2.
- Nine boards in the `SWEEPS` registry rendering as honest "sweep in
  flight" rows; totals untouched at 63%/3,981/6,366 until
  `entries25-sweeps.sh` (written, same Air discipline as the cut
  driver) runs.
- Every track canaried end-to-end at a 5 s throwaway budget, VAL
  green on each solve: simple-pref 5/20, qual-pref 2/20, sat-2023
  quantum-layout 19/20 (!), opt-2023 8/20 proofs, opt-2018 petri-net
  4/20 proofs, numeric-2023-opt and opt-2026-full nonzero.
  complex-pref canaried 0/20 — the expected RED Phase 2 converts.
- **The vs-field column is live in `STANDINGS.md`:**
  `benchmarks/field-results.json` (the rankings-page numbers as
  data, provenance in `_meta`) plus the vendored 2023n CSVs parsed
  live (their own Total rows — summing the summary rows in triples
  every count, caught at first render). Merged boards split by the
  rows' own `ipc` stamp; sparse cohorts carry a conditional
  `rank_floor` so thin data cannot flatter (2018 renders ≥13th of
  25, below the field median; 2014 seq-sat ≥8th of 21). Spot-checked
  row-by-row against ipc-rankings.md — all match; the rankings page
  now carries the pointer note and keeps prose + provenance.

## Phase 2 — the complex-preferences entry (light-medium)

Scoped for this cycle by the 0.24 record itself (Phase 4: "the
complex-preferences composition is one paragraph in the phase
report, entry scoped for 0.25"). Stage c shipped the
`within`/`always-within` enforcement the track's preference bodies
lean on; the composition rides the same lowering.

- RED fixture first: a complex-preferences instance that today
  cannot be attempted end-to-end (the operators PARSE — the rankings
  row is precise about this — but the preference-over-timed-body
  composition doesn't score) grounds, solves, and scores its
  violated/satisfied preferences against a hand-checked metric.
- Referee: the new board against the three-planner field — SGPlan5
  swept all 5 domains (105 raw solves), MIPS-XXL second at 25.
  Ferroplan is currently "last of 3, until the feature ships"
  (`docs/ipc-rankings.md`) — any entry is instant placement, and
  the rankings row's leverage is marked high.

### Recorded — the entry lands: three tiers, zero new search machinery

The feature ships as ROUTING plus SCORING — the search itself never
learned what a preference is:

- **Preferences never gate validity.** The temporal router banks
  coverage first (soft constraints dropped; goal preferences already
  lower to trivially-true conjuncts at grounding — a fact that was
  sitting in `ground.rs` all along), then CHASES QUALITY with every
  preference hardened on the remaining wall. plans(hardened) ⊆
  plans(banked), so the chase can never lose the banked row — the
  0.24 promotion lesson applied from birth. A STATIC-LIVENESS middle
  tier saves the satisfiable majority from one hopeless preference:
  bodies `peval_static` proves dead (the fixture's
  `never-obtainable`; grounding CANNOT see this class through the
  monitor lowering — measured, three probes all read Task) drop out
  and the live subset is chased. Search-level joint infeasibility
  stays all-or-nothing — TRUE partial optimization is the named 0.26
  residue.
- **Scoring is post-hoc and search-independent**
  (`temporal::score_soft`): the ORIGINAL soft constraints fold over
  the plan's replayed, timestamped trajectory — the same `Fold`
  machinery `validate` uses for hard constraints, so the scorer and
  the oracle share one semantics — goal preferences evaluate in the
  final state, and the `:metric` is computed with PDDL3
  `(is-violated name)` instance counts (one per preference × outer
  forall binding). The metric rides `Solution.plan.metric` with the
  honest note ("N satisfied, M violated (names); metric X").
- **First rows in this planner's history, canaried at a 10 s
  throwaway budget, VAL green:** storage-complex i1 solved at
  metric 6 ("6 satisfied, 1 violated (P6A)" — the middle tier's
  partial win, not the naive bank); trucks i1/i2/i6 at **metric 0**
  (perfect scores — the full chase); pathways 6/30 with graded
  metrics 8–24. The 0.24-era "stay rejected" pins converted to
  their scheduled 0.25 forms (scored, never silently ignored);
  fixtures in `tests/complex_prefs.rs` (RED first against the old
  fence). One bug caught by the middle tier's first run:
  `static_predicates` scans classical actions only, so on a
  durative pair every fluent read init-frozen and all three probes
  died — the deadness scan now subtracts durative-effect predicates.
- Board registered (`ipc5-complex-pref.jsonl`, 108 rows) with its
  field cohort; NOT added to the running entries sweep's driver (a
  bash script must not be edited mid-run) — sweep it when the
  entries sweep finishes:
  `python3 benchmarks/ipc67.py --track complex-pref-2006 --timeout 60 --jobs 2 --mem-gb 6 --out benchmarks/air25-entries/ipc5-complex-pref.md`

## Phase 3 — Wing II (the centerpiece, heavy)

The wing's unclaimed +16–50, taken in diagnosis-first order. The
0.24 shortfall's two live hypotheses (Phase 7 of that record) are
the opening moves, and everything downstream is gated on what they
find:

1. **The arming audit (a one-day trace, NO code):** why do the
   storage-t / parc-printer-t families never reach the SAT face?
   The required-concurrency detector (`sat.rs
   requires_concurrency`) fires on fire-kiln/match-cellar envelope
   shapes; the exhaustion rung (`temporal.rs`) arms only at ladder
   exhaustion with wall remaining. Trace the field constituency
   through both gates and name which one refuses each family.
2. **Detector widening (or exhaustion-arming repair), per the
   audit.** Whatever the trace names, with a micro-fixture in the
   `sat_wing.rs` style: a storage-t-shaped task promoted RED today,
   GREEN after.
3. **The conflict-rate bail inside the promo slice** — the 0.24
   regression record's own priced residue (~15 s/row refunded on
   grinding horizons: match-cellar solves at ~30.5 s today, 0.02 s
   of which is the ladder). Extends `tests/sat_promo_wall.rs`'s
   child battery.
4. **CEGAR layer-specific refutation clauses** — the named 0.25
   residue with receipts attached: TMS-2011 i1 exhausts at 46 s
   with **404 refutations**, cores already cut to 4–18 events by
   the duration-endpoint reduction. The i1 trace re-run is the
   referee.
5. **Planning-specific branching** — the sanctioned in-tree solver
   improvement (the reason the solver was absorbed and owned),
   explicitly gated behind 1–4's profiling: it lands only if the
   horizon-ramp profile says branching is where the time goes.

- Band, priced humbly and BELOW the residual: **+10–25** across
  storage-t (0/40), TMS, parc-printer-t, floor-tile-t — against
  60 s walls, where the field receipts are 1800 s ITSAT numbers;
  the budget gap is named now so the cut doesn't have to.
- NOT taken this cycle: **ITSAT-style in-CNF timing** (the heaviest
  residue) — one wing bet at a time; it waits for 1–4's report.

### Recorded — the arming audit REDIRECTS the centerpiece (step 1, no code)

The one-day trace ran (`examples/sat_arming_probe.rs`, kept as the
standing audit tool), and it answers the 0.24 shortfall's two
hypotheses with a third the pricing missed:

- **TMS and match-cellar: the detector fires exactly as designed**
  (BAKING/READY and LIGHT are envelope-only). Working as built.
- **storage-t (both years): detector quiet, and CORRECTLY so under
  its own criterion** — the window is a TWO-ACTION envelope (LIFT's
  start adds LIFTING, a different action DROP deletes it; every
  other over-all fact is init-true). Widening to that shape is NOT
  taken: the 0.24 in-phase receipt says armed SAT proves UNSAT
  through h32 and walls on storage-t — promotion would spend half of
  40 rows' walls on a face that cannot yet reach the solving
  horizon. The blocker is ENCODING DEPTH, not arming.
- **parc-printer-t: ZERO over-all predicates — the detector can
  never fire there**, and it doesn't need to: i1 solves in 0.08 s
  and i12 in 0.29 s via the ladder. The +4–12 band was mispriced
  against a family the ladder already covers; the residual tail is a
  measurement question, not an arming one.
- **floor-tile-t: every over-all fact is init-true — quiet, and LPG-td's
  20/20 already said this is not pure-SAT territory.**
- **The exhaustion arm is starved from the other side:** storage-t
  i1 under a 60 s wall shows ZERO [sat] lines — the ladder eats the
  entire wall, so "exhaustion with >1 s remaining" never happens on
  exactly the rows that need it. The obvious fix (cap the ladder at
  ~85% of wall) is NOT free: 10 of the 511 temporal board solves
  land past 85% of wall (turn-and-open ×5, sokoban ×3, elevator,
  map-analyzer) — a blanket reserve risks real rows for speculative
  gains. If taken later it must be conditional on the ladder already
  failing/capped, which is real engineering, not a knob.
- **Verdict: steps 3–5 (efficiency) come BEFORE any arming change,**
  and the +10–25 band now rests on them plus a re-audit — the
  diagnosis-first order caught the pricing error before it became a
  second band-that-missed.

### Recorded — the conflict-rate bail lands, and refunds more than it priced

Step 3, taken immediately off the audit (`FF_NO_SAT_RATEBAIL=1`
restores; armed only under a promoted slice — Mode::Sat and the
exhaustion rung have no ladder behind them to refund):

- Once the measured conflict rate says a horizon cannot finish its
  budget inside RATE_BAIL_FRAC (0.8) of the remaining slice, the
  RAMP is abandoned (every deeper horizon is strictly bigger) with
  the honest "conflict-rate bail … (NOT a proof)" note, and the
  ladder inherits the rest.
- **Measured on the regression family the 0.24 fix left slow:**
  match-cellar-2014 i1 30.7 s → **17.5 s** (bails 1.9 s into h32:
  ~6.5k conflicts/s, est 29 s vs 13 s slice left), i2 31 s →
  **1.2 s**, i20 30.4 s → **10.4 s**. The ~15 s/row pricing was
  conservative. TMS-2011 i2 untouched at 0.54 s; both SAT batteries
  and the full release suite green.

### Recorded — step 4: the layer clauses land, and surface a soundness hole older than they are

Building the generalization forced the soundness question, and the
answer indicts the 0.24 clause itself:

- **THE PAIRING GUARD (a soundness fix, always on — not hatchable):**
  the bare co-placement refutation clause assumed each duration arc
  bound its OBSERVED pair — but a model can hold the same op@layer
  placements with an interposed same-op run between a pair's
  endpoints; the intervals re-pair and the schedule can become
  feasible, yet the clause forbade it. An over-prune that could fake
  "proven UNSAT at horizon" — the wing's strongest verdict — since
  the wing's birth. The guard adds the interposer placements as
  POSITIVE literals: the teacher's own model can never contain one
  (no-self-overlap blocks a same-op start under an open token), so
  CEGAR progress is untouched while every re-paired model goes free.
  **storage-t i1 re-verified under the sound clause: h1–32 still
  proven UNSAT** — the 0.24 verdict stands, and is now trustworthy.
- **The generalization** (`FF_NO_SAT_LAYERGEN=1` restores): REDUCED
  cores (duration-sum ≥ 0 — their infeasibility never leaned on
  ε-step counts) re-assert at every sound uniform layer shift and
  re-emit into every later horizon's fresh solver, capped at 2M
  generalized literals; full-cycle cores stay observed-placement-
  only (their positivity is ε-financed and context-dependent).
- **The TMS i1 referee:** 377 refutations (was 404) and the ramp now
  reaches h16/h32/h64 budget-capped instead of thrash-bailing at h8
  — deeper progress, same honest no-solve. TMS i2 unaffected;
  match-cellar unaffected; batteries and full suite green.

### Recorded — step 5: branching is a MEASURED NEGATIVE for default-on; the hook stays

The profiling read sanctioned the bet (the wing's sinks are
conflicts-to-verdict — mc's 200k-conflict grind has zero
refutations, storage's wall is deep-proof conflict counts), the
in-tree solver gained the seam the absorption was FOR
(`Vsids::seed` + `Solver::seed_activity`, user→global mapped), and
the measurements killed the default:

- **Forward layer-ordered op seeding (`FF_SAT_BRANCH=fwd`): a real
  1.8× on the deep UNSAT-proof stack** — storage-t h1–32 proofs
  6.8 s → 3.7 s, stable across reps — **and a disqualifying loss on
  the SAT side**: TMS-2011 i2's h16, one STN refutation and a 0.6 s
  solve unseeded, floods the teacher with 247 refutations and caps
  its budget seeded. The early-packed models the gradient steers
  toward are exactly the schedule-infeasible ones.
- `back` measured 2× WORSE than off on the proofs; `uniform` (no
  gradient) worse on BOTH faces — the gradient is the proof-side
  win and the SAT-side poison at once, so no knob setting dominates.
- **Shipped: the hook, opt-in** (`FF_SAT_BRANCH=fwd|back|uniform`,
  default off — stock heap order). Phase saving already defaults
  false, so the sparse-plan polarity was free all along. The 0.26
  residue is named: CEGAR-aware seeding — arm the gradient only on
  proof-shaped horizons, where it measured 1.8×.
- Doc-gate note: the private-intra-doc-link class struck a THIRD
  cycle running and was caught at authoring this time, exactly as
  the 0.24 record asked.

## Phase 4 — the design reads (light, NO code)

The three biggest undiagnosed pots get attribution sittings in the
0.23 style (`benchmarks/metrics/` reports), not levers:

- **transport** (−211/300, six-plus boards — the single biggest
  cross-board bleed; the 0.23 sitting left its fuel-visibility
  signature "recorded-not-diagnosed").
- **floor-tile** (−177/220, six boards — carrying the i11 driver
  casualty alongside).
- **the 2006 metric-time family** (~−340) — including a named-reason
  classification of the ~35 early-exits on the metric-time and
  constraints boards (pathways alone exits early on 11 of 30): a
  decline with a trivial feature cause converts to a Phase 5
  side-dish with a per-fix receipt; everything else gets its
  mechanism named for 0.26.
- **The model-train encoder probe** — the anti-pot's own exit
  clause ("priced at 0 until an encoder probe says otherwise"):
  probe the encoding, record the price, build nothing.
- Output: named mechanisms or named negatives. A read that ends in
  "none-known" is a result, not a failure.

### Recorded — the sitting decoded all three pots, and two of them were hiding bugs

Full report: `benchmarks/metrics/attribution-0.25.md`. The headlines:

- **transport:** the 0.23 "fuel-visibility" framing was a CATEGORY
  ERROR (fuel exists only in the 2008 temporal variant). Real
  mechanisms: the 2014 boards start past the wall (coverage is
  monotone in package count; the engine's line is ~12–14 packages,
  2014 carries 25 everywhere), capacity-blind h, cost-blind first
  plans, LAMA degenerating to goal-count. Levers L1–L3 priced at
  **+8–20 of 211, on 2008/2011/mco only** — the 2014 sequential
  boards are explicitly NOT claimable, in writing, before any code.
- **floor-tile:** plateau confirmed (the 0.22 measurements stand);
  one NEW lever named with a no-code pricing probe — a sound
  dead-end test for irreversible consumption (paint deletes `clear`
  forever; the README's own dead ends are invisible to h⁺). The i11
  casualty verdict stands; `FF_NOV_LAZYH` remains the fallback.
- **metric-time: two bugs, fixed with fixtures, RED first.**
  (1) Zero-duration durative actions were silently SKIPPED
  (`eval_duration`'s `> 0.0` guard) — pathways-metric-time's thirty
  "early exits" were FALSE INSTANT FAILURES on an empty reachable
  space, pinned by `tests/zero_duration.rs`. (2) The relevance mask
  pruned EVERY op on sum-goals it could not read ([TREL] 0/88
  measured; 33/88 after the conservative fallback). Also landed: the
  unsolved temporal path now names its story into the raws' notes —
  this sitting's 35 unclassified early exits can never recur
  unnamed. P2/P4 stay named probes.
- **h-economy: DROPPED with the proof.** The lever is already
  in-engine (GBFS/LAMA evaluate on pop since before this cycle);
  the P6.7 report was never committed — the second lost deliverable
  of 0.24, alongside the parking counted-case read. New house rule
  earned twice over: **a design read is a committed artifact, never
  a conversation.** landscape-2026 bet #4 corrected.
- model-train / onlycraft / parking queue behind the entries sweep
  in `benchmarks/post-entries25.sh`, which also carries the
  sitting's named probes (transport L3, tpp empty-constraints, the
  pathways 30-row re-measure).

## Phase 5 — the side-dishes (light, each with its own receipt)

- **Parking's counted-case PDB** (+1–4 via the sprint slot, i2–i4
  named — the 0.24 banked read, taken as scoped).
- **The 30 s → 60 s tier move** for `ipc5-time` (77/130) and
  `ipc5-metric-time` (54/200) — the last two 30 s boards on a 60 s
  table. The 0.23 tier-move pattern (registry flip + per-row budget
  stamp, `standings.py`) is the template. Provisional +5–20,
  re-priced at the cut; the movement column marks the budget change
  so the rows stay honest.
- **h-economy, pitch-or-drop:** DROPPED (see Phase 4's record and
  `benchmarks/metrics/attribution-0.25.md`) — the retrieval found
  the report was never committed, and the archaeology found the
  lever already in-engine; the old-binary rule prices any residual
  out.
- Any early-exit declines Phase 4 classified as trivial.

## Phase 6 — cut 0.25.0 (two headlines, by design)

The standing template, plus the shape this cycle forces:

- **The cut record carries TWO headline numbers:** the like-for-like
  22-board movement (the instrument every prior cut used) AND the
  new full table with the Phase 1 entries. The denominator grows
  and the total percentage may DROP on entry day — the record says
  so first, loudly, so a bigger honest table never reads as a
  regression.
- The doc gate runs EARLY, not at publish — two consecutive cycles
  caught rustdoc private-item links at the publish gate; not a
  third.
- Pre-flight is the four-crate order now (ferroplan-sat →
  ferroplan → ferroplan-cli → ferroplan-mcp), per the 0.24 publish
  record.
- Wing II's band reads against the field file with the budget gap
  named; under-delivery, if it happens, gets the 0.24 treatment —
  measured shortfall, hypotheses named, never papered over.

## Anti-pots — priced at zero, standing

- **Code at the transport / floor-tile / metric-time walls before
  Phase 4's decode.** No mechanism, no band, no code — the
  ten-negatives ledger was bought exactly this way, and 0.24's
  band-that-missed is the fresh receipt.
- **Temporal delete-relaxation anything:** the ledger is CLOSED
  (ten mechanism-precise negatives across five cycles; what remains
  needs different search, not better relaxation).
- **org-synth i11** (refused twice, the second with a lower-bound
  simulation), **agricola's coin-flip class**, **ricochet**
  (symbolic style-mates own it), **openstacks-opt PDBs**
  (probe-NEGATIVE, mechanism named), **a second classical driver
  swing** (cliff-shaped, no pitch without a new decode), **temporal
  orbit-iso** (DEAD, 1.27× against a 10× bar) — all stand as
  recorded.
- **The 1998–2004 corpora.** "All the years" tempts it; unvetted
  pre-PDDL2.1 formats are unbounded scope. At most a late-cycle
  vendoring probe, no boards promised.
- **New 300 s tiers.** The agile-300s precedent exists and a 300 s
  temporal companion would flatter Wing II — but ~200 instances ×
  300 s of mostly-failures is a day of sweep per board. Deferred,
  and named as the honest way to test the wing against its 1800 s
  field receipts IF Wing II lands its band.

## Deferred, on the record (carried forward)

- ITSAT-style in-CNF timing; incremental assumptions (only if
  horizon-ramp profiling demands them).
- caldera's selectivity-aware route gate; block-grouping's search
  residue (10 rows, field ceiling proven); the or-aware hoist for
  folding p01 (sized, not taken — and folding's 300 s face is a
  MEMORY ceiling, 10 mem-caps + 2 engine kills, not a time wall).
- The proof-track gap as its own future centerpiece candidate:
  onlycraft-opt 2/20 against its own 20/20 satisficing row, barman
  0/14, parking-opt 0/20 — honest node-cap non-proofs, no named
  lever beyond parking's counted case; a design read rides Phase 4
  if the sittings leave room.
- The 0.26 direction question — decided at the 0.25 cut with
  Phase 4's three decodes and Wing II's verdict on the table.
- Cross-mind planning; continuous `#t`; dynamic derived predicates
  — the standing lists.
