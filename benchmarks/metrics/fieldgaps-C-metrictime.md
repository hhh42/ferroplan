# Field-gaps Sitting C — the metric-time decode, widened to rovers (F0c / roadmap-0.26 Phase 3)

Sat 2026-08-27, post cut25-sweep + promote (binary `target/release/ff` 0.25.0,
the promoted cut binary, not rebuilt; verified `ff --version` before the first
run). Box quiet throughout: one planner process at a time, strictly serial, no
builds, no concurrent CPU work. Receipts:
`benchmarks/metrics/probes-0.26/C-metrictime/` (referenced below as `R/`).
Probe ledger: **32 timed runs** — 4 pathways baselines, 9 pathways-i1
matrix+slice runs, 4 pathways-i2 matrix runs, 1 pathways-i1 300 s leg,
7 tpp runs (incl. the tpp-mtc A/B and two 300 s solos), 7 rovers runs
(incl. three 300 s solos) ≈ 40 min box time, inside the ~2 h envelope.
This file is the report the dossier names `decode-metrictime-0.26.md`
(Sitting C); committed here per the sitting order. Ground truth quoted is the
fresh 60 s-tier rows (`benchmarks/air25/ipc5-metric-time.jsonl`,
`air25/ipc5-constraints.jsonl`), not the memo's 30 s-era shapes: tpp is 8/40
(i4–i8 solve 12.17–41.53 s — the "cliffs at i4" story was already dead),
pathways 0/30, rovers 6/40 (i3 converted at 45.13 s), tpp-mtc 0/30.

## VERDICT — mechanism NAMED: numeric-accumulation relaxation blindness (flat or ZERO best_h on consume/produce goals, the numeric charge inert on temporal groundings), compounded by the ladder's wall-abandonment. OUTSIDE the closed temporal h-accounting ledger.

One mechanism unifies all three riddles, and it is numeric-side, not
temporal-side: **the temporal path's heuristic cannot count numeric
accumulation.** Every wall instance in the family churns at a frozen best_h —
pathways i1 at h=2 across 1,485,047 evals per pass, tpp i9 at **h=0** across
~750k evals per pass, rovers i6 at h=22 across 494,778 evals — because the
goals are accumulation comparisons (`(>= (+ (available A) (available B)) 4)`,
`(>= (stored g) (request g))`, energy-gated science) whose repetition structure
the relaxed extraction never charges: `charge_pre_num` is FALSE on every
stratified/temporal grounding by the `ground.rs:3114` `!stratified` entry rule,
exactly the F3 gate's stated adjacency. On top of that plateau the escalation
ladder wastes and then ABANDONS the wall (verbatim Full-tier quartet re-runs
with demand total=0 — Sitting A's S2 shape — then an early exit that leaves
33–271 s unused). The failures are **wall-independent**: 300 s solos convert
zero of five probed walls.

### S1 — pathways-metric-time 0/30: flat h=2, wall-independent, immune to the whole hatch matrix

Domain structure (read from `domain.pddl`): goals are numeric sums over
`(available ?molecule)`; reactions decrease `available` at start and increase
products at end; `choose`/`initialize` gate which substrates enter. Reaching a
goal quantity requires REPEATED reaction chains — the exact shape the a1+a2
`charge_pre_num` chain converted on the classical twin (pathwaysmetric-2023n
i2: 948,388 dead evals → 173, the 0.24 P6.3 receipt), and the charge is inert
here purely by the grounding-entry rule.

| run | condition | result | receipt |
|---|---|---|---|
| i1, 60 s | default, full narration | FAIL in **27.1 s** — "budgets exhausted with 33 s of wall left"; 8 passes, best_h **2 on every pass** (ev 1,285,662 / 947,784 / 1,179,429 / 1,485,047 — then the same four numbers again verbatim) | `R/C-pw-i1.{json,log}` |
| i1, **300 s** | default | FAIL in **28.6 s** — "271 s of wall left"; identical 8-pass anatomy | `R/C-pw-i1-300s.*` |
| i1, 60 s | `FF_NO_ESCALATE=1` | FAIL, 4 passes, 46 s left, best_h 2 | `R/C-pw-i1-noesc.*` |
| i1, 60 s | `FF_TDEMAND=1` | FAIL, 46 s left, best_h 2 (demand still total=0) | `R/C-pw-i1-tdemand.*` |
| i1, 60 s | `FF_NO_TDEMAND=1` | FAIL, 2 passes, 51 s left, best_h 2 | `R/C-pw-i1-notdem.*` |
| i1, 60 s | `FF_NOREL=1` | FAIL, 43 s left, best_h 2 | `R/C-pw-i1-norel.*` |
| i1, 60 s | `FF_TDECOMP=1` | FAIL, 29.2 s used / 31 s left, best_h 2 — decomposer finds **1 initial contract** (no decomposition exists) | `R/C-pw-i1-tdecomp.*` |
| i1 | `FF_TEVAL_BUDGET` 10k/30k/100k/300k | best_h **2 from the first 10k evals** — the plateau is total, not asymptotic | `R/C-pw-i1-ev*.{log,json}` |
| i2, i3, i5 baselines | default | FAIL; best_h frozen at 5 / 9 / 19 per pass; i2's post-decomposer contract passes reach h=3 then die; i3's contract entries all "pass entry refused"; i5 ends in "grounding checkpoint expired mid-enumeration" during decomposer re-ground | `R/C-pw-i{2,3,5}.log` |
| i2 matrix | noesc/tdecomp/norel/notdem | all FAIL, best floor h=3 (tdecomp, norel — the contract mask `[TREL] sound 35/104` shifts the landscape but no conversion) | `R/C-pw-i2-*.*` |

The ladder anatomy, quantified on i1: passes 5–8 are **byte-identical
recomputation** of passes 1–4 (identical eval counts AND identical cap-hit
budget-left values — `[TDEMAND] total=0` makes the Full tier the same task by
construction), then `[TDECOMP] 1 initial contracts` is a structural no-op and
the ladder returns with 33 s (60 s wall) / 271 s (300 s wall) unspent. The
"temporal ladder exhausted its budgets with N s of wall left" note class is
hereby decoded: **the ladder is node/eval-budgeted (400k-node caps), not
wall-budgeted; when its rung list and merge chain are exhausted it gives up
early regardless of remaining wall.** But the abandoned wall is NOT the
binding constraint here — h is flat, so no amount of returned wall converts
(the 300 s receipt proves it directly). Pathways' 0/30 is pure mechanism,
zero budget component.

### S2 — tpp-metric-time 8/40: the solved band is DECOMPOSER solves over an h=0 plateau; the i9+ wall is relaxation blindness at its limit, and the tail does NOT keep converting

The sharpest trace in the sitting: tpp's monolithic search sits at
**best_h = 0** — the relaxed extraction believes the goal is already reached
(the `buy → load → unload` increase/assign chain satisfies
`(>= (stored g) (request g))` under monotone relaxed semantics) while the real
search never closes. h gives NO gradient at all.

| run | result | receipt |
|---|---|---|
| i1, 60 s | SOLVED 0.00 s (12 ops, first pass) — the solved-side instrument | `R/C-tpp-i1.*` |
| i5, 60 s | SOLVED **19.4 s** — but every monolithic pass caps at best_h 0 (ev 660,012 per pass, ~3.1 s each); the solve comes from `[TDECOMP] 5 initial contracts`, whose per-contract searches (`[TREL] sound 30/148`) finish the job. **The i4–i8 band = decomposer solves after ~12 s of wasted monolithic ladder.** | `R/C-tpp-i5.*` |
| i9, 60 s | FAIL; h≡0 at ev 722,214 per pass; `[TDECOMP] 9 initial contracts` enters at ~57 s → contract entries refused | `R/C-tpp-i9.*` |
| i9, **300 s** | FAIL in 297.7 s ("2 s of wall left"); **40 passes**; h≡0 throughout (ev up to 809,727); the decomposer gets FULL time (0 refused entries), runs the complete degenerate merge chain 9→8→…→1 — contract 0 "UNSOLVABLE from current state" at every rung — and ends monolithic again | `R/C-tpp-i9-300s.*` |
| i10, **300 s** | FAIL in 277.6 s ("22 s left"); 44 passes; **33 stats lines, every one best_h 0**; merge chain 10→1 | `R/C-tpp-i10-300s.*` |

Leg-3 answer: **the tail does not keep converting** — the 60 s tier's i4–i8
band is the entire budget tail; i9/i10 fail at 5× wall with the full ladder
AND full decomposer executed. The i9+ wall is mechanism: with h≡0 both the
monolithic search and every contract search are blind, and the decomposer's
merge chain degenerates (contract 0 is refused/unsolvable every round — the
same refused-entry⇒UNSOLVABLE conflation Sitting A recorded rides along at
the 60 s tier).

### S3 — tpp-metric-time-constraints 0/30: the "empty `(:constraints (and))`" riddle is DEAD — the constraints live in the DOMAIN

The A/B (leg 4): tpp i1 solves in 0.00 s (12 ops, fv 7). tpp-mtc i1 — whose
INSTANCE constraints block is indeed the empty `(:constraints (and))` — fails
in 13.8 s with 46 s of wall left. The narration names why
(`R/C-tppc-i1.log` line 1, the `pddl3.rs:1183` statics pass): **"[P3]
constraint static simplification: dropped 2 of 15 hard, 0 of 0 soft
member(s)"** — the DOMAIN file carries a real `(:constraints …)` block (four
quantified schemes: two `at end (= … 0)` flushes, an `always` one-truck-per-
market exclusion, a `sometime` per-truck load obligation) that grounds to 15
hard members, 13 surviving statics. Compilation inflates the task 12→57 ops,
fv 7→22, degenerates TREL to keep-all (57/57 both masks), and the search dies
on a **best_h 1** plateau with `viol_dead 9596` + `dead_end 63410` at 524,667
evals per pass ×8 (two verbatim quartets — same S1 duplication), then exits
with 46 s unused. The board-wide "budgets exhausted with wall left" notes on
this variant (i1/i2/i11/i12/i21) are the same early-exit signature. So: the
empty instance block never was the mechanism and no longer needs explaining —
the mechanism is **domain-level constraint compilation blowup + monitor
dead-ends stacked on the same numeric h blindness** (h floor 1, flat).

### S4 — rovers-metric-time bimodality: split delivered — MECHANISM, not budget (0/3 at 300 s), plus a node-rate collapse

Domain: every science action is energy-gated (`>= (energy ?x) 8/5/3/2/1`,
consume at start) and `recharge` increases energy by `(* ?duration
(recharge-rate ?x))` — numeric accumulation again.

| run | result | receipt |
|---|---|---|
| i3, 60 s solo | SOLVED **29.1 s** (45.13 s on the jobs=2 board) — a SINGLE first-quartet pass, solved in-run before any cap; `avg_helpful 0.0–0.1`, ~6k nodes/s | `R/C-rov-i3.*` |
| i5, 60 s | FAIL; pass 1 eats 59.9 s (87,856 nodes), best_h 4 flat at ev 184,096; `[TDECOMP] 25 initial contracts`, all entries refused | `R/C-rov-i5.*` |
| i5, **300 s** | FAIL; pass 1: 173.8 s to the 400k node cap, best_h 4 flat at ev 1,027,942; the Full-tier quartet re-runs (105.5 s more of duplicate); TDECOMP 25 contracts, 96 refused entries, merge loop | `R/C-rov-i5-300s.*` |
| i6, **300 s** | FAIL; pass 1 consumes **299.8 s for 152,797 nodes (~510 nodes/s)**, best_h 22 flat at ev 494,778; TDECOMP 34 contracts, contract 0 unsolvable/merge loop, 136 refused | `R/C-rov-i6-300s.*` |
| i9, **300 s** | FAIL; pass 1: 299.7 s, 156,724 nodes, best_h 22 flat at ev 342,002; TDECOMP 42 contracts, 169 refused | `R/C-rov-i9-300s.*` |

The bimodality decoded: the 0.01–0.07 s solves (i1/i2/i4/i7/i8) and i3 are
instances the first helpful pass closes in-run; everything else sits on a flat
h (4/22/22 — later-pass values 31/44/49 are the unmasked h scale, equally
frozen) that no wall crosses. The budget-vs-mechanism split the checklist
demanded: **i3 was the only budget-shaped row and the 60 s tier already
banked it; i5/i6/i9 are mechanism (5× wall converts zero).** Aggravator,
recorded: the temporal eval rate collapses to 0.5–2.3k nodes/s on rovers
(vs ~100k/s on pathways) — at i6's rate even the 400k node cap is
unreachable inside 300 s, so the ladder cannot even finish its FIRST rung.
Instrument note for F6: the rovers pass-start narration reads `fv=2
rel_fluents=2` (words=4–10) — implausibly low for an energy-per-rover task;
flagged, not chased.

## INSIDE or OUTSIDE the closed temporal h-accounting ledger — the gate question, answered explicitly

**OUTSIDE.** The named mechanism is *numeric*-side relaxation blindness — the
missing a1/a2 numeric-precondition/accumulation charge on stratified
groundings (`ground.rs:3114` `!stratified` entry rule) — plus *search-shape*
waste (ladder duplication and wall abandonment, Sitting A's S2 class). It is
not a temporal delete-relaxation or temporal interval-accounting claim: tils=0
on every pass of every probed instance, no deadline/TIL structure anywhere,
and no `FF_TRPG`/`FF_H_ENDGATE` run in this sitting. The closed ten-negative
temporal h-accounting ledger stays closed and untouched. Declared adjacency,
honored: the 0.22 pre-a2 charge-on-temporal negative (model-train plateau
re-leveled and stayed flat) is the base-rate risk for the build this opens —
the distinction the F3 spec stands on (a2 chain landed later; constituency is
2006 metric-time, with the pathwaysmetric-2023n i2 948,388→173 receipt on the
classical twin) is exactly the distinction these traces support.

## Gate verdicts (one line each, quotable by the cut record)

- **`FF_NUMPRE_TEMPORAL` (charge_pre_num) gate: OPENS.** Mechanism named
  (numeric-accumulation h blindness on temporal groundings, flat best_h
  receipts on pathways i1 h≡2 / i2 h≡5 / i3 h≡9 / i5 h≡19), outside-and-
  distinct from the closed ledger. RED fixture: pathways-metric-time i2 (the
  standing spec default — still 0-for-everything here, and the 2023n twin of
  the a2 receipt), with i1's wall-independent 8-pass trace
  (`R/C-pw-i1-300s.*`) as the decode instrument. The workshop-economy
  plan-length pin and the `FF_H_ENDGATE`/`FF_TRPG` co-fire declaration carry
  as spec'd.
- **AIBR gate: condition MET on a named wall, with one honest caveat.**
  tpp-metric-time i9 is a wall instance whose best_h trace is flat AT ZERO
  under the existing extraction (33 consecutive h=0 stats lines across 40
  passes and ~750k evals/pass at 300 s, `R/C-tpp-i9-300s.log`) — Metric-FF-
  class relaxation blindness by its textbook signature (relaxed reachability
  satisfies the accumulation goal instantly; no gradient exists to charge).
  An interval-subgoaling estimate is non-flat there by construction (gap =
  `request − stored` > 0 at every probed state ⇒ repetition estimate ≥ 1 and
  decreasing with accumulation) — but that half is an argument, not a
  measured trace (no such estimator exists in the binary to probe); the AIBR
  build spec must carry it as its RED obligation. Secondary named instances:
  rovers i5 (h≡4), pathways i1 (h≡2).
- **Ladder wall-return (Sitting A's F3 candidate), priced for THIS family:
  ~zero conversions.** The duplicate-quartet skip and early-exit fix would
  return 25–46 s of a 60 s wall on these boards, but the 300 s solos prove
  returned wall converts nothing while h is blind (0/5 solos). Unlike
  trucks/storage, wall-return is not a lever here; it remains correct
  engineering priced by Sitting A, not by this report.
- **tpp-mtc: riddle CLOSED as posed.** The empty instance-level block is a
  red herring — the domain's own `(:constraints …)` grounds to 13 surviving
  hard members (P3 narration receipt, `R/C-tppc-i1.log:1`), and the variant's
  0/30 is constraint-compilation blowup (12→57 ops, TREL keep-all, viol_dead
  9596) over the same numeric blindness. Any future tpp-mtc work rides the
  same two builds plus PDDL3 monitor pruning; no separate mechanism remains
  unnamed.

## Fence compliance

No code changes of any kind. No `FF_TRPG`/`FF_H_ENDGATE` runs. The exit is
not a temporal-h-accounting claim (stated above, in terms). No code at the
pathways/tpp/rovers walls — decode only; the standing fence now lifts per the
sitting's own clause: gates open as recorded, builds ship only through their
F3 specs with the declared referees. Every number above traces to a receipt
under `benchmarks/metrics/probes-0.26/C-metrictime/` or a committed raw row
cited by path.
