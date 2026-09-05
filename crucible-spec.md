# Crucible — Ferroplan Benchmark Sweep Harness

**Status:** design spec, ready for implementation
**Language:** Rust (no AI/LLM anywhere in the runtime path)
**Form factor:** single native executable, TUI-first, runs resident in a Zellij pane

> Name is a placeholder — `crucible` (where metal gets tested). Alternatives if you'd rather: `bellows`, `forge`, `anvil`, `ferrosweep`. Pick before scaffolding, it shows up in the ASCII banner.

---

## 1. Purpose

Replace the current pile of shell scripts (plus a model babysitting them) with one resident program that owns the entire IPC benchmark sweep for [Ferroplan](https://crates.io/crates/ferroplan) end to end.

It should:

- Notice a new tagged version, build it, and start sweeping it — unattended.
- Run continuously, forever, without ever needing to be told what to do next.
- Be a good citizen on a machine that is also a personal workstation and a gaming machine.
- **Never lose completed work to contention.** Contention may cost a timing number. It must never cost hours of computation.
- Maintain durable, queryable history so any two tags can be compared after the fact.
- Look good enough to be worth leaving on screen.

### Non-goals

- No LLM calls, no network intelligence, no agent loop. This is a scheduler and a supervisor.
- Not a distributed system. One machine.
- Not a replacement for the planner's own test suite.

---

## 2. Target machine assumptions

Detect all of this at startup rather than hard-coding it; the numbers below are the expected shape.

- macOS on Apple Silicon, asymmetric cores: ~4 performance cores + ~6 efficiency cores.
  Detect via `sysctl hw.perflevel0.logicalcpu` (P) and `hw.perflevel1.logicalcpu` (E).
- 16 GB unified memory. Memory is not usually the binding constraint; CPU is.
- **macOS has no thread affinity API.** You cannot pin a process to P-cores. You influence placement via Darwin QoS class:
  - `QOS_CLASS_USER_INITIATED` / default → scheduler will use P-cores.
  - `QOS_CLASS_BACKGROUND` → **confined to E-cores**, which is exactly the lever needed for polite mode.
  - Set it at spawn with `posix_spawnattr_set_qos_class_np`, or trivially by exec'ing under `taskpolicy -c background`. Prefer the former; keep the latter as a fallback.
- Write the platform layer behind a `trait Platform` so Linux (cgroups + `sched_setaffinity`) can be added later without touching the scheduler.

---

## 3. Domain model

The unit of work is a **Run**:

```
Run = (tag, domain, problem, config)
```

- **tag** — a git tag of the Ferroplan repo (e.g. `v0.14.0`). Identifies an exact planner binary.
- **domain / problem** — a PDDL domain and one of its problem instances, grouped under a **track** (IPC year: IPC5, IPC6, IPC7…).
- **config** — a named set of planner flags. Default is a single config (`default`); the dimension exists so search-config comparisons don't require a schema migration later.

A **Sweep** is the set of all Runs for one tag. Sweeps are resumable by construction: the queue lives on disk, and every Run transitions through explicit states.

### Run states

```
pending → running → (solved | unsolved | timeout | error | invalid)
                 ↘ suspended → running
```

- `solved` — plan produced and validated.
- `unsolved` — planner terminated cleanly without a plan (legitimate result).
- `timeout` — hit the time limit. **This is a first-class result, not a failure and not a retry.** Expected on a large benchmark.
- `error` — non-zero exit, crash, signal, malformed output. Retry once, then record.
- `invalid` — plan produced but rejected by VAL. Always a bug worth surfacing loudly.

Each terminal Run also carries `timing_quality ∈ {clean, dirty}` — see §7.

---

## 4. Lifecycle

```
poll tags → build → materialize sweep → schedule → execute → validate → publish
```

1. **Tag detection.** Poll `git ls-remote --tags <origin>` on an interval (default 5 min) against the GitLab remote. A tag that isn't in the DB is a new sweep. Tags are the trigger — not build artifacts — because a tag means it was meant.
2. **Build.** `git worktree add` a clean checkout of the tag, `cargo build --release`, hash the resulting binary (blake3), record path + hash. Keep the last N tag binaries (default 5), garbage-collect older worktrees. A build failure is recorded and surfaced as a toast; the tag is not retried until its ref changes.
3. **Materialize.** Expand the benchmark manifest into pending Runs for the new tag. Manifest lives in the repo (§9) so the set of problems is versioned with the planner.
4. **Schedule / execute.** §5–§6.
5. **Validate.** Run VAL (or the internal validator) on every produced plan; store plan cost and validation result.
6. **Publish.** On track completion and on sweep completion, write standings files back into the repo working tree in the existing format (§8). Optionally auto-commit on a configurable branch.

---

## 5. Scheduler

### 5.1 Tiering from history

Before scheduling, classify each Run using results from previous tags for the same (domain, problem, config):

| Tier | Criterion | Policy |
|---|---|---|
| **A** | solved on previous tag, p50 runtime < 2s | pack densely — many concurrent, timing not precious |
| **B** | solved, 2s ≤ p50 < 30s | moderate concurrency |
| **C** | solved, p50 ≥ 30s | low concurrency, prefer isolation |
| **D** | previously `unsolved`/`timeout`, or never run (new problem, new domain) | one at a time, most-isolated slot, full timeout budget |

Rationale, stated plainly so the implementation doesn't drift: most of a sweep is a re-run of problems whose behaviour is already known. **Coverage is the metric that matters; absolute timing matters only at the frontier.** Tier A exists to burn through the known-fast bulk quickly so regressions surface within minutes instead of days.

**Execution order:** A → B → C → D. Fast coverage signal first; the expensive frontier work runs last, overnight if the estimator says it'll land there.

### 5.2 Core budget

Maintain a budget in "P-core equivalents," not process count:

- A Run declares its core demand from the manifest (default 1; some configs request 4).
- Total budget by throttle level (§6), e.g. full mode = `P_cores` (leave E-cores for the OS and the harness itself).
- Never oversubscribe P-cores. Tier A may pack multiple single-core Runs; a 4-core Run takes the whole budget on a 4 P-core machine and therefore runs alone.

### 5.3 Quiet hours

Configurable window (default **21:00–06:00**) during which the machine is known to be unattended: full throttle, contention detection relaxed (a background sync at 3am shouldn't demote anything), and Tier C/D preferentially scheduled here. The estimator should actively steer long work into this window.

---

## 6. Contention and throttling

Three levels, with hysteresis so it doesn't flap:

| Level | Trigger | Action |
|---|---|---|
| **FULL** | no foreign CPU pressure | P-cores, full budget |
| **POLITE** | sustained foreign CPU above threshold (default >25% of machine for >20s), e.g. mail sync, a big build, browser doing something stupid | re-set children to `QOS_CLASS_BACKGROUND` (E-cores only), cut budget to E-core count. **Runs keep running.** |
| **SUSPENDED** | a game is actually consuming CPU (§6.1), or foreign load >60% sustained, or memory pressure critical | `SIGSTOP` all children, hold. `SIGCONT` when clear for the de-escalation dwell time (default 60s). |

De-escalation is deliberately slower than escalation. Every transition is logged and toasted.

**This is the fix for the incident that motivated the project:** a mail client doing a one-time sync demotes the sweep to E-cores. It does not, under any circumstances, discard hours of completed computation.

### 6.1 Game detection

- Poll the process table (`sysinfo`) on a 2s tick.
- A process is a *game* if it is a descendant of the Steam client, or matches the configured `known_games` list.
- Presence alone is not enough — Steam idling in the background is fine. Trigger on a game process exceeding a CPU threshold (default 30% of one core) for >10s.
- Seed `known_games` with `Timberborn` (it's simulation-heavy and will absolutely fight the sweep for cores); the list is user-editable in config.
- On game exit: dwell, then `SIGCONT` and return to FULL.

### 6.2 Suspension and timeouts

**A suspended Run must not time out.** Two options, implement (a), optionally refine with (b):

- **(a) Effective wall clock.** Track cumulative suspended duration per Run; `effective_elapsed = wall_elapsed − suspended_total`. Compare the timeout against that.
- **(b) True CPU time.** Poll `proc_pid_rusage(pid, RUSAGE_INFO_V4)` via `libproc` and sum `ri_user_time + ri_system_time`. More faithful, especially under POLITE where the process is running slowly rather than not at all.

Record both `wall_ms` and `cpu_ms` on every Run regardless. IPC-comparable numbers come from `cpu_ms` on clean runs.

### 6.3 Process hygiene

- Every spawned child goes in its own process group; kill the group, never a bare pid, so a wedged planner can't leave orphans.
- Record live pids + pgids in the DB. **On startup, reap orphans from a previous instance** — this matters especially because a crashed parent leaves `SIGSTOP`'d children stopped forever.
- Hard kill escalation: `SIGTERM` → 5s → `SIGKILL`.
- Per-Run memory cap; a planner ballooning past it is killed and recorded as `error` with reason `oom`.

---

## 7. Result semantics

The central rule: **contention may invalidate a timing; it may never invalidate a result.**

- Any Run whose execution window overlapped a POLITE or SUSPENDED period, or any foreign-CPU spike above threshold, is marked `timing_quality = dirty`.
- `solved` / `unsolved` / `timeout` outcomes from dirty runs are **kept**. Coverage is coverage.
  - One caveat to encode: a `timeout` produced during a dirty window is suspect if timing was measured on wall clock. With CPU-time timeouts (§6.2) this mostly evaporates, but flag dirty timeouts distinctly (`timeout_dirty`) so they can be re-run in the clean pass rather than silently counting as coverage loss.
- **Clean-timing pass.** After a sweep's main body completes, re-run — in exclusive mode, one at a time, FULL only, ideally in quiet hours — every Run where `timing_quality = dirty` AND the problem is flagged `timing_matters` in the manifest (frontier problems, anything used for published comparisons). Everything else keeps its dirty number, clearly marked in the UI and the standings output.

---

## 8. Persistence

**Source of truth is the repo.** Standings files committed to git are the durable record. The database is a fast, queryable cache and work queue, and must be rebuildable from the repo's standings files plus the manifest.

- **Driver: `rusqlite` with the `bundled` feature.** Turso (the pure-Rust rewrite, formerly Limbo) is interesting and worth tracking, but its maintainers still label it beta and advise against mission-critical use. Weeks of benchmark history qualify as mission-critical. The schema is plain SQLite, so swapping the driver later is a one-file change.
- WAL mode, `synchronous = NORMAL`, single writer thread.
- `crucible db rebuild` reconstructs from standings files; `crucible db export` writes standings out.

### Schema sketch

```sql
CREATE TABLE tag (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,          -- v0.14.0
  commit_sha TEXT NOT NULL,
  binary_path TEXT,
  binary_blake3 TEXT,
  built_at INTEGER,
  build_status TEXT,                  -- ok | failed
  build_log TEXT
);

CREATE TABLE track   (id INTEGER PRIMARY KEY, name TEXT UNIQUE);   -- IPC5, IPC6...
CREATE TABLE domain  (id INTEGER PRIMARY KEY, track_id INTEGER, name TEXT,
                      UNIQUE(track_id, name));
CREATE TABLE problem (id INTEGER PRIMARY KEY, domain_id INTEGER, name TEXT,
                      pddl_path TEXT, timing_matters INTEGER DEFAULT 0,
                      UNIQUE(domain_id, name));
CREATE TABLE config  (id INTEGER PRIMARY KEY, name TEXT UNIQUE, args TEXT);

CREATE TABLE run (
  id INTEGER PRIMARY KEY,
  tag_id INTEGER, problem_id INTEGER, config_id INTEGER,
  state TEXT NOT NULL,                -- pending|running|suspended|solved|unsolved|timeout|error|invalid
  tier TEXT,                          -- A|B|C|D
  started_at INTEGER, finished_at INTEGER,
  wall_ms INTEGER, cpu_ms INTEGER, suspended_ms INTEGER, peak_rss_bytes INTEGER,
  timing_quality TEXT DEFAULT 'clean',
  plan_cost REAL, plan_length INTEGER, plan_path TEXT,
  validated INTEGER, exit_code INTEGER, error_reason TEXT,
  pid INTEGER, pgid INTEGER,
  attempt INTEGER DEFAULT 1,
  UNIQUE(tag_id, problem_id, config_id, attempt)
);
CREATE INDEX run_state_idx ON run(state);
CREATE INDEX run_tag_idx   ON run(tag_id, problem_id);

CREATE TABLE event (                  -- the rolling log, and the audit trail
  id INTEGER PRIMARY KEY,
  at INTEGER NOT NULL,
  level TEXT,                         -- info|warn|error
  kind TEXT,                          -- throttle|game|build|run|track|system
  run_id INTEGER, message TEXT
);

CREATE TABLE throttle_window (        -- what makes a run dirty
  id INTEGER PRIMARY KEY,
  level TEXT, started_at INTEGER, ended_at INTEGER, reason TEXT
);
```

---

## 9. Manifest

A versioned file in the Ferroplan repo (`benchmarks/manifest.toml`) enumerating tracks, domains, problems, per-problem overrides, and which problems have `timing_matters = true`. The harness reads the manifest **from the checked-out tag**, so adding IPC6/IPC7 domains is a repo change, not a harness change.

---

## 10. Comparison / regression engine

- Default view: current tag vs previous tag.
- `crucible diff <tagA> <tagB>` for any pair, also reachable from the TUI.
- Comparison table, per track and per domain:
  - **coverage** — solved count, with delta (`+3`, `−1`), and the specific problems gained/lost named. Losses are the loud case: a problem solved on the previous tag and not on this one is a **regression** and should be red, toasted, and pinned in the UI.
  - **quality** — mean/total plan cost over the problems solved by *both* tags (never compare cost over differing solved-sets).
  - **timing** — p50/p95 `cpu_ms` over commonly-solved problems, clean runs only; explicitly show how many runs were excluded as dirty.
- Historical rank view: for each track, coverage per tag over time — this is the sparkline that makes it worth watching.

---

## 11. TUI

**Library:** `ratatui` + `crossterm`.
**Render budget:** fixed tick, default **4 fps**, redraw only on state change or tick. The dashboard must never meaningfully compete with the thing it is measuring — target well under 1% of one core. All animation is cheap.
**Layout:** one screen, no paging, reflows to terminal size. Runs inside Zellij + Kitty.

### Layout

```
┌───────────────────────────────────────────────────────────────────────────┐
│  ██████ ██████ ██  ██  ...   (figlet banner)      v0.14.0   ● FULL        │
│  Ferroplan sweep · uptime 3d 04:12 · quiet hours in 2h 41m                │
├───────────────────────────────────────────────────────────────────────────┤
│  SWEEP  ████████████████████░░░░░░░░░░  61%   1284/2103   ETA 14h 22m     │
│  coverage 1102 (+7 vs v0.13.0)   regressions 1 ⚠   dirty 43              │
├──────────────────────────────────────┬────────────────────────────────────┤
│ TRACKS                               │ SLOTS                     4 P-core │
│  ✔ IPC5   86/86  +0        [collapsed]│  0 ▶ rovers/p12      A   00:04.2  │
│  ▶ IPC6   412/598 +5  ███████░░░ 68%  │  1 ▶ tpp/p21         A   00:01.8  │
│      openstacks  ██████████ 30/30     │  2 ⏸ storage/p18     C   suspended│
│      pathways    ████░░░░░░ 12/30 ⚠   │  3 · idle                         │
│      trucks      ███████░░░ 21/30     │                                    │
│  · IPC7   0/419            [queued]   │  throughput ▁▂▄▆█▆▄▂▁  22 runs/min │
├──────────────────────────────────────┴────────────────────────────────────┤
│ 14:02:11  ⚠  POLITE — foreign CPU 34% (mail) — demoted to E-cores         │
│ 14:03:40  ✔  IPC6/openstacks complete — 30/30 (+1 vs v0.13.0)             │
│ 14:04:02  ✖  REGRESSION IPC6/pathways/p07 — solved in v0.13.0, timeout    │
├───────────────────────────────────────────────────────────────────────────┤
│ j/k move  ⏎ expand  tab pane  d diff  f filter  p pause  q quit           │
└───────────────────────────────────────────────────────────────────────────┘
```

### Progressive disclosure

- A finished track **collapses to a single line** with its final coverage and delta.
- The active track expands to per-domain progress bars; the active domain expands to in-flight problems.
- Queued tracks are a dim single line.
- Manual override: `⏎` toggles expansion on the selected node, `z` collapses all finished.

### Toasts

Bottom-right overlay, 4s dwell, stacked, dismissible with `esc`:
`game detected — suspending` · `resumed` · `new tag v0.14.1 — building` · `build failed` · `REGRESSION` (sticky, red, requires dismissal).

### Keys

Vim and arrows both: `j/k/↑/↓` move, `h/l/←/→` collapse/expand, `g/G` top/bottom, `tab` cycle pane, `d` diff view, `f` filter log, `/` search, `p` pause/resume sweep, `s` force-suspend, `r` re-run selected, `q` quit.

### Style

Truecolor (Kitty supports it). One coherent palette — forge/iron: deep charcoal ground, ember orange for active, steel blue for structure, green for solved, amber for dirty/timeout, red for regression. Banner via `figlet-rs` with an embedded font (no runtime figlet dependency). Unicode block progress bars, braille sparklines. Degrade gracefully to 256-color and ASCII if `TERM` says so.

---

## 12. Configuration

`~/.config/crucible/config.toml`:

```toml
[repo]
url            = "git@gitlab.com:harold/ferroplan.git"
local          = "~/src/ferroplan"
tag_poll_secs  = 300
keep_tags      = 5

[sweep]
default_timeout_secs = 1800
retry_errors         = 1
validator            = "val"          # or "internal"

[scheduler]
reserve_p_cores      = 0              # cores held back from the budget
tier_a_max_secs      = 2
tier_c_min_secs      = 30

[quiet_hours]
start = "21:00"
end   = "06:00"

[contention]
polite_threshold_pct     = 25
polite_dwell_secs        = 20
suspend_threshold_pct    = 60
resume_dwell_secs        = 60
game_cpu_threshold_pct   = 30
game_dwell_secs          = 10
known_games              = ["Timberborn"]
steam_process_names      = ["steam_osx", "steamwebhelper"]

[ui]
fps         = 4
theme       = "forge"
banner_text = "CRUCIBLE"
```

---

## 13. Crate layout

Single binary, library core, so a daemon/client split is cheap later.

```
crucible/
  src/
    main.rs           CLI: run | daemon | attach | diff | db | status
    config.rs
    db/               rusqlite, migrations, queries
    git/              tag polling, worktrees, build
    manifest.rs
    sched/
      tier.rs         history-based classification
      budget.rs       core accounting
      queue.rs        state machine, resumption
      estimator.rs    ETA from historical runtimes
    exec/
      runner.rs       spawn, wait, capture
      supervise.rs    SIGSTOP/SIGCONT, kill escalation, orphan reaping
      rusage.rs       cpu time sampling (libproc)
    platform/
      mod.rs          trait Platform
      macos.rs        QoS classes, sysctl, memory pressure
    monitor/
      cpu.rs          foreign load sampling, hysteresis
      games.rs        steam/process detection
      throttle.rs     FULL/POLITE/SUSPENDED state machine
    validate.rs       VAL integration, plan cost
    standings.rs      repo-format read/write
    compare.rs        tag diff, regression detection
    tui/
      app.rs          state + event loop
      layout.rs  tracks.rs  slots.rs  log.rs  toast.rs  theme.rs  banner.rs
    event.rs          shared event bus
```

**Concurrency:** `std::thread` + `crossbeam-channel`, not tokio. This is process supervision and a 4 fps redraw; async buys nothing and complicates signal handling.

**Threads:** scheduler, N runners, monitor, db writer, tag poller, UI. All communicate over the event bus; the UI is a pure consumer of state snapshots so it can never block the sweep.

**Dependencies:** `ratatui`, `crossterm`, `rusqlite` (bundled), `crossbeam-channel`, `sysinfo`, `libproc`, `nix`, `serde`/`toml`, `figlet-rs`, `blake3`, `time`/`jiff`, `tracing` + `tracing-appender`, `color-eyre`, `clap`.

---

## 14. Edge cases to handle explicitly

- Parent dies with children `SIGSTOP`'d → orphans stopped forever. Reap on startup from recorded pids.
- Machine sleeps mid-run → wall clock jumps hours. Detect via monotonic-vs-wall divergence; treat as suspension, mark dirty, don't time out.
- New tag lands mid-sweep → finish the current track, then decide by policy (`preempt` | `finish_track` | `finish_sweep`, default `finish_track`). Never lose the partial sweep; it stays queryable.
- Tag deleted or force-moved upstream → don't silently rewrite history; record and warn.
- Disk fills from plan files → quota per sweep, prune plans for `solved` runs older than N tags (keep costs, drop plan text).
- Terminal resize below minimum → render a compact fallback, never panic.
- Manifest changed between tags → new problems are Tier D; removed problems stay in history and are excluded from diffs with a note.
- Two instances started at once → advisory lock file on the DB.

---

## 15. Implementation phases

1. **Skeleton + persistence.** Config, schema, manifest parsing, CLI, `db rebuild/export`. No UI beyond stdout.
2. **Runner + queue.** Spawn, timeout, capture, validate, checkpoint, resume. Prove a sweep survives `kill -9` of the harness with zero lost completed work.
3. **Monitor + throttle.** Foreign CPU sampling, game detection, QoS demotion, SIGSTOP/SIGCONT, dirty marking, orphan reaping. Prove it by launching Timberborn mid-sweep.
4. **Tiering + estimator.** History-based classification, core budget, quiet hours, ETA.
5. **TUI.** Layout, progressive disclosure, toasts, keybinds, theme, banner.
6. **Compare.** Diff engine, regression detection, standings publication.
7. **Polish.** Daemon/`attach` split over a Unix socket so a Zellij restart can't kill a three-day sweep; clean-timing pass; sparkline history views.

Phases 1–3 are the ones that actually get your machine back. Everything after is quality of life.

---

# R2 — the smarter sweep (revision of 2026-09-02)

**Status:** design revision, scoped while the 0.26 cut sweep runs; code starts
after the 0.26 cut. Roadmap phase and gates: `docs/roadmap-0.27.md`.
**Supersedes:** §5.1–5.2, §6.2, §7 and §11 above, and the tier.rs / budget.rs
module headers that corrected §5 the first time. Everything not named here
stands: the database is the truth, the JSONL is an export, `kill -9` loses
nothing, the Python oracle stays.

## R2.0 The case, from the 0.26 cut sweep's own record

The R1 rule — a row measured while *anything else* on the box exceeded 25 %
pcpu is owed again — was written for a box that was empty at night. The
0.26 cut sweep ran for four days on a box that was not:

| what the watcher saw (17,280 samples, 4 days) | |
|---|---:|
| samples under the clean line (< 25 % foreign pcpu) | 11,814 (68 %) |
| 25–60 % | 1,428 |
| ≥ 60 % | 4,038 (23 %) |
| top competitors (samples present) | Docker's VM 3,135 · WindowServer 2,627 · Brave renderer 2,598 · Docker Desktop 1,318 · claude 1,256 · Timberborn 1,125 (avg **181 %**) · backupd 849 |
| swap in use, steady | ~4.9 GB |

Pass 1 owed 2,660 of 8,444 rows; pass 2 banked **zero** on ten boards and
re-owed them whole; the ETA went from a day to "three-plus". And the rows
being re-owed were mostly honest: of the 1,814 dirty *timeouts* over 50 s,
**1,618 (89 %) had used ≥ 90 % of their own wall-clock as CPU** and 1,772
(98 %) had used ≥ 80 %. The planner had its core. The referee was looking at
the wrong thing.

Four things the record also shows about the code, none of them in the 0.26
cut record until now:

1. **`Platform::cpu_ms` is 41.67× low on Apple Silicon.** `pidrusage`'s
   `ri_user_time`/`ri_system_time` are in Mach absolute-time units, not
   nanoseconds; `mach_timebase_info` on this box is 125/3. Every `cpu_ms` in
   the database carries the error (mean cpu/wall reads 0.023 where it should
   read 0.96). It is a units bug, and it was invisible because nothing read
   the column.
2. **Short runs undercount CPU** regardless of units: the value is the last
   0.25 s poll before exit, so a 300 ms run records whatever the first poll
   caught, often nothing. The exact figure is available for free from
   `wait4`'s `rusage` at reap and was not taken.
3. **The throttle never reaches the child.** `attempt()` builds the control
   channel as `let (_tx, rx) = mpsc::channel::<Ctl>()` and drops `_tx` on the
   spot. FULL/POLITE/SUSPENDED is computed, logged, and used only to dirty
   rows; no `SIGSTOP`, no demotion. Timberborn ran at 181 % of a core
   against un-stopped planners for ~6 hours of samples.
4. **`sched::tier` is not called** from the sweep. History-ordered
   execution is built and tested and unwired. And every row crucible has
   written is stamped `jobs = 2` (the manifest default, the Python's width)
   while `attempt()` runs one instance at a time.

Also: `cpu_speed_limit` is NULL on every sample — `pmset -g therm` reports
nothing on Apple Silicon — so the thermal instrument the spec assumed has
never existed on this box.

## R2.1 The referee — per run, not per box

**The verdict on a run is a property of that run's process, read from the
kernel, not of the box's process table.**

Definitions, for a run with `threads = 1`:

```
effective_wall = wall_ms − suspended_ms
ρ (starvation ratio) = cpu_ms / effective_wall        cpu_ms from wait4 rusage
```

| outcome | banks? | timing_quality |
|---|---|---|
| **solved**, VAL-valid | **always** — coverage is coverage | `clean` if ρ ≥ ρ_min ∧ window clean ∧ no neighbours; `packed` if it had neighbours; otherwise `dirty` |
| **unsolved / timeout** | iff ρ ≥ ρ_min ∧ no clock jump ∧ no thermal flag ∧ no swap-growth flag | `clean` / `dirty` as above |
| unsolved, otherwise | **no** — re-queued **SOLO** (§R2.2), attempt + 1 | — |
| error (crash, signal, malformed) | retry once, then bank as today | — |
| invalid (VAL rejected) | bank, loud, as today | — |

`ρ_min` defaults to **0.90**. It is the one number in this revision that
must not be tuned to make a sweep finish: 0.90 is where the clean record's
own distribution sits (median 1.00, p5 0.88 on 10–50 s runs), and the
roadmap's Phase 0 re-derives it from the corrected instrument before it is
trusted.

What ρ cannot see, and what covers it:

- **Memory bandwidth and thermal clock.** A starved-of-bandwidth or
  down-clocked core still reports 100 % CPU. Covered by the canary (§R2.3)
  and by the Phase 0 packing calibration, which measures exactly this.
- **Swap.** The box ran at ~4.9 GB swapped through the whole 0.26 sweep. A
  run whose window saw swap grow by more than `swap_growth_mb` (default
  512) is flagged; a timeout under the flag is re-queued.
- **Sleep / clock jump.** Unchanged from R1: a monotonic-vs-wall divergence
  is a suspension, never a timeout.
- **`threads > 1` (the mco boards).** ρ is not meaningful for a planner
  that may not saturate its threads. These boards keep the R1 rule whole:
  solo, no neighbours, box-wide window gate, competition wall-clock.

The box-wide window (`Reader::window_gate`) is **kept** — it qualifies
*timing*, it feeds the throttle, and it draws the timeline — but it no
longer decides whether a row banks.

### The instrument, fixed before the referee is trusted

- `cpu_ms` comes from `wait4(2)` `rusage` (`ru_utime + ru_stime`) at reap:
  exact, in microseconds, no polling gap. `pidrusage` polling stays for the
  live view only, with the Mach timebase applied.
- New column `run.cpu_instrument TEXT` (`'wait4'` | `'pidrusage-mach'` |
  `'pidrusage-ns'`), on the `mem_instrument` pattern. **The referee trusts
  `wait4` rows only.** Existing rows are not rescaled in place — the factor
  is exact but the poll undercount is not recoverable — they are labelled
  `pidrusage-ns` and treated as ρ-unknown, which means they stand exactly
  as R1 judged them. Nothing already banked moves.

## R2.2 The scheduler — one queue, three classes, a width

**The sweep is one queue of runs across all boards.** Boards are the
display and publication unit, and the artifact writer still emits a board
when its last row lands; they are no longer the unit of execution. This
retires the "one board at a time" rule and the `budget.rs` argument that
two boards at once make chimera rows — that argument rested on `jobs` being
identity, and R2 stops pretending it is (below).

### Class, from history

"History" is the most recent measured row for the same
(instance, budget, mode, threads) on **this box**, any engine — the
previous tag, or the last sweep's own row. Never a different box.

| class | when | width |
|---|---|---|
| **PACK** | prior solved with `time < pack_max_frac × budget` (default 0.5), `threads = 1` | up to `pack_width` (default 4 = P-cores) concurrent, bounded by memory (below) |
| **SOLO** | prior solved at ≥ 0.5 × budget (near-wall); prior unsolved/timeout; never run; re-queued by the referee | one at a time on the P-cores, nothing else of ours running |
| **EXCLUSIVE** | `threads > 1` | one at a time, box-wide window gate, as R1 |

Order: PACK first — the fast coverage signal and the regressions land in
the first hour, not the third day — then SOLO near-wall, then SOLO prior
timeouts, then never-run. The ETA steers the 300 s tier and the timeout
tail into quiet hours.

On the record from the 0.25 promoted raws: **4,284 of 8,444 instances are
PACK-class**, 219 are near-wall, 202 sit at ≥ 75 % of budget, and 3,739
are prior timeouts. The timeouts are ~62 core-hours of the sweep's ~70;
packing the solves buys minutes, and the honest re-owing of timeouts buys
days. Whether prior timeouts may run 2-wide under ρ is **not decided
here** — it is the roadmap's Phase 0 calibration, and until it reports
they run SOLO as the operator chose.

### Recorded 2026-09-04 — the packing calibration (roadmap 0.27 Phase 0.c)

40 PACK-class instances (prior solo 5–30 s, fourteen boards), solo ×3
then 4-wide ×3 on the four P-cores, the box in ordinary use:

| | median | p95 | max |
|---|---:|---:|---:|
| wall inflation, 4-wide over solo | **+73 %** | +106 % | +335 % (tpp-strips i26, 10.4 s → 45.3 s) |
| packed ρ (cpu / wall) | 0.991 | | |

Every packed planner had its core — ρ 0.99 — and ran 1.73× slower.
Two packed runs failed where solo solved (no-mystery-opt i5, factory-robot
i7). **By the pre-registered thresholds, 4-wide packing is a RECORDED
NEGATIVE; the referee ships alone**, and the `pack_width` default is 1. The
2-wide figure (the width the Python sweeps ran at as `jobs = 2`) is in the
roadmap's Phase 0 record.

The finding reaches further than packing: **ρ cannot see a slow box.** A
process with its core can still be starved of memory bandwidth or clock by
a neighbour on another core — ours or anyone's. So the canary (R2.3) is
the referee's second input from Phase 1, not a later nicety: an unsolved
row banks only if ρ ≥ ρ_min *and* the canary's clock factor across the
run's window stayed under `canary_max_factor`.

### Packing can lose time; it can never lose a row

A PACK run that does not solve is never banked from the packed slot. It is
re-queued SOLO, and the SOLO result is the one that stands. So the
worst case of a wrong width is a wasted packed attempt, and the tier.rs
objection — "an instance slowed by a neighbour crosses the wall it would
otherwise have beaten, and the board loses a solved row" — cannot happen
by construction. It is answered, not overruled.

### Width, live

- **FULL:** `pack_width`, less the memory bound.
- **POLITE:** width 1 and children demoted to the background band
  (E-cores). Solves still bank (`dirty` timing); unsolved re-queue.
- **SUSPENDED:** `SIGSTOP` every child, `suspended_ms` accrues, nothing
  times out. **This is the `_tx` that was dropped**, wired.
- **Memory:** width is also `⌊(free − mem_reserve_gb) / Σ expected_rss⌋`,
  where a PACK run's expected RSS is its prior `peak_rss × rss_headroom`
  (default 1.5) and an unknown one is the manifest `mem_gb`. 16 GiB with a
  Docker VM resident does not fit four 6 GB caps; it fits four 1 GB priors.

### The truth on the row

- New columns: `run.neighbours INTEGER` (our own concurrent planners at
  spawn, and the max seen during the run), `run.pack_class TEXT`.
- `jobs` keeps its place in the export — the raw's shape is byte-stable —
  but it **leaves the resume gate's identity**. Identity is
  (engine BLAKE3, budget, mode, threads). Recorded plainly: the Python
  stamped 2 on rows measured 2-wide; crucible has stamped 2 on rows
  measured 1-wide; R2 stamps the manifest value and writes the real width
  beside it.
- Standings never read `neighbours`; coverage is coverage. `crucible diff`'s
  timing columns (p50/p95) read `clean` rows only, as R1 said, and now say
  how many `packed` rows they excluded.

## R2.3 The canary — the thermal referee this box can run

`pmset -g therm` is empty on Apple Silicon and `powermetrics` needs root.
So the sweep carries its own clock:

- A fixed **canary instance** — a solve of ~2 s with no variance across the
  record; the default is `trucks-propositional` i8 on `ipc5-prop`
  (1,820 ms on all three engines in the database, spread 1.000) — runs
  **solo** every `canary_interval_secs` (default 1200) and in the first idle
  gap after any throttle transition. Cost: 2 s in 20 min.
- Its baseline is the median of its first `canary_baseline_n` (default 5)
  clean solo runs on this box, stored per box in the database, never
  carried across boxes.
- `clock_factor = wall / baseline`. Above `canary_max_factor` (default
  1.15) the window is **thermal** (the name covers every way a box gets
  slower than its baseline — clocks and bandwidth alike): solves bank,
  timeouts re-queue, the header shows the factor, the timeline draws it.
  The factor rides on every watcher sample (`sample.canary_factor`) until
  the next reading, so a run's window is asked the same way it is asked
  about competitors. The canary is also what
  the Phase 0 packing calibration reads to separate "neighbours slowed it"
  from "the box was hot".

## R2.4 The TUI — `crucible sweep` is the dashboard

**Process model:** the sweep hosts the TUI. `--headless` prints the R1
log; `--dump` still renders one frame off-screen for a transcript or CI.
Resilience is the database's, not the terminal's: a dead terminal is a
dead renderer and nothing else. (The daemon/attach split stays deferred.)

Render budget unchanged: 4 fps, redraw on tick or change, under 1 % of one
core. The grid is O(cells) to draw and its cells change only at run
boundaries.

### Views

**1. Grid** (home). One row per board, in queue order:

```
 board                 banked   owed  ▕ instances ─────────────────────────────────────▏
 ipc5-prop           358/450     0   ▕████████████████████▓▓▓▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▏
 ipc67-results       498/580    14   ▕██████████████████▓▓▓▓▶▶▶▶░░░░░░▒▒▒▒▒▒▒▒▒▒▒✖▒▒▒▏
 ipc2023-agile-300s   38/140    68   ▕███▓▓▶·······················▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▏
```

One cell per instance; when a board has more instances than the strip has
columns, a cell aggregates *k* instances and shows the worst state among
them (owed > running > error > timeout > solved). Cells:

| glyph | state |
|---|---|
| `·` | queued |
| `▶` | running (ember) |
| `█` | solved, timing clean (green) |
| `▓` | solved, packed or dirty timing (green, dim) |
| `▒` | unsolved/timeout, banked (steel) |
| `░` | unsolved, **owed** — re-queued SOLO (amber) |
| `✖` | error (red) |
| red underline / green underline | regression / gain vs the comparable predecessor |

Header: engine + hash, throttle level and reason, canary factor, width in
flight (`3/4`), banked/owed/ETA, top-3 competitors right now. Footer: the
live slots (below).

**2. Board** (`⏎` on a row). A table of the board's instances: label,
state, this time, previous tag's time, best-ever on this box, Δ, ρ,
neighbours, attempt, timing quality. Sort by any column (`o`). Beside it a
histogram of solve times against the budget with the near-wall band
(≥ 75 %) shaded and counted — where the flips live.

**3. Instance** (`⏎` again). Every attempt of the row; the box timeline
across the run's window (competitors stacked by name, throttle band,
canary factor, our own neighbours); the last `stderr_tail_lines` (default
40) of the planner's stderr, captured live through a ring buffer on the
existing pipe reader; plan cost and VAL verdict.

**4. Timeline** (`t`). The whole sweep's box-wide timeline: competitors by
name, throttle level, canary, swap, with every run as a mark and the owed
ones highlighted. "Was something else running?" answered without sqlite.

**5. Slots** (always visible, footer). The running instances: board,
instance, class, elapsed ticking against budget, cpu %, RSS, ρ so far,
last stderr line.

### Keys

R1's, plus `t` timeline, `b` back to the grid, `o` sort, `/` filter by
board or domain. **There is no manual re-run key.** The operator's
position, recorded: if the automatic retry is right there is nothing to
press; if it is wrong the fix is the referee, not a key.

## R2.5 Unchanged, stated so nobody has to check

Exports byte-identical to the committed raws; `crucible standings --check`
against the Python oracle; the resume gate's engine BLAKE3; the orphan
reaper; the `.done` marker only when nothing is owed; `--no-db` writing the
pre-database shape.

## R2.6 Configuration additions

```toml
[scheduler]
pack_width          = 1      # 4 was the design; the calibration refused it (R2.2)
pack_max_frac       = 0.5    # prior time / budget below which a run is PACK
mem_reserve_gb      = 4
rss_headroom        = 1.5

[referee]
cpu_ratio_min       = 0.90   # ρ_min — re-derived in Phase 0, not tuned
swap_growth_mb      = 512
canary_board        = "ipc5-prop"
canary_variant      = "trucks-propositional"
canary_instance     = "8"
canary_interval_secs = 1200
canary_baseline_n   = 5
canary_max_factor   = 1.15

[ui]
stderr_tail_lines   = 40
```
