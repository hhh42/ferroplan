# Recommendation: retry contended instances, not whole boards

**Status:** IMPLEMENTED 2026-08-22 (steps 1-4; see the commit that carries this file). Validation: the reuse gate unit-tested branch-by-branch (version/budget/mode/threads mismatches, missing stamps, dirty windows, span-outs, corrupt tail lines all rejected; clean windows and prior judgments reused), and an all-reused board reproduced its prior coverage verbatim in 0.04s with zero ff invocations, carrying a STITCHED note. Originally: Written after the 0.25 entries sweep
(`benchmarks/entries25-sweeps.sh`) burned most of 2026-08-21 re-running
boards from scratch over brief contention windows — three passes, ~9
board-hours, most of it re-measuring rows that were already clean the
first time.

## The problem

`run_board()` in `entries25-sweeps.sh` wraps each board with a
`contention.py` watcher and, on a DEGRADED verdict, throws the *entire*
board away and re-queues it for the next pass. A board is 100–400
instances and can run 40 minutes to 2.5 hours; the contention that
kills it is often a 5–15 minute window (a background `cargo build`, a
Spotlight reindex) somewhere in the middle. Everything measured before
and after that window was fine and gets discarded anyway.

This is the right call *today* because the two watchers don't talk in
enough detail to do better:

- `contention.py` (`summarize()`) only ever emits one aggregate verdict
  for the whole run — median idle%, mean competitor load — computed
  over the full sample list. It has the raw samples in memory
  (`idles`, `loads`, `comp_totals`) but never writes them out
  per-timestamp, only the final rollup.
- `ipc67.py` (`run_instance()`) records each instance's *relative*
  elapsed time (`time.perf_counter()` delta) but not its wall-clock
  start/end. There's no way to ask "was instance N running during the
  bad window?" after the fact — the two logs can't be joined.

## What would close the gap

1. **`contention.py`: persist a timeline, not just a rollup.**
   Add each sample (`ts`, `idle`, `competitors_total`) to a list in the
   output JSON alongside the existing aggregate `summarize()` block.
   At a 20s interval a 2-hour board is ~360 samples — trivial size.

2. **`ipc67.py`: stamp each row with wall-clock window.**
   Add `start_ts`/`end_ts` (epoch seconds) to the record built in
   `run_instance()`, next to the existing `time`/`budget` fields. This
   is what lets a later pass intersect an instance's run window against
   the contention timeline.

3. **`ipc67.py`: a resume mode.**
   Something like `--resume-raw PRIOR.jsonl --resume-conditions
   PRIOR.conditions.json`. For each instance: if the prior raw has a
   record whose `[start_ts, end_ts]` window doesn't overlap any
   contended sample in the prior conditions timeline, **and** the run
   params match (same `ff --version`, `--timeout`, `--jobs`,
   `--mode`, `--threads` — this needs an explicit guard, a silently
   stitched row measured under different settings is worse than a
   discarded board), reuse the row without re-invoking `ff`. Otherwise
   run it fresh. Write the merged result as the new raw + summary.

4. **`entries25-sweeps.sh`: pass the prior artifacts forward.**
   `run_board()` already knows the board's `.md`/`.log`/`.conditions.json`
   paths; on a DEGRADED verdict, instead of just leaving them for the
   next pass to ignore, pass them as `--resume-raw`/`--resume-conditions`
   to that pass's invocation.

## Why this is worth the complexity

The two big numeric/optimal boards (`ipc2023-numeric-opt`, 400
instances; `ipc2026-opt-full`, 260 instances) are where this pays for
itself — each takes 2+ hours and got re-run from zero multiple times
yesterday for a contention window that was a small fraction of that.
The small pref boards (100–130 instances, ~40 min) aren't worth
touching — whole-board retry there is already cheap.

## Risk / care points

- **Version drift across a merged board.** A stitched board must never
  mix rows from two different `ff` builds. Gate resume on an exact
  version match (and probably also the git SHA if the binary carries
  one), not just "0.25.x".
- **Verdict semantics change.** The current whole-run verdict
  (`competitors_total_pcpu < 25%` → clean) needs a per-instance
  analogue — an instance's window is "clean" only if every sample
  overlapping it was under threshold, not just the run's overall
  median. Getting this wrong silently reintroduces the exact failure
  mode (contention-suppressed coverage) the watcher exists to prevent.
- **Instances that straddle the resume boundary** (running when the
  prior pass was killed/timed out) have no `end_ts` — treat as
  "needs re-run", not "clean by omission".

## Suggested first cut

Prototype steps 1–3 against `ipc2023-numeric-opt` only (single board,
biggest payoff), confirm a merged board's coverage number matches a
from-scratch clean run, then wire step 4 into the sweep driver and
extend to the rest of the registry.
