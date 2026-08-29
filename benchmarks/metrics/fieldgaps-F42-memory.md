# F4.2 — the folding/elevator memory-profile sitting (0.26)

Executed 2026-08-29 ~07:45–08:15, the sitting of
`docs/field-gaps-execution-0.26.md` §F4.2, solo on a quiet box (the probe
chain runs one planner at a time; Sitting B's watcher read the box clean all
morning). Binary: `target/release/ff` = the **0.26.0 candidate** (F1 on; the
grounding and temporal paths this sitting reads are untouched since 0.25.0).
Receipts: `benchmarks/metrics/probes-0.26/F42-memory/` (per-run json, stderr
under `FF_WALL_DEBUG=1 FF_RES_DEBUG=1`, `/usr/bin/time -l` peak RSS in the
log tail). Guard: `perl -e 'alarm'` at wall + 20 s (this box has no
`timeout`; a run that reaches the guard has already overrun its wall).

## 0. The rows the sitting was sent to explain — re-read from the FRESH raws

The dossier quoted mem-caps from the committed 0.24-era raws. The 0.25 cut's
own rows (`benchmarks/air25/`) say something different for two of the four:

| row | board (air25, ff 0.25.0) | dossier's read |
|---|---|---|
| folding i9 @300 s | **timeout**, 300 s, no note | "mem-cap 16.2 s" |
| folding i15 @300 s | **mem-cap, 13.7 s** | "mem-cap 12.1 s" |
| elevator-2008-strips i29 @60 s | **timeout**, 60 s, no note | "mem-cap 8.47 s" |
| elevator-2011 i10 @60 s | **mem-cap, 8.66 s** | "mem-cap 9.46–13.73 s" |

So the family's face is half timeouts and half mem-caps even on the board,
and the sitting's first job is to say which phase each row is in.

## 1. The ledger (solo, `/usr/bin/time -l`)

| run | outcome | real | peak RSS | phase at death (narration) |
|---|---|---|---|---|
| folding i9, 300 s | no verdict | 320 s (guard) | **2,965 MB** | `wall: grounding checkpoint expired mid-enumeration (no task, no verdict)` |
| folding i15, 300 s | no verdict | 315 s | 1,234 MB | `grounding stopped at the declared budget: wall budget exhausted during binding enumeration` |
| folding i9, node cap 10k / 100k / 1M | no verdict ×3 | 312–320 s | 1,476 / 1,268 / 502 MB | same grounding stop, all three — the cap never mattered |
| elevator-08-strips i29, 60 s | **no output at all** | 80.7 s (guard) | 576 MB | no `wall:` line, no JSON — nothing checkpointed |
| elevator-08-strips i29, node cap 10k / 100k / 1M | no output ×3 | 80.4–80.8 s (guard) | 1,146 / 584 / 1,209 MB | same |
| elevator-2011 i10, 60 s | no output | 80.6 s (guard) | 1,204 MB | same |
| elevator-08-**numeric** i29, 60 s | **solved** | 46.9 s | 123 MB | — |

## 2. folding — NAMED: a GROUNDING wall, not a search-memory wall

Every folding run, capped or not, spends the whole 300 s inside binding
enumeration and dies at the grounding checkpoint with no task built. The
RSS-at-forced-cap method the dossier prescribed (the 0.19 Phase 4 slope
read) cannot be applied — `FF_SEARCH_NODE_CAP` governs a search these runs
never reach, which is exactly why the three capped rows are identical to
the uncapped one in everything but their peak (the peak varies 0.5–3 GB
across identical runs: allocator timing, not model). What the board booked
as folding's "memory ceiling" (10 mem-caps at 12–18 s) is grounding's
allocation under `--jobs 2` reaching the runner's 6 GB RSS watchdog before
the wall does; solo, the same enumeration runs to the wall at 1.2–3 GB.
Either way the search's byte model, `per_node_model_bytes`, the RSS-trip
build candidate and `FF_NO_MEMTRIP` are all **inapplicable to folding** —
there is no node to charge. **The lever is grounding-side**, and the
dossier's own condition for the or-aware-hoist rider ("enters ONLY if the
ledger attributes folding RSS to grounding tables") is met by this ledger.
The rider stays what the record calls it — sized, not taken — until it is
priced; folding's coverage on the 60 s boards (0/20 either way) says the
domain is hard past grounding as well, so the honest band for the rider is
the 300 s board's capped rows only.

## 3. elevator (temporal) — NAMED: grounding again, behind a checkpoint too coarse to hold the wall

The temporal runs are the sitting's real finding. Under a 60 s wall,
elevator-2008-strips i29 and elevator-2011 i10 print **nothing** — not the
ladder's "remaining … affordable" line, not a grounding checkpoint, not a
temporal pass start — and are still running when the guard kills them at
80 s, at 0.6–1.2 GB resident. The board's runner kills at 60 s and books
the row a timeout (i29) or, under two jobs, a mem-cap (i10); solo there is
no 6 GB anywhere near. **So these rows are not memory rows at all: they are
wall overruns inside GROUNDING** — before the temporal fold's first
narration, in a phase whose wall checkpoint is too coarse to stop them. The numeric-fluents twin of the same
instance (i29) solves in 46.9 s at 123 MB, which rules out the instance
size and points at the strips variant's grounding/fold path. The memo's
"+3 elevator mem-cap fix" is therefore mis-shaped: nothing to fix in the
search's memory accounting; a grounding wall with a checkpoint-granularity
defect in front of it.

**Where it hangs:** The follow-up (`F42b`, same directory) answers it twice over. (1) Under a
**10 s wall the same run RETURNS** — a 223-byte no-verdict JSON and, on
stderr, `wall: grounding checkpoint expired mid-enumeration (no task, no
verdict)` printed **three times** (the temporal ladder retries grounding
under its variant flags, and each attempt dies at the checkpoint). (2)
Under the 60 s wall, `sample` at 20 s and again at 70 s finds every thread
in the same frames: `api::solve → temporal::solve_prefless →
solve_monolithic → solve_inner → ground::ground_v` — **binding
enumeration**, Phase B of grounding, both times. So the phase is not
un-checkpointed after all; its checkpoint is too COARSE for this instance:
the clock is read between enumeration units, and on elevator-2008-strips
i29 a single unit runs longer than the 20 s the guard allowed past the
wall (the 10 s run happened to land its check early). The row is a
grounding-time row — the same shape as folding — with a checkpoint-
granularity defect on top, which is why the board could book it as either
a timeout (i29, jobs 2) or a mem-cap (2011 i10: enumeration's RSS under two
jobs reaching the 6 GB watchdog first).

## 4. The per-board split of the +3–10 band

- **folding-300s (≤+10 optimistic in the memo):** re-routed to grounding.
  The 10 capped rows are grounding-time rows; the memory build claims none
  of them. Unpriced until the hoist rider is probed.
- **elevator-2008 (+3):** the three "mem-cap" rows of the memo are, on the
  fresh board, grounding-time rows (binding enumeration past the wall, a
  coarse checkpoint on top); the numeric twin solving 30/30 says the strips
  encoding is what enumerates badly. A grounding lever's rows, not a memory
  build's.
- **elevator-2011 (≤+7):** same shape as 2008 (i10 solo: overrun, 1.2 GB).

**Verdict for the build candidate:** the RSS-trip/model-correction build
(`FF_NO_MEMTRIP`) is **refused on this ledger** — none of the six target rows
dies in search, so a search-side memory trip cannot reach them. ONE named
mechanism replaces it on both families — **the cartesian binding
enumeration (grounding Phase B)** — with the or-aware-hoist rider's gate
now open on the evidence, plus one defect to fix on its own: the temporal
path's grounding checkpoint is read too coarsely to hold a 60 s wall on
elevator-2008-strips i29 (≥ 20 s overrun solo; the runner's kill is what
ends it on the board). That fix is small, ships with a fixture (i29 at a
60 s wall must RETURN with the no-verdict note inside the wall + reserve),
and touches no search.

**Landed (same day).** `WallTick::STRIDE` in `ground.rs` 8,192 → 256: the
stride counts BINDINGS, and one binding's DNF work is unbounded, so 8,192 of
them could carry the enumeration 20 s past the wall. Receipt, i29 @60 s solo
on the rebuilt binary: **returned after 60 s**, 223 bytes of JSON, six
checkpoint lines on stderr ending in the no-verdict note. `ground_wall`,
`novlight`, `enrich`, `refill` green under the new stride.
