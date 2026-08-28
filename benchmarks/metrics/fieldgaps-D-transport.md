# Sitting F0(d) — the transport L1–L3 probe widening (0.26)

Executed 2026-08-27, sitting F0(d) of `docs/field-gaps-execution-0.26.md`
(Sitting D + Amendments; memo `docs/field-gaps-0.26.md` §1c). Binary:
`target/release/ff` = **ff 0.25.0**, the promoted cut binary (same `ver` as
every fresh board row cited below — the roadmap-0.21 old-binary referee is
satisfied by identity: probe and board ran the SAME tagged engine).
Receipts: `benchmarks/metrics/probes-0.26/D-transport/` (per-run json + stderr
log + `matrix.jsonl` summary rows + `progress.log`). No code changes; no new
hatches; every condition below uses hatches that already exist.

**Conditions, honestly — this sitting was measured twice.** Run 1 (13:39–16:28,
166 rows) was NOT quiet: the watcher stamped it DEGRADED (load peaks to 117,
swap to 11.7 GB; competitors the Docker VM, a `swift-frontend` build, and the
assistant's own home-directory grep/find sweeps for an unrelated question).
The 2011 legs ran before the noise and are clean; on the 2008 classical and
the whole tempo-sat leg `ff` got 35–60% of a core (`utime` far under `wall`
— e.g. `t08-i6-default` 36 s CPU in a 60 s wall; the tempo-sat eval-budget
slices 9–14 s). A coverage-at-the-wall verdict from a starved run is not a
receipt. Rule applied: every threads=1 row with `utime < 0.9 × wall` (54 of
166) was quarantined to `contaminated-run1/` (with run 1's `matrix.jsonl` and
`contention.json`) and re-measured in run 2 (17:36–18:47) under a driver that
gates each run on a quiet box (idle ≥ 65%, load < 6, two samples) and
discards-and-retries any starved row. Run 2: 54/54 rows passed the per-row
check on the first attempt, no retries; the watcher's whole-run rollup still
reads DEGRADED by its own line (idle min 46%, median 87%, load max 4.75 —
the Docker VM at ~8% of a core and another agent's `node`/`wrangler` job),
so both records stay in the directory and the per-row CPU accounting is the
basis every number below stands on. `matrix.jsonl` is append-only; the LAST
row per tag is the receipt (run-2 rows carry `"run": 2`). The two SIGSTOPped
hogs of `benchmarks/air25/SUSPENDED-PROCESSES.txt` stayed stopped throughout.

## 0. Provenance settled first — the banked L3 receipts carry a THREADS confound

The record's two L3 receipts (`benchmarks/air25-entries/transport-L3-i{4,6}.json`,
produced by `post-entries25.sh` §3b) are **ipc-2011** rows, as the spec
verification found — that part is confirmed. But the widening found a second
provenance defect, worse than the board mislabel:

- Every board row is measured with **`--threads 1`** (`ipc67.py:420,449`;
  rows carry `"threads": "1"`).
- The §3b probe invocation passed **no `--threads`**, and ff's default is
  `--threads 0` = auto = all cores.
- The banked `transport-L3-i4.time` reads **16.18 s real / 71.41 s user** —
  a ~4.4× user/real ratio on a 4-P-core box. The receipt ran multi-core
  against a single-thread board row.

So "i4: 16.18 s under the L3 pair vs 59.59 s on-board" compared a 4-core run
to a 1-core run and attributed the whole difference to the hatch pair.
This sitting re-ran the replica (all-cores, L3 pair) AND the controlled arm
(`--threads 1`, L3 pair) — see §2. All widening runs below pin `--threads 1`
to match the board instrument, solo and serial (jobs=1 vs the board's
jobs=2 — solo is the quieter side; coverage-at-timeout is the metric, which
`ipc67.py`'s own header notes is jobs-insensitive while jobs < cores).

## 1. Fresh baselines (re-read before any probe, per the checklist)

All boards banked with `.done` markers; rows are `ver: ff 0.25.0`, budget 60,
threads 1, jobs 2.

- **2008 classical** (`air25/ipc67-results.jsonl`,
  transport-sequential-satisficing-strips): **20/30**. Unsolved:
  i8, i9, i10, i18, i19, i20, i27, i28, i29, i30 (all wall at ~60 s).
  Near-wall solves (speedup witnesses): i6 59.52 s, i16 59.65 s, i17 59.84 s.
- **2011 classical** (`air25/ipc67-results.jsonl`,
  transport-sequential-satisficing): **2/20** — i4 59.61 s, i5 59.89 s, all
  else wall. (Matches the older `ipc67-default.jsonl` 2/20 shape.)
- **2008 tempo-sat numeric** (`air25/ipc67-temporal.jsonl`,
  transport-temporal-satisficing-numeric-fluents): **4/30** — i1 0.01 s,
  i2 0.03 s, i11 0.09 s, i12 1.00 s; i3–i10, i13–i30 all wall.
- **mco** (`air25/ipc7-mco-t{2,4,8}.jsonl`, transport-sequential-multi-core,
  read-only leg — no new runs per the checklist): t2 **4/20** (i3, i4, i5,
  i11), t4 **5/20** (i2, i3, i4, i5, i11), t8 **5/20** (i2, i3, i4, i5,
  i11). Cores still enumerate a plateau: +1 instance from t2→t4, +0 from
  t4→t8.

Band under test (memo §1c): **+8–20 of 211 aggregated across 2008/2011/mco,
no per-board split priced**; the overtake hypothesis needs **~+10 on the 2008
tempo-sat board specifically**, and classical-leg conversions do not count
toward it. Fence carried verbatim: 2014 transport is NOT claimable
(`air25/ipc2014-sat.jsonl` 0/20 stands); no temporal-relaxation exits
(ledger closed).

## 2. The threads confound, priced on its own receipt

Replica of the banked §3b invocation (L3 pair, `--threads 0` = all cores)
beside the controlled arm (L3 pair, `--threads 1`), 2011 corpus:

| inst | board (threads 1) | L3 pair, all cores | L3 pair, threads 1 |
|---|---|---|---|
| i4 | solved 59.61 s | solved **12.5 s** real / 46.8 s user (3.75 cores) | solved **32.7 s** (194,310 evals, same count as all-cores) |
| i6 | unsolved | solved 58.5 s real / 258.5 s user (4.42 cores; 500,456 evals) | solved **59.6 s** (271,592 evals) |

So the banked "i4 16.18 s vs 59.59 s" was roughly half threads and half
hatch: at the board's own instrument the L3 pair takes i4 from the wall edge
to 32.7 s (a real 1.8×, on identical eval counts — the pair changes the
search order, not the per-eval cost) and still converts i6, but at 59.6 s of a
60 s wall — a conversion with no margin. Every widening row below is threads 1.

One instrument note the grid makes visible: the engine's solves on these
boards land at 59.2–59.9 s almost uniformly (the board's own i4/i5 did too) —
the ladder keeps polishing to the wall after the first plan, so a "solved at
59.x s" is "solved inside the wall," not "solved at the last second." The
informative early finishes are the ones that end the run: i4 32.7 s (L3 pair),
2008 i6/i16 45.8/45.6 s (L3 pair).

## 3. 2011 classical leg — 20 instances × {default, L3 pair, FF_NO_NOVLIGHT, FF_NO_LAMA}

S = solved, · = wall; `e` = evaluated states. Board: 2/20 (i4, i5).

| inst | board | default | L3 pair | `FF_NO_NOVLIGHT` | `FF_NO_LAMA` |
|---|---|---|---|---|---|
| i1 | · | · e=60,830 | **S** e=397,723 | · e=95,646 | **S** e=368,027 |
| i2 | · | · e=45,213 | **S** e=232,981 | · e=54,941 | **S** e=205,589 |
| i3 | · | S e=152,825 | **S** e=202,233 | **S** e=140,537 | **S** e=180,729 |
| i4 | S | S | S 32.7 s | S 49.2 s | S 42.4 s |
| i5 | S | S | S | S | S |
| i6 | · | · e=28,154 | **S** e=271,592 | **S** e=241,128 | **S** e=239,336 |
| i7 | · | · e=28,285 | **S** e=212,022 | · e=30,589 | **S** e=193,590 |
| i8–i10 | · | · | · (e=48–59k) | · | · (e=42–50k) |
| i11 | · | · e=27,535 | **S** e=121,937 | **S** e=75,857 | **S** e=106,833 |
| i12–i13 | · | · | · (e=56–68k) | · | · (e=47–53k) |
| i14–i20 | · | · (e≤6k, three rows e=1) | · (e=11–15k) | · (e≤11k, two rows e=1) | · (e=9–12k) |
| **solved** | **2/20** | 3/20 | **8/20** | 5/20 | **8/20** |
| conversions vs board | — | i3 | i1 i2 i3 i6 i7 i11 (**+6**) | i3 i6 i11 (+3) | i1 i2 i3 i6 i7 i11 (**+6**) |

Reading it:

- **The tax is LAMA's wall slice.** `FF_NO_LAMA` alone converts exactly the
  L3-pair set (+6, identical membership); `FF_NO_NOVLIGHT` alone converts half
  of it. On every converted row the winning arm evaluates 3–7× more states in
  the same wall (i1: 60k → 368–398k) — the 25% LAMA slice at `search.rs:1324`
  is spent on a rung that never returns on this domain, and the fallback that
  does the solving is handed the remainder. Novelty-light's 10% is the smaller
  half of the same story.
- **The wall is not the only wall.** i8–i20 stay unsolved under every
  condition with eval counts that fall off a cliff (i14–i20: ≤15k evals in
  60 s vs 200–400k on the solved rows; several rows `e=1` — the first
  evaluation is the whole budget). That is the package-count line the 0.25
  decode named (~12–14 packages), measured again from the other side: past
  it the per-eval cost explodes and no rung schedule reaches it. The
  L1–L3 band's "2008/2011/mco ONLY, 2014 not claimable" fence is confirmed by
  the same numbers.
- **Board noise, priced:** `default` solo converts i3 (the board at jobs 2
  missed it) — a ±1 wall-margin row, the board instrument's own variance, not
  a lever.

## 4. 2008 classical leg — 10 unsolved + 3 near-wall witnesses × same four conditions

Board 20/30; the ten unsolved rows plus the three near-wall solves as speedup
witnesses.

| inst | board | default | L3 pair | `FF_NO_NOVLIGHT` | `FF_NO_LAMA` |
|---|---|---|---|---|---|
| i8 | · | · e=32,670 | **S** e=392,091 | · e=91,038 | · e=72,350 |
| i9 | · | · e=28,573 | **S** e=226,325 | · e=24,221 | **S** e=204,821 |
| i10 | · | · e=35,807 | · e=63,199 | · e=19,935 | · e=48,607 |
| i18 | · | · e=23,034 | **S** e=268,776 | · e=33,018 | · e=61,178 |
| i19 | · | · e=14,461 | **S** e=209,206 | · e=29,309 | **S** e=184,630 |
| i20 | · | · e=29,824 | · e=41,088 | · e=31,104 | · e=44,928 |
| i27 | · | S e=174,257 | **S** e=183,729 | **S** e=176,561 | **S** e=208,817 |
| i28 | · | · e=9,502 | **S** e=172,077 | **S** e=149,037 | **S** e=188,973 |
| i29 | · | S e=161,273 | · e=50,077 | **S** e=141,561 | **S** e=185,849 |
| i30 | · | · e=23,724 | · e=37,036 | · e=23,724 | · e=44,972 |
| i6 (witness, 59.52 s) | S | S 59.4 s | S **45.8 s** | S 59.5 s | S 59.5 s |
| i16 (witness, 59.65 s) | S | S 52.4 s | S **45.6 s** | S 59.6 s | S 56.2 s |
| i17 (witness, 59.84 s) | S | S 59.8 s | S 59.8 s | S 59.9 s | S 59.8 s |
| **solved of 13** | 3 | 5 | **9** | 6 | 8 |
| conversions vs board | — | i27 i29 | i8 i9 i18 i19 i27 i28 (**+6**) | i27 i28 i29 (+3) | i9 i19 i27 i28 i29 (+5) |

Reading it:

- **Same mechanism, same shape as 2011.** Converted rows evaluate 5–20× more
  states under the pair (i8: 33k → 392k; i28: 9.5k → 172k). `FF_NO_LAMA`
  carries most of it again (+5); i8 and i18 need BOTH rungs off — on 2008
  the novelty-light 10% is not free either.
- **The pair is not monotone at the wall:** i29 solves under default,
  no-novelty and no-LAMA (141–186k evals) but NOT under the pair (50k evals —
  a different search order that happens to miss inside 60 s). The union of
  the two single-rung arms with the pair is i8 i9 i18 i19 i27 i28 i29 = **+7 of
  the 10 unsolved**; the best single condition is the pair at +6. Any build
  prices against ONE condition per board, so the honest number is +6 (pair)
  with +7 as the ceiling a per-instance schedule could reach and a fixed one
  cannot.
- i10, i20, i30 stay unsolved under everything, with the eval-cliff signature
  (≤63k evals) — the package-count line again.
- Witnesses: the pair buys ~14 s on i6 and i16 (59.5 → 45.8 / 45.6 s); i17
  does not move. Board noise: `default` solo converts i27 and i29 (both
  59.7 s) — two ±1 wall-margin rows in the board's 20/30.

## 5. 2008 tempo-sat numeric leg — instrument-first

Board 4/30 (i1, i2, i11, i12 — all ≤1.0 s). Probed i3–i6, i13, i14 under
`FF_WALL_DEBUG=1 FF_RES_DEBUG=1` with base / `FF_TDEMAND=1` / `FF_TDECOMP=1` /
`FF_NO_TSYMM=1`, plus `FF_TEVAL_BUDGET` slices (10k/30k/100k/300k) on i3 and
i13. **Result: 0 conversions in 32 runs — every row "temporal ladder stopped
at the wall", every condition identical to base to within a second.** The
classical rung hatches do not reach this path (as the spec expected) and
none of the temporal hatches move it either. The eval-budget slices change
nothing visible: no `[search] capped` phase-split line prints on any of the
eight (the cap never fires before the wall does), so the budget is not the
binding constraint.

What the instrument shows (`[tsearch]` on the base rows, run 2, quiet box):

| inst | ops | rel_fluents | tils | nodes/s | popped `g` / time, steady state | avg_helpful |
|---|---|---|---|---|---|---|
| i3 | 1,470 | 79 | 0 | ~27k | g 59–63, t 295–354 | **0.4–0.5** |
| i4 | 2,346 | 87 | 0 | ~10k | g 47–51, t 175–213 | 0.7 |
| i5 | 3,570 | 113 | 0 | ~9k | g 43–44, t 158–206 | 1.1–1.2 |
| i6 | 6,824 | 156 | 0 | ~5.5k | g 46–48, t 290–293 | 2.4–2.6 |
| i13 | 1,464 | 65 | 0 | ~17k | g 18–35, t 279–544 | 0.5–0.6 |
| i14 | 2,376 | 91 | 0 | ~13k | **g 9–12, t 36–93** (never deepens) | 1.0–1.1 |

Named from the receipts, not pitched: the temporal best-first is not slow
(tens of thousands of nodes per second, 250k+ nodes per run on i3) and it is
not starved of budget — it is **guidance-blind**. `avg_helpful` sits at 0.4–1.2
on the small instances: the relaxed plan hands the search fewer than one
helpful action per node, the agenda is 2–3 wide, and the popped frontier
circles at a flat `g` (i3: g 59–63 for 200k+ nodes; i14 sits at g 9–12 and
never leaves). Nothing is time-shaped (`tils=0`) — what the numeric fold
adds is the fuel fluents (`rel_fluents` 65–156, every one relevant), and the
heuristic does not see through them: the plateau is the closed h-accounting
ledger's shape, on a numeric temporal domain. That is the F3 numeric-h gate
by name (AIBR/subgoaling, "Metric-FF-class relaxation blindness"), and it is
NOT a temporal-relaxation exit (the closed ledger stays closed — no `FF_TRPG`
/ `FF_H_ENDGATE` were run, by the fence). The `charge_pre_num` hatch is the
cheaper first probe on the same door and is already gated on C's decode.

## 6. The number: per-board split of the band

The memo's +8–20 aggregated across 2008/2011/mco, split by this sitting under
ONE condition per board (the L3 pair; `FF_NO_LAMA` alone is within one of it
everywhere and is the same lever):

| board | today | under the L3 pair | conversions | ceiling (per-instance union) |
|---|---|---|---|---|
| 2011 transport-seq-sat | 2/20 | 8/20 | **+6** | +6 |
| 2008 transport-seq-sat-strips | 20/30 | 26/30 | **+6** | +7 |
| 2008 transport-tempo-sat-numeric | 4/30 | 4/30 | **0** | 0 |
| mco t2/t4/t8 | 4/5/5 of 20 | not run (read-only leg) | unpriced | the plateau (+1 t2→t4, +0 t4→t8) says cores are not the lever; the rung tax plausibly is — same probe, not run here |

**Aggregate classical split: +12, inside the +8–20 band, on exactly the
boards the 0.25 decode allowed** (2008/2011; 2014 untouched by construction
and its eval-cliff signature reproduced on the 2011 tail). Referee: the
converting condition is a driver/search-order claim; the roadmap-0.21 rule
is met by identity — probe and board ran the same `ff 0.25.0`, threads 1,
same corpus files; the 0.25.0 tag and the worktree binary are the same
bytes. What the conversions are NOT: a speed story. Every converted row is
the fallback given 3–20× more evaluations inside the same wall because two
rungs that never return on transport stopped taking their fixed slices.

**The 2008 tempo-sat overtake hypothesis (memo §1c: "~+10 on the 2008
tempo-sat board specifically") is REFUSED at this sitting.** Its board moved
0 under every existing hatch and the instrument names why (§5). The band's
2008 share lives on the classical board, which does not count toward the
overtake by the sitting's own pricing rule.

## 7. Gate verdict (F3 transport build)

**OPEN for the classical L1–L3 build, priced +12 (2011 +6, 2008 classical
+6; mco unpriced), lever = the rung tax (LAMA 25% + novelty-light 10% wall
slices), fixture class = 2011 i1/i2 and 2008 i8/i18 (convert only with both
rungs off). CLOSED for the 2008 tempo-sat overtake — 0 conversions, refused
and re-priced to 0 from the receipts; the leg's mechanism (guidance-blind
numeric temporal search, avg_helpful < 1) re-routes to the F3 numeric-h
gate, not to a transport build.** Fences carried: the build ships its own
RED fixture and `FF_NO_*` restore, armed at the 0.26 cut with `standings.py`
/crucible as referee; a per-board rung schedule is a budget reallocation
and prices only after the old-binary referee (dossier F5 law); 2014
transport stays unclaimable.
