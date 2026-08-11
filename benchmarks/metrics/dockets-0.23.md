# The 0.23 Phase 1 dockets — engine-side probe record

**Provenance.** Probed 2026-08-10 in the wave-1 worktree (base 0.22.0 +
the roadmap; binaries named per table). Solo probes: `nice -n 15`,
board env (`FF_TIME_LIMIT=60`, `FF_MEM_BUDGET_GB=6`, `--json
--threads 1` — the exact `ipc67.py` invocation). **Box caveat:** three
sibling build agents shared this box all day; idle% is recorded per
run, WALL times are signals only where the arms ran interleaved
back-to-back (contention hits arms evenly) or the read is
eval-denominated. Verdict-shaped cells that need a truly quiet box are
marked so, never guessed. Adjudications gated on the v0.19/v0.21.0
backfill column stay **pending-the-column** (the sweep session owns
it).

## 1. The openstacks-2014 ramp tax, bisected — every named suspect ACQUITTED

Board deltas under bisection (air21 → air22: i1–i10 slowed
1.18–1.27×, i11/i12 lost from 55.46/55.79 s):

| arm (i3, interleaved ×2–3) | wall | evals |
|---|---|---|
| 0.22 default | 27.1–52.0 s (tracks idle%) | **41,241** |
| `FF_NO_ORBIT_CLASSICAL=1` | within noise of default | **41,241** |
| `FF_NO_RUNG_WALLCAP=1` | within noise of default | **41,241** |
| `FF_COST_SWEEP_EVALS=0` | 5.3–11.0 s | **11,205** |
| **v0.21.0 binary** (tag rebuild), same reps | **27.18 / 44.04 / 49.34 s** | **41,241** |
| v0.22 binary, same reps, paired | **27.11 / 44.09 / 49.59 s** | **41,241** |

- **The binaries are wall-identical to ~1% per paired rep and
  eval-identical** on i3 (and on i7: 45,038 evals both, walls within
  arm-order noise). The ramp tax does NOT reproduce solo on either
  binary — there is no engine-side throughput regression on these
  rows.
- **P6 L3 dedup acquitted by construction:** detection exits
  candidate-free on lifted work alone (`FF_ORBIT_DEBUG`: "no candidate
  groups (151 units, all size 1)"), so the classical canonical-dedup
  consumer never arms on openstacks-2014. The spec's hypothesized
  cheap fix — short-circuit `detect_classical` before the grounded
  parse on candidate-free tasks — **already exists** (orbits.rs
  groups-before-grounded-scan; the receipt above is it firing).
- **P2 clocks acquitted:** `FF_NO_RUNG_WALLCAP=1` moves nothing, and
  `FF_WALL_DEBUG` narrates exactly 2 lines on a solved row (no
  checkpoint churn). Rows are EHC-direct ("wall: solved by EHC").
- The cost-improvement sweep is **73% of row evals** (41,241 vs
  11,205 without) on BOTH binaries equally — a standing cost, not a
  0.22 tax.
- **Where the tax lives:** board conditions. air22's ipc2014-agile
  conditions.json is "clean" by median (75.2% idle) but carries
  p25 68.1 / **min 28% idle** with a 14.6%-mean-pcpu browser and a
  7.8% VM — the same hidden-window class the roadmap already convicted
  for ipc2026-numeric — and the air21 agile board has **no
  conditions.json at all** (one of 0.21's seven receipt-less boards).
  Both sweeps ran `--jobs 2`, so per-row co-tenancy pairing differs
  between sweeps. Final adjudication: **pending-the-column** (the
  v0.21.0 backfill re-runs the comparison with receipts on both
  sides). The "12 EHC-direct rows" canary stays dented 12→10 on the
  boards until then.

## 2. The damping docket — attribution done, conditional NOT landed, bill re-read

Split hatches landed (`FF_NUMPRE_NOSKIP=1` / `FF_NUMPRE_NOSUM=1`;
both = NODAMP exactly; separability unit-pinned on the watering mini).

| row | damped | NOSKIP (sum only) | NOSUM (first-wins) | NODAMP |
|---|---|---|---|---|
| fo-sailing i8 | **0.04 s / 287** | **0.02 s / 287** | dead: 3.52M evals | dead: 4.50M evals |
| ext-plant i10 (68–79% idle) | dead 1.02M | dead 2.30M | dead 1.61M | **dead 1.62M** |
| ext-plant i16 (68–79% idle) | dead 1.11M | dead 1.45M | dead 0.91M | **dead 1.42M** |
| sugar i18 (66–78% idle) | dead 1.60M | dead 0.94M | dead 0.42M | **dead 0.43M** |

- **fo-sailing's +7 is the SUM half alone**: NOSKIP is byte-identical
  (287 evals) — the mover-skip half is inert there; first-wins grinds
  3.5M evals into the wall. Fixture pair banked (tests/numpre.rs:
  GREEN guard ≤10k evals + RED first-wins twin; fo-sailing i8
  vendored verbatim into benchmarks/bench/).
- **The 3 bill rows recover under NO arm — including full NODAMP — at
  66–79% idle solo.** All three were near-wall 0.21 solves (i10
  56.25 s, i16 55.43 s, sugar i18 47.58 s of a 60 s board), and
  ext-plant i10/i16 are exactly the two rows the probed-and-rejected
  MAX damping traded while SUM's solo receipts kept them. Every
  conditioning is bounded by NODAMP's restoration, so the
  "NODAMP-recoverable" status lives inside the last seconds of
  quiet-box wall margin, not in the damping mechanism.
- **Verdict: the gap-magnitude conditional is NOT landed.** It would
  be tuned on boundary churn, and fo-sailing prices any SUM give-back
  at 287-evals-vs-3.5M. The bill stays OPEN as wall-margin churn,
  adjudicated against the backfill column; the attribution hatches
  are in-tree if the column re-opens it as mechanism.
- **Held, re-verified:** byte-identity triple across ALL FOUR arms at
  the record's digits (sailing i1 174/29,203; pathwaysmetric i1
  12/4,710; tpp i1 9/15,161 — no armed wall, `FF_NO_ESCALATE=1`);
  ext-plant i7 solved at the record-exact **859,772 evals**; delivery
  i18 (25.95 s/39,245), i19 (12.92 s/39,078), fo-sailing i9
  (2.65 s/82,895) all solved on the docket binary.

## 3. Casualty pre-probes (solo halves banked; column adjudication later)

| row (0.21 → 0.22 board) | 0.22 default | `FF_NOV_OLD=1` | mechanism read |
|---|---|---|---|
| org-synth-split i15 (20.22 s → dead) | dead at wall, 134,769 evals | **SOLVES 53.3 s / 40,277 evals** (39–58% idle) | **CONFIRMED driver casualty** — the old h-guided rung solves it |
| nurikabe i12 (47.32 s → dead) | dead; 363 evals total | dead; 1 eval (slot never entered) | **NOT the driver**: ~20 s grounding, then LAMA's 5A progress-conditional slice extends 17.3→33.5 s ("progress 0.00s ago" — h churn reads as progress) until remaining hits 0.0 and the novelty slot is SKIPPED; best-first gets 1 eval. h build: 37.0 s worker time inside LAMA. Quiet-box re-run owed for the solve/fail cells (probed at 6–65% idle) |
| hiking-agile i11 (33.73 s → dead) | dead; 3,841 evals | dead; 3,585 evals | **NOT the driver swap**: EHC 14.8 s + novelty-light 7.8 s + LAMA 19.2 s (2 extensions) spend to 0.278 remaining, the a3 re-read SKIPS the novelty slot (< 0.4), best-first dies. `FF_NO_RUNG_WALLCAP` legs overran to the external kill under 17–22% idle — uninformative, re-run owed |
| floor-tile-2011 i11 (8.39 s → dead) | dead; driver runs and CAPS (400,001 pops / 1.17M nodes), best-first dies at 1.26M | **SOLVES 35.7 s contended / 624,878 evals** (3–9% idle!) | **CONFIRMED hard kill, reproduces under any load.** Driver got its full slot+budget and lost on pure guidance shape — no cheap trigger exists to yield BEFORE the 400k pops are spent |

**floor-tile guard assessment: carry the casualty.** The driver
consumed its entire budget in-slot (no yield signal fires before the
spend); any shape-guard nameable today (R-size, h-informativeness,
goal count) re-routes far more than floor-tile and needs the
board-scale referee this session cannot run. `FF_NOV_LAZYH` (h on
novel-1 pops only) stays the pre-registered fallback design, gated on
the backfill column showing the casualty class grows.

**Protocol notes for the next sitting.** (1) Eval-capped probes
without an armed wall run the goal-decomposer unbounded — a
`--max-evaluated 1` openstacks probe ran >120 s before the kill;
board-env probes are the only honest solo form for ladder rows.
(2) The 5A LAMA extension's "progress 0.00s ago" loop (nurikabe
receipt above) is a named 0.23 design smell: progress-conditional
extension with no convergence test can eat the whole wall on a
descending-but-never-arriving h.
