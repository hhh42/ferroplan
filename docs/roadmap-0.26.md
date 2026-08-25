# ferroplan 0.26 roadmap — the instrument gets rebuilt

Scoped 2026-08-25. This cycle's headline is not the engine. It is the
**harness**: `crucible`, a resident Rust program that replaces the sweep
drivers, the runner, the contention watcher and the standings generator with
one supervised process. Design spec: `crucible-spec.md`. Working plan and
phase gates: the approved crucible plan.

The case, in one number from the record: the 0.25 entries sweep
(`benchmarks/entries25-sweep.log`) took **five passes and roughly 37 hours** to
bank ten boards, because a board measured under contention is thrown away
whole. Pass 1 burned thirteen hours and banked one board.
`PER-INSTANCE-RETRY.md` softened that to per-row reuse inside a pass;
crucible finishes the job by making the *database* the truth and the JSONL a
pure export, so a killed sweep loses nothing at all.

The discipline for a harness cycle is the same as for an engine cycle, and
one rule dominates: **a port that changes a number cannot prove it is a
port.** Every decision below follows from that.

---

## Phase 0 — the sitting, before any Rust

### Recorded — the oracle is rescued, not retired

The Python is **kept permanently** as a differential oracle. `standings.py` is
1,104 lines of pure, stdlib-only, build-step-free code, and it is the only
independent implementation of the failure-class taxonomy that exists. Running
it beside crucible costs about two seconds per cut. Every incident in its own
comment corpus is a case of one implementation drifting from another
*unobserved*; retiring the observer to save Python nobody has to maintain
would be the same trade that produced the incidents.

What is being retired is the **shell driver and the model babysitting it**,
not the measurement code.

### Recorded — the incident evidence was one disk failure from gone

`.gitignore` excludes `benchmarks/air*/` and every `benchmarks/ipc*.jsonl`
except the three optimal boards' raws — which it un-ignores with a comment
arguing they are "evidence rather than logs".

That argument generalizes further than it was applied. The **only** physical
evidence of the 15-instances-light incident — eight `factory-robot-2026` and
seven `data-network-2018` rows, both `val=false` because VAL could not
*ingest* the domain — lived in `benchmarks/air/`, gitignored, unbacked. So did
the only four conditions files on the box carrying a per-sample `timeline`,
without which the resume gate's contention side cannot be tested at all.

Rescued to `crucible/tests/fixtures/`, 108 KB, with a re-runnable extractor
(`extract.py --check`) that refuses to let a fixture be hand-edited into
agreement with a test. Provenance for each one is in that directory's README.

Two full 12-board backfill sets (`benchmarks/air-0.19.0/`,
`benchmarks/air-0.21.0/`) turned out to be tracked already, so the board-render
goldens are hermetic today without committing anything further.

### Recorded — a live misclassification in the published table

**Found while planning the port, verified against the committed raws and the
committed table.**

`standings.py:262` matches `ntext == "mem-cap"` by **exact equality**.
`ipc67.py:493` — the 0.24 "label hygiene" change — began emitting
`"mem-cap (self-inflicted: node byte target raised)"` for the case where the
refill re-entry raised the node byte target past the declared model. The
labelled variant matches nothing, falls past the `mem-cap` and `spawn-fail`
tests, past the timeout line, and lands in **`early-exit`** — the class the
0.20 refill loop exists to empty, and therefore the one column the refill loop
is refereed by.

Seven rows are misfiled in `benchmarks/ipc-standings.md` today:

| line | board | reads | should read |
|---|---|---|---|
| 61 | 2023 numeric | `6 early-exit, 1 mem-cap` | `0 early-exit, 7 mem-cap` |
| 52 | 2014 seq-mco t4 | `2 early-exit, 1 mem-cap` | `1 early-exit, 2 mem-cap` |

Two more sit in `ipc2014-mco-t8`, swept and awaiting promotion.

This is the same shape as the 0.20 audit's finding that maintenance-2014's
"eight rejects" were ordinary timeouts wearing that costume: a label changed on
one side of a two-file contract, and the other side kept matching the old one.
Coverage is untouched — every one of these rows is an unsolved row either way —
so no headline number moves. What moves is the attribution.

**Decision: port the bug, prove the port, then fix it as its own named
change.** `crucible`'s `classify()` ships with exact equality so byte-parity
against the oracle is demonstrable; a *separate* commit then widens the match,
regenerates the goldens, and records the −7 early-exit / +7 mem-cap movement in
the cut record. `spawn-fail` has the identical exact-equality shape and gets
the same treatment.

After the fix the drift becomes structurally impossible: in Rust the runner's
note is a typed variant and the classifier matches the variant, so there is no
string for the two sides to disagree about.

### Recorded — dirty rows are kept, dirty boards are not banked

`crucible-spec.md` §7 says to keep results measured under contention —
"coverage is coverage" — and re-run only where timing matters. This repo's law
is the opposite, and `contention.py`'s docstring says why: contention **only
ever depresses** coverage, so it "manufactures REGRESSIONS and hides GAINS",
which is the expensive direction to be wrong in when the output is a release
record.

Both are right about different things, and both hold:

- Every measured row is **kept** in the database, marked dirty. Nothing is lost
  to contention, the dashboard shows real progress, and a clean row later
  supersedes the dirty one.
- A board is **not banked and not promotable** until every row it carries is
  clean. Published semantics stay exactly as strict as they are today.

Consequence: the spec's `timeout_dirty` class and its separate "clean-timing
pass" both collapse away. Dirty implies re-run, for every outcome, not just
timeouts. `timing_matters` survives only as a scheduling hint.

### Recorded — the resume gate's version check is too weak, and is being closed

`PER-INSTANCE-RETRY.md` names the risk — "a stitched board must never mix rows
from two different `ff` builds" — and then gates on the `ff --version` string,
adding "probably also the git SHA if the binary carries one". It does not.
Every dev build of a cycle reports `ff 0.25.0`, so **two different 0.25.0
builds stitch silently today**.

crucible gates on the binary's **blake3** and keeps writing `ver` into the row
for artifact compatibility. Under the candidate-driven trigger — where the
working-tree binary is rebuilt constantly — this was the likeliest way the new
harness would have produced a chimeric board.

### Recorded — the sweep environment is scrubbed, and says what it was

`ipc67.py` builds the child environment as `dict(os.environ, ...)`. There are
**132** `FF_*` hatches in the engine. An operator with any one of them exported
in their shell silently changes every board in the sweep, and **nothing in any
row records that it happened.**

crucible starts from a scrubbed environment, injects the budgets, applies the
board's declared `env`, and stores the canonical `env_json` on the board row.
A row can no longer have been measured under a hatch nobody can name.

---

## Where `crucible-spec.md` is wrong

The spec was written from a conversation rather than from the scripts. The
corrections are in the working plan; the ones that are *decisions* rather than
plain facts are recorded above. The plain factual ones, briefly: the remote is
GitHub and not GitLab; the sweep runs **before** the tag, so tag-polling is a
backfill path and not the trigger; the manifest is a **selector** (regex over
variant directories) and not an enumeration, because the corpus is gitignored
and an enumeration would drift with nothing to notice; the default timeout is
60 s and not 1800 s; and a process cannot set another process's Darwin QoS
class, so mid-flight demotion is `setpriority(PRIO_DARWIN_PROCESS, …,
PRIO_DARWIN_BG)`.

One more, which is a decision: the spec's §5.1 tiering says Tier A should
"pack densely — timing not precious". **Here timing is coverage.** The metric
is coverage-at-60s-wall, so an instance slowed by a neighbour is coverage
removed. Packing densely *is* the contention. Tiering survives only as
within-board ordering (bank the known-fast rows early, so a mid-board
contention window costs less re-run) and as the ETA input. It never raises
`jobs`.

---

## Recorded — what the port actually reproduces

The gates below are the whole argument. A port that changes a number cannot
prove it is a port, so every one of them is an equality against the oracle or
against a committed artifact, run by `crucible/preflight.sh`.

**The published tables regenerate byte for byte.**

```
$ crucible --repo . standings --doc all --check
ok    detail   benchmarks/ipc-standings.md matches
ok    summary  STANDINGS.md matches
```

That is `standings.py`'s failure-class taxonomy, its IPC-5 archive scoring
(length and makespan, recomputed per `.soln` because the headers are empty on
the planner that dominates those tracks), the vs-field column, the Strong /
Middle / Weak split and the proof-track marks — all of it, from the committed
raws, identical to the committed documents.

**The classifier agrees with the oracle over every row on this box.**

```
314 agree, 0 MISMATCH (42,356 rows classified, 144 boards)
```

`classify()` per row, `coverage_line()` per board as an exact string, and the
corpus selector per track.

**The corpus selector survived losing its lookbehinds.** Two of `ipc67.py`'s
`TRACK_PATTERNS` use negative lookbehind, which Rust's `regex` cannot compile by
design. The manifest expresses them as include/exclude pairs, and the
equivalence is not reasoned about but RUN: all 26 tracks select exactly the same
variants, checked over the 292 variant directories on disk. Selecting one
variant too many or too few would silently change a board's denominator.

**Every committed raw round-trips byte for byte** — 43,186 rows across 144
files.

**The corpus enumeration agrees instance for instance.** All 26 tracks produce
the same variant list AND the same instance counts as `ipc67.py --track T
--list`, including the multipart labelling rule that keeps `ipc-2026n`'s 320
instances under 320 distinct keys rather than the 288 a first-group rule gives.
The independent confirmation is the sweep plan itself:

```
$ crucible sweep --set cut25 --require-version 0.25 --dry-run
set cut25: 6366 instances
```

6,366 is the denominator `STANDINGS.md` publishes. The board registry, the
track selectors and the corpus walk agree with the standing table without
having been told what answer to reach.

### Recorded — three defects the port found on its way through

1. **`serde_json` parses some floats one ULP off.** `9189.980000000001`, a real
   metric in `ipc2023-numeric`, parses to `0x…70a` where both `std::parse` and
   Python give `0x…70b`. Every version tested does it; it is the fast float
   path, not a regression. A round-tripped board would have silently rewritten a
   measured number. Closed with `arbitrary_precision`, which keeps the original
   token.

2. **`Option<T>` cannot express this row format.** `makespan` is present-and-null
   on a solved row and absent entirely on an unsolved one — seven distinct key
   sequences exist across the corpus — so presence had to become explicit. A
   writer using `skip_serializing_if` alone produces rows that differ from every
   board ever committed.

3. **The spec's politeness lever does not exist.** `crucible-spec.md` §6 says to
   "re-set children to `QOS_CLASS_BACKGROUND`". You cannot:
   `pthread_set_qos_class_self_np` is self-only, and by the time the scheduler
   wants to demote, the child has long since `exec`'d. Demoting a running
   process is `setpriority(PRIO_DARWIN_PROCESS, pid, PRIO_DARWIN_BG)`.

### Recorded — the supervisor's properties are tested, not asserted

Against a stub planner (`fakeff`) that does exactly what a test tells it:

- A child emitting 4 MiB on stdout does not deadlock the supervisor. `try_wait`
  in a poll loop without draining the pipes blocks the child in `write` and then
  waits for it forever, the moment `ff --json` exceeds the 64 KiB pipe buffer —
  which a long plan does routinely. Python's `communicate()` hid this.
- **A `SIGSTOP`ped run does not time out.** Wall exceeds its budget; effective
  time does not; the run survives and finishes. This is the property the whole
  project exists for.
- A stopped orphan is actually killed — `SIGCONT` before `SIGKILL`,
  unconditionally, because a stopped process never processes a signal it is
  asked to die by.
- A recorded pid that now belongs to a **stranger** is reported and left alive.
  The spec says to reap "from recorded pids"; pids recycle, and on a personal
  workstation the stranger could be the user's editor.

---

## Recorded — what is built, and what is not

Built and gated:

- **The publication layer, whole.** `classify`, coverage, the IPC-5 archive
  scorers, the bounds scorers, the field column, the history rules, and all
  three documents. `crucible standings --check` regenerates
  `benchmarks/ipc-standings.md` and `STANDINGS.md` byte for byte.
- **The instrument.** `benchmarks/manifest.toml`, generated from the five
  registries it consolidates and re-verified against them by
  `crucible/tools/verify-manifest.py`.
- **The corpus walk**, agreeing with `ipc67.py --list` on all 26 tracks,
  variants and instance counts alike.
- **The supervisor.** Spawn, process groups, the effective clock, the RSS
  watchdog, kill escalation, orphan reaping with pid-identity verification.
- **The contention monitor** and the FULL/POLITE/SUSPENDED machine, with game
  detection that follows Steam's descendants rather than a name list.
- **The scheduler**: the resume gate (gated on BLAKE3, not the version string),
  the quiet gate, tiering as ordering-only, the core budget as an admission
  gate, and a board loop whose atom is an instance.
- **The database**, the artifact writers, the promotion gate, the snapshot
  writer and the diff engine.
- **The dashboard**, and a `--dump` that renders one frame off-screen so the
  layout can be reviewed in a transcript or checked in CI.
- **`crucible sweep`**, end to end: manifest, corpus, measurement, artifacts,
  and the `.done` marker written only when nothing is still owed.

NOT built, and the record should say so rather than let it be discovered:

- **The sweep writes artifacts, not the database.** `db/` is implemented and
  tested, and `sched::resume` implements the per-sample window intersection --
  but `SweepRunner::attempt` still judges cleanliness from a before/after sample
  pair and keeps its rows in memory. **So resumption today survives a killed
  BOARD, not a killed PROCESS**, and surviving `kill -9` is the project's whole
  premise. This is the next thing to do, and it is wiring rather than design.
- **`crucible backfill` does not exist.** `repo.rs` carries the engine probe,
  the capability gate and the worktree-naming rule, with tests; nothing drives
  them.
- **The Linux cross-check is not armed on this box.** `preflight.sh` runs
  `cargo check --target x86_64-unknown-linux-gnu` to prove no macOS-only call
  escaped `trait Platform`, and skips with a note because the target is not
  installed. `rustup target add x86_64-unknown-linux-gnu` arms it.
- **The clean-timing pass** collapsed away with the dirty-policy decision and is
  deliberately absent: dirty implies re-run, for every outcome.
