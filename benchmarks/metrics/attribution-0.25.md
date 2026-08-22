# The 0.25 attribution sitting — the three biggest pots, decoded

Phase 4 of docs/roadmap-0.25.md: design reads, not levers — except
where a read found a BUG, which got fixed on the spot (fixtures
first). Sources: the air24 raws and conditions, the corpus domains,
the prior sittings (temporal-attribution-0.22/0.23, numeric-twins-
0.23, dockets-0.23), and targeted sub-minute probes. All boards
quoted are the 0.24 cut.

## TRANSPORT (−211/300 across six-plus boards)

One domain file, byte-identical across 2008/2011/2014. Three
actions, `:action-costs`, capacity as a PREDICATE CHAIN — and **no
fuel anywhere in the sequential family**: the 0.23 sitting's
"fuel-visibility signature" belongs to the 2008 TEMPORAL variant
(which has `fuel-left/refuel`) — the sequential framing inherited a
category error, corrected here.

Mechanisms, ranked:

1. **The 2014 boards start past the wall.** Coverage is monotone in
   package count and nothing else: the engine's wall sits at ~12–14
   packages (2011 solves exactly i4=12, i5=14, at 59.59 s and
   59.88 s — AT the line), and every 2014 sequential instance
   carries 25. All 60 rows are out of range, not failing.
2. **Capacity is invisible to h_FF** (the predicate chain relaxes to
   "every truck holds everything"), so h ≈ 2·undelivered — flat
   across all 4^25 assignments. Receipt: `ehc_fell_back` on every
   non-trivial solved row.
3. **The first-plan search is cost-blind** — `relaxed_costed` exists
   but only the post-hoc anytime sweep uses it; all 200–794 roads
   tie during search (2008 receipts: first plan ~2× the swept cost).
4. **The LAMA rung degenerates to goal-count here**: fact landmarks
   only; every `(at pkg goal)` has |vehicles| first-achievers with
   no common precondition, so backchaining stops at the goals.
5. mco coverage rises with cores alone (t2 4 → t4 5 → t8 7 of 20) —
   enumerating a plateau, not finding structure.

Levers, priced humbly: L1 cost-augmented h in the FIRST-plan rung
(machinery exists; cheapest); L2 disjunctive landmarks over carriers
+ capacity-aware tiebreak (fixes 2 and 4 together; the landmark-
count introspect on 2011 i4 — predicted == 12 == goals — prices it
before any code); L3 rung-budget re-slice (novelty-light's 300k pops
+ LAMA's 25% wall slice buy nothing here; the env-only probe is in
post-entries25.sh). **Honest band +8–20 of 211, concentrated on
2008/2011/mco — the 2014 sequential boards are NOT claimable on any
of these levers**, and saying otherwise would be the 0.24
band-that-missed again.

## FLOOR-TILE (−177/220 across six boards)

The relaxation blindness, exactly: painting deletes `(clear)`, so a
painted tile is permanently unrepaintable AND impassable — h⁺ keeps
`clear` forever and degenerates to a goal counter, flat across the
movement/ordering subplan that is the actual work, and blind to the
domain's own README-documented dead ends ("painting tiles behind
make the search reach a dead end"). Confirmed plateau, not
hypothesis: the 0.22 sitting measured best_h flat, dedup 0.0%,
b_blocked 0, both eras identical; the optimal face needs 594k
expansions for the 5×3. The coverage cliff sits at ~28 tiles.

- The i11 casualty stands decoded (dockets-0.23): the 0.22 novelty
  rung caps at 400k pops on this family and best-first dies after;
  `FF_NOV_OLD=1` solves i11 in 35.7 s under load. `FF_NOV_LAZYH` is
  the pre-registered fallback; the guard was declined for re-routing
  risk and that verdict stands.
- **New lever, named: a sound dead-end test for irreversible
  consumption** — prune states where an unpainted goal tile is not
  `clear` (or unreachable-clear). Constituency: all six floor-tile
  boards, likely sokoban corners. The pricing probe needs no search
  change: instrument the fraction of expanded nodes failing the test
  on a solved row. A 0.26 candidate with its probe attached.
- SAT reach: realistically ~1.5 ops/layer (one `clear` chain), plans
  36→~160 ops — horizons 64–128 only after paying the full ramp on a
  ~1000-op ground set. With LPG-td at 20/20 here, the 0.24 +2–8 band
  reads optimistic. Three guidance transfers have measured negative
  at this wall; nothing on record has ever moved the family.

## THE 2006 METRIC-TIME FAMILY (~−340)

The shape: fluent-valued durations (tpp's genuinely state-dependent
— time-blind in the TRPG tables by design), zero-duration gate
actions (pathways), metrics mixing fluents with total-time, purely
NUMERIC goals (pathways' a SUM on the LHS).

**The sitting found two bugs, both fixed with fixtures:**

1. **Zero-duration durative actions were silently SKIPPED**
   (`eval_duration`'s `> 0.0` guard): pathways-metric-time gates
   everything behind `(= ?duration 0)` `choose`/`initialize`, so all
   30 instances "exhausted" an empty reachable space in milliseconds
   — the board booked thirty FALSE instant failures as early exits.
   Fixed (`>= 0.0`, both the static and state-dependent sites);
   pinned by tests/zero_duration.rs (RED first). Constituency:
   pathways-metric-time + pathways-preferences-complex. i1 now runs
   a real 40 s search; whether rows convert is measured by
   post-entries25.sh's 30-instance probe.
2. **The relevance mask lied on unreadable goals**: a sum-goal
   matches no canonical threshold, both seed sets came up empty, and
   the mask pruned EVERY op ([TREL] sound 0/88 measured) — the
   unmasked backstop pass kept completeness, but half the ladder
   burned on empty-masked passes. Fixed: an unreadable goal now
   seeds every fluent it reads (conservative superset; [TREL]
   33/88 after).

Also landed: **the unsolved temporal path now names its story**
("stopped at the wall" vs "exhausted budgets with N s left") into
the raws' notes — the 35 unclassified early exits of this sitting
can never recur unnamed. Remaining named candidates: P2 (arm
`charge_pre_num` on temporal groundings behind a hatch — the a2
chained charge that converted pathwaysmetric-2023n i2 is INERT on
the 2006 boards by construction, `ground.rs` stratified-entry rule);
P4 (the tpp empty-constraints riddle — probe in post-entries25.sh);
the state-dependent-duration time-blindness stays none-known.

## H-ECONOMY (deferred evaluation) — DROPPED, with the proof

The P6.7 report is NOT in git (it lived only in the 0.24
conversation — the second lost deliverable of that cycle, with the
parking counted-case read; reads must be committed artifacts from
now on). The archaeology that replaced it: **the lever is already
pulled** — the default GBFS and LAMA rungs evaluate on POP with
successors keyed on parent h, and the 0.22 novelty driver removed
per-pop `relaxed_helpful` besides; the only eager evaluator left is
EHC, where successor h is the decision variable. Any residual claim
reallocates per-node budget and therefore needs the old-binary
referee (the 0.21 rule), against a 0.17 precedent of −51 rows from
per-instance reasoning. docs/landscape-2026.md bet #4 corrected to
ALREADY-IN-ENGINE.

## MODEL-TRAIN / ONLYCRAFT / PARKING — queued with receipts pending

The encoder probe (model-train), the contended re-check (onlycraft),
and the parking counted-case re-derivation run via
benchmarks/post-entries25.sh once the entries sweep banks — the box
owns the answers, not this sitting.
