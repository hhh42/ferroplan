# ferroplan 0.12 roadmap — the game cycle

Scope settled 2026-07-20. 0.11 closed the guidance question honestly
(three transfers, three recorded negatives: the remaining IPC walls
need research-grade heuristic work, not reweightings), and the
budgeted-think API exposed exactly what the ENGINE'S ACTUAL CUSTOMER —
the game from STATUS.md's recorded design answers — still cannot do.
0.12 is the release where the engine starts serving its purpose: the
corpus was the measuring stick; this cycle the stick has done its job.

The recorded design answers this cycle serves:

- **Real-time with episodic stop-and-think**: an agent thinks at REST
  POINTS (a bounded `replan_budgeted` call), then FOLLOWS the plan;
  the world may drift mid-follow.
- **Genuine concurrency exists**: durative actions and the
  decision-epoch machinery are load-bearing for the game — yet
  `Session` REJECTS temporal domains today. The think API only works
  on classical worlds.
- **Barter economy, any item to any item**: item×item exchange actions
  make GROUNDING SCALE the binding constraint — the same lever as the
  recorded elevator-11 tail (the ~4 GB stratum-1 enumeration
  transient).

## Phase 1 — the temporal Session

`Session::new` on a durative domain: snap-compile + ground ONCE (the
stratified path), then every `replan`/`replan_budgeted` is a bounded
temporal solve from the CURRENT world state returning a timed plan.

- **Design decision, recorded**: a session's world state between
  thinks is AT REST — no running intervals. Episodic stop-and-think
  means the agent plans at decision points; mid-interval state is the
  game's business, not the planner's. Concretely: the temporal solve
  seeds an EMPTY agenda, and `set_fact` rejects the compiler-reserved
  `RUNNING-*` tokens exactly as it rejects static facts (a game that
  wants to model an in-flight action mirrors its END effects when it
  completes).
- TILs: rejected in-session for now (a TIL pins the absolute clock;
  session thinks are clock-relative). Recorded as a follow-up if the
  game needs scheduled exogenous events.
- The budget surface carries over whole: eval budget bounds every
  pass of the temporal ladder, the memory target plumbs to
  `temporal_node_cap`, a budget-exhausted think returns `solved:
  false` honestly, and t1 ≡ t8 (suite-enforced, like the classical
  think test).
- Acceptance: a concurrency-using fixture (two agents, overlapping
  durative work) grounds once, thinks bounded, returns a VALIDATED
  timed plan, replans after `set_fact`/`set_fluent` drift; the
  `game_think` example grows a temporal act; suite determinism test.

## Phase 2 — drift-stable replanning (follow before you rethink)

A game agent whose world shifts slightly mid-plan must not thrash to a
structurally different plan. The cheap, high-value mechanism:

- **`Session::still_valid(plan, from_step)`** (name settled at
  implementation): replay the plan's remaining suffix against the
  CURRENT session state — the internal validator's replay machinery,
  pointed at a suffix. If the suffix still executes and ends in the
  goal, the agent keeps following it for FREE (no search, no think
  budget spent); only a broken suffix triggers a real rethink.
- Measured deliverable: a scripted drift fixture (N ticks, occasional
  irrelevant drift, occasional plan-breaking drift) reporting
  thinks-spent and plan-churn with and without the suffix check.
- Acceptance: irrelevant drift costs zero search; breaking drift is
  detected exactly (no false "still valid"); determinism unchanged.

## Phase 3 — grounding at barter scale (reachability-interleaved)

The one big engine project of the cycle, serving both masters: the
game's item×item exchange actions and the recorded elevator-11 tail
(stratum-1 START enumeration: ~4 GB transient, most of the grounding
wall). Today Phase B enumerates typed cross-products per action and
prunes after; the lever is enumerating FROM REACHED FACTS instead —
candidates exist only when their preconditions' dynamic atoms have
producers, interleaving reachability with enumeration.

- Fixtures first: a barter stress domain (K item types × M holders,
  generic `trade any-for-any`) alongside elevator-11 p04+ — measure
  raw-candidate counts, transient RSS, and wall before touching code.
- The bar: elevator-11's grounding transient and time drop
  materially; the barter fixture grounds within a think-sized budget;
  classical/temporal corpus paths UNCHANGED (the equivalence gate:
  identical plans and eval counts across a representative sweep, the
  compaction cycle's discipline).
- Honest exit: if the interleaved rewrite can't hold the equivalence
  gate inside this cycle, ship the fixtures + measurements + a
  recorded design, not a half-landed grounder.

## Recorded — Phase 3 (2026-07-20): SHIPPED, the measurements chose the design

Fixtures first, as prescribed, and they discriminated the term:

- **bazaar** (vendored: 12 holders × 40 items, any-for-any trade):
  DENSE-reachable — 197k of the 211k typed candidates are real ops.
  Interleaving cannot help by construction; the game answer is
  GROUND-ONCE (5.5 s / 644 MB at world load, then thinks are pure
  search). Classified, recorded, viable.
- **elevator-11 p04**: enumerated ~100× its reachable set — 11.1 GB
  unstratified / 5.7 GB stratified transient for a 16,728-op task.
  Sparse-reachable: exactly where reached-restriction wins.

**Reached-restricted fixpoint grounding shipped** (temporal entry;
`FF_NO_FIXPOINT_GROUND=1` falls back to stratified; classical entry
untouched): every action joins its positive dynamic top-level literals
against the reached-atom set, rounds to fixpoint, bindings deduped
across rounds; the producer-known stratification is subsumed. p04 A/B
same-binary back-to-back: **31.6 s / 5.7 GB → 6.9 s / 48.8 MB (~117×
transient), identical task dims**; equivalence spots exact (crew /
elev08 / openstacks / pegsol makespans identical on/off); suite 148/0;
elevator sweeps val-green (elevator08-numeric back to 29/30).

Residual, honest: elevator-11 coverage stays 3/20 at 30 s — the wall
MOVED from grounding to search (p05 now solves solo at 49 s, formerly
a grounding OOM; p04 is search-bound past 90 s). The tail joins the
recorded guidance family; the grounding lever is spent, and it was
worth exactly what the fixtures said it would be.

## Phase 4 — corpus debts (small, bounded)

- **parc-printer-t diagnosis**: 18/30 + 7/20 is the one temporal
  plateau never actually diagnosed. One instrumented afternoon:
  classify (guidance / scale / semantics / plumbing), record, fix
  only if the classification hands us something cheap.
- **Reference-cost quality scoring**: vendor best-known costs for the
  vendored subset so the runner can report a real IPC quality score
  (the docstring's recorded caveat about summed cost).
- **turn-and-open at realistic budgets**: measure the full variant at
  60 s / 120 s solo-equivalent budgets so the record reflects what
  the 0.10 keys actually bought (i1 solves in ~25 s; the 30 s / 3-job
  methodology clips the family).

## Recorded — Phase 4 (2026-07-20): debts paid, diagnoses filed

- **parc-printer-t DIAGNOSED** (the never-classified plateau): the
  complete pass drowns in start-spam — avg ~2,076 pending intervals
  per node (the TMS interleaving family; mechanism now precise). The
  cheap completeness-preserving experiment (an agenda-size ordering
  term on the complete pass's key, `FF_TAGENDA_W`) measured NEGATIVE
  at 30 s; knob stays opt-in, diagnosis recorded.
- **Self-relative quality scoring shipped**: `ipc67.py
  --score-against PRIOR.jsonl` computes the IPC formula against a
  prior run's per-instance costs — regression tracking, explicitly
  labeled NOT an official IPC score (the corpus carries no reference
  costs). Smoke: crew 5/5, quality 5.00 vs the 0.10 run.
- **turn-and-open at realistic budgets**: 0/20 at 60 s (jobs-2 +
  today's slow box), 1/20 at 120 s (i1 at 77 s, val-green) —
  search-bound, the guidance family, exactly as classified in 0.10.

## Phase 5 — 0.12.0 release mechanics

CHANGELOG `[0.12.0]`, workspace bump 0.11.0 → 0.12.0, README refresh,
`rustup update stable` first, and the FULL gate — fmt, clippy
`-D warnings`, suite, AND `RUSTDOCFLAGS="-D warnings" cargo doc
--no-deps` (the 0.11.0 publish caught the doc pass missing from the
working gate; RELEASING.md already prescribed it). Scoreboards
refreshed where phases moved them; main publish.sh-ready.

## Deferred, on the record

- **Red-black planning / semantic landmarks over numeric structure**:
  the recorded different-h lever for transport/storage/TMS/model-train
  — a research cycle for when the scoreboard matters again, now with
  three dead ends fenced off (roadmap-0.11 records).
- **TMS interleaving scale** (~47 pending ends/node): likely needs
  end-batching or symmetry reduction over identical concurrent
  intervals; not this cycle.
- **Session TILs** (absolute-clock exogenous events): only if the
  game's design turns out to need them.
