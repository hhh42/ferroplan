# Sitting F0(b) — the cliff decode (0.26)

Executed 2026-08-29 05:22–07:40, sitting F0(b) of
`docs/field-gaps-execution-0.26.md` (Sitting B + the amendment dropping
`FF_NOV_LAZYH`). Binary: `target/release/ff` = **ff 0.26.0 candidate** (F1
enrichment default-on; F2 removed). Box: **clean** by the watcher (idle median
87.7%, competitors ≤ 4% — corespotlightd, the Docker VM, node), **0 of 165
rows starved** (the driver's per-row `utime ≥ 0.9 × wall` gate; quiet-gated
entry; no retries needed). Receipts:
`benchmarks/metrics/probes-0.26/B-cliff/` (per-run json + stderr, `matrix.jsonl`
with the `wall: solved by …` rung stamped per row, `progress.log`,
`contention.json`). No code changes; every condition is an existing hatch.

The claim under test: the Scorpion-Maidu ingredients — novelty with
FORGETTING, and ALTERNATION across different heuristics' queues. The
deliverable is the mechanism forgetting/alternation would fix in THIS engine,
per family, or refusal. **Rung attribution is by narration, never by the
raw's "used weighted best-first" note** (the F1/F2 correction on the record).

## 0. The matrix (solved · wall · evals · rung)

`S` solved, `·` unsolved. Conditions: default; the rung-isolation hatches
(`FF_NOVLIGHT_ONLY`, `FF_NOVDRIVER_ONLY`, `FF_NOVELTY_ONLY`, `FF_NO_LAMA`,
`FF_NO_NOVLIGHT`, `FF_NO_REFILL`); the forgetting ablations (`FF_NOV_R_CAP`
64 / 1024 against the default 256, `FF_NOV_PART=0`, `FF_NOV_W2=0`); the
alternation probe (`FF_CLM=3`); eval slices (`--max-evaluated` 30k/100k/300k).

| inst | default | light-only | driver-only | novelty-only | no-LAMA | no-light | no-refill | R-cap 64 | R-cap 1024 | part 0 | w2 0 | CLM 3 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| rubiks i5 | S 8 s LAMA | S 13 s | · 61 s | **S <1 s (574 ev)** | S 24 s fb | S 2 s LAMA | S 8 s | S 8 s | S 8 s | S 8 s | S 8 s | S 8 s |
| rubiks i6 | · 59 s 85k | · 78k | · 1 ev | **S <1 s (814 ev)** | · 118k | · 96k | · 85k | · 85k | · 85k | · 86k | · 87k | · 85k |
| rubiks i7 | · 59 s 84k | · 73k | · 1 ev | · (whole wall) | · 109k | · 94k | · 82k | · 84k | · 82k | · 84k | · 84k | · 84k |
| floortile i7 | · 57 s 9.7M | · 8.7M | · 8.6M | · 4.9M | · 10.0M | · 9.8M | · 31 s 5.0M | · 9.7M | · 9.7M | · 9.7M | · 9.6M | · 9.7M |
| floortile i9 | · 58 s 9.0M | · 8.0M | · 7.7M | · 4.0M | · 9.3M | · 9.1M | · 34 s 5.0M | · 9.0M | · 9.0M | · 9.1M | · 9.0M | · 8.6M |
| floortile i10 | · 57 s 9.1M | · 8.2M | · 7.4M | · 4.2M | · 9.4M | · 9.1M | · 33 s 5.0M | · 8.9M | · 8.9M | · 8.6M | · 9.1M | · 9.1M |
| spider i1 | S 60 s LAMA | S 60 s LAMA | · | · | S 60 s fb | S 60 s LAMA | S 60 s | S 60 s | S 60 s | S 60 s | S 60 s | S 60 s |
| spider i9 | S 60 s driver 37.6k | · | S 48 s | S 60 s | S 54 s driver | S 60 s driver | S 60 s | **· 14.7k** | **S 27.9k** | **· 15.3k** | **· 14.3k** | S 60 s |
| labyrinth i1 | S 60 s driver 30.5k | S 60 s driver | **S 35 s** | S 40 s | S 50 s driver | S 56 s driver | S 60 s | S 60 s | S 59 s | **·** | S 60 s | S 60 s |
| slitherlink i4 | S 47 s fb 1.18M | S 52 s fb | · 883k | S 39 s | **S 36 s fb** | S 44 s fb | S 48 s fb | S 46 s | S 46 s | S 50 s | S 48 s | S 48 s |
| recharging i6 | S 1 s light | S 1 s | S 1 s | S 1 s | S 1 s | S 1 s LAMA | S 1 s | S 1 s | S 1 s | S 1 s | S 1 s | S 1 s |

Eval slices (default ladder): rubiks i6/i7 spend 26–27 s reaching 30k evals
and stay unsolved at every slice; floortile reaches 30k in 0–1 s, 300k in
4–5 s, unsolved at every slice (the plateau is FAST and flat); spider i1
converts between 30k and 100k evals; spider i9, labyrinth and recharging
solve inside 30k; slitherlink needs > 300k (solves at 1.18M).

## 1. rubiks — NAMED: the slot runs the wrong novelty rung (alternation, not forgetting)

The purest cliff on any board decodes to one line of the narration. Under the
default ladder rubiks i6 spends its wall exactly as the ladder schedules it —
novelty-light 6.15 s (57k pops), LAMA 13.2 s (43k evals, recency refused),
**the novelty-DRIVER 12.2 s (110k pops)**, then the fallback caps at 85k evals
at the checkpoint — and never solves. Under `FF_NOVELTY_ONLY=1`, which runs
the **h-guided `novelty.rs` rung** over the whole wall, **i6 solves in 814
evaluations and i5 in 574 — well under a second** (i5's default LAMA solve is
8 s). The default ladder cannot reach that rung: since 0.22 Phase 5B lever 3
the post-LAMA slot runs the partitioned h-FREE driver in its place
(`search.rs`, the `FF_NOV_OLD=1` swap), on the parking receipt (86 s of
worker time building h at 100k evals). Rubiks is the domain where that trade
loses: the driver's structural novelty (110k pops, nothing) cannot walk what
the h-guided rung walks in 814 evals — relaxed h IS informative on rubiks and
the driver throws it away. **Mechanism, in the engine's vocabulary: the
novelty slot's h-free driver is the wrong queue for this family; the
h-guided rung is the right one and costs nothing here.** This is the
alternation question answered concretely — not "alternate two heuristics'
queues inside one search" but "the slot must be able to try BOTH rungs" —
and it is NOT a forgetting question: `FF_NOV_R_CAP` 64/1024, `FF_NOV_PART=0`,
`FF_NOV_W2=0` and `FF_CLM=3` move nothing on i6/i7 (85k ± 2k evals, all
unsolved).

The limit is named too: **i7 does not fall even to the h-guided rung** — under
`FF_NOVELTY_ONLY=1` it eats the whole wall unsolved (the ladder then skips
every other rung as unaffordable). So the band this decode opens is the
instances between i5's shape and i7's, priced by the follow-up below, not the
whole 15-instance cliff.

**Follow-up probe queued (B2, `benchmarks/metrics/probes-0.26/B2-price/`):**
the rubiks board (i1–i20) under `FF_NOV_OLD=1` at `FF_NOV_WALL_FRAC` 0.05,
0.10 and the default 0.30, against the default ladder — the number the
rung-schedule build (h-guided rung in a bounded slice before the driver, or
the two alternating at the slot) is gated on, and the 0.17 −51 ledger is the
fence it must clear on the other boards.

## 2. spider — NAMED: the driver's |R| cap saturates (the forgetting lever, with the sign it should have)

spider i9 is a wall-edge driver solve by default (60.0 s, 37.6k evals). The
`|R|` cap is the one knob that moves it, in both directions: **cap 64 loses it
(14.7k evals at the wall), cap 1024 solves it with 27.9k evals — 26% fewer
than the default 256** — and `FF_NOV_PART=0` and `FF_NOV_W2=0` lose it too
(the partition and width-2 levers are load-bearing here). A cap that changes
the outcome monotonically is a cap that binds: the driver's novelty table
fills on spider, and a state's novelty is judged against a saturated,
truncated R. That is precisely the mechanism forgetting addresses — but the
receipt says the cheap version of the fix is simply a LARGER R on this
family, not eviction. spider i1 is LAMA's (60.0 s, also a wall-edge solve)
and the driver does not reach it under any hatch.

**Follow-up probe queued (B2):** spider i1–i6 (all unsolved on the board)
under `FF_NOV_R_CAP` 1024 and 4096 against the default — the number a
per-family R-cap (or a forgetting policy) is gated on.

## 3. labyrinth — NAMED: the ladder tax (the rung is right, the schedule is slow)

labyrinth i1 solves by the driver in every arm that reaches it, and the time
it takes is the time the ladder spends getting there: **35 s driver-only,
40 s novelty-only, 50 s with LAMA off, 56 s with novelty-light off, 60.0 s
(wall edge) by default** — the 25 s of light + LAMA ahead of the driver are
pure tax on this instance, which is why the board reads 0/20 with i1 at the
wall. `FF_NOV_PART=0` loses it (the partition is load-bearing, as on
spider); the R cap does not bind (30.3k–30.5k evals at 64/256/1024). No
forgetting question here; the schedule is the lever, and the same B2 arms
price it implicitly (a slot that reaches the driver sooner).

## 4. floortile — REFUSED: a fast, flat plateau that no rung, cap or term moves

Every condition fails i7/i9/i10 in the same shape: the ladder hands down
through light/LAMA/driver and the fallback then evaluates **9–10 million
states in ~57 s** (the enriched fallback at full speed) without finding a
plan; `FF_NO_REFILL` stops at 5.0M evals / 31–34 s, so the refill loop is
spending the second half of the wall on the same plateau. The eval slices
say the plateau is reached instantly (30k evals in 0–1 s, 300k in 4–5 s) and
is flat at every depth; the novelty rung alone evaluates 4–5M and fails the
same way; `FF_NOV_R_CAP` 64/1024 and `FF_CLM=3` change the eval count by
< 2% and nothing else. **Neither forgetting nor alternation names a
mechanism here** — the 0.22 read (best_h flat, dedup 0.0%) stands, and the
one lever still on the table is the irreversible-consumption dead-end test
(the F4 floor-tile probe), which is not this sitting's. Refused for this
family; the rung stays un-built on floortile's account.

## 5. slitherlink and recharging — no cliff to decode

slitherlink i4 is a fallback solve at 47 s (1.18M evals) and the only thing
that moves it is removing LAMA (36 s: an 11 s tax, the F1 story's other
half); the driver alone fails it. recharging i6 is a 1 s novelty-light solve
under every condition — it was on the list as a 5/20 board's residue, and
the residue is not this instance.

## 6. Verdict for the rung (F3's forgetting/multi-heuristic gate)

**Named, per family, with the number that shows it — and the rung as
SPECIFIED is refused.** The "forgetting + alternation" rung the field
evidence proposed is not what these cliffs want. What they want is narrower
and cheaper: (rubiks) the h-guided novelty rung back in reach of the ladder
in a bounded slice — an alternation of RUNGS at the slot, not of queues
inside one search — priced by B2 against the 0.17 −51 fence; (spider) a
larger `|R|` for the driver where its table saturates, priced by B2; (labyrinth)
a schedule that reaches the driver sooner, which B2's slot arms also price.
Floortile refuses. **Gate verdict: the forgetting/multi-heuristic rung does
NOT open on this decode; a rung-schedule build (h-guided-then-driver at the
novelty slot, bounded) and a per-family R-cap are the two candidates, each
gated on B2's number, each carrying an `FF_NO_*` restore and the old-binary
referee, and each dying the same day if a board A/B reads like 0.17.**
