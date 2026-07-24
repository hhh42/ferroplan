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

## Phase 2 — raise what the audit says is cheap (measured, per raise)

Two raises are COMMITTED by direct request; the rest are ordered by
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
- **IPC-5 time / metric-time**: five cycles of temporal work
  (required concurrency, ε-ordering, the invariant guard, orbits)
  have never been pointed at these. Expectation: real coverage from
  just showing up; walls named where not.
- **Preference-quality follow-ups** (IPC-5): only if the audit shows
  specific instances within reach of the existing optimizer knobs
  (budget, selection) — no new optimizer machinery this cycle.
- **Feature gaps stay gaps** unless one is BOTH cheap and
  standings-relevant; the four timed modal operators have survived
  three deferred lists and need a better reason than a table row.

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
