# ferroplan 0.18 roadmap — the living-village cycle

Scoped 2026-07-27 at the 0.17 cut, from the frontier cycle's own
debts ledger — every phase traces to a named debt or a recorded
next-idea. The arc: pay the correctness debt first (the 0.14-ext
precedent), then make the village LIVE and VISIBLE — the tick-loop
economy and the screens severed whole out of 0.17 — with the cycle's
engine bet being the one the novelty referee pointed at.

## Phase 1 — the ε-emission order inversion (correctness first)

The 0.17 audit's named engine bug, mechanism on file (Phase 2
record): same-epoch end pairs that the tie-scan legally reorders
INTERNALLY get their emitted ends crossed by ε-staggered starts —
the mend's emitted end lands ε past its match's emitted end and VAL
reads the invariant broken for exactly 0.001. Internally sound,
emitted unsound; the same family as the ε mutex gaps that led the
0.14 extension.

- Fixtures first: pin match-cellar-2014 i1 (the probe's witness —
  20 red instances in that variant alone, +3 map-analyzer) as a
  failing VAL-green test BEFORE the fix.
- The fix lives in the ε-separation EMISSION pass: ends riding on
  ε-shifted starts must not cross an invariant-provider's emitted
  end when the internal order had them at-or-before it — clamp or
  co-shift, preserving total ε-order and every existing green
  board's plans byte-for-byte where no same-epoch pair exists.
- The fix invalidates every temporal scoreboard by definition:
  full temporal re-sweeps (tempo-sat 630 + 240, tempo-2014 200)
  against the fixed binary with A/B attribution; expect the 23
  VAL-reds green and zero regressions elsewhere; casualties named
  and solo-checked as always.

### Recorded — twenty of twenty-three green; the other three were never this bug

The fix: `epsilon_separate` now carries end-op identities through its
execution-order sort and repairs SAME-SLOT end groups by invariant
relation — if A's deletes hit B's invariant-positives (or adds hit its
negatives), B's end emits first; a bubble pass over groups ≤16, cycles
left to the existing STN-consistency veto. Zero-slack geometries
(durations exactly filling the window) admit no strict separation and
keep the recorded STN-infeasible fallback to raw times. Fixture first:
`benchmarks/bench/eps-cross-*` pins the compressible-slack shape as a
unit test directly on the emission pass (the first draft asserted on
the zero-slack shape and "passed" through the fallback — a false green
caught by running the real witness).

The measurements, against the fixed binary at the standing 30 s budget:

- **match-cellar-2014: VAL 0/20 → 20/20.** The whole cluster green,
  plans and coverage byte-stable (20/20 solved both eras — the bug was
  always emission-only).
- **tempo-sat 630 (2006/2008/2011): 399/630 → 399/630**, zero wins,
  zero losses, zero VAL movement instance-by-instance — the fix
  touches nothing without a same-epoch end pair.
- **tempo-2014 overall: 65 solves stable, valid 42 → 62.**
- **map-analyzer: 8/11 → 8/11 — the hypothesis REFUTED.** The 0.17
  decode guessed its 3 reds were this bug; they are not. Solo-check
  (i17, verbose VAL): *"Failed duration constraint: Set the duration
  to 150"* — **state-dependent duration drift**, a different class:
  the duration expression reads fluents, ε-separation shifts a start
  across another action's fluent write, and the committed duration no
  longer equals the expression's value at the emitted start time.
  Named 0.19 debt (deferred list below).

## Phase 2 — the village, alive (the tick loop)

`bazaar_live`'s successor over `benchmarks/village/`: N workers in
one authoritative world, the loop hiring by Session goal contract
(rung 3's mechanism, now driven), the market moving, disruptions
(a poached worker, a bought-out stall) arriving as drift. Measured
like the bazaar was: ticks, thinks, follows, conflicts, tick
latency; scoreboard beside `bazaar-thinks.md`. The start-credit
plateau's game-shaped witness (gather-spam vs think budgets) gets
its numbers here — the h-surgery fence's file grows or the fence
falls, measured.

### Recorded — two workers, one economy, theft survived

`examples/village_live.rs`, the sighted-tier loop (fog rides with the
live page's next act): one authoritative world `Session`; a think is a
fresh fork restricted to the worker's own labor with their contract as
goal; validity is the free suffix replay on a PROBE FORK carrying that
contract (the world session's own goal belongs to nobody — mara
thrashed 120 thinks until that landed); dispatch is `apply_start` in
the parenthesized plan-step form; interval ends fire from `elapse`.
Theft at tick 17 (planks 3 → 1) breaks bob mid-flight. Measured run
(`benchmarks/village-live.md`): bob 2 contracts, 3 thinks, 19 steps,
1 drift rethink, done tick 47; mara 1 contract, 1 think, 13 steps,
done tick 32; ~1.5 M think evals, 13 s wall for the whole run. The
gather-spam witness stands in the record; the h-surgery fence holds.

## Phase 3 — the screens (the severed Phase 5, whole)

- **The village live page**: the wasm demo's next act — map and
  economy timeline over the live loop, craftsmen with visible
  intentions, contracts in flight, disruption buttons.
- **Plan introspection views**: temporal Gantt with invariant
  spans, classical causal chains, preference satisfaction
  breakdown — the planner made legible for any solved instance.

### Recorded — the screens stand, and their smoke test caught a seven-cycle-old corpse

The engine side is `introspect::explain` (new lib module, 4 unit
tests): **causal links** replay the plan over the solver's own
grounding recording each positive precondition's last achiever
(static init-only facts dropped as noise); **invariant spans** render
each durative step's `over all` conditions from the ORIGINAL schema
with arguments substituted — exactly what VAL checks over the
interval; **preference breakdown** scores goal preferences in the
final replayed state and soft trajectory preferences via the verify
oracle. The wasm surface adds `explain` plus the village Session verbs
(`restrict_contains`, `apply_start`, `elapse`, `set_fluent`/`fluent`,
`plan_valid_json` — the probe-fork validity shape). The pages:
**Explain this plan** on the solver demo renders all three views;
`village-live.html` runs the Phase 2 loop in-browser — map, economy
sparklines, contracts and visible intentions per worker, theft and
till-raid disruption buttons.

The find: the smoke test's first run showed EVERY temporal example
failing in wasm while classical passed. Root cause:
`NODE_CAP_TARGET_BYTES = 8 << 30` — on wasm32 usize is 32-bit and shl
silently DROPS the high bits, so the "8 GiB" default node-cap target
wrapped to ZERO and every default-cap search (all of temporal, the
classical best-first fallback) died at its first insertion — since the
cap landed in 0.8, invisibly, because EHC-solvable classical demos
still passed and the bazaar thinks pass explicit memory budgets.
Fixed with a width-guarded 2 GiB 32-bit ceiling (64-bit byte-identical);
forge-order/jobshop wasm probes went unsolved → solved (makespans 65 /
12.015). `crates/ferroplan-wasm/smoke.js` (Playwright, headless) is now
part of the cut drill: causal chain, invariant spans, village first
ticks, page-error sweep — SMOKE PASS on the shipped pages.

## Phase 4 — the budget-aware ladder (the referee's next idea)

The novelty rung's +7/−51 verdict named the mechanism: rungs
SEQUENCED on one wall clock tax the fallback. The bet: make the
classical ladder budget-aware — spend a bounded rung only when the
remaining wall budget affords its cap, or interleave rungs on one
clock — so the novelty rung's six real h-dies-outright gains stop
costing forty-five budget-edge solves. This is also exactly what
the agile track's scoring rewards; the 2018/2023 referee boards
re-run as its judges. The numeric-heuristic upgrade (landscape
memo bet #2; the 2023 numeric track's 112/400 is its baseline)
stays the named NEXT swing — taken this cycle only if the ladder
work lands early and clean.

### Recorded (implementation) — the gate is in; the referee boards ride the cut

`FF_TIME_LIMIT=<secs>` arms a wall clock at `solve` entry (grounding
counts as spent budget); a bounded rung (LAMA, novelty) is entered
only while MORE than 40 % of the budget remains — early rung failures
still buy their wins, late ones stop starving the complete fallback.
Unset ⇒ all-rungs, byte-identical to 0.17; on wasm the frozen clock
degrades the gate to always-affordable, correct for Session thinks.
`benchmarks/ipc67.py` now passes its per-instance `--timeout` to every
ff spawn; `FF_WALL_DEBUG=1` narrates the verdict (the probe eyes).
Gate verified on sokoban-2008 i1: unset → `None`/affordable, 300 s →
0.999 remaining/affordable, 0.05 s → 0.0 remaining/SKIPPED.

**The referee's verdict** (all eight gate-touched classical boards
re-swept under the wired runner at the cut, final binary, standard
60 s budgets):

- **Base boards (gate armed, no novelty): neutral within noise.** The
  580-instance seq-sat flagship is variant-for-variant identical at
  441/580; 2018-sat and 2023-agile are instance-for-instance
  unchanged; 2014-sat +1 (genome-edit-distances i13), 2014-agile
  +2/−1, 2023-numeric −2. Every casualty was solo-checked
  UNCONTENDED: sugar-numeric i2 (29 s), zenotravel-numeric i20
  (59 s — on the 60 s line), hiking-agile i5 (23 s) all solve
  identically with and without `FF_TIME_LIMIT=60` — the losses are
  jobs-3 contention noise at the budget edge, not gate tax, and the
  scattered +3 wins are the same coin's other face.
- **The novelty rung under the gate: +4/−0** — 2018-sat-nov 41/240 =
  +3/−0 over the gated base (organic-synthesis-split i15, termes
  i12+i16) and +2/−0 over 0.17's ungated novelty board;
  2023-agile-nov +1/−0 (quantum-layout i10). Against 0.17's ungated
  verdict of **+7/−51**, the mechanism is confirmed: the tax was the
  rung starving the complete fallback near the budget edge, and with
  the wall budget declared the rung keeps its h-dies-outright wins
  and pays nothing. `FF_NOVELTY` stays opt-in this cycle (the +4
  arrives only when the runner declares a budget), with
  default-on-under-FF_TIME_LIMIT the recorded 0.19 candidate.

## Phase 5 — cut 0.18.0

The standing template: every scoreboard the cycle touched re-swept
against the final binary (temporal boards from Phase 1, referee
boards from Phase 4, village/bazaar boards from Phase 2), records
complete, full pre-flight, finish in main; the user publishes.

### Recorded — cut

Every touched board re-swept against the 0.18.0 binary: the two
temporal boards (Phase 1's verdict above), all eight gate-touched
classical boards under the wired runner (Phase 4's referee above —
including repairing the flagship's empty raw JSONL, which now rides
beside its md again), the village boards from Phase 2. Standings
regenerated: 2014 seq-sat 96/280, seq-agile 95/280, tempo-sat 62/200
(VAL-RED 23 → 3), 2023 numeric 110/400 (the two contention-edge rows,
solo-verified green). Full pre-flight green on rustc 1.97.1: fmt
--check, clippy `--all-targets --all-features -D warnings` (one MSRV
catch: `is_none_or` → `map_or`, stable-since 1.82 vs the 1.74 floor),
`test --all --release`, the heavy qual-metric tier
(`--include-ignored`, 4/4 in 209 s), doc `-D warnings`, bench
`--no-run`, mcp build, `publish -p ferroplan --dry-run`, the
0.18.0 maturin wheel, the wasm build + browser smoke (SMOKE PASS).
Two container restarts hit the cut sweeps mid-flight; the
resume-aware driver (`benchmarks/cut18-sweeps.sh`, per-board done
markers) lost only the in-flight board each time.

## Deferred, on the record (carried forward)

- **State-dependent duration drift** (NEW, Phase 1's refuted-hypothesis
  find): map-analyzer-2014's 3 VAL-reds — the duration expression
  reads fluents, ε-separation shifts a start across another action's
  fluent write, and the committed duration no longer equals the
  expression at the emitted start time (VAL: "Failed duration
  constraint"). The emission pass needs to re-evaluate state-dependent
  durations at emitted times, or veto ε-shifts across writes to
  duration-read fluents. Witnesses: i17/i18/i20.
- The h-surgery bet (end-gated interval credit) — unless Phase 2's
  measurements force it sooner.
- Numeric-heuristic upgrade (subgoaling/AIBR class) — named next
  swing, see Phase 4.
- Lifted/lazy grounding — watch item (the stress test priced it
  out of urgency).
- VAL-side red clusters (drone-numeric, data-network-2018 domain
  parse rejects) — runner work, revisited if a VAL upgrade lands.
- The four timed modal operators, cross-mind planning, continuous
  `#t`, dynamic derived predicates, fixpoint/stratified
  unification — unchanged from the 0.15/0.16/0.17 lists.
