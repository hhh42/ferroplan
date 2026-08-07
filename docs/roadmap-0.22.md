# ferroplan 0.22 roadmap — the coverage cycle

Scoped 2026-08-05, the day after the 0.21.0 cut, by direct request:
**improve solving coverage — think big.** The scoping ran as a fresh
per-domain decode of all thirteen promoted boards (the first cycle
whose raws carry a clean same-box A/B against both 0.20 and the
v0.19/v0.18 backfills), plus five deep design reads on the
deferred-ledger bets. The decode reproduces the standings to the row:
1,923 unsolved = 1,891 timeout + 16 mem-cap + 14 early-exit + 1
VAL-RED + 1 adjudicated engine-reject. Timeouts are 98% of the
failure mass. The ledger is a pure guidance-and-allocation ledger
now, and the classes 0.19–0.21 built machinery for are almost all
EMPTY — which is what "think big" gets to build on.

Four numbers order the cycle, largest honest pot first:

- **645 distinct classical-satisficing timeout instances** (734 rows
  across six boards) — the centerpiece deferred at 0.20 AND 0.21,
  carrying the field's recipe by name since the IPC-2026 results
  landed. Wall-shaped, not budget-starved: every one of the 300 s
  entry's 89 timeouts also times out at 60 s, and 5× budget converts
  19 of 108.
- **503 optimal-proof timeouts, single-class** — the ENTIRE failure
  mass of all three proof boards is hard wall kills with empty notes;
  the largest single-mechanism pot on the books, and 0.21's root gate
  just proved the mechanism moves (LM-cut certificates 13 → 56).
- **344 temporal failures**, of which the 110-instance three-domain
  zero block (storage 0/40, TMS 0/40, model-train 0/30) is now THREE
  RELEASES DEEP and EIGHT mechanism-precise negatives deep — this
  scoping ran the last two cheap levers and killed both (below).
  Temporal mechanism work is honestly deferred again, with 0.23
  pre-scoped rather than waved at.
- **318 numeric-satisficing timeouts** across the two numeric boards,
  now attributed family-by-family (the sitting 0.21 parked ran this
  scoping): a 139-instance PLATEAU POOL that is the numeric face of
  the same partitioned-novelty recipe as the classical centerpiece,
  plus 2048's grounding pathology, plus a wall-honesty hole with
  receipts.

And the gate that is not a pot: **markettrader i3 is the only VAL-RED
row on 4,076** — new at 0.21, on a domain VAL ingests fine: the
engine claimed a 453-step plan in 4.87 s and VAL rejected it. Phase 1
opens with it, because a scoreboard with an unexplained rejected plan
on it outranks every coverage number on this page.

Two cross-cutting facts the phases lean on:

- **A quarter of all failures sits in five named families** —
  transport 148, floor-tile 119, parking 88, openstacks 84,
  child-snack 44 — each spanning three to seven boards, so a
  domain-shaped win pays out several boards at once.
- **The wall is being silently overrun.** The armed `FF_TIME_LIMIT`
  is enforced only at eval-count checkpoints and rung boundaries, so
  slow-eval domains run to 67–76 s of a 60 s budget (solo receipts:
  2048 at 67–74 s — grounding never checks the clock at all —
  gear-car 72.1 s, block-grouping 76 s), and coverage is sitting just
  past the line: onlycraft-sat i7 solves at 60.2 s, gear-car i6 at
  72.1 s. The boards are honest (the runner kills at 60); the ENGINE
  is not spending the wall it was given — it is spending somebody
  else's.

Field refresh: reused from 0.21's, four days old — the answer key
(Panino's partitioned numeric novelty, Count Downward's numeric
PDBs/CEGAR, LNP-optimal nearly open at 83/260) is unchanged, and the
comparability column landed at the 0.21 cut. One correction from this
decode: the deferred ledger's "2018 wall (136)" does not reproduce
from either the 0.20 or 0.21 raws; the fresh anchor is **167 timeouts
on 2018, 150 of them search-shaped** (organic-synthesis's 17+3 are
grounding). And one hygiene note carried loudly: only 6 of the 13
air21 boards have `conditions.json` files on disk — the "all 13
verdict clean" claim in the release record rests on the driver's
retry discipline for the other seven; 0.22's driver writes conditions
for EVERY board, no exceptions.

## Phase 1 — the gate and the bill (fixtures before levers)

Nothing in this phase is a coverage lever. All of it is the
discipline that makes the levers safe to pull.

- **The VAL-RED, decoded by hand first.** markettrader i3: reproduce
  solo, diff the plan against VAL's complaint step by step. Engine
  bug ⇒ it becomes THE phase (a satisficing planner that emits one
  unsound plan cannot be trusted on 2,152 others); VAL-config or
  domain subtlety ⇒ adjudicated like settlersnumeric i7 was, on the
  record. No numeric work lands before this verdict.
- **The charge's bill, pinned.** 0.21's numeric-precondition charge
  bought +43 and cost 8, all eight named: ext-plant-watering i7
  (was 0.15 s!) and i13, counters i12/i16, delivery i18/i19, rover
  i19, zenotravel i20 — all former fast solves, now timeouts. The
  loudest (ext-plant-watering i7) becomes a charge-regression
  fixture BEFORE any new heuristic work; the decode then chooses:
  charge damping (the gradient distortion shape lever a2's design
  already carries), or the honest per-domain record of the trade.
  The −8 is this cycle's opening debt.
- **block-grouping's pre-search hang, instrumented.** i3 hangs 76 s
  under a THREE-eval think budget — before or at search entry, and
  partition mode emits no `FF_WALL_DEBUG` narration at all
  (partition.rs is silent; every other driver narrates). Instrument
  first, then decode where i3 sits. Field is 20/20 here; 18
  instances ride on this answer.
- Bookkeeping riders: expedition is verbatim on BOTH numeric boards
  (scope once, bank twice — the referee tables must not double-count
  it); flashfill i10 closes as converted-by-0.21 (solves on the
  board at 30.7 s); parking-2011's docket entry stays open (12/20
  now, frontier at 47 s).

## Phase 2 — the honest wall (spend it, all of it, and no more)

0.20 taught the engine to spend the whole wall; 0.21 denominated the
rungs in it; the decode says the remaining hole is that the clock is
CHECKED in evals, and grounding never checks it at all. Slow-eval
domains overrun to 67–76 s and get killed holding work; fast-eval
domains near the line lose solves the engine had time for.

- **Lever 1: time-based checkpoints.** Wall checks every K ms
  (clocked, not counted) in every rung's loop — the eval-count
  cadence stays as the fast path, the clock catches the slow-eval
  case. Grounding gets a wall check (2048's 67–74 s overruns are
  grounding, and Phase 7 wants an honest failure there, not a
  zombie).
- **Lever 2: the sailing-wind node cap.** 9 early-exit rows die at
  the 4.08M-node cap with 20–40 s of wall LEFT — post-fold numeric
  nodes are small, and the byte model should be letting these rows
  spend their remainder. Raise-by-model, or refill re-entry with a
  raised cap; the 0.20 refill loop is the pattern.
- **Referee:** the near-wall converts pot (42 instances whose solves
  or kills sit within seconds of the line) plus the boundary-sliver
  churn domains (turn-and-open oscillates 26.2–28.6 s of a 30 s
  wall on BOTH temporal boards). Band: +6–15, and a cleaner
  instrument for every phase behind it. No armed budget ⇒
  byte-identical, as always.

## Phase 3 — the optimal ladder, third lesson (allocation, then resumption)

0.21's root gate turned LM-cut from 13 certificates into 56 and took
seq-opt to 275/550 — and the same-box A/B names, per instance, what
it cost and what it left: 12 losses vs 0.20, ten of them h^max
certificates killed by the unconditional 24 s sprint slice; city-car
−6 vs v0.19 on thin gate margins; and every one of the 53 LM-cut
certificates on 2008/11 paid the full 24 s sprint before LM-cut got
its first node, with three certs landing within 1.7 s of the wall.
The gate was right; the SLICE is still denominated wrong. Levers,
smallest first, each with its own receipt:

- **Lever 1 — the margin b-flip (+7 receipted):** the gate is binary
  (`lc > hm`, optimal.rs:603); city-car's six v0.19 proofs gate
  c-branch at ratios 1.09–1.36 while every probed TRUE-c domain sits
  at 2.2–6.0. b-branch below ratio ~1.4: recovers city-car ×6
  (i8 re-proven solo, cost 76) + tetris i5. Referee: 2014-opt ≥ 64,
  the v0.19 column, exactly what the release record demanded.
- **Lever 2 — margin-scaled sprint slice (+4–12):** at ratio ≥ 2 the
  landmark structure is unambiguous — scale the sprint fraction down
  (~0.1) and give LM-cut the wall it keeps almost-missing with
  (min cert 24.12 s, median 27.7 s, three within 1.7 s of the kill
  line).
- **Lever 3 — sprint-resume (+11 named instances):** the ten lost
  h^max certs have root ratios 2.59–3.5 — INSIDE the true-c range,
  so no threshold saves them; they need the h^max seconds the ladder
  currently throws away at handover. Keep the sprint's A* state;
  LM-cut gets a bounded probe slice; on LM-cut failure h^max RESUMES
  its open list with the leftover wall. This is the one lever with
  new machinery; it lands behind the other two.
- **The instrument repair that gates all three:**
  `opt-differential.py` runs at 90 s and books timeouts as
  "inconclusive, not failures" — which is how ten dying certificates
  sailed through an all-306-recertify gate at 0.21. The differential
  gains a board-budget mode: a certificate that cannot re-certify at
  60 s is a REGRESSION, named per instance. This lands FIRST and
  re-runs against 0.21.0 to establish the honest baseline.
- Band for the phase: +20–30 across the two classical proof boards.
  Incremental LM-cut graduates from the deferred list to a LIVE
  candidate the moment lever 2's boards show LM-cut
  running-and-near-missing as the dominant residue — the three
  near-wall certs say it is close.

## Phase 4 — the numeric-admissible bound (the proof boards learn numbers)

Mode::Optimal is numerically sound and numerically BLIND: onlycraft's
pure numeric goal reads h=0 and blind Dijkstra drowns at 8.4M
expansions on an instance the satisficing path solves in 23 steps at
~0 s. The design is a layer-index lower bound from the interval RPG
that already ships: monotone widening over-approximates reachability,
so the first goal-satisfiable layer lower-bounds plan LENGTH — an
admissibility argument that holds case-by-case in `widen()` and
FAILS CLOSED behind an audit.

- **L0:** `build_rpg` returns its exit reason (GoalAt/Fixpoint/Cap);
  `admissible_goal_layers` wraps it; `numeric_interval_audit` rejects
  by name the three shapes that break containment — scale-up/down
  effects (one-sided widening), undefined-at-init relevant fluents,
  monitored blocks. Audit reject ⇒ unarmed ⇒ byte-identical. The
  0.20 conditional-effect repair is the precedent for taking these
  rejects seriously.
- **L1:** a numeric arm in `optimal::solve` — `eval_h` becomes
  max(prop_h, num_h) (max of admissible bounds is admissible); the
  PROVEN note names its prover "+numRPG". Armed only when the
  certificate currency is length (all three -opt domains: no active
  `:metric`) and the audit passes. **No repetition-count shortcut**:
  ceil(gap/rate-now) is inadmissible when a plan can raise the rate
  first — sailing-wind's velocity assign is the live witness; the
  layer bound already encodes the admissible form.
- **L2:** root-drop guard — num_h(root) uninformative ⇒ disarm for
  the solve (dropping a max-component is always sound; caps the
  per-eval tax where the bound buys nothing).
- **Fixtures:** numopt-p01..p03 pins (h_num(init) ≤ known optimum);
  a new gap-60 pump-chain fixture pinning the layer counter's exact
  off-by-one forever; audit-reject pins with teeth; sailing-band
  through Mode::Optimal on the admissible side; trader-cycle stays
  the negative control.
- **Referee:** the (repaired) differential extended to all 354 +
  the 21 numeric certificates as the fresh regression surface,
  BEFORE boards. Band: sailing-wind-opt +5–9 (its 11 losses are
  blind node-caps; the last blind proof cost 2.86M expansions for a
  15-step optimum), onlycraft +1–4, rainbowttles +1–4 with an honest
  possible zero. This lever moves ONE 60-row board and is priced
  that way — it is the beachhead for the numeric-optimal cycle
  (PDBs/CEGAR, named deferred), not the centerpiece.

## Phase 5 — THE CENTERPIECE: the partitioned driver

The bet deferred at 0.20 ("the guidance swing") and 0.21 ("a
centerpiece for a classical cycle, not a side dish"), taken at last —
and the numeric decode says it is BIGGER than classical: the
139-instance numeric plateau pool (sailing-wind-sat, expedition,
petri-net, line-exchange, settlers-snp, ztalloc, coins — millions of
evals to nowhere on flat h) is the SAME mechanism wearing fluents.
One recipe, two families of boards.

**Phase 5A gates 5B — the ladder finishes learning the clock.** The
scoping receipt that demands it: tetris i4, a board TIMEOUT, solves
in 37.8 s under `FF_NOVELTY_ONLY` — the existing width-1 rung
converts it when given wall; the board loss is LAMA's 400k-eval
budget eating the clock first. (a1) LAMA gets a wall slice
(`FF_LAMA_WALL_FRAC` 0.25); (a2) the novelty rung gets one
(`FF_NOV_WALL_FRAC` 0.30 — last bounded rung, it can afford more);
(a3) rung affordability re-evaluated AT EACH RUNG ENTRY, not once.
Casualty watch from the fresh A/B: tidybot-2011 −2 and
openstacks-2011 −1 are the named EHC-slice suspects — solo-checked
here, quiet box, before 5B.

**Phase 5B — the driver, four levers smallest-first:**

1. **The R-partition:** subgoal feature set R = goal ∪ the
   extraction's need_fact stamps (heuristic.rs already stamps them;
   ~20 lines to read out), capped 256 by lowest layer; per-node
   achieved-R bitset, path-monotone; novelty cell becomes
   (unachieved_goals, r_count) — filling the (u16,u16) slot
   hardwired 0 today. Hatch `FF_NOV_PART=0`. Measurable A/B alone.
2. **Width 2, conjunction-cost bounded:** rank novel-1 < novel-2
   (new R-PAIR in cell) < non-novel; pair tables R×R only (4 KB per
   cell), marked new×true — per-state cost bounded by construction,
   never n_facts².
3. **The h-free driver queue** — the receipt is brutal: parking i1
   spends 86 s of cumulative worker time building h per pop at 100k
   evals. The new rung drops per-pop `relaxed_helpful` entirely:
   single serial open list, key = (rank, unachieved_g, |R|−r_count),
   deadline every 4096 pops. `FF_NOV_LAZYH` stays as a probe hatch
   (h on novel-1 pops only) if the tie-break proves too flat.
4. **The numeric-feature rider:** per-subgoal quantized log2 gap
   buckets as novelty features on numeric tasks (the charge's gap
   machinery + the 1e-6 quantizer) — this is the numeric plateau
   pool's lever, entering as a probe, promoted only on a measured
   numeric-board win.

**Placement:** the new rung REPLACES the h-guided novelty rung at its
slot (post-LAMA, pre-fallback). novelty-light stays untouched ahead
of LAMA — its +19 visit-all receipt is bankable and its slice proven.
`FF_NOV_OLD=1` restores the 0.21 rung wholesale. h stays OUT of the
partition cell (the in-tree degeneracy receipt, novelty.rs:8-12).

**Anytime restarts: not for coverage** — this codebase already
measured that shape (len_anytime: −9 coverage for +4 quality), and
the refill loop is the terminal wall-spender. But the decode found a
receipt that motivates a SEPARABLE probe: data-network i12 solves in
8.9 s with the fold hatched OFF (3.3 GB, byte-cap trip forcing a
refill restart) and takes 57.6 s with it ON (1.4 GB, no trip) — the
accidental restart was WORTH 6.5×. A deliberate
diversification-on-refill probe (new seed/weights per round) rides
this phase, pre-registered, cheap to drop.

**Referee discipline — the history is 2-for-2 against naive novelty
promotion** (0.17: +7/−51; 0.20: −26 until repriced), and both were
caught only by corpus or old-binary referees, never hatches. So: the
cut referee is the OLD-BINARY COLUMN — the v0.19 backfill plus a
fresh v0.21.0 backfill sweep — with casualties named per domain.
Canaries fixed now: visit-all 20/20 ×2, barman-agile 20/20,
quantum-layout 19/20, thoughtful 18+2, maintenance 16/20,
hiking-agile 14/20, openstacks' 12 EHC-direct rows. Pre-registered
solo probes before any board: transport-agile i1–i3, parking i1,
tetris i4 (must stay converted), snake i4, folding i1.

**The pot, honestly banded:** plateau trio 115 + ipc67 siblings 54 +
2018 search wall 150 + 2023 puzzles 107 classical, 139 numeric —
bands, not sums: classical +25–60, numeric +15–40. The 2023 puzzle
pot may simply not be width-≤2 (the field is near-zero there too);
it is priced at +4–14 of 195, not at 195. Excluded up front and
pinned as a negative-control fixture: child-snack (symmetry's, Phase
6's) — referees stay undiluted.

## Phase 6 — the symmetry engine (the zero-traction unlock)

Confirmed at 0.21, three engines deep on one box: child-snack 0/20
opt + 8/20 sat with a factorial core, barman-2014 0/14, barman-2011
frozen — flat wall timeouts, no notes, no movement, ever. New
machinery, five levers, each sound before the next lands:

- **L1 (~60 LOC): optimal-mode canonical keys** — swap
  `task.state_key` for a canonical form at the three optimal.rs
  sites; nodes stay CONCRETE (canonicalization touches only
  visited/closed keys — the temporal pattern, so plan extraction and
  VAL are untouched). Orbit-space A* with concrete states;
  admissible h + the existing re-opening keeps first-goal-pop
  optimality. child-snack needs NOTHING else — detection fires today
  (sandwiches k≤20 SOLO orbits).
- **L2 (~120 LOC): the metric/cost-uniformity gate** — cells sharing
  an equality class must carry equal op cost or detection bails.
  This lands BEFORE any total-cost domain is enabled, with a
  negative fixture proving the bail (the soundness trap is merging
  quality-distinct states). Unlocks barman-opt, city-car,
  cave-diving.
- **L3 (~200 LOC): satisficing canonical dedup** in the parallel
  successor phase — canonical hash + canonical equality on
  collision, pay-per-duplicate so memory stays Phase-4-shaped.
- **L4 (~80 LOC): unary-goal SOLO units** (cave-diving's four
  identical divers, child-snack's children).
- **L5 (~60 LOC): goal-free-SOLO passdown into partition mode** —
  child-snack's SAT path routes through partition (probe receipt:
  "6 initial groups"), so the sat payout rides this lever.
- **Priced honestly:** the folklore "~30 barman pot" DEFLATES on
  probe — i1 shows a 2.77M-expansion wall against only ~4× orbit
  collapse (the goal pins most shots): barman is +0–4, not 30.
  Bands: child-snack-opt +2–6, cave-diving-opt +1–3, city-car-opt
  +1–4, child-snack-sat +2–8 via L5, hiking/transport +0–3. Phase
  total +8–20, and the TMS temporal constituency (goal-paired piece
  symmetry, the 0.13 fence) is named for 0.23 once the engine
  exists.
- **Referees:** the 354-certificate differential with orbits ON
  (zero cost mismatches or the phase stops); barman-sat 20/20 and
  GED 20/20 as no-loss canaries (the canonicalization tax has 10×
  headroom there and must keep it); t1≡t8 determinism pinned.
  Hatches: `FF_NO_ORBIT_CLASSICAL` isolates the new consumer.

## Phase 7 — grounding scale (the walls that never reach search)

The decode receipts are unambiguous: 2048 spends its ENTIRE budget
enumerating 435M binding nodes to produce a 111-fact task whose
plan then takes 4,825 evals (~7 s) — every static literal names the
LAST-declared parameter, zero pruning above the leaf; the domain
header literally assumes a join-aware grounder. caldera i4: 2,292 of
2,292 stack samples inside the binding recursion. organic-synthesis:
all four precondition predicates DYNAMIC — static pruning has
nothing to hold (the 0.21 fixpoint receipt stands: memory flat,
time is the wall).

- **Lever 1 (byte-identical by construction):** MCV join ordering
  inside `for_each_binding` above a per-action typed-product
  threshold (`FF_MCV_THRESHOLD` 1e6): greedy bound-connected
  most-constrained-variable order, then survivors SORTED BACK to
  declaration row-major order before emission — the RawOp stream and
  fact-intern order are byte-identical, so the recorded sokoban-t
  regression class is structurally impossible here. 2048's 435M
  nodes collapse to ~50k analytically.
- **Lever 2:** threshold-routed fixpoint on classical/numeric solve
  entries (`FF_FIXPOINT_THRESHOLD` 1e8) — org-synth's and caldera's
  dynamic gating literals finally give MCV selective joins. Fact-id
  order shifts ONLY for tasks that today ground NOTHING inside the
  budget — "order changed" is vacuous there. A product-audit script
  asserts every currently-solved domain on all 13 boards sits BELOW
  the threshold (agricola, 246,879 ops on the plain path, is the
  named near-threshold case and the negative-control fixture).
- **Lever 3 (escalation, pre-priced):** indexed hash-joins, +1–2
  days, only if the pre-registered org-synth gate misses.
- **Gates, pre-registered:** 2048 i8 grounds <1 s; org-synth i01/i11
  <30 s (the exact 0.21 Phase 8 gate); caldera i4 <10 s — all solo,
  all before any board time. Bands: 2048 +2–8 (floor honestly 0 —
  the search behind the wall is unmeasured), org-synth +3–6,
  org-synth-split slice +1–3, caldera +1–4.
- Named out of the pot by probe: onlycraft (grounds in 1.3 s now —
  the 46 s transients died with Phase 6's fold era), settlers-snp
  (the wall is PARTITION-mode machinery, re-attributed to Phase 1's
  instrumentation), gear-car and line-exchange (search-bound).

## Phase 8 — the denominator (the whole table on one box)

The think-big move that is not an engine bet: **2,290 instances sit
outside the honest table** in nine cloud-era boards the re-baseline
never touched. Three of them re-enter this cycle — propositional
(450), net-benefit (270), constraints (120) — 840 instances, ~5–6 h
of sweep at the standing discipline, no engine risk, and the table
grows 13 → 16 boards measured on one box. constraints was 5/120 even
on the cloud container: it is a coverage OPPORTUNITY wearing a
bookkeeping coat. The mco trio (840 more) waits for its methodology
sitting (wall-clock-per-rule on a 4-Super-core box is a decision,
not a default), and time/metric-time re-baseline with 0.23's
temporal tier so they are measured once, not twice.

Riders, all cheap, all named:

- **Makespan recording starts NOW** (one runner line + a pin): the
  column fills at this cut's sweep so 0.23's temporal boards can
  score quality without a second re-baseline. A 0.14-era runner debt
  closes.
- **The temporal attribution sitting** the h-surgery probe was
  supposed to buy and never did: the 100-timeout non-zero-block mass
  (floor-tile 35, sokoban-t 34, driver-log 19, satellite 12) decoded
  per family on solo probes — the free seeds from this scoping are
  already on record (floor-tile i2 pure plateau; turn-and-open i2
  dedup-heavy grind). Findings pick 0.23's temporal lever.
- **Vendor the IPC-2011 temporal field results** — no per-instance
  archive is vendored for 2008/2011, so the 110-instance zero block
  is currently priced against NOBODY: if the field also scores ~0 on
  storage-t/TMS at comparable budgets, the roadmap should say the
  ceiling is shared. Bounding the pot costs an afternoon.
- **Conditions for all**: the sweep driver writes conditions.json
  for every board, closing the 6-of-13 gap.

### Recorded — the riders land; the boards wait for the cut

- **Harness in** (the day the build opened): cut22-sweeps.sh /
  promote-air22.sh carry all sixteen boards with the 0.21 contention
  machinery wholesale — conditions-for-all comes free; the three
  re-entry tracks verified to enumerate; makespan recorded on every
  solved row from now on; the three labels moved to "sweep in
  flight" in the standings.
- **The sitting ran, and re-attributed a family.** floor-tile-t is a
  pure plateau both eras (best_h flat, zero duplicates, zero
  blocked — the clear-chain lever's constituency, symmetry
  secondary); driver-log plateau-provisional; satellite honestly
  PENDING, never guessed. The hard finding: **sokoban-t's bulk is a
  GROUNDING wall, not a search wall** — two of three probes died
  pre-search with stack samples inside `for_each_binding` (the
  Phase 7 MCV class; push's typed product runs 1e7–1e8). Its 34 do
  not price into 0.23's temporal pot until Phase 7's gates
  re-referee them. Protocol note for every future sitting:
  `FF_NO_ESCALATE=1` is mandatory for eval-denominated probes —
  escalation re-arms the eval budget and the goal-decomposer pass
  ignores it entirely. Table: benchmarks/metrics/
  temporal-attribution-0.22.md.
- **The zero block is priced against the field at last**
  (benchmarks/ipc2011-temporal-field.md, provenance labeled per row
  — the official 2008/2011 per-instance tarballs are LOST, and the
  file says where it looked): at the field's 30-MINUTE budget the
  block falls only to machinery we do not have — TMS to
  required-concurrency planners (POPF2 5/20; ITSAT 18/20 while
  every non-SAT 2014 entrant scored 0 valid), model-train to
  reachability at 60× our budget, storage to the ITSAT/SCP2/LPG-td
  class while OPTIC and TFD, the styles nearest ours, score 0/20.
  Half the field sits at zero on every one of the three: a
  planner-STYLE ceiling, shared with our nearest relatives. And the
  budget consolation, on the record: the 2011 coverage leader
  banked 144 of its 145 solves by t=196 s — the 30-minute tail was
  nearly worthless even to the winners.

## Phase 9 — cut 0.22.0 (the swing meets its referee)

The standing template — every board re-swept against the final
binary (sixteen now), records complete per phase, full pre-flight,
finish in main, the user publishes — plus this cycle's specifics,
all named above: the repaired differential runs at BOARD BUDGET
before any board; the centerpiece is refereed against the old-binary
columns (v0.19 + a fresh v0.21.0 backfill — budget reallocations are
invisible to hatches, the rule is two cycles old now) with
casualties named per domain; the three re-entered boards join the
snapshot; the comparability column re-prices the numeric boards
against the published field numbers.

The ambition, stated as bands that will defend themselves at the
cut, not as a promise: the engine phases sum to **+80–190 instances
on the 4,076** (Phase 2 +6–15, Phase 3 +20–30, Phase 4 +8–15,
Phase 5 +40–100 across classical and numeric, Phase 6 +8–20,
Phase 7 +5–16, less Phase 1's −8 debt repaid or recorded), which
puts the thirteen-board headline between **55% and 57.5%**, and the
grown sixteen-board table in the same band if the re-entered boards
hold near their cloud-era rates. The stretch — everything landing at
the top of its band — reads 58%. The floor — the centerpiece
repeating 0.17's history — is caught by the backfill column and
recorded, because that is what the column is for.

## Anti-pots, recorded so nobody reaches for them

- **A 120 s classical tier:** of the 19 instances that fail at 60 s
  and solve at 300 s, twelve need >150 s — a 2× tier buys ~2–4
  solves. Killed by arithmetic.
- **A 60 s temporal tier:** the fresh distribution tightens 0.21's
  estimate — the last budget doubling bought 18 solves at a 0.72
  decay ratio; [30,60) projects ~13–18 of 336 (3.9–5.4%). Mechanism
  first; the tier waits for 0.23 where it pays for the time/
  metric-time re-baseline at the same sitting.
- **Another temporal h-accounting variant:** eight
  mechanism-precise negatives, the last two run BY THIS SCOPING —
  the numeric charge armed on temporal groundings re-levels
  model-train's plateau 6 → 13 and stays flat at board scale
  (683,555 evals, no plan); pairwise agenda-doom kills 92% of TMS
  candidates AT BIRTH and best_h re-levels 110 → 180, flat across
  4×. The plateau is not an accounting artifact. The accounting
  lever class is EXHAUSTED on this wall; what remains is structural
  (window/deadline propagation, or the symmetry engine's temporal
  constituency) and belongs to 0.23 with pre-registered reads.

## Deferred, on the record (carried forward)

- **The temporal-focused 0.23, pre-scoped:** the 60 s tier move +
  IPC-5 time/metric-time re-baseline in one sitting (with the
  epsilon_separate 2000-happening pin BEFORE the sweep); then ONE
  pre-registered structural bet gated by probe pair — goal-
  isomorphism symmetry (the 0.13 fence, cheapest, sound by
  construction, cross-track constituency) before TRPG-lite
  (time-stamped relaxation, the full-fat version of what the
  endgate approximated; 486 solved rows are its regression
  surface). Storage stays honestly unattributed-to-a-winnable-
  mechanism until the Phase 8 sitting says otherwise.
- **Numeric-optimal proper** (PDBs/CEGAR on the Count Downward
  shape) — Phase 4 is the beachhead; the cycle after it decides on
  Phase 4's boards.
- **Incremental LM-cut** — promoted to LIVE-candidate-on-evidence:
  three certificates within 1.7 s of the wall say the near-miss
  class exists; Phase 3's boards referee whether it is the dominant
  residue.
- **The mco methodology sitting** (three boards, 840 instances,
  wall-clock per competition rule on heterogeneous cores).
- **Per-node CoW leftovers; IPC-5 complex preferences; cross-mind
  planning; continuous `#t`; dynamic derived predicates** —
  unchanged standing lists.
- **The epistemic track (IPC 2026's other track)** — a watch item
  only: EPDDL is a different planner, not a phase.
