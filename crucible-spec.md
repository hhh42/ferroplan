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
