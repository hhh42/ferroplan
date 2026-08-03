# ferroplan 0.21 roadmap — the new box

Scoped 2026-07-31, immediately after the migration from the cloud
container to the M5 MacBook Air (`docs/migration-m5.md`). This cycle
opens differently from every cycle before it: not with a scoping decode
of the boards, but with the machinery and the measuring stick, because
neither survived the move intact. Phase 0 is the port. Phase 1 is the
re-baseline the migration guide makes mandatory before any A/B claim.
Only after those does the ledger get read.

Two facts set the frame:

- **The 0.20 cut never happened.** 0.20's phases 1–5 all landed in main
  on the container, and the cut prep (version bumps to 0.20.0, the
  IPC-2026 corpus fetcher, the sweep driver) landed with them — but
  there is no `v0.20.0` tag, no `0.20.0` CHANGELOG entry, and
  `docs/roadmap-0.20.md` Phase 6 has no Recorded section. The container
  died with the cut sweeps unrun.
- **So the cut sweep and the re-baseline are the same pass.** Twelve
  canonical boards, one final 0.20.0 binary, on the Air. There is no
  version of this where they are two sweeps: 0.20's boards have to be
  Air-baselined anyway, and the migration guide's rule is absolute —
  nothing on the Air may be compared against a cloud-box number.

## Phase 0 — the port (recorded)

Fixtures first, on a box where the fixtures themselves were the thing
in doubt. Three blockers stood between this machine and a single
honest row; none of them are scoreboard numbers, and all three are
committed with their receipts.

1. **The runner could not spawn one job.** `RLIMIT_AS` on macOS reports
   INFINITY and rejects every `setrlimit` on it with EINVAL — surfacing
   in Python as `ValueError`. Raised inside `preexec_fn`, subprocess
   re-raises it as `SubprocessError`, and `ipc67.py`'s spawn-retry then
   booked EVERY instance as `spawn-fail` after its 5 s breather. The
   twelve-board sweep would have run ~5.6 hours and produced 4,016
   garbage rows, each one looking like an environmental fork failure.
   migration-m5 predicted the cap "may not fire"; the truth is worse
   than a silent no-op. Now probed once at startup — lower the soft
   limit, put it back, side-effect-free on both kernels — and
   `preexec_fn` is installed only where the cap actually takes.
2. **The mem-cap column got its instrument back.** With `RLIMIT_AS`
   unavailable, the rusage watchdog migration-m5 names: a 0.25 s RSS
   poll that kills a job over `--mem-gb` and books it `mem-cap`, read
   BEFORE the generic nonzero-exit branch so a SIGKILLed job does not
   masquerade as `engine-exit--9`. Pinned against a 400 MiB balloon
   under a 200 MiB cap: killed in 0.5 s, rc=-9, verdict `mem-cap`.
   **On this path the column measures RESIDENT bytes, not address
   space** — a different instrument reading the same column, and the
   0.20 Phase 4 mem-cap referee must be read with that substitution in
   mind. It is also the more honest instrument for the question the
   column is asked: RSS is what drives a box into swap.
3. **The IPC-2026 corpus lost three instances to its own normalizer.**
   `get-ipc.sh` mapped a 0-indexed `p000.pddl` to the empty string via
   `sed 's/[^0-9]*0*//'`, producing an `instance-.pddl` in gear-car and
   both sailing-wind variants — which the runner's
   `int(re.search(r"\d+", name))` died on mid-listing, taking the whole
   board down with it. Fixed at the source (`instnum()` keeps a final
   digit) and defended in the runner, which now skips un-numbered files
   LOUDLY rather than crashing: a corpus glitch must not read as a
   smaller corpus. 16 domains × 20 instances all present.

Also ported: `get-val.sh` builds again (CMake 4.x removed the pre-3.5
compatibility VAL's CMakeLists declares; `nproc` is GNU-only, so `-j`
silently fell back to 4). VAL 4 builds and validates on ARM.

**Green on the Air**: `cargo test --all --release` — 213 passed, 0
failed, across 32 suites. Corpus: 1.7 GB, eight competitions,
including the IPC-2026 numeric dataset that was a blocked rider at
0.20 scoping (the organizers' repo is public now).

## Phase 1 — THE RE-BASELINE (the new measuring stick)

The mandatory pass. Twelve canonical boards, the 0.20.0 binary,
`benchmarks/rebaseline-air.sh`, resume-aware, superseding
`cut20-sweeps.sh` on this box. Every number it produces is
Air-baselined; the cloud-era boards stay in git history as the record
of the old box, and from here on A/B is Air-vs-Air only.

### Conditions, recorded before the numbers

An honest board carries its conditions, and this one has two the cloud
boards never had:

- **The box is heterogeneous.** The M5 Air is 4 "Super" cores + 6
  Efficiency cores, not 10 equal ones. This matters more here than
  thermals do: coverage-at-timeout is the boards' metric, and a job
  scheduled onto an Efficiency core does materially less work per wall
  second, so job placement alone can flip an instance at the budget
  edge. The job count is therefore a measured decision, not a taste —
  see the calibration below.
- **The box is not dedicated.** A GitLab CI runner runs in Docker on
  this machine and picks up jobs on its own schedule, ~1.7 Super cores
  per job. By explicit decision the sweep runs anyway rather than
  pausing the user's CI, so **these boards are measured under variable
  CI contention** and that is a property of the boards, not a footnote:
  coverage near the budget edge is noisier than the cloud boards' was,
  and a future Air-vs-Air A/B inherits the same noise on both sides
  only if it runs under comparable load. Any single-domain claim taken
  off these boards gets a solo re-check on a quiet box before it counts
  as a win. This is the cycle's standing caveat.

### The job count, measured (`--jobs 2`, and why not more)

The guide asserted `--jobs 2` for a fanless chassis. That was written
predicting this box, so it was re-derived by measurement: K identical
copies of hiking-2014 i6 (~23 s solo, `--threads 1`), levels
INTERLEAVED across three repetitions so a CI burst inflates every level
rather than biasing one.

| K | median | inflation | throughput vs K=1 |
|---|---|---|---|
| 1 | 23.3 s | 1.00x | 1.00x |
| 2 | 27.0 s | 1.16x | 1.72x |
| 3 | 25.0 s | 1.08x | 2.79x |
| 4 | 27.8 s | 1.19x | 3.35x |
| 6 | 59.2 s | 2.54x | 2.36x |
| 8 | 58.5 s | 2.51x | 3.18x |

Two results, and the second is the one that decided it:

- **A cliff past K=4**: inflation jumps to ~2.5x and throughput
  actually FALLS. That is the 4-Super-core boundary — beyond it jobs
  land on Efficiency cores, which is precisely the placement lottery
  that makes coverage-at-timeout meaningless. Nothing above 4, ever,
  on this chassis.
- **The medians cannot discriminate K=2 from K=4.** Per-rep spread
  swamps them: K=2 measured 24.9 / 57.6 / 27.0 s — a 2.3x swing at
  IDENTICAL job count. That is the CI runner, and it is the honest
  headline of this calibration: on a shared box the fine-grained job
  decision is not measurable at three reps.

So the number comes from arithmetic the medians cannot supply: four
Super cores, minus the ~1.7 the CI runner holds while a job is active,
leaves ~2.3. `--jobs 2` fits inside that; `--jobs 3` oversubscribes
whenever CI picks up work, and buys throughput by spending exactly the
comparability the re-baseline exists to establish. **If the box is ever
dedicated, re-run this calibration — K=3–4 becomes the right answer
and the sweep gets ~2x faster.**

### Found mid-sweep, on the record

The re-baseline is already earning its keep as an audit, not just a
measuring stick. Two findings, neither of them a coverage number:

- **The VAL-RED clusters are VAL-side, and the boards understate us by
  15 instances.** Two domains came back with EVERY solved plan rejected
  — data-network-2018 (7/7) and factory-robot-2026 (8/8). An intermediate
  reading here recorded data-network as engine-side, reasoning that
  `val_check` already returns `None` on "Parser failed" so VAL must be
  parsing the domain and genuinely rejecting. That was wrong, and the
  correction is the finding: **VAL has more than one way to refuse a
  domain.** Both emit `Problem in domain definition!`, and both do so
  against an EMPTY plan — plan-independent, so VAL never judged our
  plans at all. 0.20 Phase 5's expectation that data-network "gets the
  same treatment as drone-numeric" was right; the 0.20 fix was simply
  too narrow, catching one VAL message out of several.
  A sweep of all 216 domains for VAL ingestibility names exactly four:
  `data-network-2018`, `drone-numeric-2023` (the known one, already
  returning `None`), `sailing-numeric-2023`, `factory-robot-2026`.
  The cost is not cosmetic, and it lands in a specific place. The RUNNER
  counts raw `solved`, so the board headlines are already right:
  ipc2018-sat 53/240 and ipc2026-numeric 121/320 stand as published.
  `standings.py` is what drops them — it scores
  `solved = r["solved"] and val is not False` — so the STANDINGS TABLE
  would have booked those same boards as **46/240** and **113/320**, 15
  instances light, while the boards beside it said otherwise. Two
  artifacts disagreeing about the same sweep is worse than either being
  wrong alone. Fixing `val_check` makes standings agree with the boards;
  it does not raise the boards. (An intermediate note here had this
  backwards, adding the 15 to the headlines instead of recognising the
  headlines already contained them.)
  That is the third time this cycle the scoreboard has been caught
  fibbing and the third time not in our favour — 0.20 Phase 1 found the
  other two. Because the refusal is domain-level, every affected row
  reclassifies soundly from the raw JSONL; no re-sweep is owed. Fix
  `val_check` to test a LIST of unavailability signatures, and apply the
  reclassification at promotion.
  The contrast that proves the mechanism is right and only its signature
  list was short: drone-numeric on the 2023 board solves 16/20 and every
  one of those rows carries `val: None`, counted and correct — the same
  treatment, on the one signature 0.20 already knew.
- **A latent runner misattribution, named before it bites.**
  `val_check` ends `except Exception: return False`, which swallows the
  120 s `TimeoutExpired` — so a VAL that runs out of time books as a
  REJECTED PLAN. That is the 0.20 Phase 1 shape exactly (graceful
  wall-exits booked as engine-rejects), on the one column standings.py
  calls a first-class signal. Confirmed latent on these boards, so it
  corrupts nothing here and no re-sweep is owed; fixed AFTER the sweep
  so the instrument stays identical across all twelve.

### Recorded — twelve boards, one box, one honest table

21.5 hours, twelve canonical boards, zero interventions. **48% across
12 boards (1,917/4,016), of which 306 are certified optima** — on the
optimal tracks coverage is proof rate. The at-a-glance table is
`STANDINGS.md`; the per-track detail `benchmarks/ipc-standings.md`;
the raw evidence the twelve JSONLs plus `benchmarks/air/`.

- **482 of 485 temporal plans validate** (419/419 and 473/473 green on
  the IPC-6/7 boards); the three failures are exactly the map-analyzer
  rows 0.20 Phase 5 recorded as its honest negative — reproduced, not
  approximated.
- **LM-cut proves 13 of 306 certificates (4.2%)**, all in the
  elevator/woodworking families; the ladder is wired correctly but at
  60 s its per-node cost does not pay. WHY it never gets the chance is
  now decoded — see Phase 4 below.
- The two mid-sweep findings landed AFTER the last board finished, so
  all twelve shared one instrument: `val_check` tests the signature
  LIST and a VAL timeout returns `None`; `val-availability.py` probed
  all 216 domains and `standings.py` reads the resulting map — the
  table and the boards tell one story (2018-sat 53, 2026-numeric 121).
- `benchmarks/promote-air.sh` promotes all twelve or none — a partial
  promotion would put an Air board and a cloud board under one name.
  BOTH optimal raws are tracked as evidence (`opt-differential.py`
  replays every certificate from a fresh clone), and the snapshot is
  banked in `standings-history.json` so the 0.21 cut can show the
  first Air-vs-Air movement column this project has ever had.
- 0.20.0 was cut and tagged from this record, front page and all.

## Phase 2 — the backfill, and what it found (recorded)

The re-baseline broke the trend line by design, leaving "improvement"
unsayable. The repair, proposed by the user: re-measure an OLD tagged
engine on THIS box. `benchmarks/backfill-air.sh` builds a tag in a git
worktree and sweeps it with the CURRENT harness via `$FERROPLAN_FF`, so
only the engine varies — checking out the old `benchmarks/` too would
vary the instrument, and this cycle's `val_check` fix is exactly why
that matters.

v0.19.0, twelve boards, ~20 h, same `--jobs 2 --mem-gb 6`:

| board | v0.19 | v0.20 | Δ |
|---|---|---|---|
| ipc-opt-2008-11 ⚖️ | 235/550 | 250/550 | **+15** |
| ipc67-results | 472/580 | 473/580 | +1 |
| 2023 numeric | 193/400 | 194/400 | +1 |
| ipc67-temporal | 419/630 | 419/630 | **0** |
| 2023 agile ENTRY (300s) | 49/140 | 48/140 | −1 |
| 2014 tempo-sat | 68/200 | 66/200 | −2 |
| 2023 classical | 30/140 | 27/140 | −3 |
| 2026 numeric | 124/320 | 121/320 | −3 |
| 2014 seq-sat | 115/280 | 110/280 | −5 |
| 2014 seq-opt ⚖️ | 64/256 | 56/256 | −8 |
| 2018 seq-sat | 63/240 | 53/240 | −10 |
| 2014 seq-agile | 114/280 | 103/280 | −11 |
| **total** | **1946** | **1920** | **−26** |

**0.20 is a net coverage regression against 0.19.** Two independent
causes, one shared mistake: a two-rung ladder that starves the rung
that was already working.

### (A) The satisficing ladder — novelty-light taxes every search

Per-domain, the same domains win and lose on two independent boards:

    2014 seq-agile   visit-all 1->20 (+19), maintenance +4
                     hiking -10, child-snack -8, openstacks -8,
                     thoughtful -3, cave-diving -3, city-car -1, parking -1
                     gains +23, losses -34, net -11

The rung sits after EHC and BEFORE LAMA, default-on under a declared
budget, so it spends wall ahead of the rung that would have solved.
Phase 3 priced its tax at "~1 s" from a single sokoban probe; domains
going to ZERO (child-snack 8→0) say that probe was not representative.
The differential confirms the win is real (`FF_NO_NOVLIGHT`: visit-all
20/20 → 1/20, worth +19) — it is simply bought at roughly 1.5× its
value elsewhere.

### (B) The optimal ladder — an unconditional quarter-budget sprint

    ipc-opt-2008-11   v0.19 h^max x235      v0.20 h^max x237 + LM-cut x13   +15
    ipc2014-opt       v0.19 h^max x64       v0.20 h^max x56  + LM-cut x0     -8

The prover counts are the whole story. Where LM-cut can fire
(elevator, woodworking, tidybot — landmark-rich), 0.20 gets MORE h^max
certificates AND 13 LM-cut ones: the trade is strongly positive. Where
it never fires (2014's city-car, genome-edit-distances), the sprint's
quarter budget simply starves h^max and nothing compensates. The split
is not wrong, it is UNCONDITIONAL. Net across both optimal boards: **+7**.

### Where the tax bites, and where it cannot

Coverage loss scales with how many instances sit NEAR THE WALL, not
with board size:

- ipc67-temporal, 630 instances, **exactly 0** — temporal runs its own
  machinery, the classical ladder never fires. The cleanest control in
  the set.
- ipc67-results, 580 instances at 82% coverage, **+1** — almost
  everything solves far inside the wall, so a few seconds of tax cannot
  flip it.
- 2014-agile, 280 instances at 37%, **−11** — a dense population at the
  budget edge, and the tax converts directly into losses.
- The 300 s entry board regresses −1 where its 60 s sibling regressed
  −3: at five times the budget the tax is a fifth of the proportion.
  Predicted before that board ran, and confirmed.

### The methodological finding: a hatch only tests what it gates

Three hatch differentials all reported 0.20 favourably (`+19`, `+9 at
zero cost`, `+0 but spends the wall`). Every one of them is an internal
0.20-vs-0.20 comparison. `FF_NO_LMCUT` keeps the sprint's budget split;
`FF_NO_HMAX_SPRINT` gives LM-cut-only; NEITHER reproduces 0.19. An
architectural change to how BUDGET IS ALLOCATED is invisible to a hatch
that only removes a component. **Any future phase that reallocates
budget needs an old-binary referee, not a hatch.** The backfill caught
what three differentials could not, and it exists because the user
proposed it.

## Deferred ledger, carried in for scoping (from migration-m5)

Read after Phase 1 lands, against a fresh standings audit — not
before, because on this box the ledger's priorities are exactly the
thing the re-baseline is entitled to reorder:

- the temporal emission-layer repair (the map-analyzer third decode,
  0.20 Phase 5's honest negative)
- the numeric-reachability wall (the sailing class: sailing-numeric,
  markettrader, pathwaysmetric — all 0/20)
- per-node fv/fdef sharing (State's type ripples through
  temporal/session/wasm)
- lifted grounding watch (organic-synthesis, agricola)
- the h-surgery bet (end-gated interval credit)
- the optimal-mode entry for the 2026 `-opt` domain pairs — the
  vendored corpus ships `onlycraft`, `rainbowttles` and `sailing-wind`
  as -sat/-opt pairs, so this is now a concrete entry, not a wish
- IPC-5 complex preferences, cross-mind planning, continuous `#t`,
  dynamic derived predicates — unchanged standing lists

## The ledger, read — the numbers that order the cycle

Scoped 2026-08-01, at the 0.20.0 cut, per the rule above: after
Phase 1, against a fresh decode of all twelve Air JSONLs. The decode
reproduces STANDINGS to the row — 2,099 unsolved = 2,011 timeout +
74 mem-cap + 10 early-exit + 3 VAL-RED + 1 engine-reject — so the
boards, the table, and this scoping tell one story. Every line of the
carried-in ledger above was read against it and landed in a phase, a
probe, or the carried-forward list at the bottom.

The headline of the decode: **the ledger is now almost purely a
guidance ledger.** Timeouts are 95.8% of all failures. The classes
0.19/0.20 built machinery for are essentially EMPTY on these boards —
zero spawn-fails, zero legacy rejects, zero `engine-exit-*`, and
early-exit collapsed to 10 rows, every one of them sitting in the
[90%, 95%) wall window between the refill loop's re-entry floor
(`> 0.10` remaining, search.rs:1086) and the classifier's timeout
line (≥95%). That is a 3-second definitional seam, not a give-up: the
0.20 refill loop verifiably emptied the class it was built for.

Four numbers order the cycle, largest honest pot first:

- **817 classical-satisficing timeouts** (729 distinct instances —
  the 2023 corpus sits on two boards). The 300 s entry is the witness
  that this mass is wall-shaped, not budget-starved: 5× budget
  converts 27→48 with zero losses and 88 still standing.
- **500 optimal-proof timeouts** — the ENTIRE failure mass of both
  proof boards, single-class: all 500 are hard 60 s runner kills with
  empty notes, zero of anything else.
- **404 numeric losses** (363 timeouts + 31 mem-caps + all 10
  early-exits on the books), with NINE domains at 0/20 and one
  attributed single mechanism owning 111 of them.
- **331 temporal timeouts**, of which 110 sit in a three-domain zero
  block (storage 0/40, temporal-machine-shop 0/40, model-train 0/30).

And the gate that is not a pot: **3 VAL-RED rows + 1 grounding
verdict.** The map-analyzer three are Phase 7's soundness repair; the
one engine-reject on twelve boards — settlersnumeric i7, declared
"unsolvable at grounding" in 0.01 s on an official IPC instance —
gets hand-verified before any numeric work builds on that grounder
(Phase 3 opens with it).

**The honest inversion this cycle makes:** the biggest pot is not the
centerpiece. The classical mass's named lever is the full
novelty-as-driver guidance swing — a centerpiece-sized bet 0.20
already took one measured bite of — and the field just published a
sharper recipe for it (below), so it is deferred WITH its shape named
rather than half-taken. The numeric pot is smaller but is the one
0.20 explicitly named for this cycle, and its core mechanism is now
decoded to a single line of extraction code with a field ceiling
attached. Numeric is the centerpiece; classical gets the one slice
that is cheap and witnessed (Phase 5).

### Field refresh (for the record)

IPC 2026 published its results: the track was NUMERIC (agile /
satisficing / optimal, 13 domains — our vendored corpus), plus a
first-ever epistemic track; no classical, temporal, or HTN track ran.
**Panino** (Melbourne/RMIT) won agile and satisficing on *partitioned
numeric novelty* — novelty over subgoal-induced features (Boolean
achievement + numeric distance-to-subgoal), width ≤2, evaluated
inside h-add-induced partitions, with anytime cost-bounded restarts —
the width-for-numeric theory landing as an IPC win. **Count
Downward** swept the simple-numeric awards with numeric PDB and CEGAR
abstraction heuristics on Numeric-FD — numeric LM-cut is no longer
the frontier there. **LNP-optimal is nearly an open field**: the
winner scored 83/260 coverage against blind A*'s 74 — a fact Phase 4
gets to exploit. And **a second Rust numeric planner now exists**:
PlanForge (a Numeric-FD port, SNP-only, self-described experimental,
no crates.io release) — its headline fix, tolerance-canonicalizing
numeric values before duplicate detection, is the bug class our
fluent-bearing StateKey has quantized against since 0.19
(packed.rs 1e-6); Phase 8 carries the cheap audit that we do it
everywhere. No temporal track exists anywhere on the calendar, VAL
remains the field's validator, and the 2026 scoring formulas plus
per-planner per-domain results are public — which makes direct
comparability a cut-phase rider rather than a wish.

## Phase 3 — the numeric-precondition charge (the sailing class)

The centerpiece, and the mechanism is one line short of already
existing. The interval relaxation the ledger asked for is ALREADY IN
THE RPG — `Scratch.lb/ub`, monotone widening, interval evaluation and
satisfaction (heuristic.rs) — and both numeric achiever chargers ship
since 0.19 (`numeric_achiever_linear` heuristic.rs:828,
`numeric_achiever` :911). But extraction charges numeric distance
ONLY for top-level goals (`for np in goal_num`, heuristic.rs:448);
a selected op's UNSATISFIED NUMERIC PRECONDITIONS contribute zero.
Sailing's goal is propositional (`(saved p0)`) and save_person's four
band preconditions are exactly that — so **h is identically 1 across
the entire ~200–500-step approach**. EHC's lookahead dies on a flat
h; the novelty rungs are structurally blind (sailing has ONE
predicate, so every successor has identical bits and novelty sees
bits only, novelty.rs:74-88); the 5M-eval fallback burns the wall.
Probe receipt: instance-1 caps at 5,000,048 evals with 97 s of
cumulative h-build worker time.

The pot is 111 instances across both numeric boards, but it is NOT
one class, and the record says so up front: **sailing** (20+19 with
fo-sailing) is a pure extraction hole — the ENHSP family scores 20/20
on the same heuristic idea, fully winnable; **sailing-wind** (2026,
20 sat + 12 opt-set rows) confirmed the wall on instances 0.20 never
saw; **pathwaysmetric** (20) is the same hole chained through a
reaction DAG plus 9 mem-caps that belong to Phase 6; **markettrader**
(20) is a CYCLIC resource flow — LP-RPG's own paper domain, field
best 2/20 — re-attributed OUT of the winnable pot with a negative-
control fixture, not quietly dropped.

- **The gate, first (ten minutes, outranks everything):** hand-verify
  settlersnumeric i7's "goal fact (CONNECTED-BY-RAIL LOCATION6
  LOCATION3) unreachable" verdict. If the claim is wrong, reachability
  pruning is silently deleting coverage and THAT becomes the phase.
- **Instrument riders, before any referee sweep:** (a) the runner
  keeps multipart instance labels — ipc2026's
  `instance-3_10_50_10.pddl`-style names collapse to "3" via
  `int(re.search(r"\d+", ...))`, so the board holds 320 rows under
  288 keys and per-instance identity is broken for the diff and
  score-against workflows; (b) the text path prints "problem proven
  unsolvable" after a CAPPED search — an honesty bug (boards are
  unharmed; classification is elapsed-based), one line.
- **Fixtures FIRST:** a mini sailing-band domain (d=-30) pinning
  h(init) = 1 + ceil(gap/2) under the charge and an end-to-end
  mode-AUTO solve <1 s (sailing routes through partition mode — the
  fixture must prove the routing); tpp-numeric i1–i3 rows
  byte-equivalent (the 0.20 convention); the markettrader mini as the
  pinned negative control.
- **Lever a1 (smallest):** in extraction, after the goal_num loop,
  walk each selected op's unsatisfied `pre_num` and charge one level
  through the existing achievers. Hatch: `FF_NO_NUMPRE`.
- **Lever a2, only if pathwaysmetric stays flat:** recurse charged
  achievers' own `pre_num` through the worklist, depth-capped — the
  chained-resource shape can mis-charge against current-state fv, so
  damping is part of the lever, not a follow-up.
- **Probe rider b — numeric novelty** (`FF_NUMNOV`): quantized
  fv envelope per cell (reusing packed.rs's 1e-6 quantizer),
  numeric-task-gated. This is the field's winning direction, but it
  overlaps a1 on sailing and fixes neither markettrader nor the
  mem-caps; it enters as a probe and is promoted only on a measured
  win — 0.17's novelty promotions lost coverage twice, and that
  guardrail stands.
- **Probe rider c — the Eq refusal:** `numeric_achiever_linear`
  returns None on `CompOp::Eq` (heuristic.rs:842); block-grouping
  (0/20 here, field 19–20) likely hangs on exactly that shape. Probe;
  take only if small.
- **Risks, named:** the charge touches h for EVERY domain with
  numeric preconditions — the 315 currently-solved numeric rows are
  the regression surface; LAYER_CAP=2000 vs very large gaps can
  false-dead-end the RPG (safe for sailing's |d|≤500, unverified for
  2026 domains).
- **Referee:** both numeric boards; changed-class domains solo first
  as the cheap gate, full boards at the cut. The expectation, honest
  band: sailing 0/20→~18–20 bankable, sailing-wind-sat +3–8,
  pathwaysmetric +2–6, markettrader +0–2 ceiling recorded up front —
  net +25–40 across the two boards, or the negative is recorded.

### Recorded — the plateau becomes a gradient (boards at the cut)

- **The gate passed.** settlersnumeric i7's verdict is a CORPUS
  ARTIFACT and the grounder is exact: the goal fact's only adder,
  build-rail, requires a STATIC `(connected-by-land location6
  location3)` that init does not carry, and nothing adds
  connected-by-land — every other rail goal in the instance has land
  support, matching the grounder's named fact precisely.
- **Lever a1 landed** (hatch `FF_NO_NUMPRE`), plus the Eq rider —
  from a point value an Eq is one-sided (~6 lines), so probe rider c
  became code. **Lever a2 was NOT needed**: pathwaysmetric i1 moved
  on a1 alone (3,603,865 evals unsolved at the wall → 12 steps at
  4,710 evals).
- **Receipts, solo (contended box; eval counts deterministic):**
  sailing-numeric i1, the 0.20 5,000,048-eval cap-out, now SOLVES —
  174 steps, 29,203 evals, ~2.3 s. block-grouping i1 (0/20 board,
  field 19–20): 22 steps in 24 evals via the Eq charge; either hatch
  restores the 5M cap-out. The unit pin went RED→GREEN exactly as
  scoped (h(init) 1 → 21 on the sailing-band fixture); the mini
  fixture's library receipt: 3,019 → 22 evals, same 21-step optimum.
- **tpp-numeric i1–i3:** byte-identical under the hatch, and — the
  honest surprise — byte-identical with the charge ON too: tpp's
  selected ops already satisfy their numeric preconditions in the
  charged states, so the pass never fires there.
- **A scoping the spec did not ask for, taken deliberately:** the
  charge is OFF on temporal groundings (`charge_pre_num=false` on the
  stratified entry). Un-scoped, it re-routed the village workshop
  economy (27 → 47 steps, the pinned deep-make witness gone) — the
  temporal h is Phase 7/8 territory and keeps 0.20 behavior
  byte-identical until measured on its own boards.
- **FF_NUMNOV landed opt-in** (quantized per-fluent seen-envelopes,
  numeric-task-gated, zero cost off) with a mechanism pin; its
  promote/drop referee stays with the boards. The markettrader
  negative control is pinned on BOTH sides (finite h, no cycle
  gradient — the re-attribution is a fixture, not an assertion).
- The honesty rider grew honestly: `capped` had to thread through
  `PlanOutcome` and `resolve::Solved` to reach both text paths; the
  capped wording avoids the word "unsolvable" so no substring
  classifier can misread it. Suite 232/0 in-worktree; integrated
  242/0 on main. Boards: the cut's referee.

## Phase 4 — spend the whole wall, Mode::Optimal (+ the third ⚖️ board)

0.20 Phase 1's principle — an engine holding a time limit never
leaves budget unspent — was never extended to the optimal mode, and
the decode says that is where the cheapest proofs on the books are.
`optimal.rs` contains NO wall check at all (zero `FF_TIME_LIMIT`
reads); the ladder is denominated in NODES (h^max sprint on
`max_nodes/4` stored nodes, optimal.rs:561) while the boards budget
WALL SECONDS — and on this box the node cap is the fixed 8 GiB model
because `rlimit_budget` reads `/proc/self/limits` (search.rs:74),
which does not exist on Darwin. On medium tasks h^max cannot fill a
quarter of that cap in 60 s, so the sprint never returns, **LM-cut
gets zero wall on exactly the domains it dominates**, and every one
of the 500 timeouts is a runner SIGKILL mid-sprint with an empty
note. That is why 293 of 306 certificates are h^max's, and why the
0.20 cut record's "does not yet earn its keep" was the right verdict
for the wrong reason.

Scoping probes (0.20.0 binary, run under live CI contention, so the
positives are hard signals): with `FF_NO_HMAX_SPRINT=1`,
scanalyzer-08 i4/i7/i10/i13 PROVE in 0.2/1.4/5.5/19.4 s at 8–14
expansions; scanalyzer-11 i4 in 0.3 s; parc-printer-08 i7 in 0.6 s;
no-mystery-11 i4 in 27.9 s; and parking-11 i1 proves at ~2,924
expansions — **parking's first proof ever**, on a domain h^max
scored 0/40. The sprint cannot simply go: the differential's barman
class (h^max proves in 22 s what LM-cut cannot inside 100 s) is why
it exists. The fix is a sprint TIME-box.

Phase 2's backfill then priced the other half of the mistake, from
the side no hatch could see: the split is not wrong, it is
UNCONDITIONAL. Where LM-cut can fire, the trade is strongly positive
(2008/11: h^max ×235 → h^max ×237 + LM-cut ×13, +15); where it never
fires, the quarter-budget sprint just starves h^max and nothing
compensates (2014: h^max ×64 → ×56 + LM-cut ×0, −8). So the phase
carries TWO levers, separately hatched: the time-box, and a gate that
decides whether LM-cut deserves the remainder at all.

- **Fixture first:** a scanalyzer-shaped task — h^max needs far more
  stored nodes than the sprint quota, LM-cut proves in <100
  expansions — asserting the DEFAULT ladder certifies under a small
  `FF_TIME_LIMIT`. RED on today's node-split.
- **Lever 1 — the time-box:** thread a deadline through
  `optimal::solve`/`astar`, clock check every ~1k expansions; sprint
  slice = min(node cap/4, `FF_OPT_SPRINT_FRAC` of remaining wall,
  default ~0.4 — a 25% slice would kill the 22 s barman class);
  rung 2 gets the remainder. No armed `FF_TIME_LIMIT` ⇒ bit-identical
  to today, so dev boxes and every existing test are out of blast
  range by construction. The existing hatches stay the discriminators.
- **Lever 2 — the rung-2 gate (the backfill's demand):** LM-cut gets
  the remainder only where it is INFORMATIVE — one LM-cut evaluation
  at the root against h^max's root value (a one-node cost, ~30× one
  h^max eval). Strictly greater ⇒ landmark structure exists and
  LM-cut earns the wall (the scanalyzer/elevator class); equal ⇒
  h^max CONTINUES with the full node budget and the remaining wall
  instead (the city-car/genome class the sprint was starving — the
  −8). Hatch: `FF_OPT_NO_ROOTGATE` restores unconditional LM-cut.
- **Referee gates, in order:** `opt-differential.py` — all 306
  certified costs must re-certify — then both opt boards. If the
  differential bleeds h^max certificates, raise the fraction; if it
  still bleeds, record the negative and keep the node split. And per
  Phase 2's methodological finding, hatches cannot price a budget
  reallocation: the cut's referee for this phase is the v0.19
  BACKFILL COLUMN — 2014-opt must recover toward 64 while 2008/11
  holds its +15.
- **Rider, only on green:** memoize h beside g for re-opened states
  (LM-cut is admissible-not-consistent; the A* re-opens) — measure,
  drop if noise.
- **The entry — `ipc2026-opt`, the third ⚖️ board:** the corpus's
  three -opt pairs (onlycraft, rainbowttles, sailing-wind) are
  vendored, all genuinely numeric, and NONE has an active `:metric`
  (sailing-wind's is commented out; rainbowttles declares
  `:action-costs` with zero total-cost effects) — so certificates are
  LENGTH optima and the board says so. Mode::Optimal is already
  sound on numeric tasks (exact numeric expansion and goal test,
  admissible-by-relaxation h, fluent-bearing StateKey); the scoping
  probe certified **14/60 at a 15 s budget with zero code changes**.
  Sweep 3×20 at 60 s under `--mode optimal`, wire standings
  (SWEEPS / PROOF_TRACKS / AIR_REBASELINED), and track the raw as
  evidence beside the other two optimal JSONLs. Field context that
  makes this entry honest rather than brave: IPC 2026's LNP-optimal
  winner scored 83/260 with blind A* at 74. onlycraft stays
  near-blind without a numeric-admissible heuristic (pure numeric
  goal ⇒ h=0 Dijkstra, ~2–3/20) — that heuristic is a cycle of its
  own, named in the deferred list, and the entry does not wait on it.
- **Expectation:** +15–45 certificates out of the 500 (120 of them
  sit in the probe-positive domains; the estimate extrapolates from
  12 single-instance probes and the re-sweep is the number), plus a
  new board opening at ~14–20/60. Denominator note: the new board
  moves the twelve-board total from 4,016 to 4,076 instances.

### Recorded — the ladder learns the clock (differential + boards at the cut)

- **Both levers landed** exactly as narrowed: root gate BEFORE the
  split, everything conditional on an armed `FF_TIME_LIMIT`, the
  no-wall path byte-for-byte the 0.20 node-split. Hatches verified
  and pinned: `FF_NO_LMCUT` / `FF_NO_HMAX_SPRINT` keep pure-rung
  meanings on every path (the gate cannot resurrect the sprint);
  `FF_OPT_NO_ROOTGATE` restores the unconditional ladder;
  `FF_OPT_SPRINT_FRAC` (0,1], default 0.4.
- **The RED observation is the 500-timeout shape to the letter:** on
  the gatecheck fixture the old ladder ran 786,070 h^max expansions
  for 16.2 s straight through an armed 5 s wall. GREEN: gate reads
  h^max 4 vs LM-cut 24, sprint trips at 2 s, LM-cut certifies inside
  the wall. The battery (tests/opt_wall.rs, five child scenarios in
  the refill.rs convention) pins default, both pure rungs, the
  b-branch, and the no-rootgate hatch.
- **Receipts, solo (contended box):** scanalyzer-08 i4 — PROVEN cost
  24 by LM-cut at ~24–25 s against 0.20's 60 s SIGKILL (the sprint
  slice is the price; pure LM-cut reference 8 expansions / 0.09 s).
  barman-11 i1 — still PROVEN cost 90 by h^max inside the 0.4 slice.
  genome-edit-distances i4 — the gate's b-branch (1 vs 1, LM-cut
  uninformative): h^max keeps the whole wall and proves cost 4.
  city-car i1 gates c-branch (22 vs 24) and the sprint proves in 371
  expansions anyway — the 2014 recovery is explicitly NOT guaranteed
  per-domain by the gate; the v0.19 backfill column at the cut is
  the referee, as specced.
- **The h-memo rider is KEPT**: elevators-2011 i1 expansions
  IDENTICAL (47,453, cost 56), evaluated −4.6%, wall within noise.
- **The numeric-optimal soundness floor is pinned** independent of
  the corpus (benchmarks/bench/numopt-*: exact numeric goal test,
  fluent-bearing StateKey, numeric preconditions in exact expansion —
  three pins, each names its unsoundness axis).
- **Named honestly:** wall-denominated slices make PROVEN-note
  expansion counts load-dependent (certificates, costs, provers stay
  deterministic); scanalyzer-class proofs now pay the sprint slice
  (~25 s, not 0.2 s) — the differential referees whether the slice
  should be cheaper. `opt-differential.py` (all 306 re-certify) runs
  BEFORE the boards at the cut, per the referee order.

## Phase 5 — the ladder tax (rung budgets denominated in wall)

This was scoped as "the one classical slice this cycle takes";
Phase 2's backfill promoted it to the regression repair. The −26 is
mostly THIS phase's territory: novelty-light sits after EHC and
before LAMA, default-on under a declared budget, with an
UNCONDITIONAL 300k-pop cap that 0.20 priced at "~1 s" from one
sokoban probe — and on big tasks those pops cost tens of seconds
spent ahead of the rung that would have solved. The backfill's
per-domain receipt (2014-agile): gains +23 (visit-all 1→20,
maintenance +4) against losses −34 (hiking −10, child-snack −8 to
ZERO, openstacks −8, thoughtful/cave-diving/city-car/parking). The
win is real — `FF_NO_NOVLIGHT` costs visit-all 19 — it is bought at
~1.5× its value. EHC is the same mistake one rung earlier: its
op-count-scaled eval budget (search.rs:1134-1143) spends 30–55 s of
a 60 s wall on exactly the boards where every solved row says "EHC
found no improving state". Every rung's budget gets denominated in
the currency the board charges: WALL.

- **Fixture:** pin the openstacks shape — under an armed budget, the
  ladder reaches novelty-light (and LAMA) with real wall remaining;
  today EHC + the 300k-pop rung consume it.
- **Lever 1 — novelty-light's slice:** under an armed budget, the
  light rung's pop cap becomes min(300k, what fits in a small share
  of REMAINING wall — its wins need plan-length pops, two orders
  below the cap, so the slice keeps every receipted win). No declared
  budget ⇒ byte-identical. The `FF_NOVLIGHT` family stays; the slice
  gets its own knob.
- **Lever 2 — EHC's slice:** same currency: cap EHC at a share of
  remaining wall rather than op-scaled evals. Hatch:
  `FF_NO_EHC_WALLCAP`.
- **The honest risk, stated before the referee:** openstacks' three
  current solves ARE EHC-direct at 34–54 s — cut EHC too hard and
  the phase loses the very rows it chases. Casualties get named and
  solo-checked; the fixture holds the floor.
- **Referee — and per Phase 2, NOT a hatch:** a budget reallocation
  is invisible to hatch differentials, so this phase is refereed by
  the v0.19 backfill column at the cut: the −34 loss side (hiking,
  child-snack, openstacks, thoughtful, cave-diving) must shrink
  toward zero while visit-all 20/20 and maintenance hold. Band:
  recover +15–30 of the −26; a negative is recorded as gladly as a
  win.

## Phase 6 — the static-fluent fold (+ the Darwin byte budget)

The mem-cap column, re-attributed on the new instrument: 74 rows,
and **54 of them sit in domains whose per-node payload is dominated
by fluents that never change.** Fluents never got the 0.20 fact
compaction (facts: keep/renumber, ground.rs:1679; fluents: plain
resize, :1745), so price tables, drive costs, and duration tables are
interned into `fv0` and cloned into every node. Receipts, measured
against the engine's own byte model (search.rs:104): tpp i12
20.6 KB/node, 99% of it fv+fdef, 62% static; pathwaysmetric 80%
static; data-network i12 386 of 387 fluents static (only total-cost
moves); elevator-2011-temporal 129 of 129 — a PURE duration table;
woodworking 83%. The 0.20 deferral feared "State's type ripples
through temporal/session/wasm"; the decode says the fear was
over-broad — State's TYPE never changes under this lever, wasm never
touches State at all, and the surgery dissolves into grounding-layer
id remaps plus one two-source lookup in temporal duration eval.

- **Lever 0, the plumbing (smallest, and it pays on every future Air
  board):** on Darwin the engine's internal byte cap never arms
  (`rlimit_budget` → `/proc/self/limits`), so the runner's RSS
  watchdog kills externally with wall unspent — woodworking dies at
  2.9–11.4 s of a 60 s budget and the refill loop never gets to run.
  Plumb the runner's byte budget into the engine (env, from
  `--mem-gb`); the cap trips INTERNALLY, returns capped, and the
  refill loop spends the remaining wall. 15 mem-caps on ipc67-default
  alone are this shape; the seq-sat decode calls it the successor to
  the early-exit class.
- **Lever 1:** fold defined-static `Fluent` refs to `NExpr::Num`
  across every grounded expression holder (pre_num, effect values,
  conditionals, goal_num, metric). Hatch: `FF_NO_FLUENT_FOLD`.
- **Lever 2:** compact `fv0`/`fdef0`/names/rel_fluents to WRITTEN
  fluents behind a remap, with a task-side static table for
  name-resolved readers (temporal `eval_expr`, introspection). Hatch:
  `FF_NO_FLUENT_COMPACT`, mirroring the fact-compaction hatch. The
  node-cap model reads `fv0.len()` and raises itself for free.
- **The contract that must not break:** the session/MCP world-edit
  path REQUIRES `set_fluent` on op-untouched fluents to stay live —
  fold stays OFF the session grounding entry (already a separate
  function), and that is a FIXTURE, not a discipline note.
- **The constraint carried from 0.20 Phase 4:** byte-identical plans,
  dedup verdicts, and expansion order — statics never distinguish
  states and the fold substitutes the same f64 bit-for-bit. Undefined
  statics stay unfolded; a debug assert poisons pre-compaction ids.
- **Referee:** the 0.20 forced-cap RSS instrument re-run (bytes/node
  before/after on data-network i12, elevator-2011 i12, tpp i12, plus
  city-car/block-grouping for continuity with the 0.20 receipts),
  then the mem-cap columns at the cut: 74 today, target band ~25–35,
  every residual named. Honest projection: +5–15 solves (elevator-sat
  rows capped at 31–58 s of 60 mostly convert class, not coverage),
  plus unbudgeted throughput spillover — every heuristic call stops
  loading dead fv. Block-grouping is EXPLICITLY not in this pot
  anymore: 0.20's compression moved its constraint from memory to
  time (18 mem-caps then, 18 timeouts + 2 now). The CoW/hash-consing
  idea proper stays deferred; after fold+compact its only remaining
  constituency is all-dynamic domains.

### Recorded — the tables leave the node (mem-cap columns at the cut)

- **All three levers landed.** Lever 0 end to end: `FF_MEM_BUDGET_GB`
  read ahead of `/proc/self/limits`, the temporal solve entry now
  draws from the same `retained_bytes_budget()` as the classical cap
  (it consumed the raw 8 GiB constant — the reason temporal jobs died
  to the external watchdog), and ipc67.py exports the budget beside
  `FF_TIME_LIMIT`. Levers 1+2 behind `FF_NO_FLUENT_FOLD` /
  `FF_NO_FLUENT_COMPACT`, mirroring the 0.20 fact compaction in the
  same file.
- **One deliberate narrowing, argued and receipted:** foldable =
  defined-static AND NOT RELEVANT. Pre/goal/cond-read statics stay in
  `fv0` regardless (rel_fluents must not move or visited-key contents
  shift), so folding them buys zero bytes while flipping
  shape-sensitive dispatch (achiever bare/linear splits, thresholds,
  resource grouping). The byte wins survived the narrowing intact:
  **data-network i12 3,683 → 209 B/node (17.62×, the estimate to the
  decimal); tpp i12 24,418 → 4,672 B/node (5.23×, better than the
  2.6× estimate)** — measured on the engine's own byte model.
- **The identity bar held where it matters most:** elevator-2011
  temporal plans BYTE-IDENTICAL hatches on/off, durations resolved
  from an 82-entry static travel-time table; the fluent_fold suite
  compares plans and eval counts exactly across six fixture shapes,
  per hatch and both together.
- **The session contract has teeth, proven:** with the session entry
  experimentally flipped to fold, the set_fluent-into-durations
  fixture PANICS at the exact contract site; reverted, green. Fold
  never applies on session/verify/trace/introspect entries.
- The forced-cap RSS re-run and the mem-cap columns (74 → target
  band ~25–35, every residual named) referee at the cut.

## Phase 7 — the temporal emission repair (the 0.20 negative, closed)

The third decode said "the repair belongs in the temporal emission
layer"; this cycle's fourth decode found the exact inversion.
Reproduced live: at raw epoch 1.0 the search fires build_road's START
in block (a) — `op_applicable` certifies `(clear junction0-2)` true
at decision time — and block (b) then fires the deleting
vehicle_start ENDs. The emission layer never sees that certified
order: `reconstruct` DROPS all END events (temporal.rs:2849) and
re-derives ordering by an ends-before-starts tie-break
(temporal.rs:3237-3243), which INVERTS it — the deleter-ENDs are
placed ahead of the reader-START in the ε-chain. Neither standing
repair can reach a cross-kind inversion (0.18 reorders ends among
ends, 0.20 starts among starts), and no bubble can: the threatened
start must cross unrelated starts. The repair direction is
VAL-verified by hand — moving only the threatened start before the
deleting ends turns i17 green.

- **Fixture first:** `benchmarks/bench/eps-threat-domain.pddl` — a
  distilled reader-START vs deleter-END same-epoch pair — with a pin
  that is RED today; both standing eps pins stay green.
- **Lever 1:** replace the two bubble passes with ONE per-slot
  topological order (Kahn) over four must-precede relations — end→
  start when the end's adds provide the start's preconditions (keeps
  ε-chaining), start→end when the end's dels hit them (the witness
  class), end→end by the 0.18 invariant relation, start→start by the
  0.20 PROVIDES relation — plus an InvMap guard edge so a start moved
  inside a still-open interval cannot break its `over all`. No-edge
  groups tie-break ends-first/construction-order and emit
  byte-identically; a cycle leaves the group unchanged, so the floor
  is the status quo and the STN veto stands. ~60–100 lines, one
  function, one file.
- **Lever 2, escalation only:** if a numeric or conditional-effect
  threat survives lever 1, thread the father chain's decision order
  through `reconstruct` — stop dropping ENDs — and retire the
  guess-the-order family outright.
- **Referee:** solo map-analyzer ×20 (expect 12/12 VAL-green), suite
  green, both temporal boards at the cut with VAL-RED 3→0. Payout is
  +3 exactly, and the board/standings disagreement (66 raw vs 63
  scored) reconciles — but the real purchase is soundness: the
  emission layer becomes load-bearing BEFORE any future work raises
  same-epoch concurrency, and the 0.20 honest negative closes with a
  fixture instead of a sharper apology.

## Phase 8 — the probe basket (attribution first)

- **The h-surgery probe, pre-registered.** The bet carried since
  0.15 finally gets its half-day trial: `FF_H_ENDGATE` — a start→end
  pair table on the packed task (populated only by the temporal path
  from `Kind::Start { end_op }`), plus a post-pass in extraction that
  discounts a selected START whose paired END is selected in the same
  generation. Selection itself untouched (helpful sets unchanged —
  emptying start selection would replay the 0.11 FF_LAX_HELPFUL
  negative); classical path provably byte-identical (no pair table).
  Pass/fail reads FIXED IN ADVANCE, both already on the record:
  (a) the village pair contract solves at THINK_EVALS=200k (today 1M
  sails and 200k dies — examples/village.rs:24); (b) TMS-2011 i1's
  best_h ladder breaks the 110 floor 0.15 pinned to the decimal.
  Either read fails ⇒ the negative is recorded and the ledger line
  DIES. Both pass ⇒ the full phase: accounting edges (at-start-only
  selection not discounted; reps>1 capped at 1), then the A/B over
  both temporal boards with canaries named (match-cellar 40/40,
  crew-planning 50/50, openstacks 110/110, parking 38/40). Pot if it
  runs: TMS 40 + parc-printer 25; honest expectation +5–25 and
  possibly 0 — four prior probes on this wall were all negatives.
  Off-board payout either way: the village think budget drops ~5× if
  the fence falls, which prices the game project's tick loop.
- **The 2026 attribution sitting:** 167 of the new board's timeouts
  have NO mechanism on record (2048 20, settlers-snp 19, petri-net
  16, the onlycraft pair 30, line-exchange 16, gear-car 14,
  factory-robot 12, ztalloc 12, ...). A per-family decode
  (FF_WALL_DEBUG probes, plateau vs reachability vs scale) BEFORE any
  engine work — the pot is not one mechanism yet and pretending
  otherwise would violate the discipline this file exists to enforce.
  Findings feed 0.22. Corpus riders: 2048 ships instances 8–26 and 29
  only (the organizers flagged it "challenging to ground" — check
  whether smaller instances exist); verify the vendored corpus
  matches the organizers' final 2026-07-09 Domains release; note
  expedition appears VERBATIM on both numeric boards (5/20 each) —
  scoped once, banked twice.
- **The organic-synthesis join-ordering probe, gated.** The watch
  item's premise was HALF WRONG and the record needs both halves:
  agricola is NOT grounding-bound (grounds in 13.7 s of 60 into
  246,879 ops at 298 B/node; the wall is search churn on a
  quarter-million-op task; fixpoint grounding provably yields the
  same op set) — RETIRED from the watch as a measured negative.
  organic-synthesis IS grounding-bound, but not the way the ledger
  guessed: a fixpoint probe ran >3.5 min CPU-bound at flat ~2 MB —
  the blowup is TIME in declaration-order binding recursion, not
  memory, so the missing piece is join ORDERING (most-constrained-
  variable selection in `for_each_binding`) behind a conservative
  typed-product threshold hatch that sokoban-t never fires (dodging
  the recorded fixpoint A/B regression by construction). A day-scale
  probe; only "<30 s to ground i01/i11" buys a 0.22 phase. Rider:
  check whether the same hatch flips onlycraft i19/i20, whose two
  mem-caps are 46-second grounding transients.
- **The noise docket (solo, quiet box):** Phase 2's backfill already
  closed most of it — child-snack-2014, hiking, openstacks and
  thoughtful reattribute from "noise-suspect" to the novelty-light
  tax, which is Phase 5's territory, not noise. Still open:
  parking-2011 6/20 — the one cross-box DROP the tax only partly
  explains (was 11/20; backfill delta −1 on agile), four of six
  solves in the 52–54 s band; and flashfill i10 (solves <15 s idle,
  timeout on the board). Classified noise or real, on the record,
  before either shapes a phase.
- **The refill/classifier seam, closed:** all 10 remaining
  early-exits sit between the refill floor (10% of wall) and the
  classifier's timeout line (5%). Reconcile the two lines (classifier
  to 90%, or size the last round to the actual remainder) and RECORD
  the early-exit class as closed — it was 0.20 Phase 1's referee
  column, and it finished its job.
- **Re-attributions recorded, no code:** markettrader out of the
  winnable numeric pot (cyclic resource flow, field best 2/20);
  agricola out of the grounding watch (above); block-grouping out of
  the memory pot (0.20 Phase 4 did its job — the constraint is time
  now, and probe rider c in Phase 3 owns the domain's real hope).
- **The PlanForge audit rider:** tolerance canonicalization before
  dedup is the bug class our quantized StateKey already guards; one
  sweep over heuristic-side caches for un-quantized f64 keys, result
  noted, done.

## Phase 9 — cut 0.21.0 (recover the −26, then beat it)

The standing template: every board re-swept against the final binary
— all twelve plus the new `ipc2026-opt` — records complete per phase,
full pre-flight (all eleven gates, latest stable), finish in main,
the user publishes. What is new at THIS cut:

- **The movement question is now double-ended.** Phase 2's backfill
  made the trend line real (v0.19 1,946 → v0.20 1,920, −26), so the
  cut answers two questions, not one: did 0.21 beat 0.20, and did it
  RECOVER THE −26 against the v0.19 column — the ladder-tax and
  optimal-gate phases are refereed against the backfill, not against
  hatches (Phase 2's methodological rule). The v0.18 backfill running
  as this is scoped extends the same trend line backward.
- **The comparability rider:** compute the 2026 track's own published
  formulas (agile 1−log(T)/log(300), satisficing C*/C, optimal
  coverage) over our numeric boards and put Panino / ENHSP-2024 /
  Count Downward's published per-domain numbers beside ours in the
  audit record. The field printed the answer key; the honest table
  should read against it.
- The `--score-against` self-relative quality column runs its first
  real A/B (the 0.20.0 raws are the reference).

## Deferred, on the record (carried forward)

- **The classical guidance swing** — the biggest pot on the books
  (817), now deferred WITH the field's recipe named: partitioned
  novelty over subgoal-induced features with anytime cost-bounded
  restarts (the IPC-2026-winning shape). The 2014 plateau trio (119)
  and the 2018 wall (136) are its referee-in-waiting. A centerpiece
  for a classical cycle, not a side dish for this one.
- **Numeric-admissible heuristics** for the -opt entry — the field
  says numeric PDBs/CEGAR now beat numeric LM-cut on simple-numeric,
  and LNP-optimal is nearly open (83/260). Own cycle, own
  differential gate.
- **Incremental LM-cut** — defer until a post-Phase-4 board shows
  LM-cut running-and-near-missing as the dominant timeout mechanism;
  today's binding constraint is that it never runs.
- **Symmetry/orbit pruning** — child-snack 20 + the barman fifty's
  zero-traction core: probes prove NOTHING in 65 s under either rung;
  factorial object symmetry needs new machinery, not tuning. Named so
  it stops diluting referees.
- **Organic-synthesis join planning** — gated on the Phase 8 probe.
- **The temporal block** — storage/TMS/model-train (110 instances at
  zero) plus the budget question: the solve-time tail says a 60 s
  tier converts only ~15–30 of 331 timeouts, so mechanism work comes
  first and the tier re-baseline waits for a temporal-focused cycle
  (and must watch `epsilon_separate`'s 2000-happening cap if longer
  plans arrive). The h-surgery probe's diagnostics will classify
  floor-tile/driver-log/sokoban-t/satellite for free.
- **Per-node CoW for all-dynamic fluent domains** — the fold's
  leftovers (block-grouping's chunked-CoW ~2 instances; tpp drives
  after cost externalization). Small pot, real surgery; waits.
- IPC-5 complex preferences, cross-mind planning, continuous `#t`,
  dynamic derived predicates — unchanged standing lists.
