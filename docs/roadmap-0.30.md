# ferroplan 0.30 roadmap — the wall that memory throws away, and the price of an evaluation

Scoped 2026-10-15, the day 0.29.0 was published. Order agreed with the
operator, and it is the order the work runs in: **Phase 0 (the instrument and
the owed differentials) → the folding grounder fix → Lane 5 (the lazy
frontier, the headline) → Lane 6 (the price of an evaluation) → Lane 2b
(metric-time beyond rovers) → Lane 1b (points on the complex board) → the
cut.** All 0.30 work lives on `engine-0.30` in its own worktree
(`/Users/harold/ferroplan-0.30`, to be created when Phase 0 opens); every
completed phase pushes to `main` as well as the branch (`CLAUDE.md`).

The cycle's discipline is unchanged: fixtures first; a measured win or a
recorded negative; every claim through the crucible (`crucible-spec.md` R3
subsets for questions smaller than a release, the 32-board sweep for the
cut); the comparative estimator declared before the sweep runs; sweeps at
night, no cargo on the box while one runs. Two lanes are probe-only this
cycle (constraints, seq-opt) and say so below: a probe records, it does not
claim.

---

## The finding this cycle is built on

0.29.0 moved rows on the boards it aimed at (metric-time +14, 2023 numeric
+27) and on boards no lane aimed at (2014 seq-sat +7, the 2014 multi-core
boards +6/+6/+7, the IPC-6/7 seq-sat board +5), and the record could not
attribute the second group. The one engine-wide change in the cycle was the
`widen` inlining, −23 % per evaluation; the unattributed rows are the size
of what a per-evaluation speedup would buy at the wall edge. That is the
first finding: **the engine's speed is a lane, and it moves every board.**

The second is Lane 5's measurement (`probes-0.29/lane5/measure.tsv`):
**6–15 KB per evaluated node** on city-car / child-snack / maintenance
against a per-node byte model of ~200 B, because the best-first search
stores every GENERATED successor's full `State` in the arena at generation
(`search.rs::Node` holds `state: State`). The arena is the frontier times
the branching factor. City-car dies at the runner's 6 GB cap in 20 s with
40 s of wall unspent; so do ~140 rows across the 2014/2018/2023 sat, agile
and numeric boards, and the cut moved the opt boards INTO this class (2018
seq-opt mem-cap 10 → 21, 2026 opt-full 4 → 24, 2023 numeric 33 → 52). The
mechanism was named in 0.29 and not built: a lazy frontier.

The third is the trucks probe (`probes-0.29/trucks/probe.tsv`): storage-
propositional evaluates at **4.5k/s against trucks' 35k/s**, eight times
dearer, on the board where the leader is at 99 % and we are at 86 %. Two
of five probed cells are speed (1.4× and 3.7×), three are lost searches.

And the gap table, re-read on 0.29.0 (`cut29/compare.txt`, `STANDINGS.md`):

| board | 0.29.0 | leader | rows to leader's rate | pool shape |
|---|---|---|---|---|
| IPC-5 propositional | 386/450 | SGPlan5 218/220 | +60 | wall-edge timeouts; storage's evaluations 8× trucks' — Lane 6 |
| 2014 seq-opt | 82/256 | SymBA*-2 151/280 | +56 | admissible-heuristic work; now mem-cap as well as wall — probe only |
| IPC-5 constraints | 28/120 | SGPlan5 47/80 | +42 | a heuristic question; two negatives (0.28, 0.29 Lane 3) — probe only |
| IPC-5 metric-time | 119/200 | SGPlan5 151/200 | +32 | tpp / pathways "ladder stopped at the wall" — Lane 2b |
| complex-pref | 81/108 | SGPlan5 105/108 | +24 | 14 storage mem-cap (Lane 5), 13 tpp wall; points 54.5/100 — Lane 1b |
| 2014 seq-sat | 180/280 | IBaCoP2 198/280 | +18 | mem-cap + near-wall timeouts — Lanes 5, 6 |
| 2018 seq-sat | 99/240 | field mean 94 | — | 24 mem-cap + timeouts — Lanes 5, 6 |
| 2023 seq-sat | 52/140 | — | — | folding 16 rows: the grounder gives up at 0.0 s — the fix |
| preference points | simple 97.3/115.1, qual 64.6/91.4, complex 54.5/100 | SGPlan5 | points | Lane 1b |

Ahead of the leader already: time, tempo-sat 2008/2011, 2008 seq-sat,
net-benefit, 2023 numeric (new this cycle), 2026 numeric-opt. Not touched
unless a lane's loss side lands there.

---

## The two declared measures (pre-registered, before any lane runs)

1. **Rows**: first attempt against first attempt over the 8,444, `crucible
   compare --a 7fdf185b91e6 --b <cut30 hash>`; equal-N beside it; banked
   coverage published as the boards' standing number and never as a delta;
   no delta claimed where the estimators disagree in sign.
2. **Points**: the IPC quality score on the three preference boards against
   SGPlan5's `; MetricValue`, by `ipc-standings.md`'s quality column on ONE
   engine. Lane 1b is judged on this.

Both are read on the cut sweep, and on subsets during the cycle only with the
subset's engine hash named beside the number. Lane 6 adds a THIRD number
that is not a claim but a receipt: **evaluations per second, solo, one
thread**, on named cells, before and after — the lane's mechanism is judged
on it before its rows are.

---

## Phase 0 — the instrument and the owed differentials (blocking)

0.1 **The twelve reproduced losses, attributed.** rovers-metric-time i28,
   pipesworld-metric-time i36, settlers-snp i5, ztalloc-sum i9 and i12,
   sugar i17, driver-log i6, elevator i19 / i20 and transport i18 on mco-t2,
   elevator i19 and transport i18 on mco-t4: each solo, one thread, three
   reps, on 0.29.0 with `FF_NO_CONSUME=1` and without, and on the rebuilt
   0.28.0 (`~/.crucible/worktrees/v0.28.0`, `bb07d50d2015`). Pre-registered
   fork: the hatch restores a cell → the consumption charge has a defect on
   that shape and it is a fixture for Lane 2b; the hatch does not and 0.28.0
   reproduces → the duration compile or the achiever index is next under
   the same table (`FF_NO_NEED_DIRS`, the compile has no hatch — one is
   added first); nothing restores it → the box, recorded. Receipts in
   `probes-0.30/losses29/`.
   **RUNNING 2026-10-15 from 12:49** (`losses29/run.sh`, rows.jsonl; the
   box is NOT idle — Apple's `textunderstandingd` / `mediaanalysisd` and a
   Docker VM hold 150–350 % foreign CPU through the afternoon, recorded on
   every row as `foreign_pcpu`). First cells read:
   - **rovers-metric-time i28 is the consumption charge's loss.** 0.29.0
     unsolved 3/3 at the wall; `FF_NO_CONSUME=1` solved 3/3 at **24.4 s,
     metric 278.03**; 0.28.0 solved 3/3 at 25.8 s, the same 278.03. The
     fork's first branch: the charge has a defect on this shape, and the
     cell is a Lane 2b fixture (the compressed task's search under the
     charge does not reach the plan the uncharged relaxation reaches).
   - **pipesworld-metric-time i36 on 0.29.0 ran PAST the probe's 75 s
     alarm** on its first rep (`solved: crash`, 75.6 s) — a 60 s wall
     overrun of more than 15 s beside ~330 % foreign load: Phase 0.6's
     report-reserve receipt, reproduced on a second shape.
   **READ 2026-10-15 14:50 (`losses29/summary.txt`, all twelve cells, the
   box at a median 62 % foreign CPU and bursts to 1,038 %):**

   | cell | 0.29.0 | `FF_NO_CONSUME=1` | 0.28.0 rebuilt |
   |---|---|---|---|
   | rovers-metric-time i28 | 0/3 | **3/3 @ 24 s** | 3/3 @ 26 s |
   | settlers-snp i5 | 0/3 | **3/3 @ 8 s** | 3/3 @ 8 s |
   | sugar i17 | 0/3 | **3/3 @ 47 s** | 2/3 @ 52 s |
   | ztalloc-sum i9 | 0/3 | **3/3 @ 34 s** | 3/3 @ 34 s |
   | ztalloc-sum i12 | 0/3 | **3/3 @ 3 s** | 3/3 @ 3 s |
   | pipesworld-metric-time i36 | 0/6, every rep past the 75 s alarm | 0/3, alarm | 0/3, alarm |
   | driver-log-temporal i6 | 0/3 | 0/3 | 2/3 @ 22 s |
   | elevator-mco i19 t2 / t4, i20 t2 | 0/3 each | 0/3 each | 3/3 @ 58–60 s each |
   | transport-mco i18 t2 / t4 | 0/3 | 0/3 | 0/3 / 3/3 @ 60 s |

   **The fork's first branch, five times over: the consumption charge is
   the loss on every numeric cell.** Each is restored by the hatch to
   0.28.0's own solve time — ztalloc i12 from unsolved at 60 s to 3 s. The
   charge that bought +27 rows on 2023 numeric takes these five; Lane 2b
   opens on them as fixtures (the question is what the charge does to the
   relaxed plan's cost on these shapes — a plateau or a cliff — not whether
   to keep it). The elevator cells and transport t4 are 0.28.0 solves AT
   the wall (58–60 s): coins, as the record suspected, not a mechanism.
   driver-log i6 (0.28.0 2/3 at 22 s, 0.29.0 never) is a real loss not the
   charge's — temporal, so the duration compile is the next suspect, owed
   a hatch. pipesworld i36 overran a 60 s wall by 15 s on BOTH engines
   under this load: the Phase 0.6 reserve question, not a differential.
0.2 **A solve the referee voids must have COST something.** Three of cut29's
   21 lost cells (pipesworld-prop i50, counters i16, mco-t8 transport i18)
   were solved on a first attempt the referee voided as thermal / swap and
   never reproduced. `sched/referee.rs::judge`'s suspect-solve table (0.29
   Phase 0.1) must not void a solve that spent under a fixed fraction of
   its wall — a 3 s solve under a 1.5× canary is a solve. Fixture: the
   counters-i16 shape (solved in seconds, canary high) banks; the
   pathways-simple shape (solved at the wall, unpriced, canary 3×) is still
   owed. Measured: `recheck29.py`'s population must select the same 24 rows
   minus these three.
   **READ 2026-10-15 — a record, no change.** The premise was wrong: none
   of the three was a seconds-long solve. From the database (engine 23):
   counters i16 solved at **33.5 s** (peak 5.5 GB; its two re-runs
   `mem-cap` at 6.2–6.5 GB, "self-inflicted: node byte target raised"),
   pipesworld-prop i50 at **40.2 s** beside one neighbour, mco-t8 transport
   i18 at **59.5 s**. All three are past `solved_wall_frac` (0.5), so the
   rule did exactly what 0.29 Phase 0.1 wrote: a solve that spent half its
   wall under a bad clock is re-run and the later row banks. They are wall-
   edge (and one memory-edge) coins, not a referee defect. The counters
   shape — a 5.5 GB solve whose refill re-entry raises the node byte target
   past the watchdog — is Lane 5's fixture, not Phase 0's.
0.3 **The canary refuses to calibrate under foreign load.**
   `sweep.rs::calibrate` took its baseline beside Timberborn (2.58 s), a
   folding grounding (1.775 s) and Spotlight (1.27 s) against an idle line
   of 0.520 s; every daytime number in 0.29 paid for it. The throttle's
   foreign-load reading gates calibration: above the configured foreign
   fraction, the sweep WAITS and says so, and a baseline is stamped with
   the foreign load it was taken under. Fixture in `crucible/tests/`: a
   calibration under synthetic foreign load is refused; at idle it is
   taken and stamped.
   **BUILT 2026-10-15 (`6e5bdc0`).** `monitor::calibration_gate` is the
   pure rule (the sample's own clean line, 25 %, so a refused calibration
   and a dirty row never disagree); `sweep_body` samples the box before
   `calibrate` and waits 30 s at a time, naming the loudest competitor;
   the canary table gains `foreign_pcpu` (schema v11), every reading —
   baseline and in-sweep — is stamped with the named-competitor total
   sampled beside it, and `Reader::canary_baseline` leaves stamped-foreign
   readings out of its percentile (unstamped history kept). Fixtures:
   `a_baseline_is_refused_beside_foreign_load` (sample.rs) and
   `a_reading_beside_foreign_load_does_not_set_the_baseline`
   (db_roundtrip.rs). Not measured on a sweep yet: the first night sweep of
   the cycle says whether the wait ever fires and how long it holds.
0.4 **The baseline is reproducible or it is not a baseline.** The 0.28.0
   rebuilt by `backfill --tag` (`bb07d50d2015`) is not byte-identical to the
   board's `89cfdc5f06ed`. Before any engine is rebuilt from a tag again:
   pin the toolchain in the backfill (`rust-toolchain.toml` at the tag, or
   the recorded `rustc -vV` beside the engine hash), and record whether a
   rebuild of `v0.29.0` reproduces `7fdf185b91e6`. If it does not, the
   record says what differs (`cargo --locked`, the profile, the host) and
   the compare's baseline is the BANKED 0.29.0 rows, never a rebuild.
   **READ 2026-10-15 — the 0.28 difference is SOURCE, not the toolchain.**
   The cut28 binary `89cfdc5f06ed` was built at the bump (`310eb22`,
   09-21); the tag `v0.28.0` is `843138b` (09-28), and between them
   `crates/ferroplan/src/tcompress.rs` gained nine lines (`94b067d`, the
   pre-flight's fixture fix: the compression rung declines under
   `FF_TCONC=1`). Gated on a flag no board sets, so the rebuilt
   `bb07d50d2015` is behaviourally the board's engine and byte-different
   by construction. Between the 0.29 bump (`43847be`) and `v0.29.0` only
   `Cargo.lock`'s own version lines and a test file moved, so a rebuild
   from that tag should reproduce `7fdf185b91e6` — verified below once the
   box is free. The rule this writes: **the engine the boards run is built
   from the commit the tag will point at**, or the tag is moved to the
   bump; a pre-flight edit to `crates/` after the sweep's binary is built
   is a different engine, and `backfill --tag` records the tag's commit
   against the board's engine hash so the next reader sees it.
   **REBUILT 2026-10-15 13:15 (`probes-0.30/rebuild29/`, worktree
   `~/.crucible/worktrees/v0.29.0-repro`, commit `67f359b`, rustc 1.99.0,
   `cargo build --release -p ferroplan-cli`, 27 s):** blake3
   `cf01f4b0f2a4…` against the board's `7fdf185b91e6…` — NOT byte-identical,
   and `cmp -l` says why: **47 bytes of 3,250,336 differ**, the 16-byte
   Mach-O `LC_UUID` payload at offset 1,945 and 31 bytes inside the ad-hoc
   code signature at 3,224,976+; same size, 1,645 identical symbol names,
   no build path embedded (`strings`). ld64 derives the UUID from its
   inputs and the signature covers the UUID page, so no rebuild at another
   path can ever match the board's blake3 — and the toolchain DOES
   reproduce the engine. So 0.28's `bb07d50d2015` was the tcompress change
   AND this, and 0.29's rebuild is this alone. **Built:** `repo::Engine`
   carries a `content` hash (blake3 with the UUID payload and the signature
   blob zeroed, plain blake3 for anything that is not a 64-bit Mach-O),
   `crucible engine` prints it beside `blake3`; the rule is **two engines
   are the same engine when their content hashes agree**. The database
   still keys on `blake3` (an identity, not an equivalence); wiring
   `compare`/`backfill` to match on content is owed, not done.
   **Confirmed 2026-10-15 13:45:** `crucible engine` on the board's binary
   and on the rebuild — blake3 `7fdf185b91e6…` / `cf01f4b0f2a4…`, content
   **`c0b1fe63376e…` on both**. The v0.29.0 tag reproduces the 0.29.0
   engine; Phase 0.4 closes with the rule above and the content hash in
   the tool.
0.5 **The owed probe rows, run.** pathways-propositional i17–19 (the trucks
   probe's script asked for one shared domain and pathways ships one per
   instance — fix `probes-0.29/trucks/run.sh`, re-run solo at 60 s and
   300 s); trucks i11 / i13 optima and storage i19 / i20, three reps solo
   each, so the wall-edge coins are read as coins or as cells. Record only;
   they feed Lanes 5 and 6.
   **Re-read 2026-10-15:** the two names in the owed list are not probe
   rows. "trucks i11/i13 optima" are simple-preferences POINTS cells
   (0.29 Lane 1: metric 2 and 6 against SGPlan5's 0, the optimizer short
   of the 12 s the first polish takes) — Lane 1b's. "storage i19/i20" are
   `storage-preferences-complex` `mem-cap` rows at **2.3–3.8 s with
   6.2–6.5 GB** on engine 23 — the compiled preference task's grounding,
   Lane 5's grounding class. Only the pathways-propositional rows remain
   a probe here (`probes-0.30/pathways/run.sh`, per-instance domains).
   **RUN 2026-10-15 (`pathways/probe.tsv`, solo, 60 s ×3 and 300 s ×1):**
   i17 unsolved at 0.5–0.64 M evaluations in 60 s and **5.09 M in 300 s**;
   i18 unsolved at 1.0–1.7 M and 7.95 M; i19 unsolved at 0.3–0.47 M and
   6.66 M. All three are **lost searches**, not wall-edge speed: five
   minutes and 5–8 M evaluations do not reach a plan. They join the trucks
   i15/i16 and storage i27 note — a heuristic question for the
   propositional board, and evidence that Lane 6's speed buys pathways
   nothing by itself (10–27k evaluations/s here already).
0.6 **Small and owed:** the probe scripts read `statistics.evaluated_states`
   (Lane 5's probe wrote `None` for every row); the report reserve on a
   loaded box (pathways-simple i16 returned at 81 s past a 60 s wall when
   the grounding checkpoint expired mid-enumeration) gets a fixture that
   pins the return inside the wall plus reserve, loaded or not; the
   `--max-passes` subset fixture from 0.29 Phase 0.5 is confirmed to still
   hold on the cut29 database.

   **0.6, READ 2026-10-15:** the probe scripts of this cycle read
   `statistics.evaluated_states` (and `plan.makespan` / `plan.cost`);
   `crucible/tests/subset.rs` (the pass cap on a subset) passed in today's
   suite (6/6); the report reserve on a loaded box is OPEN with three
   receipts now — pathways-simple i16 at 81 s (0.29), pipesworld-metric-
   time i36 at 75+ s on both engines, folding i3 at 68.7 s solo — all
   three a grounding whose clock stride (8,192 bindings) is long when each
   binding is a wide product; the fixture owed is a grounding that must
   return inside wall + reserve however slow the box, which means a
   time-based stride, not a count. It joins Lane 6 (the enumeration's
   cost is Lane 6's question too) rather than holding Phase 0.

**Phase 0 closed 2026-10-15** (0.1 read, 0.2 record, 0.3 built, 0.4 read
and built, 0.5 run, 0.6 read with one item handed to Lane 6); the crucible
changes are on `engine-0.30` (`6e5bdc0`, `c193948`), and `main` was
fast-forwarded to `493e70f` at 15:50 with the engine suite green (the one
red, `ehc_extend`'s timing fixture, passed alone a minute later — the
0.29 ledger's loaded-box shape, mediaanalysisd at 246 %).

Phase 0 ships as a crucible release on `main` before the folding fix
opens; nothing in it changes a board number except by measuring it again.

---

## The folding grounder fix — a defect, before any lane

**What is known.** folding i3 under `FF_TIME_LIMIT=20` returns "wall budget
exhausted during binding enumeration" in 0.0 s with the whole wall unspent
(`probes-0.29/lane5/measure.tsv`); the 16 folding `mem-cap` rows on 2023
seq-sat trip at 7 s on the board. `ground.rs::GroundWall` reports the wall
clause when `call_stop_reason()` is `None` and the clock has gone — so
either the clock it reads is already past at entry (a deadline inherited
from a scope that has not been reset), or the per-call budget fires at its
first look. The latch in `mem.rs` ("this scope has tripped once already:
whatever it opens next trips at its first look") is the first suspect: a
memory trip in a previous rung's grounding latches the scope and the next
grounding trips at zero.

**Claim:** folding's 16 cells reach the search with the wall unspent.
**Band:** 16 of 16 ground (receipt: the grounding's `FF_RES_DEBUG` line and
peak RSS); rows are NOT banded — folding may still not solve. **Kill:** the
grounding completes and exhausts the memory budget honestly → recorded as
the grounding class, joins Lane 5's caldera / organic-synthesis note.
**Fixture:** a task whose first rung trips the memory wall and whose second
rung grounds afresh must ground, not trip at its first look — the latch is
per-rung-scope or it is a defect. **Measured:** `sweep --board ipc2023-sat
--only folding`, `max_rss` beside every row.

**READ 2026-10-15 — there is no grounder defect; the 0.29 probe's path was
wrong.** `probes-0.29/lane5/measure.sh` named the cell
`folding-sequential-satisficing/3`; the corpus directory is
`ipc-2023/domains/folding-satisficing` (one shared `domain.pddl`). `ff`
failed on a missing file: the receipt is peak RSS **2 MB, 0.0 s, "no
json"** on both caps — not a grounding, not a trip. On the board itself
(engine 23, `ipc2023-sat`) folding i2–i13 read `mem-cap` at **6.1–7.4 s with
6.2–6.4 GB** peak on the quiet pass, and 2.6–4.2 GB at 60 s on the demoted
passes — the memory grows with the speed of the search, ~900 MB/s on a
healthy core. folding is the memory class, Lane 5's, and the question it
carries there is whether that growth is the node store or the grounding;
answered by one solo run under `FF_WALL_DEBUG` / `FF_RES_DEBUG` at
`--max-evaluated 1` and at 200k once the box is free. The "fix" closes
as a probe-path defect; the 0.29 record's sentence about `GroundWall`
tripping at once is withdrawn.

**MEASURED 2026-10-15 13:20 (`probes-0.30/lane5/`, solo, the board's env,
`--max-evaluated 1`):** folding i3 is **the grounding, and the grounding
never finishes**: peak RSS **6,112 MB**, `68.74 s real` against a 60 s
wall — "grounding checkpoint expired mid-enumeration (no task, no
verdict)", 0 facts, 0 actions. The binding enumeration of an `:adl`
domain (five actions, one over `?n1 - node ?n1x ?n1y - coord ?n1dir -
direction`, 65 objects) allocates to the runner's cap in 7 s on the board
and to 6.1 GB solo, and the measured memory wall never looked, because
`MemWall::arm` is gated on bounded work and a first grounding is not.
Two things for the ledger: (a) the enumeration overran the wall by 8.7 s
solo (the 8192-binding clock stride is long when each binding is a wide
ADL product — Phase 0.6's overrun, a third shape); (b) folding joins
caldera, organic-synthesis and storage-preferences-complex i19/i20 as
**the grounding class** — a grounder that cannot finish the task inside
the budget, ~50 rows across 2023 sat/agile, 2018 sat and complex-pref.
That class is not Lane 5's node store and not Lane 6's evaluation; it is
a candidate lane of its own ("the grounding that never finishes"), named
here and NOT opened this cycle unless Lanes 5 and 6 close early. The
cheap honest step is Lane 5's mechanism 1 anyway: an armed wall ends the
enumeration at 4.5 GB in ~5 s with the note, instead of the SIGKILL.

---

## Lane 5 (headline) — the lazy frontier

**Claim:** the ~140 mem-cap rows on 2014/2018/2023 seq-sat, 2014 agile and
2023 numeric (and the opt boards' new mem-cap rows) spend their wall
instead of dying at the runner's cap at 19–27 s. **Band:** +15 to +30 rows
across the three seq-sat boards on the first-attempt estimator, no row
lost; bytes per evaluated node on city-car i6 under 1 KB (from 6–15 KB).
**Kill:** bytes per node falls and the rows still end unsolved with the wall
spent → the search, not the store, is the limit; recorded, and the note says
what the wall was spent on.

**The mechanism, in order:**

1. **Arm the memory wall whenever a budget is DECLARED.** `MemWall::arm`
   is gated on `search::bounded_work()`, so a plain classical solve runs
   unarmed and dies in weighted best-first at the runner's SIGKILL.
   Declared `FF_MEM_BUDGET_GB` (or `Options`' budget) arms it, so the rung
   ends at the trip fraction with a "memory wall" note and the ladder's
   next rung gets the rest of the wall with a bounded closed set (EHC /
   novelty). Fixture on the `tests/mem_wall.rs` pattern: a classical task
   under a small budget ends "memory wall", the next rung runs, the process
   is never killed. This step alone may bank rows and is measured alone.
2. **The lazy frontier.** An open node holds (father, op, key-hash, g, h)
   and regenerates its state on pop from its father's state in the closed
   arena; the exact duplicate check stays against closed states, a hash
   against open ones (a hash collision on the open side costs a
   regeneration, never correctness). `Node` sheds `state: State` and
   `lm_acc` moves to the closed side. Byte model re-derived and measured
   on city-car i6, child-snack i10, maintenance i4 before any row is
   claimed; evaluations per second beside it, since a regeneration is an
   `apply` that the eager store did not pay.
3. **The closed set packed** (states once in an arena, 64-bit hashes with
   a collision list, `father` / `op` as `u32`) — only if step 2 leaves the
   band unmet and the measurement says the closed side is now the peak.

**Measured:** `sweep --board ipc2014-sat --only city-car,child-snack,
maintenance`, `--board ipc2018-sat --only caldera,organic-synthesis,snake`,
`--board ipc2023-sat --only folding`, then the loss side of all three
boards and of `ipc2014-agile` / `ipc2023-numeric`, `max_rss` beside every
row, `compare` on the first-attempt estimator. The grounding class
(caldera 5.9 GB in 44 s, organic-synthesis 3.6 GB still grounding at 120 s)
is NOT this lane: record only, unless the profile in Lane 6 shows the
packed-op tables are the peak.

**The subset sweep LAUNCHED 2026-10-15 16:05** (`lane5/sweep.sh`, engine
`9df92e323f0a` = `493e70f`'s build, `--only` city-car|child-snack|
maintenance|caldera|organic-synthesis|snake|folding on the 2014/2018/2023
sat boards; the regex also takes the `-split` variants). Its first launch
at 15:40 sat at the new calibration gate for sixteen 30 s rounds
(mediaanalysisd, spotlightknowledged at 136–387 %) — the gate's first
night, working. **Instrument note:** for its first five minutes the sweep
ran beside a probe chain of mine (ztalloc traces, a partition-mode check)
whose `ff` the box sample excludes by name; packed solves in that window
are solves, but caldera i5 (16:09) and i6 (16:10) banked `rho` unsolved
beside that load and are re-run before the lane's read. The chain was
stopped by PID at 16:10; nothing else runs beside the sweep.

### Lane 5 — MEASURED 2026-10-15 18:12 (`lane5/compare.txt`, `memory.txt`)

**SWEEP COMPLETE, 180 cells banked in four passes** (16:05–18:10, the box
at 0–460 % foreign load, 12 % of B's runs clean against 42 % of A's — B
measured on the louder afternoon). `crucible compare --a 7fdf185b91e6 --b
9df92e323f0a` on the subset:

| estimator | 0.29.0 | 0.30 | delta | 95 % CI |
|---|---:|---:|---:|---|
| first attempt (the claim) | 55 | 54 | **−1** | [−6, +3] |
| banked | 56 | 55 | −1 | [−5, +2] |
| any | 56 | 57 | +1 | [0, +3] |
| equal-N | 55 | 54 | −1 | [−6, +3] |

1 gained (child-snack i?), 2 lost: **caldera i3 and i4** — 0.29.0 solves at
58.9 s on the grounding class's edge (i3's first 0.30 attempt solved at
58.87 s and was voided `swap`); re-run solo on both engines before they
are named (`lane5-lost`). **The band (+15 to +30) is not met; the kill
clause holds: the rows end unsolved with the wall spent**, and the note
says what the wall was spent on — the best-first beneath the rungs runs
its 35–45 s on what the rungs leave and reaches no plan on these cells.

**The memory half, banked rows, satisficing boards (`memory.txt`):**

| variant | solved 0.29.0 → 0.30 | `mem-cap` rows | mean peak RSS |
|---|---|---|---|
| caldera | 6 → 4 (the two coins above) | **14 → 3** | 5.4 → 3.7 GB |
| caldera-split | 3 → 3 | 0 → 0 | 3.5 → 1.7 GB |
| child-snack | 8 → **9** | 0 → 0 | 4.3 → 2.9 GB |
| city-car | 4 → 4 | 0 → 0 | 4.8 → 3.4 GB |
| folding | 1 → 1 | 17 → 14 | 5.8 → 5.6 GB (the grounding) |
| maintenance | 16 → 16 | 4 → 2 | 1.3 → 1.2 GB |
| organic-synthesis | 4 → 4 | **10 → 5** | 4.2 → 3.5 GB |
| organic-synthesis-split | 8 → 8 | 0 → 0 | 2.6 → 2.2 GB |
| snake | 6 → 6 | 0 → 0 | 2.6 → 2.2 GB |

`mem-cap` rows on the subset **45 → 24**; mean peak RSS down 20–50 % on
every search-class variant.

**The two lost cells, read 2026-10-15 18:15:** caldera i3 and i4 both
SOLVED on the 0.30 engine (58.9 s and 39.6 s, 5.6 GB) on attempts the
referee voided as `swap`, and their re-runs were **`mem-cap` kills at 19 s
and 6.17 GB**; 0.29.0 solved both at 38.8 s. The cause is mechanism 1 on
the grounding class: caldera's grounding alone sits at 5.9 GB of 6, so an
armed rung trips at its FIRST look (the resident set is already past the
4.5 GB line), hands straight down, and the unwalled best-first grows to
the cap — where 0.29.0's rungs ran on the sliver the grounding left and
solved. **Revised (`MemWall::arm_rung`):** a rung entered above the line
is not armed — above the line there is no headroom to spend honestly and
the rung is the ladder's only chance; it runs as it did before, under the
runner's cap. Fixture arm `a_rung_entered_above_the_line_runs_unwalled`
(the budget declared below the child's baseline: no checkpoint, LAMA runs
to its own cap). Measured next on caldera i3 solo and the caldera cells
through the crucible on the revised engine.
**caldera i3 solo on the revised engine (`b2769c241a16`, 18:19):** solved
— **by EHC in 27 evaluations after a 38 s grounding, peak 6.49 GB**, the
bounded rungs SKIPPED as unaffordable (36 % of the wall left). So the
rungs were never the cause on this cell: the 19 s `mem-cap` re-runs were
the grounding itself peaking past the runner's cap, and 0.29.0's 38.8 s
solves lived at the same edge (its caldera peaks 6.2–6.5 GB). caldera i3
and i4 are **memory coins of the grounding class**, on both engines; the
`arm_rung` revision stands as the right behaviour for a rung entered above
the line, not as the fix for these two. The caldera-only sweep on the
revised engine says whether the board agrees.

**The caldera-only sweep on the revised engine (`b2769c241a16`, 18:19–18:33,
20 cells, two passes; `compare-caldera.txt`): 6 = 6, +0 [−4, +4]** — i5
and i14 GAINED (5.9–6.0 GB solves), i3 and i4 lost again (`mem-cap` at
6.19 GB). Caldera's twenty cells run at 5.1–6.4 GB peak on both engines;
which four of them cross the runner's cap on a given pass is a coin, and
the lane does not claim or concede them. The subset's reading with
caldera flat is child-snack's +1 and nothing else: **+1 of 180, inside
every interval**.

**The lane's verdict: a measured win on memory, a recorded negative on
rows** — the band (+15 to +30) is not met; `mem-cap` rows fall 45 → 24
and peak RSS 20–50 % on the search class, and the cells that were
memory-bound are now search-bound and unsolved at the wall. The engine
changes stay.

**A loss the subset could not see, found 2026-10-16 10:45 through Lane
2b's probe:** rovers-metric-time i28, uncharged (`FF_NO_CONSUME=1`), solo
×3 at 60 s — **0.29.0 board binary 3/3 at 24 s; the Lane 5 engine
(`c7fd313`, `~/.crucible/worktrees/lane5-final`) 0/3; the Lane 2b engine
0/3.** A temporal cell: the compressed task's search runs the same
classical ladder, and the lazy frontiers (duplicate detection by hash at
generation, a different tie order among equal keys) or the rung wall
changed where it goes. Bisected next across `de90ccb` (driver lazy + the
rung wall) and `493e70f` (LAMA lazy), with `FF_NO_MEM_WALL` and
`FF_NO_NOVELTY` as the hatches on the first failing one; the cut's
compare on the metric-time board is where this is finally counted, and
the lane's "no row lost" clause now has one candidate against it.
**Bisected 10:45 (`~/.crucible/worktrees/bisect-*`):** `de90ccb` (the
driver's lazy frontier and the rung wall) **solves** rovers i28 uncharged;
`493e70f` (LAMA's lazy frontier) **does not** — the loss is LAMA's lazy
frontier. One mechanism is visible in the code before any trace: the
lazy LAMA's node cap counts `nodes.len() + norm_heap.len()`, and the
normal heap keeps a stale copy of every entry the preferred heap already
closed, so the modelled cap trips EARLIER than the eager shape's exact
count did and LAMA hands down sooner; the other is tie order among equal
keys (`(key, father, op)` against `(key, index)`). Read next with
`FF_WALL_DEBUG` on both builds — which rung solved on `de90ccb`, and
where the lazy LAMA ends — and fixed before the numeric sweep runs,
since that sweep reads the engine as a whole. The sweep is held at its
gate meanwhile (nothing banked).

**Read 10:45–11:10.** Not the cap: with `FF_SEARCH_NODE_CAP=0` the lazy
LAMA stalls the same way (key 44 → 42 → 36 → 34 across its windows where
the eager one goes 42 → 16 → 10 and solves at 7.7 s), and it leaves by the
per-call wall, silently. Not the order: every tie-break path traced is
equivalent (`applicable_ops` is ascending by op index; a father's closed
index follows its pop order). **The hash.** Instrumented, the lazy LAMA
on rovers i28 reports **3 hash-only drops against a DIFFERENT closed
state** among ~865,000 keys (15,049 evaluations, 849,771 held) — a
collision rate ~10⁵ above a random 64-bit hash's. `state_key_hash` is
FxHash — rotate, xor, multiply, no avalanche — and was always a ROUTER:
the eager frontiers verified `state_key_eq` behind it. The lazy frontiers
trusted it, and on rovers' structured fluent vectors (energies quantized
×10⁶) it collides; the open side, which cannot be checked, dropped
distinct successors on the path. **Fixed:** `PackedTask::state_key_hash128`
— the same content through two independently seeded passes, each
finished with murmur3's fmix64 — keys the lazy LAMA and the lazy driver;
the eager router keeps its 64 bits. Eight bytes more per open entry.
Measured next: rovers i28 both arms, the Lane 5 fixtures, city-car's peak
on the 128-bit keys; then the held numeric sweep runs on this engine.
**Measured 11:20:** rovers i28 uncharged **2/3 at 26–27 s with 0 false
drops** (the miss: LAMA at 16,411 evaluations when the per-call wall
fell — its windows are wall-time, so a loaded moment moves a 26 s solve
past its slice; a wall-edge coin now, where it was 0/3 with collisions);
charged unsolved (the charge's own rovers shape, open). City-car i6 peak
**2,646 MB** (from 3,525 — the collision-free dedup holds fewer
entries). `rung_mem_wall` read red on its `peak < budget` arm: at fixture
scale the 24-byte-entry key map doubles in one allocation larger than a
16 MB headroom's 25 % allowance; the fixture now trips at half the
budget (`FF_MEM_TRIP_FRAC=0.5`, the engine's own knob) so one doubling
fits between the line and the budget, as 1.5 GB of 6 does on a board.
The bisect worktrees are removed. Owed from here: the bounded open list for the bounded rungs
(the search question the lane uncovered), and the grounding class as its
own lane — folding, caldera, organic-synthesis — where the memory is
spent before any search begins. What the record names for the
rows: (a) the cells are not memory-bound once the rungs are honest — they
are search-bound, and the complete rung beneath does not solve them in
the wall it is handed; (b) a bounded open list for the bounded rungs
(the lever named under LAMA's build) would let the rungs run their whole
slice without handing down at all, which is the next search question;
(c) the grounding class (folding, caldera, organic-synthesis) does not
move and was never this lane's. The engine changes stay (every fixture
green, the suite green, the peak memory lower on every search cell); the
claim on rows is withdrawn for this cycle.

### Lane 5 — day one, re-measured 2026-10-15 (`probes-0.30/lane5/`)

Solo, one thread, the board's env (`FF_TIME_LIMIT=60`, `FF_MEM_BUDGET_GB=6`),
`/usr/bin/time -l`, engine `7fdf185b91e6`:

| cell | cap 1 (the grounding) | the board's run | reading |
|---|---|---|---|
| folding i3 | **6,112 MB, 68.7 s, no task** | 6,152 MB, no task | the grounding class (recorded under the "fix" above) |
| city-car i6 | 30 MB (696 facts, 3,058 ops) | **5,784 MB**, unsolved; novelty-light 6.1 s (86k pops), LAMA 13.3 s (276,909 evals), best-first 38,548 evals to the wall, `[h] build 9.7 s` | the search — but NOT "6–15 KB per node" |

The 0.29 figure was read at `--max-evaluated 200000` on a different route;
on the board's route city-car's states are eleven words (88 B), the
per-node model says ~220 B, and the run peaks at 5.8 GB after 38,548
best-first evaluations — **~150 KB per evaluated node** if the best-first
holds it, which no state store explains. Either an earlier rung (LAMA's
276,909 evaluations, or the novelty driver, which printed nothing) holds
the peak and never gives it back, or the best-first's arena is the
frontier at a very wide branching factor. The next read is per rung with
an RSS timeline (`measure2.sh`: best-first alone, LAMA holding the wall,
novelty alone); the mechanism is chosen from that, not from the 0.29
sentence. The "lazy frontier" stays the candidate only if the best-first
arm reproduces the peak.

**Per rung, MEASURED 2026-10-15 13:40 (`measure2.sh`, RSS sampled every
second beside `FF_WALL_DEBUG`):**

| arm | peak RSS | evaluations | the timeline |
|---|---:|---:|---|
| best-first alone (`FF_NO_NOVLIGHT FF_NO_LAMA FF_NO_NOVELTY`) | **905 MB** | 89,502 | 170 MB at 5 s, 437 at 21 s, 874 at 56 s — ~15 MB/s |
| LAMA holding the wall (`FF_LAMA_WALL_FRAC=0.9`, no novelty-light) | **5,892 MB** | 44,994 (the best-first after it) | **1,982 MB at 5 s, 3,536 at 10 s, 5,892 at 20 s**; the best-first that follows runs at 2.5–4 GB on what the allocator kept |
| the ladder with novelty first (`FF_NOVELTY_ONLY=1`) | 5,192 MB | 21,444 | 1,846 MB at 5 s, 4,341 at 15 s, 5,192 at 35 s |

**The memory hog on city-car is the LAMA rung** (and the novelty rungs),
not the best-first fallback: LAMA allocates ~400 MB/s and reaches the
runner's cap at 20 s — the board's death time — while the complete rung
beneath it would have run the whole wall in under a gigabyte. The 0.29
sentence named the wrong rung, because it was read at `--max-evaluated
200000` with the refill disarmed. And no bounded rung reads the memory
wall: `MemWall::arm` is called by the grounder, `search_from` and the
temporal ladder only; `lama.rs` and `novelty.rs` have no kernel read at
all, and the wall is unarmed outside bounded work anyway.

**The mechanism, re-ordered:** (1) **the bounded rungs ARE bounded work**
— LAMA, novelty-light and novelty are bets with a complete rung beneath
them, which is what `mem.rs`'s "bounded work only" rule was written for;
each gets the kernel-read checkpoint at its batch boundary, armed when a
budget is declared, and ends with a "memory wall" note handing the ladder
down with the wall it did not spend. Fixture: city-car's shape at fixture
scale under a small `FF_MEM_BUDGET_GB` — LAMA ends on the wall, the
best-first runs, peak RSS stays under the budget, the process is never
killed. Measured first on city-car i6 solo (peak RSS and whether the
best-first's 60 s is bought), then the subset sweep. (2) LAMA's node
store itself — `lama::Node { accepted: Vec<u64>, .. }` per GENERATED
successor — gets the lazy frontier if (1) leaves the band unmet. (3) The
best-first's lazy frontier moves to third: 15 MB/s is not what kills the
rows.

**Mechanism 1 BUILT 2026-10-15** (`mem::MemWall::arm_rung`; LAMA, novelty,
novelty-light and the novelty driver read it at their batch or every 256
pops; fixture `tests/rung_mem_wall.rs`: LAMA on a visit-all grid in a child
process whose budget is derived from its own post-grounding RSS — walled,
"LAMA MEMORY checkpoint" with the peak under the budget; hatched, the peak
over it; 2.2 s release). **First read, city-car i6 solo on the built engine
(`measure3.sh`):** novelty-light 6.3 s, LAMA slice 13.3 s at ~3.4 GB (under
the line), then **"novelty-driver MEMORY checkpoint at 29,952 pops,
3,880,241 nodes, handing down"** — ~130 stored successors per pop — and the
best-first ran 50,438 evaluations to the wall; **peak 4,836 MB**, the board
would not have killed it. Still unsolved: the best-first had 35 s on the
3–4.8 GB the allocator kept. So (1) turns a SIGKILL at 20 s into an honest
unsolved row at 60 s on this cell; whether it turns any `mem-cap` row into a
solve is the subset sweep's question (the cells whose novelty rung trips
early get most of the wall back). The 3.88 M-node driver store is (2)'s
first target, ahead of LAMA's.

**Mechanism 2, the driver's lazy frontier — BUILT 2026-10-15** (`novelty::
search_driver`: an open entry is `(key, father, op)` on the heap plus a
state-key hash in a set, the state regenerated from its father's on pop;
duplicate detection by the 64-bit key at generation where the eager shape
compared stored states; the node cap counts open + closed as before).
**City-car i6 solo on the built engine (`measure3.sh`, `citycar6-lazydriver`):**
the driver ran to its NODE cap — **17,179,987 held nodes, 179,622 closed**
— against the eager shape's 3,880,241 at the memory wall after 29,952
pops: six times the pops and 4.4× the frontier inside ~0.8 GB of growth
over LAMA's residue (RSS 3.4 → 4.2 GB across the slice; ~47 B per open
entry against ~1.2 KB). Peak **4,357 MB** (from 4,836); no "MEMORY
checkpoint" line any more; still unsolved, the best-first 44,026
evaluations after it. **The peak is LAMA's now** (3.2 GB at 10 s, 3.4 GB
when its slice ends at 13 s), and LAMA's `Node { state, accepted }` per
generated successor is the next target — the same change, with the
batch pop regenerating its batch. Fixture `tests/lazy_frontier.rs`: k
agents on a ring whose goal reverses their cyclic order (impossible,
invisible to the relaxation, C(40, 8) states), the bytes the child
process grows by per held node under 160.

**LAMA's lazy frontier — BUILT 2026-10-15** (`lama::search_subgoal`: the
same shape; both heaps carry `(key, father, op)`, a key map marks a hash
open or closed so the preferred heap's copy of an entry the normal heap
already closed is skipped as the eager `expanded` flag skipped it; the
node cap counts closed + the normal heap). **City-car i6 solo
(`citycar6-lazylama`): peak 3,525 MB** — 5,784 → 4,836 → 4,357 → 3,525
across the four Lane 5 steps — LAMA's slice unchanged (268,205
evaluations at 13.3 s), the driver at its node cap as before, the
best-first 32,808 evaluations after; unsolved. What is left is the COUNT:
~130 successors per LAMA evaluation make ~35 M open entries at ~64 B
(two heap tuples where the op was helpful, a key-map slot) — 2.2 GB by
6 s. A cheaper entry (one heap, a packed key set) buys at most 2×; the
next real lever on this cell would be a bounded open list for the
bounded rungs (keep the best N entries, drop the tail — LAMA is a bet
with a complete rung beneath it), and that is a search change to be
measured on rows, not a store change. **Named, not built.** Two fixture
notes: `rung_mem_wall` had to lift the MODELLED node cap
(`FF_SEARCH_NODE_CAP=0`) because at ~50 B an entry the model's ~200 B
charge trips first — the model is conservative now, which is the right
side to be wrong on; and the rung-wall fixture moved to the ring task
when the lazy LAMA stopped outgrowing a 32 MB headroom on the grid.

---

## Lane 6 — the price of an evaluation

**Claim:** an evaluation on storage / pathways / tpp costs what it costs on
trucks, or the record says exactly which table it spends the difference in.
**Band:** evaluations per second solo on storage-propositional i27 / i28
at least 2× (from 4.5k/s); IPC-5 propositional +10 to +25 rows on the
first-attempt estimator; no board loses net. **Kill:** the profile says the
cost is the task's (branching factor, arity, conditional-effect count) and
no table change moves evals/s by a fifth → recorded with the profile; the
three lost-search cells go to the heuristic note.

**The questions, in order:** (1) where an evaluation goes on storage i28
against trucks i10 (`PROFILING.md`'s method; samples in `probes-0.30/
lane6/`) — the RPG build, the achiever index, the helpful-action
extraction, `State` cloning, the orbit / novelty bookkeeping; (2) the two
speed cells (trucks i10 at 1.4×, storage i28 at 3.7×) as the fixtures:
each must solve at 60 s solo after the change; (3) the engine-wide
receipt — evaluations per second on a fixed six-cell panel (one per
board family: a 2008 seq-sat cell, a 2014 cell, a 2018 cell, a numeric
cell, a temporal cell, a preference cell) measured before and after every
change in this lane, so a speedup on storage that slows blocks is seen
the day it lands. **Measured:** `sweep --board ipc5-prop --only
trucks,storage,pathways`, then the loss side of `ipc5-prop`, `ipc2014-sat`,
`ipc2018-sat`; the panel's evals/s beside every row claim. The `widen`
precedent (−23 %/evaluation, ~+20 unattributed rows) is the prior; this
lane names what it buys this time.

### Lane 6 — the first profile, 2026-10-15 (`probes-0.30/lane6/`)

Solo, one thread, the 0.30 engine (`493e70f`), the box at 250–400 %
foreign CPU all afternoon (every absolute number below is depressed; the
ratios are the reading). Evaluations in 30 s on the release binary:
**storage-propositional i28 70,633 · trucks i10 880,375 · pathways i18
154,195** — trucks evaluates 12.5× faster than storage, 5.7× faster than
pathways. The headless hotspot list (`benchmarks/profile.py`, the
profiling build under samply, 40 s): on storage i28 **~75 % of self-time
is `heuristic::build_rpg`** (a dozen address ranges inside it, 15.2 %
the largest), the candidate sort 2.6 %, `achiever` ~3 %, orbit sigma
0.5 %. The relaxed planning graph IS the evaluation on storage; the
question the lane now asks is what in storage's grounded task makes one
layer-by-layer build cost 12× trucks' (ops × preconditions per layer, the
conditional-effect expansion, or a fixpoint that runs to the cap) — read
from the two tasks' shapes (`grounded_actions`, preconditions per op,
layers to the goal) before any code moves.

**The shapes, read 2026-10-15 18:30:** storage i28 grounds to **1,760 facts
and 19,540 ops** (five schemas, ten negated preconditions); trucks i10 to
656 facts and 3,168 ops (four schemas, two `forall`s); pathways i18 to 772
facts and 1,396 ops (29 schemas, 24 disjunctions). Storage's evaluation
costs 12.5× trucks' because its relaxed planning graph is 6.2× the ops and
2.7× the facts per layer — the size of the grounded task, not a defect in
the build. (The 0.29 trucks probe also found the three remaining
propositional cells, trucks i15/i16 and storage i27, to be lost searches;
today's pathways i17–19 are the same, at 5–8 M evaluations.) So the lane's
mechanism is generic speed of `build_rpg` per op — a layer that visits only
the ops whose preconditions changed (the incremental RPG, Metric-FF's own
shape) rather than every op every layer — and its measure is evaluations
per second on the six-cell panel. And one more reading from the profile:
`profile.py` runs `ff` in its DEFAULT mode, partition-and-resolve, and in
that mode **trucks i10 and pathways i18 printed plans inside 40 s** where
the auto ladder reaches none in 60 s. The release engine in auto mode is
unsolved on both at 60 s (1.74 M and 1.68 M evaluations); a direct
`--mode partition` check at 60 s was interrupted by the sweep and is owed
— if it holds, the propositional board's lever is the route, not the
speed.
**Checked 2026-10-15 18:38 (`--mode partition`, 60 s solo, engine
`b2769c241a16`):** trucks i10 unsolved (58 s), pathways i18 unsolved (60 s)
— the 40 s "plans" in the profile were the profiling run's own luck or its
shorter wall reading differently — but **pathways i17 SOLVED at 35 s, plan
length 172**, a cell the auto ladder never solves (0.5 M evaluations in
60 s, 5.1 M in 300 s this afternoon). The route moves a row the search
cannot. Probe owed and launched tonight: every unsolved cell of the IPC-5
propositional board under `--mode partition` at 60 s solo
(`probes-0.30/lane6/partition.sh`); if more than a handful solve, the
lane's cheapest lever is the auto route's choice on this board's shapes,
ahead of any `build_rpg` work.
**RUN 2026-10-15 18:45–19:45 (`lane6/partition.tsv`, 64 cells, solo, 60 s):
6 solved, all pathways** — pathways-propositional i17 (34 s), i19 (29 s),
i21 (25 s) and pathways-propositional-strips i17 (12 s), i19 (7 s), i21
(7 s); pathways i18 and every trucks (32 cells), pipesworld (15), rovers
(6) and storage (4) cell unsolved. The partition-and-resolve route is a
**pathways lever, not a board lever**: six rows the ladder never reaches
on any budget (i17 at 5 M evaluations in 300 s), nothing elsewhere. The
mechanism this names is small and precise — the auto route tries the
partition subplanner on the pathways shape (29 schemas, 24 disjunctive
preconditions, goals that decompose by reaction) — and its band is those
six rows with no loss elsewhere on the board; the question before any
code is what, in the grounded task, the partition's invariant synthesis
sees on pathways that it does not on trucks, so the route can be keyed
on the task and not on a domain name. Lane 6's `build_rpg` work stays
behind it in the order.

**The route as it stands, read 2026-10-16 12:05 (`api.rs::solve`,
`resolve.rs`):** `Mode::Auto` sends every classical task to the ladder
(`Mode::Ff`); partition-and-resolve is reached only by `--mode partition`.
Inside it, `interaction_partition(task, mutex_groups)` is the first
decision — how many goal groups the task falls into — and the subplans
run under a 100k-evaluation cap each, with the process wall read for
merges. **The proposal:** in auto, when the task falls into several
groups, run the partition route first under a slice of the wall (the
ladder's own affordability rule, a rung like LAMA's), and the ladder
after it; the key is the group count, not a domain name. Before any
code: `lane6/groups.sh` reads that count on three instances of every
IPC-5 propositional variant (`FF_WALL_DEBUG`'s "partition entry: N
groups") — if pathways decomposes and trucks does not, the key exists;
if both decompose, the key is elsewhere (what the subplans' merge does
on pathways' reaction goals). Measured on the whole `ipc5-prop` board
through the crucible, with the other classical boards' loss sides, since
a route change touches every classical cell. Runs after the numeric
sweep.

---

## Lane 2b — metric-time beyond rovers

**Claim:** tpp and pathways metric-time, where the note is the ladder's
("temporal ladder stopped at the wall") and not the compile's, reach a plan.
**Band:** +8 to +20 on the `ipc5-metric-time` tpp|pathways subset (70
cells; 0.29 Lane 2 left them where they were) on the first-attempt
estimator; rovers i20 and i28 counted with them. **Kill:** the compression
rung reaches no plan on the subset after (1) and (2) below → the decision-
epoch search is the limit; recorded, and the partition subplanner's
Phase-A step is the next cycle's question, not this one's.

**Phase 0.1 handed this lane five fixtures (2026-10-15):** rovers-metric-
time i28, settlers-snp i5, sugar i17, ztalloc-sum i9 and i12 — each
unsolved on 0.29.0 and solved under `FF_NO_CONSUME=1` at 0.28.0's own
time. **The shape, read on ztalloc i12 (`FF_HTRACE`):** charged, `best_h`
falls 4264 → 1 by 1,352 evaluations in 9 ms and then **sits at 1 for
305,000 evaluations to the wall**; uncharged, it falls to 2 by 2,500 and
LAMA solves. ztalloc's actions are TRANSFERS — `(decrease (value ?r)
(value ?r))` beside `(increase (work-value ?r) (value ?r))`, read from
below by `(>= (value ?r) 1)` — and the charge bills a transfer as
consumption: the goal-adjacent state's relaxed plan "spends" the value it
is about to move, the deficit prices a raiser, and every near-goal state
reads h = 1 whatever it does. A plateau at the last step, not a cliff at
the first. The fixture is that shape at fixture scale (a transfer the
goal needs, charged h at the goal-adjacent state against the true
distance); the mechanism is for the charge to see a decrease whose amount
the same op moves into another fluent as a transfer, or to charge only
when the deficit's raiser is not the plan's own transfer — decided on the
five cells plus the 2023-numeric rows the charge won (+27).

**The fixture, RED 2026-10-15 18:45 (`tests/consume_transfer.rs`, ignored
until the mechanism lands):** ztalloc i12's own shape embedded — five
registers at 1, the goal a sum of 4,269 with every register normal — through
`ferroplan::solve` at 400k evaluations: **uncharged solved; charged
unsolved at 400,086 evaluations.** A single register with a point goal
(730 → 243, the first draft) did NOT show it: both arms solved at once. So
the plateau needs the SUM goal over several registers — the linear
achiever (`numeric_achiever_linear`) prices the sum's gap through `double`
(reps = ⌈gap / value⌉), and the charge then bills the division sequences
each register must run; which of the two the h = 1 floor comes from is the
first thing the mechanism's work reads, on the fixture, with the state at
h = 1 printed.

**The state at h = 1, read 2026-10-15 18:40 (`FF_HTRACE=2`, new: the best
state's depth and defined fluents):** both arms climb the same way —
`double` r1 twelve times to **(4096, 1, 1, 1, 1)**, sum 4,100 of 4,269, at
g 12 — and there the charged h reads **1**, the uncharged **2**. The true
distance from that state is twenty-odd actions (the last 169 = 128 + 32 +
8 + 1 is four more registers doubled up and a division or two); both
heuristics are optimistic by an order of magnitude because the linear
achiever prices the sum's gap with one more `double` (reps = ⌈169 /
value⌉ against a value that doubling would overshoot). The uncharged
ladder survives it — LAMA's landmarks cross the plateau in 3 s; the
charged ladder's LAMA fails its slice and the best-first wanders 575,000
evaluations at h = 1. So the charge is not inventing the plateau, it is
flattening a heuristic that was already flat here (2 → 1) and taking the
landmark rung with it. The next read is the two RELAXED PLANS at that
state (what each selects for the last 169), which decides whether the
mechanism is in the charge (a transfer billed) or in the linear achiever
(a doubling priced as if it could stop at 169).

**The relaxed plans, read 2026-10-15 19:55 (`FF_HTRACE=3`, new: the plan
the extraction selects at a best state, with each op's charged firings):
IDENTICAL on both arms** — at g 10 `DOUBLE R1 ×4`, at g 11 `DOUBLE R1 ×2`,
at g 12 (4096, 1, 1, 1, 1) **`DOUBLE R1`, one firing**. The linear
achiever (`numeric_achiever_linear`) prices the sum's gap of 169 by the
op that raises the sum fastest, and doubling r1 raises it by 4,096 — one
firing "covers" the gap and OVERSHOOTS an equality goal. The relaxation is
blind here on both arms; the charge adds nothing to the plan and shifts h
by a constant (1 against 2), and that constant changes LAMA's
deterministic order from a 3 s solve to a failed slice. **So ztalloc's
mechanism is not the charge's semantics; it is the Eq-goal overshoot in
the linear achiever:** a raiser whose per-firing delta exceeds the gap
does not achieve `=`, and the honest price is the smallest-delta raiser
(DOUBLE R2 at delta 1: 169 firings), which is a gradient toward growing
the other registers. The charge's own semantics are still open on the
other three cells (rovers i28, settlers i5, sugar i17), read next with
the same dump. The fixture stays as written (both arms must solve); the
mechanism it will green is the achiever's, with the charge left on.

**settlers i5 and sugar i17, read 2026-10-15 20:10 (`--search best-first`,
`FF_HTRACE=3`, 20 s):**

| cell | arm | evaluations/s | best h and its plan |
|---|---|---|---|
| sugar i17 | charged | **~3.1 k** (48,532 in 15.5 s) | 7 — 5 ops, the same shape as uncharged |
| sugar i17 | uncharged | **~44 k** (117,285 in 2.7 s) | 7 — 4–5 ops |
| settlers i5 | charged | **~2 k** (39,802 in 20 s) | 16 — 8–9 ops: `LOAD1 … STONE ×3`, `UNLOAD1 … STONE ×3`, `UNLOAD1 … ×2`, `×2`, … |
| settlers i5 | uncharged | **~37 k** (156,527 in 4.3 s) | 1 — `BUILDHOUSE1 LOCATION1`; solved in round 2 |

**The charge's first defect is its COST: 14× per evaluation on sugar, 18×
on settlers**, on tasks where it changes the landscape little (sugar) or
flattens it (settlers: a load/unload chain billed at every hop, a
transfer again). `consumption_charge` begins with `(0..task.n_ops)
.filter(selected)` — a scan of every grounded op on every evaluation —
and then chases each deficit's raiser through `select` and the queue;
the uncharged extraction never touches `n_ops`. Mechanism (a), first and
mechanical: the extraction keeps the list of ops it selects, the charge
reads that list, and the chase is bounded by the plan, not the task.
Measured on the five cells' evaluations/s and solves, then the
`ipc2023-numeric` loss side (the charge's +27 must stay). Mechanism (b),
the transfer: a decrease whose op moves the same amount into another
fluent (ztalloc's `copy-reset`, settlers' `load`/`unload`) is not spending
— after (a), on the cells (a) leaves. Mechanism (c), the achiever's
Eq-goal overshoot (ztalloc) — its own fixture, after (b).

**(a) WITHDRAWN before it was built — the profile, 2026-10-15 20:20
(`profile.py`, settlers i5, `--search best-first`, 25 s, both arms):** the
two self-time lists are the same list — `eval_iv` 5 %, `build_rpg` 2–3 %
a dozen times, `state_key_eq` / `state_key_hash` 1–2 % — and
`consumption_charge` is not in either; settlers grounds to 950 ops and
sugar to 236, so the `n_ops` scan is nothing. The 14–18× in evaluations
per second is not the price of an evaluation, it is WHERE the charged
search goes: states with many more applicable ops per pop (settlers'
load/unload fan-out), each pop paying for its successors. That is the
landscape, which is (b)'s question. The order is now **(b) transfers,
then (c) the Eq-goal overshoot**, each measured on the five cells and the
2023-numeric loss side; `tests/consume_transfer.rs` stays the fixture
for (c), and (b) gets one of its own on the settlers shape (a load that
moves stock onto a vehicle, an unload that moves it off, a goal that
needs it moved twice).

**(b) and (c) BUILT 2026-10-16 morning.** (b) `consumption_charge`'s
ledger skips a decrease whose op increases another RESOURCE by the same
evaluated amount — a resource being a fluent some op reads from below
(`Scratch::read_from_below`, computed once per task), so `(increase
(labour) 1)` beside `(decrease (stone) 1)` still bills the stone. (c)
`numeric_achiever_linear` skips, for an `=` goal, a mover whose single
firing passes the point (`toward > gap`); Ge/Gt keep the fastest mover.
Fixtures: `tests/consume_move.rs` (the haul: a cart, a quarry, a site
that needs two stones — **charged evaluations == uncharged**, green) and
`tests/consume_transfer.rs` (ztalloc i12's shape — **charged solved in
0.30 s**, from unsolved at 400,086 evaluations, green). Measured next:
the five cells solo ×3 both arms on this engine (`lane2b/cells.sh`), then
the 2023 / 2026 numeric boards and metric-time in full through the
crucible (`lane2b/sweep.sh`) — the charge's +27 must stay, with the five
cells back.

**The five cells on the first build (`lane2b/rows-first.jsonl`, 09:57–10:17,
solo ×3, 60 s):** ztalloc i9 **3/3 at 0 s** and i12 **3/3 at 1 s** on both
arms (0.28.0 took 34 s and 3 s), sugar i17 **3/3 at 44 s** both arms —
three of five back, and faster than they ever were. settlers i5 charged
0/3 against uncharged 3/3 at 7 s — the transfer rule did pair its
load/unload (each decrease beside an increase of a resource by one), so
settlers' charge is something else, read next with the dump on this
engine. And **rovers i28 0/3 on BOTH arms**, where yesterday's uncharged
arm solved 3/3 at 24 s: a regression of mechanism (c) on the uncharged
path — skipping every overshooting mover outright leaves a precondition
with NO mover where the only one overshoots, and the relaxed plan reads
a dead end. The sweep launched on that build (`c7fc54037d88`, 10:17) was
stopped at its calibration gate by PID; nothing of it banked. **(c)
narrowed:** the non-overshooting mover is PREFERRED for an `=` goal when
one exists, and the fastest mover stays the fallback — the fixtures (the
ztalloc shape has `DOUBLE R2`), the five cells and the sweep again on the
narrowed build.

**The five cells on the narrowed build (10:20–10:40, same probe):** ztalloc
i9 / i12 3/3 at 0–1 s both arms; **sugar i17 charged 0/3, uncharged 3/3 at
51 s** (the first build's harsher rule carried a 44 s solve across a 60 s
wall; the narrowed one does not — a wall-edge cell either way, the board
decides); settlers i5 charged 0/3, uncharged 3/3 at 11 s; **rovers i28 0/3
on both arms again, so it was never (c):** yesterday's "uncharged 3/3 at
24 s" was read on the 0.29.0 BOARD binary (`losses29/run.sh`), today's on
the 0.30 engine — Lane 5's lazy frontiers changed the compressed task's
search order, and the attribution is a build of the Lane 5 commit
(`c7fd313`) run on the cell, owed before anything is said. **settlers,
read with the dump on this engine:** charged `best_h` 4 at 75,000
evaluations with the plan `BUILDHOUSE1 LOCATION3 ×2, BUILDHOUSE1
LOCATION1 ×2` plus the raisers the builds' timber and stone owe —
consumption billed exactly as the charge was designed to, and it still
misleads the search where the uncharged plan (`BUILDHOUSE1 LOCATION1 ×2`,
h 2) walks to a solve in 7–11 s. That is the charge's one open cell: the
deficit is priced from THIS state's stock against the plan's spend
times its repetitions, and on a task where the plan is "build twice,
having hauled twice" the raisers' chase is a landscape of its own. Not
built this cycle; the board read says what it costs.

**The board read, in progress (`lane2b/sweep.sh`, engine `13eb6b53a361`
= `83e8151`'s build, from 11:35; interim at 15:00, banked cells only):**
metric-time 146 cells 118 = 118; 2023 numeric 110 cells flat; **2026
numeric 268 cells: +10, −0 — every one a ztalloc cell**: i9 (1.0 s), i11
(1.8), i12 (1.8), i13 (9.9), i14 (2.3), i16 (2.5), i17 (4.6), i18 (2.8),
i19 (2.8), i20 (3.1). Two of those were the lost cells; the other eight
neither 0.28.0 nor 0.29.0 ever solved — the Eq-goal rule (c) opened the
register machine's whole upper half. The final compare follows the
sweep; nothing is claimed from a sweep in flight.

**The questions, in order:** (1) does the compression rung REACH a plan on
tpp i10 / pathways i10 solo (`FF_WALL_DEBUG`), as it did on rovers in 10 ms,
or does it stop at `FF_TCOMPRESS_WALL_FRAC`? If it reaches one, the route is
the question again (0.29's finding); if it does not, the heuristic is. (2)
pathways at 8 s a ground: the grounding Lane 1 named as its ceiling is this
board's too; the achiever index and Lane 6's profile are checked here
first. (3) The deferred Lane 2 step (5): the partition subplanner keeping
its Phase-A subplans, only if (1) and (2) leave the band unmet and the solo
probe says the search is what is left. **Measured:** `sweep --board
ipc5-metric-time --only tpp,pathways,rovers`, then the loss side of
`ipc2023-numeric` / `ipc2026-numeric` (the consumption charge touches
both), `compare` on first-attempt.

---

## Lane 1b — points on the complex board

**Claim:** the complex-preferences quality score rises toward SGPlan5's,
and simple / qualitative do not fall. **Band:** complex 54.5 → 65 of 100;
qualitative 64.6 → 70 of 92; simple 97.3 → 105 of 120 (0.29's unmet band,
carried). **Kill:** points flat on complex with the whole wall spent in the
polish → the polish's candidate search is the limit; recorded with the
evaluation counts.

**The mechanisms, in order:** (1) **the anytime loop** — the polish re-seeds
from its best plan and runs until the wall or a fixed-point, instead of a
single polish on the seed's task; `FF_PREF_POLISH_FRAC`'s single share
becomes a loop share; fixture: a task where the second polish improves on
the first (openstacks p01 at 6 both ways is the pin that nothing regresses).
(2) **Grounding reused across attempts** — one grounding per SOLVE, not per
polish: the compiled task is kept and the polish's op-penalty table rebuilt
over it; pathways at 8 s a ground is the fixture's shape. (3) The 13 tpp
wall rows and 14 storage mem-cap rows on complex are rows, and Lanes 5 / 6
may move them for free; counted here only after those lanes' records.
**Measured:** `sweep --board ipc5-complex-pref` and the two sibling boards
in full (130 + 100 + 108 cells), the quality column on ONE engine.

---

## Probes only — recorded, never claimed this cycle

- **Constraints (28/120, +42 to the leader).** Two negatives in a row (0.28,
  0.29 Lane 3). SGPlan5 partitioned on the constraint structure; ours treats
  a trajectory constraint as a monolith on the compiled task. The probe:
  for tpp-metric-time-constraints i1–i5 and pipesworld-mt-constraints i1–i5
  solo, what does the compiled task look like (`FF_CONSTRAINTS_REJECT`, the
  automaton size, the goal's shape), and does an `always` / `sometime-
  before` compile to anything a landmark can see? A record in
  `probes-0.30/constraints/`; a lane for 0.31 only if the record names a
  mechanism.
- **2014 seq-opt (82/256, +56).** The opt rungs now mem-cap where they timed
  out. After Lane 5 lands, the loss side of `ipc2014-opt` and `ipc2018-opt`
  is re-read with `max_rss` beside it: if the lazy frontier's closed-set
  half moved the proofs, the record says so; if the wall is now spent
  honestly, the heuristic (LM-cut's probe fraction, `FF_OPT_LMCUT_PROBE_FRAC`)
  is the next cycle's question. Nothing symbolic or bidirectional.

---

## Anti-pots — priced at zero, standing

Everything 0.28 and 0.29 listed, plus:

- **A rows claim from the unattributed group.** The 2014 multi-core and
  seq-sat gains of 0.29 are the `widen` inlining's candidate; Lane 6 names
  what IT buys with a receipt, and the 0.29 rows stay unattributed.
- **A constraints or seq-opt number as a lane result.** Both are probes
  this cycle; a probe's finding is a record, and the band for a lane is
  written in 0.31's roadmap, not here.
- **"The folding rows" without the grounding receipt.** The fix is judged
  on 16 of 16 grounding; a row claim on folding goes through Lane 5's
  sweep with the engine hash beside it.
- **Evals/s from a loaded box.** Lane 6's receipt is solo, one thread, on
  an idle box with the canary's idle line (0.520 s) named beside it; a
  panel reading beside foreign load is discarded, not averaged.
- **Reopening the pinned knobs.** `FF_TCONC`, the polish's first-share
  fraction, and the memory-skip quarter stay where 0.29 left them.

---

## The cut rule

The 32-board `cut30` set, the same instrument; both declared measures read
on it; the 21 cells 0.29.0 lost and every lane's named cells re-run on both
engines before any is named; `recheck` is the referee's job. The baseline
for `compare` is the banked 0.29.0 engine `7fdf185b91e6`, never a rebuild
unless Phase 0.4 shows the rebuild reproduces it. Nothing is published until
`main` has it and the pre-flight — with the token's age on the checklist
(the crates.io token dates from 2026-09-29) — is green.
