# ferroplan 0.20 roadmap — the guidance cycle (contest cycle II)

Scoped 2026-07-31 at the 0.19 cut, by direct request: keep improving
the standings across the board. The scoping research ran first — a
fresh per-domain failure decode of every canonical JSONL against the
0.19.0 boards, plus a field refresh. The decode's headline: 0.19
opened the front door (rejects 60→0 and 60→1 on the two audited
boards), and what remains is almost entirely TIMEOUTS — the ledger
has become a guidance ledger. Three numbers order the cycle:

- **554 seq-opt timeouts** (346 on 2008/2011, 208 on 2014) where
  coverage = proof rate and the heuristic is h^max — the biggest
  single pot on the books, with its five named walls.
- **~750 satisficing timeouts** across the classical boards
  (2018-sat 165, 2014-sat 144, 2014-agile 148, seq-sat flagship 121,
  IPC-5 prop 96, 2023-agile 84 at 300 s) — plus the witness below
  that names the mechanism as search guidance, not budget.
- **~200 mem-caps** across all boards (~145 on the modern
  satisficing set: numeric 58, 2014-sat 30, 2014-agile 31, 300 s
  entry 18, 2018 12) — 0.19's attribution stands: search-state
  owned, not grounder owned.

The audit also caught the scoreboard fibbing twice, in our favor
neither time — Phase 1 exists because the honest table comes first.

Field refresh (for the record): the numeric LM-cut family
(Numeric-FD) won the IPC 2023 numeric track — the classical LM-cut
core this cycle takes has been the field's workhorse admissible
heuristic since 2009, and its numeric variant is now
competition-proven. Novelty/multi-queue portfolios are the current
satisficing-numeric literature direction (SOCS), matching this
cycle's Phase 3 shape. IPC 2026 ran its numeric track at ICAPS
Dublin (June 27 – July 2, 2026); the corpus becomes a rider at the
cut (Phase 6) once the organizers publish it.

## Phase 1 — honest clocks, then spend the whole wall

Two audit receipts, both mechanisms reproduced solo:

- **Graceful wall-exits are counted as engine-rejects.** With
  `FF_TIME_LIMIT` armed (every sweep since 0.18), the engine exits
  just under the wall; the runner's `TimeoutExpired` never fires and
  the row lands as `time=None, notes=None` — which the standings
  classifier calls engine-reject/error. maintenance-2014's "8
  rejects" are this: runner-instance 9 (`instance-17.pddl`) runs the
  full 65 s solo without finishing. The reject columns overstate
  rejects and understate timeouts on every budget-armed board.
- **The ladder leaves budget on the table.** tpp-numeric runner-i19
  returns no-plan at 35 s of a 60 s budget (rc=1, ladder exhausted) —
  10 of that domain's 20 instances share the shape, and the class
  exists wherever `time=None` rows are genuinely early.

The work, smallest first: (a) runner records elapsed wall for
UNSOLVED rows too — one line on the non-solved path; (b) standings
classifier splits wall-exit (elapsed ≥ budget−1 ⇒ timeout) from true
fast reject; boards re-attributed, no re-sweep needed (the JSONLs
gain the field on the next sweep; until then the md tables carry the
correction note). Then the engine lever: **after ladder exhaustion
with wall remaining, escalate and go again** — restart the ladder
with larger node caps and relaxed pruning until the wall is spent;
an engine holding a `FF_TIME_LIMIT` never returns unsolved with
double-digit budget share unspent. Hatch: `FF_NO_REFILL`. Fixture:
the tpp-numeric i19 witness (must now run to the wall; solving it is
upside, not the gate). Referee: re-attributed failure classes plus
any coverage the refill loop buys.

### Recorded — the clocks are honest and the wall gets spent

The bookkeeping first, both ends receipted:

- **Runner** (`ipc67.py`): elapsed wall is recorded for UNSOLVED rows
  (previously solved-only); a nonzero exit with no JSON verdict is
  stamped `engine-exit-<rc>`; an unsolved JSON verdict's mechanism
  notes (the 0.19 named verdicts) are carried into the row. Smoke:
  tpp-numeric i1–i3 rows byte-equivalent to before; a sailing run at
  12 s produces `"time": 12` on its unsolved row.
- **Standings** (`standings.py`): the classifier now reads the
  elapsed — wall-exit (≥95% of budget) is TIMEOUT even when the
  engine exited gracefully at an armed `FF_TIME_LIMIT`; named
  mechanisms (`engine-exit-*`, grounding verdicts, rejects-by-name)
  stay engine-reject; the residue is a NEW `early-exit` class (gave
  up with wall left — the refill loop's referee column). Legacy
  `time=None` rows keep their old class and the regenerated
  `ipc-standings.md` came out byte-identical — the truth arrives as
  boards re-sweep.
- **Engine** (`search.rs plan_avoiding`): the refill loop. A CAPPED
  fallback (eval cap or node-cap memory model — genuine exhaustion
  returns immediately, completeness is weight-independent) with >10%
  of an armed wall remaining re-enters GREEDIER: w_h ×4 and
  max_eval ×4 per round, memory bound untouched, at most 6 re-entries
  (escalation saturates; a saturated re-run is deterministic waste).
  An explicit api `max_evaluated` is a budgeted-think contract and
  disarms the loop; no declared budget ⇒ single round, byte-identical
  to 0.19. `FF_NO_REFILL=1` is the discriminator hatch.
- **The witness, before/after**: tpp-numeric runner-i19 returned
  unsolved at 35 s of a 60 s wall; with the refill it runs round 2
  (w_h 20) and exits at 57 s — 95% spent, an honest timeout row. The
  solo probes for the record: i19 stays unsolved under every alternate
  bet tried (w_h 20 → capped 18 s, w_h 60 → capped 8 s, novelty-only
  → 42 s) — WHY a solvable instance's ladder exhausts at all is
  Phase 5's question, on the record there.
- **Fixture** (`tests/refill.rs` + `benchmarks/bench/visitgrid-*`):
  subprocess pin, three scenarios — armed wall + forced 3000-node cap
  shows exactly 6 narrated refill rounds then an honest unsolved;
  `FF_NO_REFILL` shows zero; no budget shows zero. The 7×7 visit-all
  grid fixture needs ~40k insertions under best-first at any weight,
  so every round caps deterministically. Full suite green (32/32),
  fmt + clippy `-D warnings` clean.

A scoping probe recorded while calibrating the fixture: weighted
best-first CANNOT solve even a 10×10 visit-all within 100k evals at
ANY weight (5/20/60) — h_FF's plateau defeats greed entirely, while
EHC and the novelty rung both dispatch it instantly. Phase 3's
novelty-first bet is aimed at exactly that shape.

## Phase 2 — LM-cut (the 554-instance pot)

The 0.19 stretch that wasn't taken, now the cycle's centerpiece.
Coverage on the three optimal boards IS proof rate, the heuristic is
h^max, and the walls are named in the 0.19 record: floor-tile,
parking, tidybot-2014, barman-2014, child-snack — all ZERO across
their boards; scanalyzer 9/50, woodworking 9/70 combined,
elevator-08 8/30, transport-08 10/30.

- Built over the existing CSR relaxation machinery: h^max labels
  give the justification graph; extract a disjunctive action
  landmark (cut) from the goal's supporter frontier; charge the
  cut's minimum cost, decrement, repeat to fixpoint. Same packed
  task, same `op_costs` (0.19's static-fluent extractor carries
  over), serial and deterministic like the rest of `optimal.rs`.
- Fixtures FIRST: hand tasks with known LM-cut values (the classic
  two-path example where h^max=1 but LM-cut=2), plus an
  admissibility differential — LM-cut ≤ true cost on all 252
  certified optima from 0.19, and A* + LM-cut must reproduce every
  one of their costs exactly.
- Hatch: `FF_NO_LMCUT` falls back to h^max.
- Referee: all three opt boards re-swept. The literature says LM-cut
  roughly doubles h^max coverage on exactly these domains; the
  honest record takes whatever the boards say.

## Phase 3 — the classical guidance swing (novelty first)

The witness that names the satisficing mechanism: **visit-all-2014
instance 1** — the canonical width-2 domain, the one BFWS-class
planners dispatch in milliseconds — takes 35 s SOLO under sweep
conditions here, and forcing the novelty rung changes nothing
(34 s). The board shows 0/20 under 3-way contention. Our novelty is
a late tiebreak inside an hFF-driven search; the field's result is
that novelty works as the DRIVER with the heuristic as the tiebreak.

- The bet: a novelty-first rung — lexicographic open list ordered
  (novelty width 1/2, then hFF), seeded early in the budget-armed
  ladder; hFF still prices, novelty decides. BFWS(f5) is the shape,
  our existing novelty tables are the parts.
- Witnesses, in expectation order: visit-all-2014 0/20,
  transport-2014 0/20, parking-2014 0/20, cave-diving 0/20,
  openstacks-2014 2/20; the 2018 board's agricola 0/20 and spider
  1/20 as stretch.
- The guardrail is the project's own history: every premature
  promotion of the novelty rung LOST coverage (0.17's referee, twice
  re-confirmed). The rung enters behind a fixture set and the cut
  A/B decides; casualties named and solo-checked, hatch
  `FF_NO_NOVFIRST`.

## Phase 4 — retained-state compression (the ~200 mem-caps)

0.19's attribution stands and inverts 0.9's lesson: the RSS probe at
a forced 10k-node cap showed 24 MB on 4–276-op tasks — the search
STATE store owns the memory, not the grounder. Named winners if the
class moves: block-grouping-numeric 18, city-car 15, folding 14 (on
the 300 s entry board), child-snack 13, markettrader 10,
organic-synthesis-2018 7, snake 5.

- Levers smallest-first: stop double-storing state keys between the
  dedup map and the node arena (intern once, reference twice);
  pack closed-set states as parent-delta chains (reconstruct on
  plan extraction, never during search); drop per-node fields the
  satisficing path never reads back.
- Referee: the forced-cap RSS probe re-run (bytes/node before and
  after, recorded), then the mem-cap columns on the numeric and
  2014 boards. Byte-identical search order is the constraint —
  compression must not change expansion order, or it's a different
  phase.

## Phase 5 — the debt basket (attribution first)

- **ε-separation START-vs-provider surgery** — map-analyzer
  i17/i18/i20, twice-decoded at 0.19: ε-shifted STARTs land where
  the duration source moved AND a propositional provider
  (`(clear junction0-2)`) hasn't fired yet. Extend 0.18's same-slot
  END repair to START happenings against their providers; the three
  2014-tempo VAL-RED rows are the fixture and the referee.
- **drone-numeric VAL-RED ×16** — too many to leave unattributed:
  decode whose bug (engine plan, runner plumbing, or VAL's numeric
  handling). data-network-2018's 8 VAL-RED + 5 rejects get the same
  treatment. VAL-side findings go to the runner record; engine-side
  findings get fixtures and fixes this cycle.
- **sailing-numeric 0/20** — parses and searches since 0.19, never
  solves at 60 s. One honest attribution pass (plateau shape,
  linear-subgoal miss, or scale) with markettrader 0/20 and
  pathwaysmetric 0/20 in the same sitting; findings feed the next
  numeric swing, fixes taken only if small.
- **tpp-numeric's early exhaustion, the WHY** — Phase 1 makes these
  spend the wall; this basket asks why a solvable instance's ladder
  exhausts at all (over-pruning? numeric guard? helpful-action
  starvation?). A completeness bug here would outrank everything
  else in the basket.

## Phase 6 — cut 0.20.0 (+ the 2026 corpus rider)

The standing template: every touched board re-swept against the
final binary; the 300 s official-budget agile entry refreshed; new
attribution columns regenerated; records complete per phase; full
pre-flight (all eleven gates, latest stable); finish in main; the
user publishes.

The rider: **the IPC 2026 numeric corpus.** The track ran at ICAPS
Dublin in June; when the organizers publish domains and results
(this session's proxy 403s their site — retry at the cut, the
corpus conventionally lands on GitHub), extend `get-ipc.sh`, vendor
it, and sweep a first board — brand-new corpus rows in the modern
section, the direct continuation of "enter tracks we're not doing."
Not public by cut time ⇒ recorded as a watch item, not blocking.

## Deferred, on the record (carried forward)

- The h-surgery bet (end-gated interval credit) — the village
  gather-spam witness file stands.
- Lifted/lazy grounding — organic-synthesis and agricola keep it a
  watch item; Phase 4's probe may promote it.
- The temporal 30 s boards' walls — temporal-machine-shop 0/20,
  storage-temporal 0/20, turn-and-open 1/20 (the deep
  required-concurrency cluster) plus the IPC-5 metric-time timeout
  mass (tpp 36, rovers 35, pathways 30). Raising those budgets to
  60 s re-baselines every temporal row; deliberately deferred until
  a temporal-focused cycle can pay for the re-baseline.
- IPC-5 complex-preferences / timed modal operators, cross-mind
  planning, continuous `#t`, dynamic derived predicates,
  fixpoint/stratified unification — unchanged from the standing
  lists.
