# ferroplan 0.11 roadmap — the guidance cycle

Scope settled 2026-07-19. 0.10 closed every non-guidance wall class
(memory, grounding, semantics, scheduling, validation); what remains
standing failed for ONE reason across every classification: the
heuristic. transport11 — h^FF is the delete-relaxation floor and
throughput is exonerated; storage11-t — 3 M nodes, live heap,
`helpful → 0`; model-train — `avg_helpful 0.0`, every pass exhausts;
TMS — interleaving scale with no gradient; floor-tile/visit-all/sokoban
seq-sat tails — the classic h^FF-blind families. 0.11 attacks guidance
directly, plus the game-embedding think API (shovel-ready per the
recorded design answers, independent of the guidance work, and better
guidance shrinks think budgets anyway).

Baselines (0.10.0 binary): tempo-sat **387/630** at 30 s / 3 jobs, all
VAL-validated — storage11-t 0/20, TMS 0/20, match-cellar 6/20,
parc-printer-t 18/30 + 7/20, sokoban-t 10/30 + 2/20, floor-tile-t
3/20, turn-and-open 1/20 at 60 s. Seq-sat: transport11 0/20,
floor-tile11 5/20, visit-all11 8/20, sokoban 21/30 + 11/20.

## Phase 1 — the temporal LAMA rung

The strongest evidence-backed transfer available: landmark counting +
preferred-operator boosting was the 0.9 breakthrough on exactly this
plateau profile (barman 0/4 → 4/4), and the decision-epoch search has
NO fact-landmark guidance — its phase-1 key carries FF h plus only the
numeric-threshold term.

- `TNode` gains a landmark-accepted bitset (the `lama.rs` shape);
  `landmarks_for` (already generalized over (start, goal) in 0.9)
  seeds the pass; the phase-1 key adds an unaccepted-landmark term.
- Preferred-operator boosting via a second open list (the node's
  `helpful` set is already computed in the prune pass) with the
  lama-style mixed deterministic batch.
- Phase-2 complete passes untouched — completeness is theirs, so the
  rung can only add solves. `FF_NO_TLAMA=1` restores the 0.10 search
  bit-for-bit.
- Acceptance: measured coverage on storage11-t / TMS / match-cellar /
  parc-printer-t / sokoban-t moves or carries a recorded diagnosis;
  sentinels (crew, pegsol, elevator, openstacks-t) unchanged; every
  new solve VAL-validates; t1 ≡ t8.

## Phase 2 — helpful-action drift repair

storage/model-train show helpful sets thinning to ZERO away from init:
FF's strict filter (relaxed-plan ops at layer 0, really applicable)
starves, and the prune pass degenerates to full scans. Fast Downward's
laxer preferred-operator definition keeps a set alive.

- ONLY when the strict set is empty, fall back to: applicable ops
  whose add intersects the relaxed plan's selected facts (any layer).
  Strict-nonempty nodes are bit-identical by construction, fencing the
  classical sentinels.
- `FF_STRICT_HELPFUL=1` restores. Measured on storage / model-train /
  turn-and-open (prune-pass re-armed?) AND the classical sentinels
  (gripper/blocks/barman eval counts must not move — their strict sets
  are nonempty).
- Acceptance: measured wins or a recorded negative; no sentinel drift.

## Phase 3 — one bounded swing at a richer classical h

transport11 needs a different gradient, not a faster one. The bounded,
honest version: an unaccepted-landmark-count TERM in the classical
ladder's best-first ordering (not just the separate LAMA rung),
`SearchCfg`-weighted, default off unless measured.

- Measured against transport11 / floor-tile11 / visit-all11 and the
  full costs subset for regressions. House rules: measured-win-or-
  recorded-dead-end; this phase is explicitly allowed to conclude
  NEGATIVE without shame — the record is the deliverable.

## Phase 4 — the budgeted-think API (game track)

The recorded design answers (real-time, episodic stop-and-think,
plan-on-demand): a think is a BOUNDED call — eval budget + memory
target — on a long-lived `Session` (ground once, replan many).

- `Options` grows an explicit think-budget surface (evals cap exists;
  add a node-memory target that plumbs to the existing deterministic
  caps: `node_cap_for` / `temporal_node_cap`).
- `Session::replan` honors the budget on every path (resolve ladder,
  portfolio, temporal); a capped think returns its incumbent or an
  honest budget-exhausted verdict, never wall-clock nondeterminism.
- An `examples/` episodic-replan walkthrough (think → follow → world
  drifts → rethink) as the living doc for the game embedding.
- Acceptance: a think with a tiny budget returns fast and
  deterministically at any thread count; the example runs in the
  suite (`--examples` build) and the budget knobs are documented.

## Phase 5 — 0.11.0 release mechanics

CHANGELOG `[0.11.0]`, workspace bump 0.10.0 → 0.11.0, README refresh,
`rustup update stable` first, full gate (fmt / clippy `-D warnings` /
suite), scoreboards refreshed with the release binary where phases
moved them, main fast-forwarded and publish.sh-ready (the user runs
publish.sh).
