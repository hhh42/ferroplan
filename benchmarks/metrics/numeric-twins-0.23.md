# Numeric twins sitting — 0.23 Phase 6: markettrader + pathwaysmetric

**Provenance.** Sitting run 2026-08-10 against `target/release/ff`
reporting **0.22.0** (0.23-dev `main` @ `0576f1a`, wave-1 integrated).
Solo sequential probes at exact board env: `nice -n 15`,
`FF_TIME_LIMIT=60`, `FF_MEM_BUDGET_GB=6`, `--json --threads 1`,
`FF_NO_ESCALATE=1`, `FF_WALL_DEBUG=1` + `FF_RES_DEBUG=1`, external
kill at 65 s; idle 79–89% on every run. These are **wall-shaped**
reads by design — the deliverable is where the wall goes — with eval
masses cited from the JSON statistics (deterministic). The twins are
38 board rows that were never in any pot map; this sitting gives them
their first mechanism-precise decode.

**Boards checked first, as charged:** `benchmarks/ipc2023-numeric.jsonl`
holds pathwaysmetric at **1/20** (i1 only, 12 steps / 4,710 evals —
the byte-identity receipt row) and markettrader at **1/20** (i3, 4.37 s
/ 453 steps). The air21 → air22 family diff confirms **0.22's +8 on
this board was block-grouping's (2/20 → 10/20); pathwaysmetric moved
0 (1/20 → 1/20) and markettrader moved 0 (1/20 → 1/20).** Neither
twin has ever moved except markettrader's VAL adjudication (0.22
Phase 1: VAL type-check-refuses the INSTANCE files — undeclared
`fuel`/`fuel-used` fluents from a commented-out metric; the 453-step
plan hand-replays valid; NOT ours, harness knows both signatures) and
pathwaysmetric i1 (0.21 lever a1).

Classes: **PLATEAU / GRIND / BLOCKED / MEM / SCALE / MIXED**, the
standing vocabulary. Neither family routes partition mode (`"mode":
"ff"` on all six runs) — the partition machinery is acquitted of the
wall by construction.

## Per-family attribution

| family (unsolved mass) | instances probed | class | key numbers | 0.24 lever implicated |
|---|---|---|---|---|
| markettrader (19 timeouts incl. 4 mem-caps {11, 14, 17, 20}; i3 solved; VAL row adjudicated 0.22) | i1, i11, i20 — full-wall board-env runs | **PLATEAU** (h-gradient-free churn at full throughput; MEM co-factor self-inflicted at the ramp) | The wall goes to SEARCH, and search goes nowhere: i1 (71 ops / 4 facts) burns 300k novelty-light evals + 400k novelty-driver pops (both capped), then **5,122,888 best-first evals across 4 refill rounds** — w_h escalating 5 → 20 → 80 → 320 changes nothing; i11 (145 ops): 2.96M best-first evals / 5 rounds; i20 (185 ops): 2.22M / 5 rounds. Throughput **48–105k evals/s** — NOT the constraint. Wall split inside best-first (i1): h 19.1 s / expand 12.3 s / insert 15.0 s of 47.4 s; cumulative h-BUILD only 13.4 s — no h-build wall, no grounding wall (<0.1 s), no refill-machinery wall (round transitions instant). Early rounds die on the **node-byte cap** and the re-entry raises the byte target ×2/×4 — RSS lands at **6.31 / 6.86 / 7.60 GB against the declared 6 GB** (peak footprint 12.7 GB on i11): the board's four mem-cap rows are the node-raise re-entries overshooting, not a distinct memory mechanism | **None-known for coverage — the 0.21 re-attribution stands, now with the mechanism receipt:** cyclic resource flow (LP-RPG's own paper domain, field best 2/20); millions of evals at five greed levels find no gradient because the delete relaxation cannot see buy-low/sell-high cycles. No throughput, memory, or machinery lever touches that. One hygiene rider named for 0.24: on numeric tasks the ×2/×4 node-raise re-entry converts honest timeouts into mem-cap labels (RSS 27% past budget) — cap the raise or stamp the note, so the boards stop reading self-inflicted MEM |
| pathwaysmetric (19 timeouts incl. mem-cap {20}; i1 solved since 0.21) | i2, i10, i20 — full-wall board-env runs | **MIXED** (low ramp: h-blind churn, the chained-DAG hole; high ramp: SCALE+MEM — the byte model strangles every rung) | The best-first eval mass a wall buys **collapses 2,835,830 → 339,154 → 49,584** as the grounding ramps 74 → 276 → 767 ops (novelty rungs ride on top of each). i2 (74 ops — i1's size-class twin): 2.84M evals over 3 refill rounds, no contact — the 0.21 a1 charge that cracked i1 (4,710 evals) does not reach one instance up. i10: novelty-light dies on the BYTE cap at 55,780 evals / 611,239 nodes (vs its 300k eval budget). i20: enters the rung ladder with only **62.5% of the wall left** (22.5 s pre-rung in the un-narrated EHC slot at 767 ops), novelty-light byte-caps at **9,005 evals / 226,581 nodes**; the whole refill ladder totals 49,584 evals. Same 6 GB model caps 611k nodes (i10) but only 227k (i20) — nodes ~2.7× heavier at 767 fluents; the 0.21 static-fluent fold (defined-static + irrelevant only) does not reach reaction tables the h reads live. RSS 6.92 / 7.56 / 7.60 GB | Two, both priced in the record already: **(1) lever a2 — the chained numeric-precondition charge** (recurse charged achievers' own `pre_num`, depth-capped, damping included) is what i2's numbers implicate: a1 charges one level, the reaction DAG chains deeper, and 0.21 skipped a2 only because i1 moved without it. The fo-sailing SUM/first-wins receipts (dockets §2) are the design constraint it inherits. **(2) node economy for read-live fluent tables** at the ramp — the fold's exclusion is the measured 2.7× — plus the EHC wall-slice question on wide numeric tasks (22.5 s un-narrated is a 0.23 Phase 2-style honesty gap before it is a performance one) |

## What the numbers rule out

- **Throughput is not markettrader's problem:** 48–105k evals/s,
  2.2–5.1M evals per wall on 71–185-op groundings. A faster eval loop
  multiplies churn.
- **h-build is not the wall on either twin at the low ramp:** 13–14 s
  of cumulative worker h-BUILD time inside ~47 s of search; grounding
  is instant everywhere (71–767 ops, 4–767 facts).
- **Refill and partition machinery are acquitted:** round transitions
  are instant; neither family routes partition mode; the espc/RESLM
  paths never arm. The refill loop does exactly its job — it spends
  the wall; the wall just buys nothing here.
- **pathwaysmetric's twins diverge:** it shares markettrader's
  full-wall churn shape ONLY at the low ramp; from i10 up the
  mechanism is different in kind — the byte model, not the heuristic,
  decides how many evals a wall buys. Any 0.24 a2 receipt must
  therefore be read on i2–i9, not the ramp.
- **The +8 provenance is settled:** block-grouping's, on the boards
  (air21 2/20 → air22 10/20; both twins flat at 1/20 across the same
  diff). Nothing about 0.22 moved these families.

## Feed into 0.24's lever choice (no engine lever taken here)

markettrader stays honestly outside every pot — the sitting converts
the 0.21 literature re-attribution into a mechanism receipt (five
greed levels, five million evals, no gradient) and prices its four
mem-cap rows as label hygiene, not coverage. pathwaysmetric splits:
i2-class rows are the cheapest counted case on the numeric side —
lever a2 was designed and deferred at 0.21 with its damping caveat
already written, and this sitting supplies the RED instance (i2,
2.84M evals, 74 ops) a fixture pair can pin; ramp rows wait on node
economy for read-live fluent tables and are not promised. The 0.22
+8 mythology is corrected on the record: block-grouping's, not the
twins'.
