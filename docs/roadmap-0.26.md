# ferroplan 0.26 roadmap — the proof gap, decoded first

Scoped 2026-08-25, with the 0.25 cut sweep in flight, by conversation —
the shape was CHOSEN against three named alternatives, and the decision
trail is part of the record:

- **The question 0.25 left open, in its own words:** "The 0.26 direction
  question — decided at the 0.25 cut with Phase 4's three decodes and
  Wing II's verdict on the table." Both are now on the table, and they
  point in different directions.
- **Wing II is NOT taken for a third cycle.** Two consecutive cycles
  have priced a SAT-wing band and not converted it to board coverage:
  0.24 priced +16–50 and delivered +1/+0; 0.25's wing produced a real
  refund (the conflict-rate bail: match-cellar i1 30.7→17.5 s, i2
  31→1.2 s) and still moved no board. Its own step-5 verdict is a
  MEASURED NEGATIVE for default-on. A third band on the same wing,
  without a new decode, is exactly the purchase the ten-negatives
  ledger was made of.
- **Centerpiece: the proof gap — and the READ COMES FIRST.** 0.25
  nearly tripled the optimal surface. After promotion the proof tracks
  are **669/1,906 (35%)**: seq-opt 287/550, 2018-opt 89/240, 2014-opt
  77/256, 2023-opt 33/140, 2023-numeric-opt 81/400, 2026-opt-full
  80/260, 2026-opt 22/60. That is over a fifth of the table and
  **1,237 unproven instances** — the largest coherent block of missing
  coverage anywhere in the record. The 0.25 roadmap already named it "its
  own future centerpiece candidate".
- **The sharpest framing available, and why it is a MECHANISM question
  rather than a wish:** onlycraft-opt is 2/20 against its OWN 20/20
  satisficing row. Same engine, same instances, same corpus. It finds
  the plans and cannot prove them. barman-opt 0/14 and parking-opt 0/20
  are the same shape. Something specific separates finding from proving
  on these, and it has a name we have not read yet.
- **NO BAND IS PRICED IN THIS PREAMBLE.** Phase 0 prices the centerpiece
  or refuses it. That is the whole point of the shape, and it is the
  0.24 lesson applied: "a priced band delivered +1. This cycle diagnoses
  before it constructs."
- **Co-headline: the instrument.** crucible is most of the way built
  (Phase 5) and its one remaining gap is the one that matters — the
  premise it was built for.

**Addendum, 2026-08-26 — the cycle goes big.** The field-gaps memo
(`docs/field-gaps-0.26.md` — drafted as a 0.27 candidate, adopted whole by
decision the day after scoping) folds into this cycle: the SGPlan ledger,
the modern-satisficing ladder, and the cross-cutting engine gaps, every
gate intact. The expansion phases are recorded below ("The field-gaps
expansion"), the executable specs land in
`docs/field-gaps-execution-0.26.md`, and the 0.26 cut sweep runs on
crucible (F6). Phase 0 keeps its primacy — the proof gap is still read
first — and the expansion sequences behind the 0.25 cut sweep now in
flight: no builds, no probes, while it owns the box.

## Phase 0 — the proof-gap sitting (light, NO code)

A design read, in the 0.25 Phase 4 mould, and under the house rule that
cycle bought twice over: **a design read is a committed artifact, never a
conversation.** Report lands at `benchmarks/metrics/attribution-0.26.md`
whatever it concludes, including if it concludes nothing is claimable.

The three pots, each with an exit clause:

- **onlycraft-opt 2/20 vs its own 20/20 satisficing row.** Read what the
  optimal path does on an instance the satisficing path solves in
  seconds. Is it a node-cap non-proof, an admissibility ceiling on h, or
  a search-order problem? The 0.25 Phase 0 re-check already established
  the sat gain is real and load-survivable and that **opt is a real
  ceiling** — so this asks WHICH ceiling.
- **barman-opt 0/14 and parking-opt 0/20.** parking already has a
  counted-case baseline banked this cycle
  (`benchmarks/air25-entries/parking-opt-i*.json`) — the re-derivation
  starts from receipts rather than from scratch.
- **The CEGAR-seeding question, folded in HERE rather than given its own
  wing.** `FF_SAT_BRANCH=fwd` measured a real **1.8× on the deep
  UNSAT-proof stack** (storage-t h1–32, 6.8 s → 3.7 s, stable across
  reps) and was disqualified only by its SAT-side poison (TMS-2011 i2:
  one refutation and a 0.6 s solve becomes 247 refutations and a capped
  budget). The proof tracks are exactly where the gradient measured
  well and exactly where there is no SAT side to poison. If the sitting
  finds proof-shaped horizons are separable, the residue 0.25 named
  becomes this cycle's lever with a measurement already behind it.

**Exit clause, stated before the read:** if no pot yields a named
mechanism with a priced band, the centerpiece is REFUSED and recorded as
refused, and the cycle's weight moves to Phase 5. A cycle that ships an
honest "no lever found" plus a finished instrument is a better cycle than
one that ships a band it invented to have one.

## Phase 1 — the centerpiece (weight TBD by Phase 0)

Nothing is scoped here until Phase 0 reports. What IS fixed in advance:

- RED fixture first, as always — an instance that today cannot be proven,
  which proves after.
- The band is priced against the field file with the budget gap named,
  and under-delivery gets the 0.24 treatment: measured shortfall,
  hypotheses named, never papered over.
- Anything armed at the sweep ships with its `FF_NO_*` restore. An opt-in
  flag no sweep arms produces no evidence, and no evidence means no pitch.

## Phase 2 — the standing corrections (light, each already named)

- **The mem-cap classification fix.** `ipc67.py:493` emits
  `"mem-cap (self-inflicted: node byte target raised)"`; `standings.py:262`
  matches `"mem-cap"` by EXACT equality, so those rows land in
  `early-exit` — the one column the refill loop is refereed by. Seven
  rows are misfiled in the published table (ipc-standings.md lines 52
  and 61); two more sit in ipc2014-mco-t8. Coverage does not move; the
  attribution does. The cut record carries the −7/+7 movement.
- **The PyPI wheel.** `crates/ferroplan-py` is a pyo3 extension-module
  and the one artifact `publish.sh` does not touch. 0.24 built and
  verified it in pre-flight and never published it; 0.25 is on course to
  repeat that. Close it, or record deliberately that it stays staged.
  (`maturin` is not on PATH: `~/Library/Python/3.9/bin/maturin`.)

## Phase 3 — the two open riddles (light, decode only)

Both are 0.25 probes that came back with the question sharpened rather
than answered, and both are explicitly carried forward:

- **pathways-metric-time is 0/30 AFTER both metric-time bugs were fixed.**
  The zero-duration skip and the [TREL] relevance-mask hole were real,
  the fixtures stay green, and neither was sufficient. Whatever blocks
  pathways specifically is a separate, still-unnamed mechanism.
- **tpp's empty `(:constraints (and))` does not explain its 0/30-vs-3/40
  gap.** i1 is unsolved at 14.2 s — well under the wall, so not a
  timeout. The riddle stays open.

## Phase 4 — transport L1–L3 (medium, and now legitimately unlocked)

The 0.25 anti-pot was "code at the transport wall BEFORE Phase 4's
decode". The decode happened, so the gate is open — and it came with its
own boundary in writing: **+8–20 of 211, on 2008/2011/mco ONLY.** The
2014 sequential boards are explicitly NOT claimable (coverage is monotone
in package count; the engine's line is ~12–14 packages and 2014 carries
25 everywhere).

The L3 probe reported in: with `FF_NO_NOVLIGHT=1 FF_NO_LAMA=1`, i4 solved
in 16.18 s and i6 in 58.98 s against the wall. Both solve — but two
instances is not a lever. **Widen the probe before pricing it.**

## Phase 5 — the instrument gets rebuilt (crucible, co-headline)

Most of this landed while the 0.25 cut sweep was being set up; the
record below is as written then. The ONE remaining gap is the one the
whole thing exists for -- see the closing section.


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

### The sitting, before any Rust (crucible)

#### Recorded — the oracle is rescued, not retired

The Python is **kept permanently** as a differential oracle. `standings.py` is
1,104 lines of pure, stdlib-only, build-step-free code, and it is the only
independent implementation of the failure-class taxonomy that exists. Running
it beside crucible costs about two seconds per cut. Every incident in its own
comment corpus is a case of one implementation drifting from another
*unobserved*; retiring the observer to save Python nobody has to maintain
would be the same trade that produced the incidents.

What is being retired is the **shell driver and the model babysitting it**,
not the measurement code.

#### Recorded — the incident evidence was one disk failure from gone

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

#### Recorded — a live misclassification in the published table

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

#### Recorded — dirty rows are kept, dirty boards are not banked

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

#### Recorded — the resume gate's version check is too weak, and is being closed

`PER-INSTANCE-RETRY.md` names the risk — "a stitched board must never mix rows
from two different `ff` builds" — and then gates on the `ff --version` string,
adding "probably also the git SHA if the binary carries one". It does not.
Every dev build of a cycle reports `ff 0.25.0`, so **two different 0.25.0
builds stitch silently today**.

crucible gates on the binary's **blake3** and keeps writing `ver` into the row
for artifact compatibility. Under the candidate-driven trigger — where the
working-tree binary is rebuilt constantly — this was the likeliest way the new
harness would have produced a chimeric board.

#### Recorded — the sweep environment is scrubbed, and says what it was

`ipc67.py` builds the child environment as `dict(os.environ, ...)`. There are
**132** `FF_*` hatches in the engine. An operator with any one of them exported
in their shell silently changes every board in the sweep, and **nothing in any
row records that it happened.**

crucible starts from a scrubbed environment, injects the budgets, applies the
board's declared `env`, and stores the canonical `env_json` on the board row.
A row can no longer have been measured under a hatch nobody can name.

---

### Where `crucible-spec.md` is wrong

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

### Recorded — what the port actually reproduces

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

#### Recorded — three defects the port found on its way through

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

#### Recorded — the supervisor's properties are tested, not asserted

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

### Recorded — what is built, and what is not

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

### The gap that closes this phase

`SweepRunner::attempt` still holds its rows in memory and judges
cleanliness from a before/after sample pair. `db/` is implemented and
tested and `sched::resume` implements the per-sample window intersection
— they are simply not wired together. **So resumption survives a killed
BOARD and not a killed PROCESS, and surviving `kill -9` is the premise.**
It is wiring, not design, and it is the first thing this phase does.

After it: `crucible backfill` (the engine probe, the capability gate and
the worktree-naming rule already exist with tests; nothing drives them),
and arming the Linux cross-check that keeps `trait Platform` honest
(`rustup target add x86_64-unknown-linux-gnu`).

#### Recorded — the gap is closed (2026-08-28, `4336c35`, merged `8cae317`)

The wiring landed as the dossier specified it (F6 part 1), and the branch
merged to `main` under the finish-in-main agreement with
`crucible/preflight.sh` green on the merged tree: 378 boards agree with
the oracle, 0 mismatch (50,800 rows, 176 boards classified); 55,620 rows
across 186 raws round-trip byte for byte; the manifest agrees with the
registries. Every measured instance is now committed in its own
transaction the moment its run ends — before the verdict, before the
artifacts — and the verdict is `Reader::window_gate`, the per-sample
intersection over the watcher's box-wide timeline, not a before/after
pair. A restarted sweep reads back every row and every clean verdict,
regenerates the stage from them, and owes exactly what never banked.
**The JSONL is an export.**

The three unwired facts the dossier named, closed — and two it did not
name, found on the way:

- Rows are stamped with the engine's BLAKE3 under the resume gate's own
  key. That exposed a second defect: `write_row` was silently DROPPING
  `extra` columns, so the stamp never reached the raw. It keeps them now,
  last, in key order — no committed row changes (the round-trip proves
  it).
- The watcher persists the timeline; `live_child` is written at spawn
  with the KERNEL's identity (the configured path is not canonical on
  Darwin — `/var` is `/private/var` — and a reaper comparing it would
  spare every orphan as a stranger, which the first run of the test did).
- The in-memory clean set was keyed by instance LABEL alone. Every
  multi-variant board carries instance "1" once per variant, so under the
  old key such a board could never reach zero owed. Keyed by the row's
  full address now.
- `requires_version` on a `[[set]]` gates when the CLI flag is absent.

Tested, not asserted: `kill9_resume.rs` — the stamp; the `--no-db` hatch
writes the pre-database shape; a restart over a database holding clean
rows re-spawns ZERO (RED before the wiring: every restart re-measured
everything); and the real thing — a crucible `SIGKILL`ed mid-instance, its
orphaned planner found by the next one, identity-verified, killed, its row
closed. `gate_agreement.rs` holds `window_gate` and `sched::resume::judge`
against each other over every rescued timeline fixture, every window.

Two things the merge surfaced, recorded rather than discovered:

- **The crate's own tests were not hermetic**: a roundtrip test read a
  gitignored `air24/` conditions file, the e2e harness opened the
  operator's real `~/.crucible/db` from four tests at once, and three
  tests pinned counts one cut stale (77 solved on `ipc2014-opt`, the
  release list ending at 0.24.0, the tier-move warning). All now read
  git-tracked fixtures, their own scratch database, or the property
  rather than the number. The manifest is regenerated for the landed
  tier move (`ipc5-time`/`ipc5-metric-time` scored at 60 s again, so the
  manifest carries no warnings). The pre-flight's round-trip loop skips
  `benchmarks/metrics/` — a decode sitting's `matrix.jsonl` carries a
  `solved` key and is not a board.
- **The Linux cross-check (F6 part 3) is BLOCKED on this box, not
  armed.** `rustup target add` succeeds, but the gate's `cargo check`
  builds `libsqlite3-sys` from bundled C source and needs
  `x86_64-linux-gnu-gcc`, which is not installed. The target was removed
  again so the gate reports SKIPPED honestly. Arming it is an operator
  decision: a cross C toolchain (e.g. `messense/macos-cross-toolchains`)
  or a feature gate that keeps SQLite out of the check.

Still open from this phase: `crucible backfill` (part 2), the Linux
cross-check (part 3, as above), and the cut-26 runbook (part 4).

## The field-gaps expansion (adopted 2026-08-26)

The program: `docs/field-gaps-0.26.md`, verified before adoption — every
per-domain number re-derived from the raws, the anti-pot ledger re-read
against every item, feasibility checked against source (the draft's 2014
"+16" died in that verification; what survived is what is priced here).
Specs: `docs/field-gaps-execution-0.26.md`. Order of operations: nothing
below touches the box until the 0.25 cut sweep completes and promotes.

- **F0 — the decode sittings** (committed reports, exit clauses, fixed
  probe budgets): (a) trucks/storage-time — one unnamed mechanism carries
  −49 gross across three SGPlan tracks and +4 flips ipc5-time;
  temporal-relaxation exits pre-excluded (that ledger is closed). (b) the
  cliff decode that must precede any forgetting/multi-heuristic rung
  (rubiks, floor-tile-class, the 2018/2023 residue) — the rung builds only
  on its number, per the standing width rule. (c) metric-time widened to
  rovers i3/i5 — rides Phase 3's sitting. (d) the transport probe
  widening — rides Phase 4; prices the 2008 share of the +8–20 aggregate.
- Specs for every phase below: `docs/field-gaps-execution-0.26.md`
  (assembled + house-law-verified 2026-08-26; its Amendments section is
  binding — including the finding that the transport-L3 receipts are
  ipc-2011 rows, so the 2008 overtake rides entirely on F0(d)'s widening).
- **F1 — fallback enrichment** (ungated): preferred operators + `FF_CLM`
  into the bare wBFS fallback that does most of the solving; RED fixture,
  named `FF_NO_*` restore, old-binary referee; bands +10–17 (ipc5-prop
  tails) and +9-to-median (2018).
- **F2 — YAHSP-style relaxed-plan lookahead** (ungated; never tried in
  this engine): opt-in hatch first; parking-2014's four 59.5–59.9 s
  solves are the fixture class.
- **F3 — the gated builds**, opened only by F0's decodes: the
  `charge_pre_num` temporal hatch (the 0.22 charge-on-temporal negative
  declared; the workshop-economy fixture mandatory; `FF_H_ENDGATE`/`FF_TRPG`
  co-fire declared untested), AIBR/subgoaling numeric h, transport L1–L3,
  the forgetting/multi-heuristic rung.
- **F4 — quality + memory**: quantum-layout anytime polish (existing
  boards only, coverage-neutrality refereed — no new tiers), the
  folding/elevator memory sitting (+3–10 across boards), the storage-tc
  i8–10 fold probe, the floor-tile no-code pricing probe, and the
  model-train plan-then-schedule feasibility read — **executed
  2026-08-26, exit clause FIRED**: the pre-state duration core has existed
  since v0.10; the item is closed and its mass re-routes to F3's
  `charge_pre_num` gate (dossier §3.7).
- **F5 — 2014 config reconciliation**: the hiking agile-ordering
  diagnosis, then the +6-oracle config schedule, old-binary refereed and
  priced after the referee (the true sat∪agile union is 155/280).
- **F6 — crucible sweeps the cut**: Phase 5's named gap closes first (the
  DB wiring — resumption survives a killed PROCESS, the premise), then
  `crucible backfill` and the Linux cross-check; the 0.26 cut sweep runs
  on crucible gated on the byte-parity preconditions (`standings --check`,
  the 314-board classifier agreement, the 6,366-instance enumeration),
  with `standings.py` alongside as the differential oracle, and the
  mem-cap classification fix landing as its own commit AFTER parity is
  proven, carrying the −7/+7 movement in the cut record.

Standing correction already landed with the adoption (2026-08-26):
`docs/ipc-rankings.md`'s constraints row refreshed from the committed raws
— the "12/120, 70 rows rejected" text was two cycles stale; the row now
records 28/120, zero rejects, storage-tc won outright, 2nd-of-3 on the
official subset. Every fence in the memo's §4 carries into this cycle's
anti-pot list by reference.

## Phase 6 — cut 0.26.0

The standing template. What this cycle forces on top:

- **The like-for-like table is now 32 boards, not 22.** 0.25's entry day
  moved the headline from 63% (3,981/6,366) to 56% (4,743/8,444) by
  growing the denominator. From this cycle on that IS the instrument, and
  0.26 is the first cut that can show movement against it.
- The doc gate runs EARLY. The private-intra-doc-link class has struck
  three cycles running; 0.25 finally caught it at authoring. Keep it there.
- Pre-flight is the four-crate order (ferroplan-sat → ferroplan →
  ferroplan-cli → ferroplan-mcp).
- The sweep itself runs on crucible (F6) — the shell drivers stay
  runnable as the fallback, and `standings.py` referees byte-parity.
- If Phase 0 refused the centerpiece, the cut record says so in its first
  paragraph, not its last.

## Anti-pots — priced at zero, standing

Everything 0.25 listed carries forward unchanged: temporal
delete-relaxation (ledger CLOSED), org-synth i11, agricola's coin-flip
class, ricochet, **openstacks-opt PDBs (probe-NEGATIVE, mechanism
named — and directly relevant to Phase 0, which must not re-buy it)**,
a second classical driver swing, temporal orbit-iso, the 1998–2004
corpora, and new 300 s tiers.

Added this cycle:

- **A third SAT-wing band without a new decode.** The wing keeps its
  opt-in hooks and its refunds; what it does not get is another priced
  band on the strength of the last two. Phase 0 may hand it one — that
  is a different thing, and the difference is a measurement.
- **Code at the pathways or tpp walls before Phase 3's decode.** Same
  rule that governed transport, for the same reason, on the same
  evidence.

- **The field-gaps §4 fences, incorporated by reference** (added
  2026-08-26): no code at the pathways/tpp walls pre-decode (rovers
  treated the same by this cycle's own extension), no temporal
  delete-relaxation ever, no STN-shaped model-train revival, no
  2014-transport claims from L1–L3, no un-decoded novelty promotion, no
  anytime-for-coverage, no new tiers.

## Deferred, on the record (carried forward)

- ITSAT-style in-CNF timing; incremental assumptions (only if
  horizon-ramp profiling demands them).
- caldera's selectivity-aware route gate; block-grouping's search residue
  (10 rows, field ceiling proven); the or-aware hoist for folding p01
  (sized, not taken — folding's 300 s face is a MEMORY ceiling).
- floor-tile's irreversible-consumption dead-end test: one NEW lever
  named at 0.25 with a no-code pricing probe attached, unclaimed.
- Cross-mind planning; continuous `#t`; dynamic derived predicates — the
  standing lists.
