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

### Recorded — the front door is open; three mechanisms, both boards moved

The three fixes, each fixture-pinned in `tests/parse.rs` before the
code moved:

1. **Negative number literals** — the lexer emits a negative literal
   when the digit touches the dash (Metric-FF's behavior); the fixture
   pins the SIGN (a flip would satisfy its goal at init).
2. **Implicit `(total-cost) = 0`** — the PDDL 3.1 `:action-costs`
   convention, Fast Downward-compatible; only the exact zero-arity
   TOTAL-COST fluent defaults, every other undefined read stays a real
   error. Causally proven before the fix: agricola i1 with the init
   line hand-injected grounds and searches.
3. **Named verdicts** — `Outcome::GoalFalse`/`GoalUndefinedFluent`
   carry their mechanism and `api::solve` surfaces it in
   `Solution.notes` ("goal fact (DONE-PROGRAMMING) is unreachable: no
   surviving grounded action adds it" is what cracked the trio's
   diagnosis in minutes). Classic-FF text-path messages stay
   byte-identical for the differential validator.

The referee, both boards re-swept at 60 s:

- **2018-sat: 38 → 42** (+9: flashfill i1/i6/i16, settlers i1–i5/i9;
  valid 30 → 35). Engine-reject column **60 → 0**.
- **2023-numeric: 126 → 129** (+4: fo-counters ×3, fo-sailing i1;
  valid 110 → 113). Reject column **60 → 1** (a single settlersnumeric
  instance). sailing/fo-sailing now parse and SEARCH — most spend the
  full budget without solving yet, exactly the "reject moves to
  search" outcome the phase scoped; they are Phase 3's material now.
  The failure-class mix redistributed (mem-cap 93 → 23, timeouts up
  accordingly) — Phase 4's attribution reads the fresh mix.
- Casualties solo-checked UNCONTENDED: caldera i1 (40 s), caldera i2
  (37 s), data-network i4 (51 s), nurikabe i8 (53 s) all solve solo —
  contention noise from concurrent Phase 2 builds during the sweep.
  organic-synthesis-split i7 and rover-numeric i16 fail even solo at
  75 s: budget-edge flappers (org-synth i7 has flapped since the 0.18
  nov boards), on untouched domains — recorded, not fix-caused.

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

### Recorded — the fence is down: 252 certified optima across three tracks

`Mode::Optimal` shipped as scoped: serial deterministic A* over the
packed task, cost-labeled h^max (numeric preconditions ignored — a
relaxation, admissible; expansion and goal test exact),
PROOF-OR-NOTHING (cap ⇒ inconclusive, never an incumbent; exhaustion
⇒ certified UNSOLVABLE past the delete relaxation). Costs: constants
AND static-fluent expressions evaluated against init (the IPC
`(travel-slow ?f1 ?f2)` pattern); state-dependent costs reject with a
named note. Explicit `--mode optimal` only — auto stays satisficing.
The fixture ladder (5 tests) pins the certified optimum, a cost trap
(certified 4 beats the tempting 1-step cost-10 plan), proof-or-
nothing, certified-unsolvable, and the temporal reject.

The mode's claims validated three independent ways: every certified
plan is **VAL-green** (val ≡ coverage on all 35 swept variants);
certified costs match the INDEPENDENT cost-sweep oracle exactly
(scanalyzer-08 i1 = 18, woodworking-08 i1 = 170); elevators-08 i1's
certified 42 matches the literature.

First entries, 60 s: **2008 seq-opt 114/270, 2011 seq-opt 90/280,
2014 seq-opt 48/256** — 252 certified optima. Strong grounds:
peg-solitaire 26/30 + 16/20, sokoban 19/30 + 16/20, maintenance-2014
5/5, genome-edit 13/20. The honest h^max walls, named: floor-tile,
parking, tidybot-2014, barman-2014, child-snack all 0 — exactly the
LM-cut stretch's motivation, which remains open (taken only if the
cycle's later phases leave room, else 0.20's first bet).

## Phase 3 — the numeric heuristic swing (named since 0.17)

The 2023 numeric board after Phase 1 still holds ~121 timeouts — the
landscape memo's bet #2 (subgoaling / AIBR-class numeric heuristic,
replacing the current fixed-point numeric relaxation where it
degenerates). Judged on the numeric board's timeout column and the
village's think benchmarks (the game cares about numeric gradients
too — stock/money goals are exactly this shape). Measured win or
recorded negative, per house rule.

### Recorded — +52/−1: the biggest single-phase coverage jump since the modern corpora entered

The gap was PRECISE: the relaxed-plan extraction's repetition
counting (`numeric_achiever`) handled only bare-fluent-vs-literal
goals; LINEAR COMBINATIONS — the 2023 numeric track's staple shape —
fell through with no gradient at all. The swing: `linearize`
normalizes `lhs op rhs` into `Σ coeff·fluent + konst ≥ 0`
(fluent×fluent and fluent divisors honestly refuse), then the charge
is ⌈gap / combo-delta⌉ repetitions of the op whose combined
constant-delta effects move the combination fastest. Runs ONLY where
the bare path returned None — every previously-charged shape keeps
its exact charge and tie-break, classical paths untouched by
construction. `FF_NO_NUMH=1` restores the hole.

The referee: **2023-numeric 129 → 181 of 400 (+52/−0)** — farmland
+17, fo-farmland +17, counters +8, fo-counters +5, tpp +2, rover +1,
sugar +1, zenotravel +1; valid 113 → 165. metric-time-2006 55 → 54:
the one loss (tpp-metric-time i4) is CHARGE-CAUSED — the FF_NO_NUMH
discriminator solves it in 15 s while the charge misleads the
temporal metric search (its sibling i3 simultaneously got 7× faster,
8.05 s → 1.13 s: the gradient reshapes that search in both
directions). Verdict: default-ON — +52/−1 over 600 instances is the
inverse of the novelty rung's arithmetic — with tpp-mt i4 the named
witness on the debts list. sailing proper still walls at 60 s (its
plateau has more layers than the linear charge); the fresh numeric
board's mem-cap column (105) is Phase 4's material.

## Phase 4 — the mem-cap class (93 + 40)

2023-numeric carries **93 mem-caps**, 2014 classical ~40 more — the
modern instances' grounding transients against the per-job cap.
Diagnose the top offenders BY CLASS first (fact-space? op
enumeration? numeric side tables?) with `FF_RES_DEBUG` attribution
before touching anything — 0.9's lesson stands (the wall was the
grounder, not the search, and compaction was the fix). Whatever
mechanism the attribution names gets the 0.9 treatment: a targeted
structural fix with classical paths bit-identical, never a cap tune.

### Recorded — the opposite of 0.9: search-state-owned, and the cap could not see the limit

The RSS-at-forced-cap attribution (10k-node cap, rusage) on the top
offenders — markettrader (20 caps), pathwaysmetric (20),
block-grouping (18), tpp (16) — read **24 MB RSS with 4–276 grounded
ops** on every one: NOT grounding (0.9's wall), pure search-state
growth. The mechanism names itself: tiny-state numeric tasks evaluate
fast, and `NODE_CAP_TARGET_BYTES`'s fixed 8 GiB retained-bytes target
EXCEEDS the runner's per-job `RLIMIT_AS` (phys/jobs), so the OOM kill
fires before the internal insertion cap ever does. The fix is
structural, not a tune: the byte target now clamps to 60% of the
process's actual `RLIMIT_AS` (read from `/proc/self/limits`, no new
dependency; the remainder covers task tables, the open list, and
transient churn outside the per-node model). No limit set ⇒ exactly
today's caps — dev boxes and every classical baseline byte-identical.
Mem-cap rows become honest capped/timeout rows instead of OOM
casualties that could disrupt sibling jobs. Deeper retained-state
compression (key interning, state deltas) is named 0.20 material.

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

### Recorded — rider (a) shipped; rider (b) built its machinery and decoded the debt deeper

**(a) Novelty default under budget**: the rung now runs BY DEFAULT
exactly when `FF_TIME_LIMIT` is declared and >40% of the budget
remains (`FF_NO_NOVELTY=1` opts out; `FF_NOVELTY=1` still forces it
budget-less; no budget ⇒ ladder byte-identical). Probe on the gated
referee's own win: termes-2018 i12 solves with `FF_TIME_LIMIT=60`
ALONE, times out with the opt-out, times out with no budget — the
0.18 +4/−0 evidence is now the shipped default.

**(b) Duration reconciliation**: the emission pipeline now REPLAYS
the final plan chronologically and clamps every state-dependent
duration into the domain expression's `[min, max]` at its EMITTED
start state (fixpoint-iterated, since corrections move ends; any
replay failure or non-convergence returns the original plan — never
a half-correction). The tempo-2014 referee: 65/200 byte-stable,
match-cellar 20/20 valid, zero movement anywhere. The three
map-analyzer witnesses, however, REFUSED the fix and named a deeper
mechanism: their ε-shifted starts land where BOTH the duration
source has moved AND a propositional precondition's provider
(`(clear junction0-2)`) has not yet fired — the reconciliation
correctly bails on the inconsistent replay. The full fix is
ε-separation-level surgery extending 0.18's same-slot END repair to
START happenings against their providers — named 0.20 debt, one
decode deeper than 0.18 left it.

## Phase 6 — cut 0.19.0

The standing template: every touched board re-swept against the
final binary, PLUS the locked official-budget entry — 2023 agile at
the competition's **300 s** (one sweep, at the cut only; the 60 s
rows stay as baselines). New standings sections for the optimal
tracks. Records complete, full pre-flight, finish in main; the user
publishes.

### Recorded — cut

Seven boards against the final 0.19.0 binary, the whole cycle
compounding under one ladder (novelty-by-default made every
budget-declared sweep's ladder new):

- **2018-sat 58/240 (+16/−0)** — flashfill +6, org-synth-split +2,
  termes +2, settlers +2, caldera +2, data-network +1, nurikabe +1;
  valid 30 → 50 over the cycle.
- **seq-sat flagship 452/580 (+11)** — its first movement in three
  cycles.
- **2023-agile 28/140 (+2)**; **2014-sat 98/280 (+2)**;
  **2014-agile 96/280 (+1)**; **2023-numeric 181/400 (+2/−2)** —
  the two losses are sugar-numeric i2 and zenotravel-numeric i20,
  the twice-solo-documented budget-edge flappers (29 s and 59 s solo
  against a 60 s line). Cut A/B total: **+32/−2**.
- **The official-budget entry: 2023 agile 38/140 at 300 s** — the
  extra budget bought +10 over the 60 s baseline; the standings row
  is labeled a competition-methodology ENTRY, not a baseline.

Pre-flight: all eleven gates green on latest stable (fmt --check,
clippy `-D warnings`, full release suite, heavy qual-metric tier,
doc `-D warnings`, bench --no-run, mcp build, publish dry-run, the
0.19.0 maturin wheel, wasm build, browser SMOKE PASS). FIVE
container restarts hit this cycle's sweeps; the resume-aware drivers
lost only in-flight boards each time. Finish in main; the user
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
