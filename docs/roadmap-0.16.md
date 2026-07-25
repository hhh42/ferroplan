# ferroplan 0.16 roadmap — the standings cycle

Scope set 2026-07-24, mid-0.15-cut, by direct request: refocus on the
three competitions this project measures itself against — **IPC-5
(2006), IPC-6 (2008), IPC-7 (2011)** — reevaluate where ferroplan
REALLY stands on each, raise the standings where the audit says a
raise is cheap and honest, and leave the whole picture properly
documented in one place instead of scattered across cycle records.
Committed priorities from the same conversation: **the IPC-7
multi-core track gets entered**, **IPC-6 is the named competition for
standings raises**, and the IPC-5 OVERALL standing gets reconstructed
and made well-understood — the remembered "strong second against
SGPlan" is REAL and on file (the simple-preferences board is
reference-scored from the official archive and ferroplan beats
SGPlan5 on openstacks p04–p08); what was never completed is the
QUALITATIVE board's reference columns, blocked because the official
archive host is outside this container's network allowlist (both
graft paths are documented in the board itself and work from a
normal dev machine — a user-side unblock, flagged).
**[RESOLVED mid-phase, 2026-07-24: the user hand-retrieved
`IPC5-results.tgz` (the Wayback held only a 301 for it — the live
redirect still served the bytes) and it is now vendored at
`benchmarks/IPC5-results.tgz`. The qualitative graft is DONE — and
its first verdict (12W/3T/23L, computed against the board's
0.7/0.8-era ferroplan column) forced a re-measurement that
became the cycle's biggest raise: the CURRENT engine scores
**24W/4T/10L vs SGPlan5**, winning rovers/storage/tpp outright —
see the board for the full correction narrative. The archive also
unlocks reference-scored quality columns for the 2006 audit
tracks.]**

What the records already admit, going in:

- **IPC-5 is only part-entered.** The preference tracks are scored
  (simple: curated vs the official field; qualitative: 38/40,
  self-scored with the reference gap honestly recorded) — but the
  2006 corpus in-tree carries `propositional`, `time`, `metric-time`,
  and `constraints` variants across openstacks / pathways /
  pipesworld / rovers / storage / tpp / trucks that have NEVER been
  swept, and pipesworld appears in no cycle record at all. The
  temporal and constraints engines have matured five cycles since
  those directories were fetched.
- **IPC-6/7 are covered on two tracks of four-ish.** seq-sat (580)
  and tempo-sat (630) have standing scoreboards refreshed each cut;
  net-benefit was validated on a 16-instance subset, never the full
  track; **the IPC-7 sequential multi-core track was never entered**
  — for a planner whose core claim is deterministic data-parallelism,
  that is the strangest empty row on the sheet. Optimal tracks are
  out of scope by design (satisficing planner) and should say so in
  the standings table rather than by omission.
- **"Where we really are" means scored, not just covered**: the IPC
  quality formula against best-known/reference costs where official
  data exists (the simple-preferences scoreboard already does this;
  nothing else does), coverage-only where it does not, and the
  distinction marked.

## Phase 1 — the standings audit (the corpora are the fixtures)

Enumerate every deterministic track of the three competitions and
close the measurement gaps:

- Sweep everything never swept: the IPC-5 propositional /
  time / metric-time / constraints variants (standard budgets: 60 s
  classical, 30 s temporal, jobs 3, VAL on everything), the full
  IPC-6 net-benefit track, and the IPC-7 seq-mco track at t≥2 (its
  competition rule — wall-clock with all cores — is the one place
  wall-clock benchmarking is the honest currency; determinism per
  thread count still holds).
- Classify every failure: FEATURE GAP (named constructs — e.g.
  timed modal operators in trucks-time-constraints-TIL, complex
  preferences' modal ops), SEARCH WALL (named, with the probe eyes
  where cheap), or BUDGET EDGE (solo-checked). The 0.14/0.15
  discipline verbatim: mem-cap deaths tracked separately from engine
  verdicts.
- IPC-5 standing reconstruction: the simple-preferences board is the
  reference-scored anchor; the qualitative board's reference graft is
  attempted (and honestly re-flagged if the archive stays
  unreachable from this container); the never-entered 2006 tracks
  get their first sweep so "overall IPC-5 standing" finally means
  every track, not one.
- Deliverable: **`benchmarks/ipc-standings.md`** — one table per
  competition: track / entered? / coverage / quality score (with
  reference source named) or "coverage-only" / gaps by class. The
  honest sentence per competition at the top. This document is the
  phase's bar; the sweeps are its inputs.

### Recorded — every gap measured, the deliverable scripted, one verdict flipped

The audit ran everything it promised, all against the 0.15.0 binary
at standard budgets with VAL on every plan; the deliverable is
GENERATED (`benchmarks/standings.py` — in-flight sweeps fenced out by
their missing `.md` sibling), refreshed at every cut.

**First-ever numbers on the never-swept tracks:**

- IPC-5 propositional **354/450** — with the first archive-backed
  quality column: plan length vs best-of-field (IPC-4 champions
  included), **52W/48T/164L, mean quality 0.91** over 264 scored.
  96 timeouts, zero rejects — the parser/gate eats all of 2006.
- IPC-5 time **76/130** (54 timeouts) — real coverage from showing
  up, as predicted. Quality column blocked by a named RUNNER debt:
  makespan is not recorded (the track's currency).
- IPC-5 metric-time **55/200** (101 timeouts, 28 mem-cap, 16
  rejects) — the weak track, numeric-temporal shaped, exactly where
  the 0.15 model-train last-mile mechanism points.
- IPC-5 constraints **5/120** — 100 instant rejects: the four timed
  modal operators, rejected BY NAME as designed; the deferred-list
  entry now has a coverage price on it (a fifth of the track).
- IPC-6 net-benefit, full track, refreshed: **217/270** (41
  timeouts, 12 mem-cap).
- **IPC-7 seq-mco ENTERED** (jobs 1, all cores, wall-clock per the
  competition rule; 4-core box): **t2 193/280, t4 189/280, t8
  193/280** — oversubscription (t8) nets to a wash against t2; the
  t4 dip is exactly 5 floor-tile engine-rejects (i7–i12 class, a
  thread-scaled memory signature; 1 recurs at t8) — solo-checked in
  Phase 2.

**The verdict the archive flipped** (full narrative on the
qualitative board): the reference graft first read 12W/3T/23L vs
SGPlan5 — measured against the board's 0.7/0.8-era ferroplan column.
Re-measured on today's defaults: **24W/4T/10L — ahead of the track
winner**, rovers 7–1 / storage 6–2 / tpp 6–1–1 won outright,
openstacks split, trucks trailing (1–3–2 + p07/p08 600 s no-runs).
The committed "qualitative-tpp raise" thus landed chiefly by honest
re-measurement; the residuals (tpp p07 57-vs-49, trucks p04/p06
quality, trucks p07/p08 no-runs) carry to Phase 2.

**Standings infrastructure shipped alongside** (Phase 3 pulled
forward): `standings.py`, `ipc-standings.md`, the book's Standings
chapter (live-included tables), README's Benchmarks reorganization,
and the vendored official archive with provenance
(`benchmarks/IPC5-results.tgz`, ATTRIBUTION.md).

## Phase 2 — raise what the audit says is cheap (measured, per raise)

Three raises are COMMITTED by direct request; the rest are ordered by
the audit, not appetite — each ships as a measured win or a recorded
negative, standard budgets, zero-regression rule intact:

- **COMMITTED — IPC-7 seq-mco**: enter the track — t2/t4/t8 rows,
  the data-parallel evaluation story measured under competition
  rules (wall-clock with all cores is the honest currency there;
  per-thread-count determinism still holds).
- **COMMITTED — IPC-6 standings raises**: the audit names the
  cheapest IPC-6 gaps (going in, the records suggest: transport08's
  seq-sat tail, the woodworking mem-cap class, model-train-t 0/30
  with its fresh last-mile-numeric mechanism from the 0.15 probe,
  sokoban-t-08's tail, and the full net-benefit track beyond the
  16-instance subset) — the two or three with the best
  evidence-per-effort get the swings.
- **COMMITTED (added mid-phase, 2026-07-24, by direct request) —
  qualitative-tpp selection extension**: the archive graft proved
  the tpp sweep (0–8 vs SGPlan5) is the SAME selection-problem
  shape 0.6 already closed on simple preferences, plus ordering
  couplings: the big-weight families are per-goods end-state
  choices (`p4A`/`p5A`/`p7A`, quantified `exists`-level goal
  preferences) and `sometime-before` level-orderings
  (`p6A/B/C` — goods5 before goods4, weight 13 × level
  groundings), and SGPlan5's archived p05 plan (metric 27, 86
  actions, goods5 ferried FIRST) is a textbook of both. The
  extension: let the 0.6 selection layer see quantified /
  monitor-compiled preference families and emit its choice as
  ORDERED hard-goal contracts; `tresolve`'s contract decomposition
  already executes ordered subgoals — selection chooses what and
  in what order, tresolve reaches it. Probe first (attribution
  against SGPlan's simulated end state), then the machinery;
  measured on the now-reference-scored qualitative board. This is
  the one exception to the no-new-optimizer-machinery fence below,
  carved by direct request.
- **IPC-5 time / metric-time**: five cycles of temporal work
  (required concurrency, ε-ordering, the invariant guard, orbits)
  have never been pointed at these. Expectation: real coverage from
  just showing up; walls named where not.
- **Preference-quality follow-ups** (IPC-5, beyond the committed
  tpp extension): only if the audit shows specific instances within
  reach of the existing optimizer knobs (budget, selection) — no
  OTHER new optimizer machinery this cycle.
- **Feature gaps stay gaps** unless one is BOTH cheap and
  standings-relevant; the four timed modal operators have survived
  three deferred lists and need a better reason than a table row.

### Recorded — two raises delivered, one dissolved, one honest negative

- **seq-mco: ENTERED** (the Phase 1 sweeps double as the Phase 2
  delivery): t2 **193/280**, t4 **189/280**, t8 **193/280** —
  first-ever rows, competition wall-clock rules, 4-core box.
  Oversubscription (t8) nets to a wash; the t4 dip decoded to
  RUNNER fork failures under memory pressure (floor-tile i7–i12
  consecutive-spawn cluster; solo: an ordinary timeout, capped or
  not) — `ipc67.py` now retries a failed spawn after a breather and
  classifies persistent ones `spawn-fail` (environmental), its own
  standings class.
- **The qualitative-tpp selection extension: DISSOLVED by its own
  probe** — the standing pattern (0.15 Phase 1) repeats: the probe
  rewrote the phase. The extension was committed against the
  board's 0-8 tpp rout; the first `[sel]` probe showed selection
  ENGAGING (monitor facts included) and the metric landing at 13,
  not the board's 24 — the board was 0.7/0.8-era stale. The full
  re-measurement (Phase 1 record) took tpp to **6W/1T/1L** and the
  overall board to **24W/4T/10L vs SGPlan5** with ZERO new
  machinery: the 0.5.1 barrier default, 0.6 selection layer, and
  0.10 DNF static resolution had already done the work, unmeasured
  for seven cycles. No extension is built this cycle. Residuals,
  named: tpp p07 (57 vs 49), trucks p04/p06 quality (2 vs 0,
  4 vs 3), and trucks p07/p08 — which the probe demoted from wall
  to BUDGET-BOUND: p07 solves on pure defaults at ~1100 s with
  metric 6 (winner: 3); the board keeps its 600 s convention and
  the ⁶ rows stand, with this data point on file. The quality
  residue is the shared-timeline scheduling shape recorded since
  the 0.6 simple-preferences forensics.
- **IPC-6 raises: an honest negative.** The audit priced every
  gap, and every large one sits behind a fence this project has
  already named and deliberately deferred: model-train-t 0/30
  (numeric last-mile — the 0.17 numeric-heuristic bet),
  transport-t 4/30 and transport-seq 16/30 (the four-negative
  route-structure fence), sokoban-t 10/30 (the fixpoint tie-break
  lottery), woodworking/elevator mem-caps (environmental class).
  The one unfenced candidate — crew-planning net-benefit 10/30,
  never probed — probed to a GENUINE search wall: i15 grinds past
  240 s solo (4× the sweep budget) with no restart/resource debug
  events, a pure search-space grind. No cheap IPC-6 raise exists
  at standard budgets; the raises route through the 0.17 engine
  bets, with the audit as their targeting data. Zero engine code
  changed this cycle — the 0.15.0-binary scoreboards ARE the final
  ones.

## Phase 3 — documentation as the deliverable

- The book gains a **Standings** chapter: the three competitions,
  the per-track tables from `ipc-standings.md`, the honest scoring
  caveats (self-scored vs reference-scored), and links to every raw
  scoreboard. README's Benchmarks section reorganizes around the
  three competitions and links the chapter; scattered per-cycle
  claims elsewhere in the book get pointed at the one table.
- Per RELEASING.md discipline: regenerating the standings tables is
  scripted (`benchmarks/standings.py` or equivalent), not hand-run
  prose — scoreboards defend themselves.

## Phase 4 — cut 0.16.0

The standing cut template (0.14-ext lineage): CHANGELOG / README /
book refresh, both standing scoreboards plus WHATEVER NEW TRACKS
Phase 1 established re-swept against the final binary with A/B
attribution, casualties named and solo-checked, bazaar-thinks
re-emitted, full pre-flight including `--all-targets` clippy and the
wheel build, finish in main; the user publishes.

## Deferred, on the record (carried forward)

- Optimal tracks (IPC-6/7 seq-opt): out of scope by design — a
  satisficing planner; the standings table says so explicitly.
- The h-surgery bet (end-gated interval credit), transport's
  route-structure fence, cross-mind planning, belief-aware dormancy,
  continuous `#t` effects, dynamic derived predicates,
  fixpoint/stratified unification: all unchanged from the 0.15 list.
