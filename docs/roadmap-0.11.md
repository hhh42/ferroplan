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

## Recorded — Phase 1 (2026-07-20): MEASURED NEGATIVE, ships opt-in

Three variants, each measured at the 30 s baseline, none positive:

1. **Key term in the pruned pass**: the landmark gradient FIGHTS h
   where they disagree — crew-planning 50/50 → 36/50, the 0.10
   sokoban-t/floor-tile-t gains lost 6 (turn-and-open did gain 1→2/20,
   the only positive any variant showed).
2. **Unbounded dedicated rung**: crew/floor-tile restored,
   parc-printer08-t +1 — but the failed rung burned a full node-cap
   slice and sokoban-t stayed −3 at the wall.
3. **Bounded rung (50k nodes)**: zero new coverage anywhere; the
   parc-printer +1 needed more than the cap; sokoban-t still −3
   (borderline solves near the 30 s wall cannot afford even a
   seconds-scale failed bet).

Diagnosis, recorded: snap tasks' fact landmarks are dominated by
RUNNING-token chains that accept in path order regardless of choices —
the unaccepted count carries almost no branching signal on these
walls, unlike barman's classical landmarks (deep resource-chain
ordering), which is what made the 0.9 rung win. The machinery ships
opt-in (`FF_TLAMA=1`; default is 0.10 behavior bit-for-bit) with the
landmark supply counts in the debug dump (`[tsearch] tlama: N`).

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

## Recorded — Phase 2 (2026-07-20): MEASURED NEGATIVE, ships opt-in

The mechanism was real but sharper than the roadmap guessed:
`relaxed_helpful` already has a last-resort fallback, so the RAW set
is rarely empty — the drift is the temporal `eval_node`'s
Start|Classical FILTER emptying a nonempty set when relaxed plans lead
through agenda-fired END ops (storage's stored helpful averaged 0.0).
The repair (`helpful_needed_adders`: applicable ops adding a fact the
relaxed plan still needs) re-armed the sets (storage holds ~1.0–1.3
deep into the search) but RESTRICTS block (a) on exactly the nodes
where the empty set previously meant a full scan — zero new solves
anywhere. Ships opt-in (`FF_LAX_HELPFUL=1`); default is the 0.10
pruned pass.

**Measurement-conditions caveat, recorded**: today's sweeps read
sokoban-t at 8/30 + 1/20 vs the scoreboard's 10/30 + 2/20 — an A/B of
the 0.10 binary against today's box proved it ENVIRONMENTAL (i3 solo:
35.5 s on the 0.10 binary today vs its recorded 24.7 s under 3-job
contention on the scoreboard day; the current binary is within ~5% of
0.10). Wall-clock scoreboards inherit box variance; the borderline
band (solves within ~5 s of the wall) flips with it. Verdicts above
are unaffected (no variant showed gains anywhere; crew's 50/50 →
36/50 under the key-term was margin-scale, not borderline).

## Phase 3 — one bounded swing at a richer classical h

transport11 needs a different gradient, not a faster one. The bounded,
honest version: an unaccepted-landmark-count TERM in the classical
ladder's best-first ordering (not just the separate LAMA rung),
`SearchCfg`-weighted, default off unless measured.

- Measured against transport11 / floor-tile11 / visit-all11 and the
  full costs subset for regressions. House rules: measured-win-or-
  recorded-dead-end; this phase is explicitly allowed to conclude
  NEGATIVE without shame — the record is the deliverable.

## Recorded — Phase 3 (2026-07-20): MEASURED NEGATIVE, the record is the deliverable

`FF_CLM=3` vs default at 60 s: transport08 15/30 IDENTICAL solve sets
(and transport11 0/20 both — the landmark count adds no gradient where
h^FF is the floor); visit-all 7/20 identical (EHC solves these; the
term lives on the best-first fallback and never fires — plumbing
confirmed); floor-tile WORSE with the term (2/20 vs 5/20; builds
contended with that half of the sweep so the magnitude is suspect, the
direction is not). Third guidance transfer, third clean negative — the
cycle's conclusion is itself the finding: **transport/floor-tile-class
walls need a genuinely different heuristic (red-black / semantic
landmarks over numeric structure), not reweightings of what exists.**
`FF_CLM` stays as the experiment hatch; defaults bit-identical.

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

## Recorded (cycle close, 2026-07-20)

- **Phase 4 SHIPPED**: `Session::replan_budgeted(max_evaluated,
  memory_mb)` + `SearchCfg.node_bytes_target` through the per-node
  byte model; the determinism test caught EHC's op-scaled cap ignoring
  `max_eval` (a 1-eval think solved anyway) — the caller's budget now
  bounds EHC too. `examples/game_think.rs` is the episodic
  walkthrough. Suite 144/0.
- **Phase 5**: versions 0.10.0 → 0.11.0, CHANGELOG/README refreshed,
  latest stable confirmed (1.97.1), full gate green. Default-path
  behavior unchanged from 0.10.0 (all experiments hatched off), so the
  0.10.0 scoreboards remain current — no refresh needed.
- **The cycle's finding**: three principled transfers, three clean
  negatives with recorded diagnoses. The next guidance lever must be a
  genuinely different h — red-black planning or semantic landmarks
  over numeric structure — not reweightings of existing signals.
