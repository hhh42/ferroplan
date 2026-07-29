# ferroplan 0.19 roadmap — the contest cycle

Scoped 2026-07-29 at the 0.18 cut, by direct request: improve the
standings on every entered track, and ENTER the track the project has
always fenced off. The scoping audit ran before the ink dried — the
failure-class columns of `benchmarks/ipc-standings.md` were read
per-class against the raw JSONLs, and the loudest finding reorders
everything: **~120 instances across two modern boards are lost at the
front door** (parse and grounding rejects), before search ever runs.
Cheapest coverage in the project's history goes first; the new track
is the cycle's build; the named engine swings follow.

The user's two locked decisions: (a) full slate INCLUDING the
admissible mode — this is the "improve standing + new tracks" cycle,
the biggest since 0.14-ext; (b) the 2023 agile board gets ONE
official-budget (300 s) sweep at the cut — an entry, not a baseline —
while the 60 s rows stay the iteration measure.

## Phase 1 — the reject audit (fixtures first, mechanisms named)

The audit's receipts, per class:

- **Negative number literals in `:init`** — `(= (x) -5)` fails the
  problem parser (`expected number in init '=', found Dash`). Kills
  sailing-numeric (20/20) and fo-sailing (20/20) outright, and is the
  suspected mechanism behind fo-counters' 19 rejects (i1, which
  solves TODAY, has no negative init). Up to **~59 instances on the
  2023 numeric board** behind what is likely a lexer-level fix.
- **The 2018 zero-grounding trio** — agricola, flashfill, settlers
  (20 each, **60 instances**) return `solved: false` with ZERO
  grounded facts and zero actions, silently. Three separate
  diagnoses owed to the mechanism (these domains lean on modern
  `:action-costs` + numeric + conditional-effect combinations; no
  guessing in the record until each is decoded). A domain the engine
  cannot ground must REPORT WHY — the silent empty-task path is
  itself a bug, whatever else is found.

Discipline: minimized fixture per mechanism BEFORE each fix
(`benchmarks/bench/`), suite-pinned; the honest outcome may be "parse
fixed, instances now time out" — that still moves the class from
reject to search, where the rest of the cycle works. Referee: the
2018-sat and 2023-numeric boards re-swept, reject columns expected
near zero.

## Phase 2 — the admissible mode (the new track)

The fence "seq-opt: out of scope by design (satisficing planner)"
comes down. The corpus is already local: 14 sequential-optimal
variants in ipc-2014 plus 32 optimal variants across 2008/2011.

- **v1, honest and small**: `Mode::Optimal` (`--mode optimal`) — A*
  over the existing packed task with an ADMISSIBLE heuristic ladder:
  h^max first (already computable from the relaxation machinery),
  blind as the degenerate floor. Unit-cost and `:action-costs`
  metrics; optimality is a PROOF, so the mode never returns an
  incumbent it cannot certify (anytime-with-bound reporting is a
  satisficing feature, not this).
- **The stretch, memo-ranked**: classical **LM-cut** — the landscape
  memo's optimal-side family (NLM-CutPlan's numeric variant swept
  the 2023 numeric-optimal track; the classical original is the
  proven core). Taken only if v1 lands clean; h^max enters first
  regardless.
- Fixtures first: a ladder of instances with KNOWN optima (the 2014
  corpus carries reference costs where its `*-opt` archives do;
  otherwise the vendored costs subset's small instances, optima
  established by exhaustion at tiny scale). A claimed optimum that a
  reference beats is a red row, first-class.
- First entries: 2014 seq-opt, 2008 seq-opt, 2011 seq-opt at the
  standard 60 s/30 s tiers — standings rows with expansion counts
  and proof rates. Losing honestly to 15 years of optimal-planner
  engineering is expected and recorded; entering is the point.

## Phase 3 — the numeric heuristic swing (named since 0.17)

The 2023 numeric board after Phase 1 still holds ~121 timeouts — the
landscape memo's bet #2 (subgoaling / AIBR-class numeric heuristic,
replacing the current fixed-point numeric relaxation where it
degenerates). Judged on the numeric board's timeout column and the
village's think benchmarks (the game cares about numeric gradients
too — stock/money goals are exactly this shape). Measured win or
recorded negative, per house rule.

## Phase 4 — the mem-cap class (93 + 40)

2023-numeric carries **93 mem-caps**, 2014 classical ~40 more — the
modern instances' grounding transients against the per-job cap.
Diagnose the top offenders BY CLASS first (fact-space? op
enumeration? numeric side tables?) with `FF_RES_DEBUG` attribution
before touching anything — 0.9's lesson stands (the wall was the
grounder, not the search, and compaction was the fix). Whatever
mechanism the attribution names gets the 0.9 treatment: a targeted
structural fix with classical paths bit-identical, never a cap tune.

## Phase 5 — riders (small, evidence-backed)

- **Novelty default-on under `FF_TIME_LIMIT`** — 0.18's referee
  measured +4/−0 for the gated rung; the recorded candidate ships
  unless the full-board referee finds a tax the probe boards missed.
  Unset-budget behavior stays byte-identical (the rung remains
  opt-in without a declared wall).
- **State-dependent duration drift** (0.18's refuted-hypothesis
  find): re-evaluate duration expressions at emitted start times in
  the ε-separation pass — or veto ε-shifts across writes to
  duration-read fluents. Witnesses map-analyzer i17/i18/i20; the
  2014 temporal board's last 3 VAL-reds.

## Phase 6 — cut 0.19.0

The standing template: every touched board re-swept against the
final binary, PLUS the locked official-budget entry — 2023 agile at
the competition's **300 s** (one sweep, at the cut only; the 60 s
rows stay as baselines). New standings sections for the optimal
tracks. Records complete, full pre-flight, finish in main; the user
publishes.

## Deferred, on the record (carried forward)

- The h-surgery bet (end-gated interval credit) — the village
  gather-spam witness file stands.
- Lifted/lazy grounding — watch item; Phase 4's attribution may
  promote it.
- VAL-side red clusters (drone-numeric, data-network-2018 domain
  parse rejects) — runner class, revisited on a VAL upgrade.
- IPC-5 complex-preferences / timed modal operators, cross-mind
  planning, continuous `#t`, dynamic derived predicates,
  fixpoint/stratified unification — unchanged from the standing
  lists.
