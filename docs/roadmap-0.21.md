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
  The cost is not cosmetic. `standings.py` scores
  `solved = r["solved"] and val is not False`, so a misattributed row is
  dropped from COVERAGE: ipc2018-sat is really **60/240**, not 53/240,
  and ipc2026-numeric **129/320**, not 121/320. That is the third time
  this cycle the scoreboard has been caught fibbing and the third time
  not in our favour (0.20 Phase 1 found the other two). Because the
  refusal is domain-level, every affected row reclassifies soundly from
  the raw JSONL — no re-sweep is owed. Fix `val_check` to test a LIST of
  unavailability signatures, and apply the reclassification at promotion.
- **A latent runner misattribution, named before it bites.**
  `val_check` ends `except Exception: return False`, which swallows the
  120 s `TimeoutExpired` — so a VAL that runs out of time books as a
  REJECTED PLAN. That is the 0.20 Phase 1 shape exactly (graceful
  wall-exits booked as engine-rejects), on the one column standings.py
  calls a first-class signal. Confirmed latent on these boards, so it
  corrupts nothing here and no re-sweep is owed; fixed AFTER the sweep
  so the instrument stays identical across all twelve.

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
