# ferroplan 0.27 roadmap — the sweep that does not need an empty box

Scoped 2026-09-02, while the 0.26 cut sweep was still running. **This file
carries one phase — crucible R2 — and holds the engine cycle's place**: the
engine phases are scoped after the 0.26 cut record is written, not before,
so that they answer that record and not this one. Design:
`crucible-spec.md` §R2. Operator decisions taken 2026-09-02 and recorded
there: solves always bank; timeouts bank on the process's own cpu/wall;
`crucible sweep` hosts the TUI; 32 board rows of instance cells; 4-wide for
the known-fast, solo for everything else; **no manual retry**.

The case, in one line from the record: the 0.26 cut sweep is on its fourth
day because a referee that measures the *box* re-owed ~1,800 timeouts of
which nine in ten had their core the whole time. `crucible-spec.md` §R2.0
has the table.

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

### 0.a — the instrument first

Fix `cpu_ms` (wait4 rusage at reap; Mach timebase on the live poll; the
`cpu_instrument` column). **Nothing else in this cycle is measurable until
this lands**, so it is a Phase 0 item and not a Phase 1 one. Gate: a
fixture run of a known 2 s instance reports ρ within 0.95–1.02; the
`kill9_resume` suite stays green; existing rows are labelled, not rescaled.

### 0.b — ρ_min, re-derived

With the fixed instrument, re-run 60 clean-window instances (20 each from
1–10 s, 10–50 s, ≥ 50 s buckets) solo on a quiet box and take the
distribution of ρ. **Pre-registered:** ρ_min is the p5 of the ≥ 10 s
buckets rounded down to 0.05, floored at 0.85. If it comes out below 0.85
the referee is not safe on this box and the cycle says so.

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

Second question, same sitting: 20 prior-timeout instances, 2-wide vs solo.
The loss we are looking for is a solo solve that the packed slot missed —
by construction the packed miss is re-queued solo, so this measures wasted
time, not lost rows. **Pre-registered:** if ρ ≥ ρ_min on ≥ 95 % of the
packed timeouts and no solo run solves what its packed twin did not, prior
timeouts are admitted at width 2 in Phase 2. Otherwise they stay SOLO as
the operator chose and the record says why.

Phase 0 closes with both tables in this file.

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
- The engine phases of this cycle — scoped after the 0.26 cut.
