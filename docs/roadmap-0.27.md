# ferroplan 0.27 roadmap — the sweep that does not need an empty box

Scoped 2026-09-02, while the 0.26 cut sweep was still running. **This file
carries one phase — crucible R2 — and holds the engine cycle's place**: the
engine phases are scoped after the 0.26 cut record is written, not before,
so that they answer that record and not this one. Design:
`crucible-spec.md` §R2. Operator decisions taken 2026-09-02 and recorded
there: solves always bank; timeouts bank on the process's own cpu/wall;
`crucible sweep` hosts the TUI; 32 board rows of instance cells; 4-wide for
the known-fast, solo for everything else; **no manual retry**.

The case, in one line from the record: the 0.26 cut sweep ran five days
and six passes and was stopped with 184 rows still owed, because a
referee that measures the *box* re-owed ~1,800 timeouts of which nine in
ten had their core the whole time. `crucible-spec.md` §R2.0 has the
table.

**Decided 2026-09-04, after the 0.26 cut:**

- **Process: the automated gates stay, the prose rituals go.** fmt /
  clippy / doc / both test passes / `publish.sh --dry-run`, byte-parity
  against the Python oracle, RED fixtures first, `FF_NO_*` restores —
  all kept, all scripted. Decode sittings become one-page reads with a
  number at the bottom; no adjudication essays; the cut record is a
  table and a paragraph. A negative is still recorded, in one line.
- **Engine lanes, two, chosen from the weak-spot read:**
  1. **The 2023 cliffs** — folding, labyrinth, ricochet, rubiks; 2023
     sat/agile/opt at 24–29 % where h^novelty alone reads 84/140 in the
     published ablation. Lever: novelty-with-forgetting + multi-heuristic
     queue alternation (field-gaps §3.2). The standing fence applies —
     a decode read first (wall-slice instrumentation on the cliff boards
     naming what forgetting/alternation would fix in THIS engine) — and
     the build takes only the number the read produces. ~120 rows on
     three boards is the pot; the band is priced by the read, not here.
  2. **The per-eval-cost speed lane** — profile-driven, no decode needed:
     h-build per evaluation, successor generation, the open list. The
     first receipt to chase is on the record already: numeric best-first
     spends **h 19 s / expand 12 s / insert 15 s of 47 s** (markettrader
     i1, `numeric-twins-0.23.md`) — a third of the wall in the open
     list. Then the temporal ramps where throughput is the named wall
     (driver-log at 1k evals/s on 61k ops, rtam 4.5k/s, satellite's
     i5-class "converts with a faster eval loop"). Refereed with
     `benchmarks/perf.py` against the committed baseline; the honest
     expectation is +10–30 rows on the ramps, and it compounds every
     later lever. The calibration stands: 2× engine speed at the 60 s
     wall ≈ +4/140 on the 2023 cliffs — speed is not what THOSE need.
  **Recorded 2026-09-05 — the lane's first sitting, on the record's own
  receipts.** Baselines (the 0.26 engine, `FF_RES_DEBUG`, 60 s solo):
  labyrinth-agile i1 (78k ops, 786 facts) spends 18 of 21 s of best-first
  in h — the relaxed-graph BUILD is 41 s cumulative against 1.4 s of
  extraction; parking-2014 i5 (63k ops) build 50 s; markettrader i1
  (71 ops) splits h 7.6 / expand 4.2 / insert 7.1 of 19.5 s. Two builds,
  measured back to back against the pre-change engine on a quiet box:
  - **The anchored successor generator** (`PackedTask::applicable_ops`:
    each op anchored at its rarest positive precondition, candidates
    gathered from the state's true facts, sorted into the scan's order so
    search is byte-identical) — expansion per evaluation labyrinth 234 →
    34 µs (7×), parking 258 → 21 µs (12×), markettrader 10 → 5.6 µs.
    **Ships**, wired into the classical, LAMA and novelty rungs.
  - **The counter-based relaxed-graph build** (reached facts decrement
    the ops that need them; no per-layer scan) — build per evaluation
    labyrinth 1.78 → 2.02 ms, parking 0.92 → 0.96. The 2026-07-19 note in
    `build_rpg` said the scan is not the term because nearly every op
    fires; it is true on 78k ops too. **Recorded negative, removed.**
  - `apply()` gains an allocation-free path for ops with no conditional
    and no numeric effects (four temporaries per successor otherwise).
  What is left on these boards is the relaxation floor itself:
  ~1.7–2 ms per evaluation at 60–80k ops, proportional to the effects the
  fired ops carry. Moving it means firing fewer ops (relevance) or
  evaluating fewer states, not a faster scan.

- **Riders, already priced, unbuilt:** the 0.26 Phase 0 proof-gap bands
  (onlycraft's numeric-bound ceiling +1–2, barman/parking +4–6/+2–3,
  CEGAR seeding +4–9 at 300 s).
- **Not this cycle, on the record:** preferences vs SGPlan5 (qual 46/100
  vs 100/100, complex 26/108 vs 105/108 — the widest raw gap on the
  table and the least-understood machinery; a read is owed before any
  cycle takes it), transport-2014 and floor-tile (fenced; floor-tile's
  dead-end pricing probe still unclaimed), metric-time's AIBR build.

Order: crucible R2 first (it makes every probe below hours instead of
days), then the two engine lanes in parallel, R2's sweep as their
referee.

---

## Phase 0 — the sitting, and the two calibrations (light code, no scheduler)

### Recorded — what the code does today (2026-09-02, from the running sweep's database)

Found while pricing the design, none of it in the 0.26 record, all of it
verified against `~/.crucible/db/crucible.db` and the source:

- `Platform::cpu_ms` divides Mach absolute-time units by 10⁶ as if they
  were nanoseconds. `mach_timebase_info` here is 125/3, so every `cpu_ms`
  in the database is **41.67× low**. Corrected, the clean record's cpu/wall
  reads a median of 1.00 on 10–50 s runs; uncorrected it read 0.023.
- The last-poll undercount on short runs (runs under 1 s record 0–25 % of
  their CPU); `wait4` rusage was never taken.
- `attempt()` drops the control channel's sender on creation
  (`let (_tx, rx) = ...`): SUSPENDED/POLITE never reach a child. Timberborn
  at 181 % of a core is on 1,125 samples of this sweep, against un-stopped
  planners.
- `sched::tier::order` has no caller. Rows are stamped `jobs = 2` and
  measured one at a time.
- `cpu_speed_limit` is NULL on all 17,280 samples; `idle_pct` and
  `loadavg1` are NULL on the recent ones. The thermal instrument does not
  exist on this box, and the idle one has stopped reading — find out why
  before Phase 1 relies on any sample column.
- Swap held ~4.9 GB for the sweep's duration.
- **SIGTERM to crucible orphans its running planner** (found at the 0.26
  stop, 2026-09-04): the parent exits in ~1 s and leaves `ff` running
  under pid 1 until its own wall. The reaper would catch it on the NEXT
  start; the parent's own exit path must kill the process group first.

### 0.a — the instrument first

Fix `cpu_ms` (wait4 rusage at reap; Mach timebase on the live poll; the
`cpu_instrument` column). **Nothing else in this cycle is measurable until
this lands**, so it is a Phase 0 item and not a Phase 1 one. Gate: a
fixture run of a known 2 s instance reports ρ within 0.95–1.02; the
`kill9_resume` suite stays green; existing rows are labelled, not rescaled.

**Recorded 2026-09-04 — 0.a LANDED** (`crucible-r2`): `exec::run` reaps
with `wait4(2)` and takes `ru_utime + ru_stime` (µs, exact) as `cpu_ms`;
`RunOutcome::cpu_instrument = "wait4"`, threaded to `run.cpu_instrument`
(schema v2, NULL on every pre-R2 row, never rescaled). `Platform::cpu_ms`
(the live poll, now used by nothing but kept for the dashboard) converts
Mach units through `mach_timebase_info`. Fixtures: a 2 s spinning
`fakeff` reads 1.9–2.3 s of CPU at ρ ≥ 0.80, a sleeping one under 0.20,
a 120 ms spin is not undercounted; v1→v2 migration adds the column. One
thing seen on the way, not fixed: the supervisor reaps on its 250 ms
tick, so `effective` can read up to a tick long — irrelevant at 60 s,
visible on a 600 ms fixture, and worth a waiter thread when the live
view lands.

### 0.b — ρ_min, re-derived

With the fixed instrument, re-run 60 clean-window instances (20 each from
1–10 s, 10–50 s, ≥ 50 s buckets) solo on a quiet box and take the
distribution of ρ. **Pre-registered:** ρ_min is the p5 of the ≥ 10 s
buckets rounded down to 0.05, floored at 0.85. If it comes out below 0.85
the referee is not safe on this box and the cycle says so.

**Recorded 2026-09-04 — 0.b MEASURED.** 60 clean-window instances from the
0.26 database, solo, on the corrected instrument (`wait4` rusage via
`os.wait4` in `benchmarks/metrics/probes-0.27/calib/calibrate.py`), box in
ordinary use (load 2.3–2.8):

| bucket | n | median ρ | p10 | p5 | min |
|---|---:|---:|---:|---:|---:|
| 1–10 s | 20 | 0.988 | 0.967 | 0.964 | 0.938 |
| 10–50 s | 20 | 0.995 | 0.981 | 0.975 | 0.967 |
| ≥ 50 s | 20 | 0.998 | 0.987 | 0.982 | 0.973 |

p5 of the ≥ 10 s buckets = 0.975 → by the pre-registered rule
**ρ_min = 0.95**. The referee is safe on this box; `[referee]
cpu_ratio_min` defaults to 0.95 and `sched::referee::Rule::default()`
carries it.

### 0.c — the packing calibration

40 PACK-class instances with prior solo times of 5–30 s, drawn from six
boards, run **solo ×3** and **4-wide ×3** on a quiet box, canary running.
Report median and p95 inflation of wall-clock, with ρ and clock factor
beside each. **Pre-registered thresholds:**

| median inflation | p95 | ships as |
|---|---|---|
| ≤ 5 % | ≤ 15 % | `pack_width = 4` |
| ≤ 15 % | ≤ 30 % | `pack_width = 2` |
| worse | — | packing is a **recorded negative**; the referee ships alone |

**Recorded 2026-09-04 — 0.c MEASURED, and packing is a RECORDED NEGATIVE
at every width.** 40 PACK-class instances from fourteen boards, solo ×3,
4-wide ×3, then 2-wide ×2 (the width every pre-crucible sweep ran at as
`jobs = 2`), box in ordinary use (load 2.7 solo / 5.7 packed):

| width | median inflation | p95 | max | packed ρ | verdict by the table |
|---|---:|---:|---:|---:|---|
| 4-wide | **+72.8 %** | +106 % | +335 % (tpp-strips i26) | 0.991 | negative |
| 2-wide | **+22.7 %** | +32 % | +83 % (hiking-opt i12) | — | negative (median > 15 %) |

Every packed planner had its core and ran slower anyway. Two 4-wide runs
failed where solo solved (no-mystery-opt i5, factory-robot i7). So
`pack_width = 1`, the referee ships alone, and the scheduler phase loses
its packing half. Two consequences, both recorded in `crucible-spec.md`
R2.2:

- **ρ is blind to a slow box.** The canary is the referee's second input
  from Phase 1, not a later nicety.
- **Every board through 0.25 was measured 2-wide, and 0.26's were
  measured 1-wide** (crucible ran one instance at a time while stamping
  `jobs = 2`). At +23 % median inflation on solves, part of 0.26's +283 is
  the instrument's width and not the engine. F1's own attribution (+8/+4
  refereed on crucible, both legs 1-wide; +12/13 under the hatch) is
  unaffected; the rest of the movement is not decomposed. The honest
  measurement is `crucible backfill --tag v0.25.0` on a few boards —
  1-wide, like for like — and it is owed before any 0.27 movement claim.

Second question, same sitting: 20 prior-timeout instances, 2-wide vs solo.
The loss we are looking for is a solo solve that the packed slot missed —
by construction the packed miss is re-queued solo, so this measures wasted
time, not lost rows. **Pre-registered:** if ρ ≥ ρ_min on ≥ 95 % of the
packed timeouts and no solo run solves what its packed twin did not, prior
timeouts are admitted at width 2 in Phase 2. Otherwise they stay SOLO as
the operator chose and the record says why.

**Recorded 2026-09-04 — the timeouts probe.** 20 prior timeouts, solo then
2-wide: packed ρ ≥ ρ_min on 20/20 (median 0.998), no solo-solves-what-
packed-missed (0 solved either way). By the pre-registered rule they
would be admitted at width 2 — but the rule was written before the
packing result, and it is blind in exactly the way that matters: a packed
timeout BANKS on ρ, and 2-wide costs +23 %. A near-wall instance that
solves solo at 50 s times out beside a neighbour and banks as a timeout.
**Overruled by the packing result: prior timeouts stay SOLO.** The
pre-registration is kept here as written so the overruling is visible.

Phase 0 closes with these tables. Receipts: `benchmarks/metrics/probes-0.27/calib/`
(`calibrate.py`, `analyze.py`, `results.jsonl`, the manifest of instances).

**Recorded 2026-09-04 — Phase 1 BUILT** (`crucible-r2`): `sched::referee`
(the R2.1 table, ρ_min 0.95, swap growth, the canary factor; branch
tests); schema v3 puts `banked`/`verdict` on the row with the R2 rule
backfilled over the 0.26 database (solves bank, clean rows keep R1's
verdict, dirty unsolved rows owed as CPU-unknown); the watcher owns the
throttle and delivers Stop/Cont/Demote/Promote to the running child (the
dropped sender, wired); SIGINT/SIGTERM cancel and reap the child (the
orphan, fixed; tested in its own process); the canary runs solo ×5 at
start for its baseline and every 20 min after, its factor riding on every
sample (schema v4); admission is "not SUSPENDED" by default, FULL under
`--quiet-only`. Gate owed: the 184 rows the 0.26 stop left, under R2.

## Phase 1 — the referee

`crucible-core`: ρ verdict (§R2.1), the swap-growth flag, the canary and
its per-box baseline (§R2.3), the throttle wired to the child (the dropped
`_tx`), `SIGSTOP`/`SIGCONT` with `suspended_ms` proven by launching
Timberborn mid-run — the spec's own acceptance test, finally run.

Gates:

- Unit fixtures per branch of the verdict table, including the four flags
  that turn a timeout into a re-queue.
- **The referee, replayed:** a dry-run command reads the 0.26 cut sweep's
  database under the new rule and reports what it *would* have banked in
  pass 1 — expected, from the sitting: ≥ 85 % of the rows R1 re-owed.
  The number is recorded whatever it is.
- Byte-identical exports; `standings --check` green; `kill9_resume` green.

## Phase 2 — the scheduler

One queue across boards; PACK/SOLO/EXCLUSIVE from this box's history;
width by throttle and by memory; `neighbours` and `pack_class` on the row;
`jobs` out of the resume identity; `tier::order` finally called; the ETA
that steers the tail into quiet hours.

Gates:

- Differential: three boards (`ipc5-prop`, `ipc2018-sat`, `ipc7-mco-t2`)
  swept under the new scheduler against their 0.26 promoted rows. Coverage
  ≥, every difference named per instance with ρ and neighbours beside it.
- A `--threads 8` board still runs EXCLUSIVE with the window gate — a
  fixture asserts no neighbour is ever admitted beside it.
- Memory bound: a fixture with four 5 GB priors on a 16 GB box admits two.

## Phase 3 — the TUI

`crucible sweep` hosts it; `--headless`; the five views of §R2.4; the
stderr ring buffer on the existing pipe reader; no re-run key.

Gates: `--dump` frames for 80×24 and 220×60 checked in and reviewed; the
render cost measured under a live 4-wide sweep at < 1 % of one core; every
key in the table has a test the way R1's do.

**Recorded 2026-09-04 — Phase 3 BUILT** (`crucible-r2`): `crucible sweep`
hosts the dashboard when stdout is a terminal (`--headless` for the log;
the sweep runs on a scoped thread, the UI on the main one, `q` cancels the
running child the way ^C does and everything banked stays banked). The
five views of §R2.4: the grid (one row per board, one cell per instance,
attention states win a column and settled columns show their majority),
the board table (this run beside the predecessor from the promoted raw,
Δ, ρ, attempt, verdict; sortable; a solve-time histogram against the wall
with the near-wall band counted), the instance view (every attempt from
the database, ρ per attempt, the canary/swap/competitors across the
latest window, the box timeline), the whole-sweep timeline (foreign load,
canary, swap, throttle band, runs as marks, critical-pressure count) and
the running slot (elapsed against budget, live ρ and RSS from the
kernel). The sweep's narration goes through `say!` into a ring the log
pane renders and stdout gets back on exit. `crucible tui --dump --view
grid|board|instance|timeline` renders any view off-screen from synthetic
data; the four frames were reviewed at 140×34. No re-run key, by decision.
**Gate still owed:** a live sitting in a real terminal — the pseudo-tty
smoke here had no terminal size and could not deliver keys, so the
hosting path (scope, quit, flush) is verified by its parts, not end to
end. Not built: the stderr tail in the slot (the pipe reader keeps the
whole stream; a ring-buffer tap is a small change) and `live_child.stopped`
on Stop/Cont.

**Recorded 2026-09-04, from the first R2 night — the canary measures the
box, so the box must not hold our own planner while it runs.** Beside one
neighbour on an idle box (foreign 0 %, pressure normal) it read
1.14–1.15× — the calibration's single-neighbour +23 % — and beside an mco
board's eight threads 1.42×, owing 18 `ipc2014-agile` timeouts as thermal
that were nothing of the sort. The watcher now pauses the running child
(`Ctl::Stop`, 400 ms, the reading, `Ctl::Cont`) for the canary's two
seconds; suspended time is not charged to the run.

**Decided 2026-09-05 — the packed scheduler, for coverage.** The 0.c
calibration stands as a TIMING result: beside neighbours a solve is
slower, and its time is not a measurement. The operator's point is that
for an instance the predecessor solved, timing is not the question --
whether it still solves is -- so those run packed as wide as the cores
and the memory allow, and the referee's rule for a packed row is: a solve
banks (timing dirty, `neighbours` on the row, schema v7), a miss is
nobody's verdict (`packed`) and is re-run narrower, then solo, in the
same pass. The cascade per board: prior solves under 50 % of budget at
`pack_width` (default every logical core, memory-bounded by the batch's
largest prior peak RSS with headroom), prior solves under 85 % at
`pack_narrow_width` (2), everything else -- prior timeouts, this sweep's
own misses, `threads > 1` -- solo. Packing can waste time; it cannot lose
a row. `--quiet-only` restores the R1 shape.

## Phase 4 — the 0.27 cut sweep runs on it

The scoreboard for a harness cycle is the sweep itself. **Pre-registered:**
the cut-27 sweep of the 32-board set completes in **one pass plus its solo
tail, within 36 hours of wall-clock, with the box in ordinary use** (Docker
up, a browser open, an evening of Timberborn). The pass count and the
hours go in the cut record beside the coverage table. If it takes three
passes again, that is the headline and the referee is what gets decoded.

## Anti-pots — priced at zero, standing

- **A manual re-run key.** Decided against 2026-09-02: if the auto is
  right there is nothing to press; if it is wrong the fix is the referee.
- **ρ_min below the Phase 0 figure to make a sweep finish.** The number is
  derived once, from the instrument, and then it is the instrument.
- **Any packing of `threads > 1`.** The mco boards are the competition's
  wall-clock rule; they run alone, forever.
- **Timing claims from `packed` rows.** Coverage only. `diff` excludes
  them and says how many.
- **Cross-box history.** A prior from another box is not a prior.
- **Rescaling stored `cpu_ms`.** Label and move on.

## Deferred, on the record

- The daemon/`attach` split (spec §15.7) — the TUI-in-process model makes
  it a convenience, not a resilience feature.
- The Linux cross-check (0.26 F6 part 3) — still blocked on a cross C
  toolchain for `libsqlite3-sys`.
- `crucible backfill` beyond what F6 part 2 landed.
- The engine phases' detailed gates — written when each lane's decode read lands.
