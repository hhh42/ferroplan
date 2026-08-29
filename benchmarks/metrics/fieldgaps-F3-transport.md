# F3 §3 — transport: the rung tax, decoded to the rung that converts (2026-08-29)

Spec: `docs/field-gaps-execution-0.26.md` F3 §3 (L1 `FF_COSTH_FIRST`, gated on
the widened probe). Gate-opener: Sitting D (`fieldgaps-D-transport.md` §7):
"OPEN for the classical L1–L3 build, priced +12, lever = the rung tax (LAMA
25 % + novelty-light 10 % wall slices)". Receipts:
`benchmarks/air26-probes/transport-arrival/` (rows.jsonl, one `.err` per run,
engine `5f985cf51f48`/`af98f98131554a21` = 0.26.0 with F1 + ladder dedup;
60 s, `FF_WALL_DEBUG=1`, solo on a quiet box after Timberborn exited).

## What the default ladder does on transport (probe 1, the trace)

2011 i1: novelty-light 5.9 s → LAMA base slice extended twice by the recency
rule on drops 222 → 16 → 0, out at 26.0 s → novelty-driver 7.9 s → fallback
90k evals to the wall. i2/i6/i7/i11 and 2008 i8/i18 the same shape: LAMA
18–24 s, the driver ~8 s, the fallback the rest, nothing converts.

## Probe 1 — refusing LAMA's drip (arrival) or shrinking its slice

| cell | 2011 i1 i2 i6 i7 i11 | 2008 i8 i18 | spider i1 (canary) |
|---|---|---|---|
| default | · · · · · | · · | S 60.1 s |
| `FF_LAMA_EXT_ARRIVAL=1` | · · · · · | · · | **· (lost)** |
| `FF_LAMA_WALL_FRAC=0.10` | · · · · S | · · | S |
| both | · · · · · | · · | S |

The arrival flip is DEAD: 0 conversions and it loses spider i1 exactly as the
0.24 record said it would. `frac10` converts only i11 — **by the
novelty-driver** — which is the thread the next probe pulled.

## Probe 2 — which rung converts under Sitting D's `FF_NO_LAMA`

| cell | 2011 i1 | i2 | i6 | 2008 i8 |
|---|---|---|---|---|
| `FF_NO_LAMA` | **S novelty-driver** 58.4 s | S driver 56.4 s | S driver 59.2 s | S driver 58.4 s |
| `FF_NO_LAMA FF_NO_ENRICH` | · | S driver | S driver | S driver |
| `frac10 FF_NO_ENRICH` | · | S driver | S driver | S driver |
| `frac10 FF_NO_NOVELTY` | · (370k fallback evals) | · (235k) | · (202k) | — |
| bare (no LAMA/novelty/light/enrich) | · (466k) | · (354k) | · (275k) | — |

**Every conversion is the novelty driver's.** Without the driver the fallback
fails at 2–3× the eval counts Sitting D quoted as the converting budget: the
"evaluated" numbers in that sitting's `FF_NO_LAMA` receipts were the
post-plan COST SWEEP (transport has action costs; `wall: best-first checkpoint
expired` fires after `wall: solved by novelty-driver`), not the first-plan
fallback. The correction to the decode: LAMA's slice is the tax, but what it
starves is the DRIVER, whose 0.30 share of the remaining wall (~8 s after
LAMA) is just under transport's pop budget (i1 needs ~16 s of driver).

## Probe 3 — the driver's slice, LAMA kept

| cell | 2011 i1 i2 i6 i7 i11 | 2008 i8 i18 | spider i1 |
|---|---|---|---|
| `FF_NOV_WALL_FRAC=0.50` | **S S S S S** (59.0/59.2/59.4/59.6/48.3 s) | · **S** | S 44.2 s |
| `FF_NOV_WALL_FRAC=0.70` | S S S S S | **S S** | S 44.3 s |

Plan lengths identical across cells (140/153/276/241/197; 2008 i8 140, i18
276): the driver finds the same plans, only the wall decides.

## Verdict — BUILT as a default move, refereed by the cut

`FF_NOV_WALL_FRAC` default **0.30 → 0.50** (`search.rs`, the novelty slot's
slice; `FF_NOV_WALL_FRAC=0.30` restores). The 0.22 record priced this exact
knob ("at load the 0.30 default is marginal (0.5 converts loaded) — the sweep
referees the knob"); 0.70 buys one more row (2008 i8) but leaves the fallback
30 % of the remaining wall on every board, which is the +7/−51 shape the 0.17
referee killed — not taken. Priced from the probes: **+6 solo on 2011
transport (2/20 → 8/20), +1 on 2008 (i18)**, against Sitting D's +12 band
(2011 +6 in, 2008 +1 of +6). Fragility, stated: four of the 2011 conversions
land at 59.0–59.6 s of a 60 s wall; at the board's jobs 2 some will miss. The
0.26 cut sweep (like-for-like vs the 0.25.0 rows = the old-binary referee the
F5 law requires for a budget reallocation) prices the default on all 32 boards;
tetris i4 (the 0.22 witness for 0.5 under load) rides along.

`FF_COSTH_FIRST` (L1 proper, the cost-augmented first plan) is NOT built: the
decode's mechanism 3 ("first-plan search is cost-blind") is not what stops the
conversions — the driver is h-free and cost-blind and converts anyway. L1 stays
a quality lever (first plan ~2× the swept cost) with its own spec, unpriced
for coverage.
