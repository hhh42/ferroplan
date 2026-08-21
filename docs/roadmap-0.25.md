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

## Phase 5 — the side-dishes (light, each with its own receipt)

- **Parking's counted-case PDB** (+1–4 via the sprint slot, i2–i4
  named — the 0.24 banked read, taken as scoped).
- **The 30 s → 60 s tier move** for `ipc5-time` (77/130) and
  `ipc5-metric-time` (54/200) — the last two 30 s boards on a 60 s
  table. The 0.23 tier-move pattern (registry flip + per-row budget
  stamp, `standings.py`) is the template. Provisional +5–20,
  re-priced at the cut; the movement column marks the budget change
  so the rows stay honest.
- **h-economy, pitch-or-drop:** retrieve the P6.7 deferred-list
  report from the 0.24 phase commit, 0.17-history-proof it
  (old-binary column mandatory), and either pitch it at the
  2018/2023 residue (82/240, 37/140) or drop it with the proof on
  record.
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
