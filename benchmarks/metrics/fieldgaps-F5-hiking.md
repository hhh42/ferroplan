# F5 — the 2014 config schedule: the hiking diagnosis (0.26)

Executed 2026-08-29 (re-run ~08:20–09:10 after a first attempt lost every
row to a missing `timeout` binary — the perl `alarm` guard replaces it, see
the memory note), the diagnosis run of `docs/field-gaps-execution-0.26.md`
§F3.4 "The 2014 config schedule + hiking diagnosis". Solo, one planner at a
time, no watcher (the sitting's own note: attribute variance with care).
Binary: the **0.26.0 candidate** (F1 enrichment default-on — the fallback rows
below are the enriched fallback's). Receipts: `benchmarks/air26-probes/`
(json + stderr under `FF_WALL_DEBUG=1 FF_RES_DEBUG=1`, `f5-diagnosis.log`).

The dossier's read before any run stands: the sat/agile "config" difference
is the CORPUS (hiking's 20 agile instance files differ from the sat ones and
are bigger; tetris/parking are byte-identical across the pair), so the
"agile ordering dies on hiking" hypothesis had already dissolved. What the
run adds is the mechanism behind the agile losses and the tetris flip.

## 1. hiking-agile i5 / i6 / i7 — (a) which rung eats the wall, (b) cliff or tail

| run | 60 s | 300 s |
|---|---|---|
| i5 (28,530 ground actions) | unsolved: light 6.4 s, LAMA 25.2 s (three recency extensions, key 150 → 132, then flat), driver skipped, fallback **4,453 evals** at the checkpoint | **solved, 94 steps, by the fallback (round 1)** after light 31 s + LAMA 64 s + driver 57 s |
| i6 (37,215) | unsolved: EHC 14.9 s, light 6.4 s, LAMA 20.2 s (key 210 → 176), fallback 4,564 evals | **solved, 86 steps, by LAMA** after light 30 s |
| i7 (47,052) | unsolved: EHC 14.7 s, light 4.8 s, LAMA 31.4 s (key 286 → 202, four extensions), fallback 1,570 evals | unsolved: LAMA 66 s (key 202 flat), driver capped at 400k pops (9.9M nodes), fallback **129,067 evals in 143 s, of which h = 101 s** |

**(b): i5 and i6 are TAILS, i7 is a wall.** Both smaller agile losses convert
at 300 s — one through the fallback, one through LAMA's bigger slice — and
the 60 s narration shows why they miss at 60: the ladder's bounded rungs
spend 45–50 s making steady but insufficient progress (LAMA's key drops
every extension and refuses only when it goes flat), and what reaches the
fallback is 1.5k–4.5k evaluations. **(a): the wall is EVALUATION cost, not
a rung.** Under 60 s the fallback manages ~4.5k evals in ~13 s (≈3 ms each);
at 300 s on i7 the fallback's 129k evals cost 143 s with **101 s in the
heuristic** — hiking's relaxed plan is expensive (28–47k ground actions,
127–161 facts), and the 5-car/4-couple instances are simply bigger than the
wall at any ordering. The car/couple assignment plateau the dossier
suspected is visible as LAMA's flat key (202 on i7 for the last 35 s of its
slice), but the instance converts on i5/i6 given time, which makes the
family a scaling wall with a tail, not an ordering kill.

**(c) grounding/eval stats, agile i5 vs sat i16:** 28,530 vs 27,999 ground
actions, 127 vs 136 facts — the agile i5 is NOT larger in grounded size than
the sat instance the board solves in 37 s; it is harder per evaluation and
deeper (94-step plan). The size story is in i6/i7 (37k/47k actions), not i5.

**A flag, carried honestly:** hiking-sat i16 — 37.4 s solved on the 0.25
board (the slowest sat solve, chosen for that) — did **not** solve solo at
60 s on the 0.26.0 candidate: EHC 14.9 s, light 5.3 s, LAMA 20.3 s (key
198 → 176, flat), fallback 6,135 evals at the checkpoint. One solo run is not
a verdict (the F1 A/B did not sweep 2014-sat; the cut sweep will), but it is
the shape a regression would have and it is written down before the sweep,
not after.

## 2. tetris-sat i14 — (d) the identical-file flip, three reps

All three reps unsolved — and the narration says the wall is spent BEFORE
search: the ladder enters with **54%, 50% and 22% of the 60 s wall
remaining** (i.e. 27, 30 and 47 s gone to parse + ground: 60,196 ground
actions, 4,068 facts), and what is left runs light 2.9 s → LAMA 9.7 s →
fallback (25k / 20k / 15k evals) to the checkpoint; rep 3 had so little wall
that the bounded rungs were skipped outright. The agile board's 52.94 s
solve of the identical file and the sat board's 60 s failure are therefore
**one instance whose pre-search phase costs roughly half the wall with a
20-second spread** — a grounding-time row, and the class the contention
watcher exists for on the boards (no watcher ran here; the spread is
recorded as observed, not attributed).

## 3. Verdict — the schedule build is REFUSED

The oracle the dossier corrected to ≤ +2 (tetris i14 and parking i14, the
two identical-file flips) does not survive the diagnosis either: tetris i14
is a grounding-dominated wall-edge row that a driver-side config schedule
cannot touch (the schedule would split a wall that grounding has already
half spent), and hiking's agile losses are evaluation-cost tails that need
TIME, which the in-engine refill loop already spends — a two-phase cell
under one 60 s wall has nothing to add over it. **No config schedule is
built; F5 closes as a recorded negative on the diagnosis,** with two
by-products: hiking i5/i6 belong in any 300 s-tier story (they are tails,
+2 there), and hiking-sat i16 is flagged for the cut sweep's referee.
