# Where ferroplan ranks — a rough field placement, per year and track

> **0.25 note:** the placement NUMBERS on this page are now data —
> [`benchmarks/field-results.json`](../benchmarks/field-results.json)
> (plus the vendored official IPC-2023n CSVs) feeds a regenerating
> **vs field** column in [`STANDINGS.md`](../STANDINGS.md), so the
> current placements no longer wait for a hand refresh of this page.
> This page remains the prose companion: provenance, per-row caveats,
> and the confidence grades. Its ferroplan-column snapshots below are
> dated (0.22.0) where they were last hand-refreshed.

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
| time (re-entered at 0.23) | **77/130 (59.2%)** at the board's 30 s stamp; the makespan quality column debuts: vs best-of-field 27W/3T/47L, mean 0.80 (all 77 solves scored against the vendored official archive) | archive-counted on the exact 130-instance corpus: SGPlan5 80/130 (61.5%), YochanPS 58/130, MIPS-XXL 32/130, CPT2 15/130 (IPC-4 reference columns excluded: SGPlan.IPC04 62) | **2nd of 5, three solves off the 2006 winner** — and head-to-head where both solve, ferroplan's schedules beat SGPlan5's 34W/0T/11L; the quality-column losses concentrate against YochanPS (7W/33L), whose makespans lead the field it can reach. At 1/60th the official budget, a floor | medium-high — official per-instance archive, exact corpus, both currencies; but field counts are reconstructed raw solves (presence of a .soln), not the 2006 quality formula |
| metric-time (re-entered at 0.23) | 54/200 (27.0%) at 30 s; makespan quality vs best-of-field **43W/1T/10L, mean 0.94** (54 scored) — when it solves, its schedule beats the field's best 43 times in 54 | archive-counted on the exact 200-instance corpus: SGPlan5 151/200 (75.5%), MIPS-XXL 31/200, YochanPS 12/200, CPT2 6/200 (references: SGPlan.IPC04 52, CPT.IPC04 5) | **distant 2nd of 5** — barely over a third of SGPlan5's count (head-to-head 26W/3T/5L where both solve), clearly clear of the rest; a coverage gap, not a quality gap — pathways 0/30 and the rovers/tpp/pipesworld tails own the deficit | medium-high — same basis as the time row |
| complex-preferences | cannot attempt (the modal operators PARSE; the track's preference bodies lean on the timed ones — `within`/`always-within` — whose enforcement is the named stage-c feature, docs/roadmap-0.23.md Phase 2) | SGPlan5 swept all 5 domains (105 raw solves); MIPS-XXL 2nd (25) | **last of 3**, until the feature ships | high |
| constraints (stages a+b at 0.23; stage c shipped by 0.24 — row refreshed 2026-08-26 from the committed raws) | 28/120 (23.3%) — all 120 rows attempt, zero engine-rejects; 16 timed rows solve (pipesworld-mtc 3, trucks-tc 5, storage-tc i11–16/i21/i22); storage-tc is the first constraints domain WON outright (15/30 vs SGPlan5 9/30) | thin field, 3 officially-scored domains (80 instances): SGPlan5 47/80 (58.8%), MIPS-XXL 13/80 (16.3%) | **2nd of 3 on the official subset** — 20/80 (25.0%) vs MIPS-XXL's 16.3%; the remaining gap to SGPlan5 is tpp-mtc 0/30-vs-18/30 and trucks-tc 5/20-vs-20/20, both riding named decodes (docs/field-gaps-0.26.md §1b) | low |

## IPC-6 (2008)

| track | ferroplan (0.22.0) | field | rough placement | confidence |
|---|---|---|---|---|
| seq-sat | **284/300 (94.7%)** | LAMA (winner) 281/300 (93.7%); FF(h_sa) runner-up 225/270; field of 10 down to Plan-A 37/180 | **clears the official winner by raw count** — on this exact 300-instance denominator, ferroplan's coverage exceeds LAMA's | high |
| seq-opt (proof rate) | **150/270 (55.6%)** | Gamer (winner) 134/270 (49.6%); HSP*F runner-up 132/270; field of 8 down to CFDP 24/240 | **clears the official winner by raw count** | medium |
| tempo-sat | **307/390 (78.7%)** — first board at the 60 s tier; the split is proven per the 0.21 rule: 0.22's binary at 60 s scores 302, so +4 of the +9 is budget, +5 engine | SGPlan6 (winner) 318/390 (81.5%); Temporal Fast Downward runner-up 257/390; field of 6 down to TLP-GP 6/120 | **2nd of 6, eleven solves behind the winner** (twenty at 0.22's 30 s tier), fifty ahead of the actual runner-up | high — and the tier move halves the standing budget caveat: 60 s vs the field's 1800 s is a 30× gap now, not 60× |
| net-benefit (re-entered at 0.22) | **248/270 (91.9%)** — this cut's strongest board by percentage | thin field, optimization only (satisficing subtrack cancelled): Gamer (winner) 81/210 (38.6%); Mips-XXL 59/210 (28.1%); HSP*P 51/210 (24.3%) | **far ahead of the field** by rate, though on a differently-sized corpus (270 vs 210) | low-medium |

## IPC-7 (2011)

| track | ferroplan (0.22.0) | field | rough placement | confidence |
|---|---|---|---|---|
| seq-sat | 220/280 (78.6%) | 27 entrants; LAMA-2011 (winner) 250/280; field spans FDSS-2/PROBE 233, FDSS-1 232, FD-AUTOTUNE-1 223, ROAMER 213 ... down to ACOPLAN 20/280 | **~6th-7th of 28**, between FD-AUTOTUNE-1 and ROAMER — solidly upper-third | high |
| seq-opt (proof rate) | 127/280 (45.4%) | 12 scored entrants; FDSS-1 (winner) 185/280; field clusters IFORKINIT 144 down to CPT4 44/280 | **~12th of 13**, between IFORKINIT and CPT4 — ahead of only the trailing entrant | high |
| tempo-sat | **129/240 (53.8%)** at the 60 s tier (0.22's binary at 60 s: 127 — +6 budget, +2 engine over the 30 s board's 121) | 8 entrants; YAHSP2-MT 145/240 (raw leader), YAHSP2 137, DAEYAHSP (winner by score) 136, POPF2 119 (joint official runner-up), LMTD 62 | **~4th of 9** — ten raw solves clear of POPF2 now, seven behind the score-winner | high — same halved caveat: the 60 s budget is 30× short of official, down from 60× |
| seq-mco (t2/t4/t8) | not attempted this cycle | wall-clock-per-core competition rule, 4-core box | out of scope — not re-baselined | — |
| seq-mco t2/t4/t8 (re-entered at 0.23 — the last cloud-era ghosts retired; every board now shares one box) | t2 230/280 (82.1%), t4 237/280 (84.6%), t8 240/280 (85.7%) — wall-clock per the competition rule (`--threads N`, one instance at a time, 60 s); 4P+6E heterogeneous box, t8 oversubscribed by construction and recorded as such | the 2011 multi-core track ran under the wall-clock-per-core rule on a 4-core box, but this record holds no per-planner multi-core results — entrant names and numbers unknown here | placement not computable from the record — the honest internal read: the mco rows out-cover the same 280 instances' single-config seq-sat board (219/280) by 11–21 solves at the same 60 s wall | unknown — field data absent; the wall-clock methodology note is load-bearing on any future comparison |

## IPC-2014

| track | ferroplan (0.22.0) | field | rough placement | confidence |
|---|---|---|---|---|
| seq-sat | 147/280 (52.5%) | 20 entrants; IBaCoP2 (winner) ~198/280; 7 confirmed-coverage entrants span 163-198; a quality-score tier (Cedalion/ArvandHerd/FDSS-2014/DPMPlan) reads 125-137; 9 more entrants unlocatable | **roughly 10th-14th of 20** — now sits above the located quality-score tier, though 9 entrants' true numbers remain unknown | medium |
| seq-agile | 141/280 (50.4%) | 15 entrants scored by runtime, not coverage; YAHSP3 wins (score 81.2); reconstructed (non-official) coverage bands ~66-159/280, IBaCoP2/Jasper ~136-147 | **roughly mid-field** on the reconstructed coverage band | low |
| seq-opt (proof rate) | **74/256 (28.9%)**, +16 this cut — the cycle's single biggest mover | 17 entrants; SymBA*-2 (winner) 151/280; field spans down through Gamer 83/280 to Hpp 14/280 | **~11th-12th of 17**, now nearly matching Gamer (normalized ~81/280) — up from clearly-behind to essentially tied | high |
| tempo-sat | 74/200 (37.0%) at the 60 s tier (0.22's binary at 60 s: 73 — +6 of the +7 over the 30 s board is budget, +1 engine; the turn-and-open boundary-churn class the 30 s wall manufactured is retired) | only 6 entrants; YAHSP3-MT wins, TFD runner-up — no per-planner numbers locatable anywhere despite extensive search; official site is dead | cannot be estimated — field size and winner known, nothing more | unknown |
| seq-mco t4 (re-entered at 0.23) | 164/280 (58.6%) — wall-clock per the competition rule (`--threads 4`, one instance at a time, 60 s); 4P+6E box | the 2014 multi-core track ran, but no per-planner results are held in this record — names unknown here | placement not computable — internal read: +13 over the same corpus's single-config seq-sat (151/280) at the same 60 s | unknown — same basis as the 2011 mco row |

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
*Constraints row refreshed 2026-08-26 against `benchmarks/ipc5-constraints.jsonl` and the official archive (the field-gaps verification record, docs/field-gaps-0.26.md); the remaining ferroplan-column snapshots still date to the 0.22.0 refresh.*
