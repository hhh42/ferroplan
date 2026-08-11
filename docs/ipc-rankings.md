# Where ferroplan ranks — a rough field placement, per year and track

This is not a scoreboard — [`STANDINGS.md`](../STANDINGS.md) and
[`benchmarks/ipc-standings.md`](../benchmarks/ipc-standings.md) are that, generated
from measured runs, and remain the only authoritative numbers. This page answers a
different, softer question: **if ferroplan's own measured coverage on each
competition's corpus were dropped into that competition's actual field, roughly
where would it land?**

Every placement here is retrospective and informal — ferroplan did not compete in
any of these events. Read every row against three standing caveats before the
per-track ones:

- **Budget mismatch, almost always unfavorable to ferroplan.** Official IPC budgets
  are typically 30 minutes (1800 s) per instance; ferroplan's own boards are swept
  at 60 s satisficing / 30 s temporal (300 s only for the one explicit
  OFFICIAL-BUDGET agile entry). Where noted, this makes a placement a *floor*, not
  a ceiling.
- **Hardware and calendar are not neutral.** Older competitions (2006–2011) ran on
  hardware roughly 10-20 years behind today's; a modern build is fast by default,
  which flatters coverage-based placements against that era's field. This cuts the
  other way for nothing here — it is a real, unremovable confound, named rather
  than corrected for.
- **Coverage ≠ the competition's own scoring formula**, most of the time. IPC
  scoring is usually quality-weighted (`C*/C` per instance, or speed-decay for
  agile) rather than plain solved-count. Where ferroplan's own recorded metric is
  plain coverage, a coverage-based placement is an approximation — it tends to
  *overstate* the true quality-weighted score, since quality ≤ coverage. Flagged
  per row.

Confidence is graded **high** (official per-instance data, matched corpus and
metric), **medium** (official data with a real caveat — budget, corpus, or metric
mismatch), or **low** (thin field, reconstructed numbers, or an approximated
metric).

## IPC-5 (2006)

| track | ferroplan (0.22.0) | field | rough placement | confidence |
|---|---|---|---|---|
| simple-preferences | beats the winner: 19W/16T/13L vs SGPlan5 on p01-p08 | SGPlan5 won all 6 domains outright (6/0); MIPS-XXL/MIPS-BDD/YochanPS never took a domain | **competitive for 1st** — ahead of the 2006 winner in aggregate, wins openstacks/storage/rovers outright | high |
| qualitative-preferences | beats the winner: 24W/4T/10L vs SGPlan5 | SGPlan5 swept all 5 domains (100% coverage each); HPlan-P a distant 2nd (70/100) | **at or above 1st** — wins rovers/storage/tpp outright | high |
| propositional (re-entered at 0.22) | 366/450 (81.3%) | on the official 220-instance typed corpus: SGPlan5 ~218/220 (99%), Downward-reference ~178/220 (81%), MIPS-XXL ~68/220 (31%), YochanPS ~41/220 (19%) | **~2nd of 4 by rate** — essentially tied with Downward-reference, clearly behind SGPlan5's near-sweep, clearly ahead of MIPS-XXL/YochanPS. (ferroplan's board is a larger corpus than the field's 220, so this is a rate comparison, not raw count.) | medium |
| constraints (re-entered at 0.22) | 5/120 (4.2%) — the 100 constraint-bearing rows died at ONE enforcement site (the gate refusing any `(:constraints ...)` block on a durative domain); the parser PARSES all four rejected variants, so the decline is enforcement, not parsing. 0.23 Phase 2 opened the gate for the untimed operators (30 rows now attempt: storage's at-end folds solve and VAL green; tpp's monitor rows search honestly); the 40 timed rows (`within`/`always-within`) keep the named rejection pending stage c | thin field, 3 officially-scored domains (80 instances): SGPlan5 47/80 (58.8%), MIPS-XXL 13/80 (16.3%) | **last of 3** — well behind even the weaker entrant; the honest floor of a feature gap, not a tuning gap | low |
| time | not attempted this cycle | SGPlan5 dominant (80 raw solves on the Time folder); YochanPS/MIPS-XXL distant | out of scope — not re-baselined | — |
| metric-time | not attempted this cycle | SGPlan5 dominant (151 raw solves); MIPS-XXL/YochanPS distant | out of scope — not re-baselined | — |
| complex-preferences | cannot attempt (the modal operators PARSE; the track's preference bodies lean on the timed ones — `within`/`always-within` — whose enforcement is the named stage-c feature, docs/roadmap-0.23.md Phase 2) | SGPlan5 swept all 5 domains (105 raw solves); MIPS-XXL 2nd (25) | **last of 3**, until the feature ships | high |

## IPC-6 (2008)

| track | ferroplan (0.22.0) | field | rough placement | confidence |
|---|---|---|---|---|
| seq-sat | **284/300 (94.7%)** | LAMA (winner) 281/300 (93.7%); FF(h_sa) runner-up 225/270; field of 10 down to Plan-A 37/180 | **clears the official winner by raw count** — on this exact 300-instance denominator, ferroplan's coverage exceeds LAMA's | high |
| seq-opt (proof rate) | **150/270 (55.6%)** | Gamer (winner) 134/270 (49.6%); HSP*F runner-up 132/270; field of 8 down to CFDP 24/240 | **clears the official winner by raw count** | medium |
| tempo-sat | 298/390 (76.4%) | SGPlan6 (winner) 318/390 (81.5%); Temporal Fast Downward runner-up 257/390; field of 6 down to TLP-GP 6/120 | **2nd of 6**, close behind the winner, clearly ahead of the actual runner-up | high |
| net-benefit (re-entered at 0.22) | **248/270 (91.9%)** — this cut's strongest board by percentage | thin field, optimization only (satisficing subtrack cancelled): Gamer (winner) 81/210 (38.6%); Mips-XXL 59/210 (28.1%); HSP*P 51/210 (24.3%) | **far ahead of the field** by rate, though on a differently-sized corpus (270 vs 210) | low-medium |

## IPC-7 (2011)

| track | ferroplan (0.22.0) | field | rough placement | confidence |
|---|---|---|---|---|
| seq-sat | 220/280 (78.6%) | 27 entrants; LAMA-2011 (winner) 250/280; field spans FDSS-2/PROBE 233, FDSS-1 232, FD-AUTOTUNE-1 223, ROAMER 213 ... down to ACOPLAN 20/280 | **~6th-7th of 28**, between FD-AUTOTUNE-1 and ROAMER — solidly upper-third | high |
| seq-opt (proof rate) | 127/280 (45.4%) | 12 scored entrants; FDSS-1 (winner) 185/280; field clusters IFORKINIT 144 down to CPT4 44/280 | **~12th of 13**, between IFORKINIT and CPT4 — ahead of only the trailing entrant | high |
| tempo-sat | **121/240 (50.4%)** | 8 entrants; YAHSP2-MT 145/240 (raw leader), YAHSP2 137, DAEYAHSP (winner by score) 136, POPF2 119 (joint official runner-up), LMTD 62 | **~4th of 9** — now edges past POPF2, one of the two official joint runners-up, by raw coverage | high |
| seq-mco (t2/t4/t8) | not attempted this cycle | wall-clock-per-core competition rule, 4-core box | out of scope — not re-baselined | — |

## IPC-2014

| track | ferroplan (0.22.0) | field | rough placement | confidence |
|---|---|---|---|---|
| seq-sat | 147/280 (52.5%) | 20 entrants; IBaCoP2 (winner) ~198/280; 7 confirmed-coverage entrants span 163-198; a quality-score tier (Cedalion/ArvandHerd/FDSS-2014/DPMPlan) reads 125-137; 9 more entrants unlocatable | **roughly 10th-14th of 20** — now sits above the located quality-score tier, though 9 entrants' true numbers remain unknown | medium |
| seq-agile | 141/280 (50.4%) | 15 entrants scored by runtime, not coverage; YAHSP3 wins (score 81.2); reconstructed (non-official) coverage bands ~66-159/280, IBaCoP2/Jasper ~136-147 | **roughly mid-field** on the reconstructed coverage band | low |
| seq-opt (proof rate) | **74/256 (28.9%)**, +16 this cut — the cycle's single biggest mover | 17 entrants; SymBA*-2 (winner) 151/280; field spans down through Gamer 83/280 to Hpp 14/280 | **~11th-12th of 17**, now nearly matching Gamer (normalized ~81/280) — up from clearly-behind to essentially tied | high |
| tempo-sat | 67/200 (33.5%) | only 6 entrants; YAHSP3-MT wins, TFD runner-up — no per-planner numbers locatable anywhere despite extensive search; official site is dead | cannot be estimated — field size and winner known, nothing more | unknown |

## IPC-2018

| track | ferroplan (0.22.0) | field | rough placement | confidence |
|---|---|---|---|---|
| seq-sat | 79/240 (32.9%), matched subset | 24 entries on the matched 240; field mean 94.3/240 (39%), median 91/240; winners Fast Downward Stone Soup/Remix (joint); low tail: fs-sim 70, fs-blind 60, freelunch-madagascar 23, alien 15, Symple-1/2 14 | **lower-third of the field**, but now clearly past the bottom cluster (fs-sim and below), closing on the field mean | high |

*Correction on the record: Delfi did not win IPC-2018 satisficing — it won only
the separate cost-optimal track. The satisficing winners were Fast Downward
Stone Soup 2018 and Fast Downward Remix.*

## IPC-2023

| track | ferroplan (0.22.0) | field | rough placement | confidence |
|---|---|---|---|---|
| classical (satisficing) | 36/140, mean quality 0.78 vs best-known bounds (36 scored) | 23 configs, official quality-weighted SUM; Scorpion Maidu/Levitron joint winners (~71.8); field spans down to hapori-greedy (0.00) | **~20th-21st of 24** on an approximated proxy score (coverage × mean quality ≈ 28); the 60s-vs-1800s gap (30×) makes this a pessimistic floor, not measured at parity | low |
| agile (300s, OFFICIAL BUDGET) | 52/140 (37.1%) | 23 configs, speed-decay SUM; DecStar-2023 wins (40.25); field spans down to hapori-greedy (0.00) | the one track where ferroplan's local budget matches the official rule exactly, but organizers never published raw solved-counts, only the derived SUM — no rank computable | low |
| numeric | **251/400 (62.8%)** | only 2 real teams (5 configs) entered; best true competitor NLM-CutPlan 136/400 (34%); ENHSP ran as a non-competing reference (hmrp 191, hmrp+ha+ht 264, hmrp+ha 267/400) | **clears every real competitor by a wide margin** and now sits between the two stronger ENHSP reference configs — near the top of the whole comparison set, competitors and references combined | medium |

## IPC-2026

| track | ferroplan (0.22.0) | field | rough placement | confidence |
|---|---|---|---|---|
| numeric (satisficing subtrack, matched 13-domain/260-instance slice) | **136/260 (52.3%)** raw coverage | 11 ranked rows incl. 2 reference baselines; Panino-anytime wins (177.9/260 quality score); field spans a ~60-130 mid band down to Tyr-sat-lifted (40.3/260) | **roughly 4th-6th of 11** on a coverage-only proxy — likely optimistic since quality ≤ coverage, but clears the visible mid-band on raw count alone | medium |
| numeric-opt (Overall Optimal subtrack, 3 shared domains) | **22/60 — ties the official track winner** | Tyr-opt-ground (winner) 22/60 on this slice; Tyr-opt-lift 21/60; A*(h^blind) baseline 16/60 | **ties for 1st of 3** on the slice actually run, at 1/30th the official budget | medium-high on this narrow slice; low on the full 260-instance track (not run) |
| epistemic | not attempted — a different planner class (EPDDL/DEL), explicitly a watch item, not a gap | IPC-2026's first-ever epistemic track (organizers Burigana & Fabiano); results announced at ICAPS 2026 but no scoreboard was locatable | not applicable | — |

## How this page is kept honest

- Every field number above traces to an official results page, an official
  results archive/CSV, a peer-reviewed competition overview paper, or (for the
  two competitions whose primary sources are dead — 2008/2011) a Wayback capture
  or peer-reviewed re-run. No placement here is from memory.
- Where no usable field data exists (IPC-5 time/metric-time, IPC-2014 tempo-sat),
  the row says so rather than guessing.
- This page is refreshed by hand alongside a cut sweep, not generated — treat its
  "ferroplan" figures as current as of the cut date below, and the field figures
  as fixed history that will not change.
- Full source list (URLs, archive paths, and the research method) lives in the
  session record for the 0.22.0 cut, not duplicated inline on every row.

*Last refreshed: 0.22.0 cut, 2026-08-08/09.*
