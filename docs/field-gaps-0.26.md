# The field gaps, priced — the SGPlan overtake and the modern-satisficing climb

Drafted 2026-08-26, with the 0.25 cut sweep in flight, as the answer to two
questions asked against `docs/ipc-rankings.md`: **what beats SGPlan on the
tracks it still leads, and what closes the modern satisficing gap?** Drafted as a
0.27 candidate; **adopted into 0.26 by decision on 2026-08-26** — the full
program builds in this cycle, gates intact, and the 0.26 cut sweep runs on
crucible. The roadmap carries the expansion (docs/roadmap-0.26.md, "The
field-gaps expansion").

Provenance: a nine-agent read — per-domain mining of the raw boards
(`benchmarks/*.jsonl`, `air25-entries/`), the engine source, the 0.17–0.26
cycle records, the field literature — then synthesis, then three adversarial
verification passes (anti-pot ledger, data re-derivation, feasibility against
source). **Every per-domain number below was re-derived from the raws by the
data pass**; one fatal error in the draft (the 2014 union arithmetic) and
three ledger violations were caught and are corrected here. No planner was
run and nothing was built — the cut25 sweep owned the box.

One context line from the field read, flagged as literature rather than our
measurement: SGPlan's IPC-5/6 dominance is partly domain-recognition tuning
(the Saarland re-run scores SGPlan6 at 139.62 where LAMA re-scores 235.58),
and every ferroplan-side number in this memo was earned at 30–60 s against
SGPlan's 1800 s. Both facts point the same way: these tracks are takeable.

---

## 1. The SGPlan ledger, track by track

### 1a. IPC-5 metric-time — 54/200 vs 151/200 (gap 97). Mechanism-shaped, not budget-shaped.

Per-domain arithmetic (re-derived from the official archive's `.soln` counts):
tpp 3 vs 39 (−36), pathways 0 vs 30 (−30), rovers 5 vs 32 (−27), pipesworld
6 vs 30 (−24), openstacks 40 vs 20 (**+20**). The failure shapes refuse a
speed story: pathways fails i1–i15 UNDER the wall (0.02–27.2 s of 30) with no
plan; tpp solves i1–i3 at ≤0.34 s then cliffs at i4; rovers solves i7/i8 in
0.01/0.06 s while i3/i5 burn the wall. Only pipesworld is a classic tail
(i3–i6 solve at 10.8–14.0 s of 30, i1/i2 under 4 s; 2–5× buys +2–6, and
SGPlan's own 30/50 caps that deficit at −24).

- **Mechanism: the pathways and tpp open riddles — already 0.26 Phase 3's
  decode**, fenced by the standing anti-pot (no code at the pathways/tpp
  walls before the decode; the fence as recorded names those two — extending
  it to rovers is THIS memo's proposal, because rovers' 0.06 s→wall
  bimodality reads as the same family). The sitting should sample rovers
  i3/i5 explicitly: the priced ceiling without rovers (+60–70 → ~115–125/200)
  still loses to 151. **The track flips only if the decode covers all three.**
- **Build candidates, strictly post-decode.** (i) Arm `charge_pre_num` on
  temporal groundings (`ground.rs:3114` clears it via `!stratified`; the
  `FF_NO_NUMPRE` restore already exists). The receipt: the 0.24 a2 chained
  charge converted pathwaysmetric-2023n i2 from 948,388 dead evals to 173.
  **Declared adjacency:** the 0.22 record armed the *pre-a2* numeric charge
  on temporal groundings and measured NEGATIVE on the model-train/TMS plateau
  — one of the ten counted negatives that closed the temporal h-accounting
  ledger. The distinction this item stands on: the a2 chain landed later
  (0.24 P6.3, with its RED fixture converted), and the constituency is 2006
  metric-time, not that plateau. It enters ONLY if the Phase 3 decode names a
  mechanism outside the closed accounting class — and the probe must carry
  the workshop-economy temporal fixture (`packed.rs:132–139` records the
  charge re-routing a 27-step carve plan to a 47-step chisel-sale plan when
  it touched temporal tasks) and state that co-fire with
  `FF_H_ENDGATE`/`FF_TRPG` is untested. (ii) AIBR/subgoaling interval
  numeric h (the 2023-numeric-podium class; the landscape memo's gap #2) if
  the decode names Metric-FF-class relaxation blindness. The field read
  found nobody publishing modern numbers on this corpus — unclaimed
  territory.
- **Band**: unpriceable pre-decode; ceiling arithmetic above. **Cost**:
  sitting small; AIBR build moderate. **Touches**: 0.26 Phase 3 directly.

### 1b. IPC-5 constraints — 20/80 official-subset vs 47/80 (gap 27). No new constraints machinery needed.

Per-domain: tpp-mtc 0/30 vs 18/30 (−18), trucks-tc 5/20 vs 20/20 (−15),
storage-tc 15/30 vs 9/30 (**+6 — the first constraints domain won
outright**). The feature gap is CLOSED: stage c shipped, the raw board is
28/120 with zero rejection notes, and 16 timed rows solve (pipesworld-mtc 3,
trucks-tc 5, storage-tc i11–16/i21/i22).

- **Mechanism**: tpp-mtc rides the tpp metric-time decode (its untimed rows
  are the same riddle); trucks-tc rides the trucks cliff (§1d — its 5 timed
  solves at ≤0.11 s prove within-enforcement works; the untimed TIL sister
  is identically 5/20). Isolated bonus: storage-tc i8–10, where the at-end
  fold that banked i1–7 stalls — small, scoped.
- **Standing correction for the 0.26 cut refresh**: `docs/ipc-rankings.md`
  line 55 still reads 12/120 with "70 timed rows keep the named rejection" —
  stale against the 28/120 zero-rejection raw. The page refreshes by hand
  alongside a cut sweep; this cut's refresh must carry it.
- **Band**: inherits the §1a/§1d decodes ~free. **Cost**: small.

### 1c. IPC-6 2008 tempo-sat — 305/390 vs 318/390 (gap 13). The overtake goes around model-train.

≥12 of the 13 sits in model-train, which is priced-zero (the 0.25 encoder
probe fired its exit clause: state-dependent durations, nothing built). The
candidate mass elsewhere:

- **Transport L1–L3**: the recorded band is **+8–20 of 211 AGGREGATED across
  2008/2011/mco — no per-board split is priced**, so the 2008 share is
  unknown until the widening the record already demands ("two instances is
  not a lever — widen the probe before pricing it"). The L3 receipts stand —
  but as **ipc-2011 rows**: i4 at 16.18 s and i6 at 58.98 s under
  `FF_NO_NOVLIGHT=1 FF_NO_LAMA=1` ran the 2011 corpus
  (`post-entries25.sh:117–126`; this memo's earlier "2008" label was wrong,
  caught in spec verification 2026-08-26). The 2008 board therefore has NO
  direct L3 receipt yet — the widening carries the whole 2008 question. **The overtake is therefore a hypothesis the
  widened probe tests, not a claim**: it needs ~10 of the band to land on the
  2008 board specifically, plus elevator.
- **Elevator mem-cap fix**: 3 mem-caps at 8.5–11.7 s on the 2008 board
  (~+3); the same signature is worth up to +7 on 2011 (a different board —
  not nettable against this gap). A memory-profile lever, distinct and small.
- Sokoban/parc-printer already sit AT best-of-field re-run bounds — no
  vs-SGPlan mass there.
- **Fence**: L1–L3 claims are 2008/2011/mco ONLY — 2014 transport is
  explicitly not claimable (in writing before any code; package count 25 vs
  the engine's ~12–14 line).
- **Band**: conditional as stated. **Cost**: moderate. **Touches**: 0.26
  Phase 4 (the widening is already scheduled there).

### 1d. IPC-5 time — 77/130 vs 80/130 (net gap 3, gross 34). One decode, three tracks.

Per-domain: storage-time 15/30 vs 30/30 (−15 — every ferroplan solve under
2.1 s of a 30 s budget, then wall: 14× headroom already unused, speedup buys
~0), trucks-time 11/30 vs 30/30 (−19 — solves ≤5.1 s, cliff exactly at i12).
**+4 instances flips the track.**

- **Mechanism: unnamed — and this is the single highest-leverage undiagnosed
  mechanism on the SGPlan ledger.** Trucks carries −15 (prop), −19 (time),
  −15 (constraints): 49 gross instances across three tracks, same
  compiled-deadline/TIL signature, all fast-or-never. Storage-time shares the
  deadline structure. Per the 0.26 standing rule this is a **decode sitting,
  not a build**: instrument where the search collapses on trucks-time i12
  and storage-time i15/i17, exit clause on a fixed probe budget. **Fence
  stated before the sitting**: the temporal delete-relaxation ledger is
  CLOSED at ten negatives — the sitting looks at search shape, serialization
  and compiled-deadline structure; an exit that lands on "temporal h
  accounting" is dead on arrival by that ledger.
- **Band**: unpriceable pre-decode; flip threshold tiny (+4). **Cost**:
  sitting small.

### 1e. IPC-5 propositional — 188/220 like-for-like (85.5%) vs 218/220 (gap 30).

The published 82% is the 450-instance board; on the official 220-instance
slice the rate is 85.5%. Per-domain deficit: trucks 15 (SGPlan 28/30),
pathways 6, rovers 4, storage 4, tpp 1; openstacks and pipesworld tie. All
81 board failures are near-wall timeouts, zero mem-caps — at 1800 s a large
fraction flips, so part of this gap is a budget artifact SGPlan never faced.

- **Mechanism split**: trucks = the §1d decode (structural). The rest are
  genuine tails: 2–5× converts pathways +4–8, storage +2–3, rovers +3–6. The
  named engine lever for tails: **the complete fallback that does the heavy
  lifting is a bare single-queue wBFS** (`search.rs:729`) — no preferred
  operators, no landmark signal — while 128 solved rows carry "EHC found no
  improving state; used weighted best-first". `FF_CLM` already exists opt-in
  and scoped to exactly this fallback (`search.rs:233–236, 1402–1410`); the
  preferred/normal dual heap exists one file over (`lama.rs:204`) and never
  reached `search_from`. See §3.1 for the interaction fence.
- **Band**: +10–17 on tails at ~2×-equivalent, plus trucks' 15 behind the
  decode. **Cost**: small-moderate, under house law — RED fixture first, a
  named `FF_NO_*` restore, armed at a sweep, old-binary referee.

---

## 2. The modern-satisficing ladder (2014/2018/2023)

Calibration constant first, from the 2023 agile 60 s/300 s twin: **5× budget
= +15/140; a 2× engine speedup at the 60 s wall ≈ +4.** Speed is real but
modest; the ladder is ordered by evidence-per-effort.

**Rung 1 — 2018 near-wall harvest: +9 reaches the field median (91/240) and
escapes the ≥13th floor.** The mass: settlers +2–3 (7 solves >30 s, max
59.9 s), flashfill +1–2, spider +1–2, the data-network seam +1–2, nurikabe
+1 — roughly +9 at ~2× effective speed before touching any structural
domain. Delivery vehicle: §3.1 fallback enrichment + §3.4 lookahead, not a
raw optimization pass.

**Rung 2 — 2014 config reconciliation: small, honest, no longer the
headline.** The draft claimed best-of-both = 165/280 (+16); **the
re-derivation kills it: the true per-instance union of the sat and agile
boards is 155/280, oracle +6 over sat** (outside hiking/openstacks/
parking/tetris the two configs solve identical sets), which stays 8 below
the located 163–198 field band. What survives: hiking sat 20/20 vs agile
12/20 with sat solving the agile losses in 7.3–26.9 s — diagnose why the
agile ordering dies there (possibly a bug-shaped fix), and a cheap config
schedule for the remaining +6 oracle, old-binary refereed, priced honestly
after the referee (a naive time split will not keep even the +6). **Cost:
small.**

**Rung 3 — memory, a two-domain problem.** folding is THE memory domain
(at 300 s: 10 mem-caps at t=12–18 s plus 2 kill-9s — half the domain);
elevator 2008/2011 repeats the signature (§1c). Org-synthesis holds 5 more
mem-caps, but the hash-join route stands refused twice with a lower-bound
simulation — footprint/grounding-memory work only. One memory-profile
sitting covering folding+elevator, priced +3–10 across boards; the or-aware
hoist for folding p01 is the named, sized rider. **Cost: moderate.**

**Rung 4 — 2023 quality: quantum-layout plan improvement.** 20 of 36 sat
solves at 0.72 mean quality vs the board's 0.79 — over half the score mass
in one domain. A QUALITY use of the anytime machinery, which the anti-pot
permits (the ban is restarts *for coverage*: −9 coverage for +4 quality at
60 s). **Confined to the existing 300 s agile entry board and 60 s boards
for coverage-neutrality refereeing — no new tier** (new 300 s tiers are a
standing priced-zero; an 1800 s tier does not exist and is not being bought
here). **Cost: small — the machinery exists, default-off.**

**Rung 5 — the structural zeros stay behind decodes.** transport-2014 (0/20
both configs, fenced), agricola (retired: grounds in 13.7 s, the wall is
search churn), rubiks-cube (5× budget bought exactly +0 — the purest cliff
on any board). These move only if a NEW mechanism lands; §3.2's
forgetting-novelty is the one candidate with field receipts, and it is
gated on its own decode. Floor-tile keeps its unclaimed lever — the
irreversible-consumption dead-end test with the attached no-code pricing
probe: run the probe, build only on its number.

---

## 3. Cross-cutting engine gaps, ranked

1. **Preferred operators + landmark count in the complete fallback.** The
   search that eats most of the wall is a bare single-queue wBFS; `FF_CLM`
   exists opt-in, the dual heap exists in `lama.rs`. Moves: the ipc5-prop
   tails (§1e), every 2014/2018 near-wall cluster (§2 rung 1). **Interaction
   fence, found in source**: `search.rs:1404` guards the landmark/resource
   ordering terms with `cfg.h_cost.is_none() && !cfg.anytime` — so this item
   and the cost-augmented first-plan rung (§3.5) are mutually exclusive on
   the same search as coded. Either they take disjoint rungs by
   construction, or the guard is lifted deliberately with the key-rescale
   question answered and its own referee. RED fixture: a named 2018
   near-wall instance (settlers). Ships with a new named `FF_NO_*` restore.
   **Cost: small-moderate.**
2. **Novelty-with-forgetting + multi-HEURISTIC queue alternation** (the
   Scorpion-Maidu ingredients; the published ablation has h^novelty alone at
   84/140 on the 2023 corpus where ferroplan sits 37/140). **Adjacency
   declared**: this neighbors the closed novelty-promotion negatives and the
   "no width/partition variant without a NEW mechanism DECODE" rule — and a
   field ablation is a candidate, not a decode. The 0.24/0.25 SAT-wing
   lesson (bands priced from field receipts at +16–50, delivered +1/+0
   twice) says field receipts do not price this engine's walls. **So: a
   decode sitting comes first** — wall-slice instrumentation on the cliff
   boards this rung claims (rubiks/floor-tile-class, the 2018/2023 residue)
   that names what forgetting/alternation would fix in THIS engine; the rung
   builds only on that number. Mechanism-novelty stated precisely:
   forgetting is verified absent (`novelty.rs` has only per-iteration buffer
   clears), and what is untested is alternation across DIFFERENT heuristics'
   queues — dual pref/normal batch alternation within one heuristic already
   ships in `lama.rs:204`/`novelty.rs:33`. **Cost: decode small; build
   moderate.**
3. **AIBR/subgoaling numeric h** — the metric-time flank's build candidate,
   gated behind the Phase 3 decode (§1a). **Cost: moderate.**
4. **YAHSP-style relaxed-plan lookahead.** Won 2014 agile AND temporal; the
   field read's warning about our exact family at short budgets ("Metric-FF
   would have placed 17/17"). No anti-pot adjacency — never tried here.
   Moves: parking-2014 (all four solves at 59.5–59.9 s — literally at the
   wall), tetris, cave-diving, the 2018 clusters. **Cost: small-moderate,
   the biggest speed payoff per the field evidence.**
5. **Cost-sensitive first-plan rung (transport L1).** `relaxed_costed`
   exists (`heuristic.rs:1209`), is plumbed through `SearchCfg.h_cost`, and
   its only setter today is the post-hoc cost sweep (`costs.rs:155`).
   Subject to the §3.1 guard fence. **Cost: small-moderate.**
6. **Memory footprint work** — §2 rung 3.
7. **Plan-then-schedule temporal probe — the one lawful model-train
   reopen.** The IPC-08 baseline of exactly this shape beat every temporal
   entrant. A post-hoc scheduler computes each duration from the known
   pre-state, needing no fixed STN intervals — a genuinely different
   mechanism from the declined encoder, so the exit clause can lawfully
   reopen on it. **Feasibility caveat from source**: `temporal.rs:507–535`
   evaluates duration fluents against the INITIAL state by design, and the
   existing `tsched.rs` rescheduler repacks FIXED durations — the needed
   core does not exist, and it crosses the module the record prices
   expensive. **The read EXECUTED 2026-08-26 and the exit clause FIRED, on
   stronger grounds**: the pre-state duration core has been in-engine since
   v0.10 (`4c3e4e7`) — the initial-state evaluator governs only static
   durations, and model-train's 0/30 was measured WITH per-pre-state
   durations in the binary. The item is CLOSED; the mass re-routes to the F3
   `charge_pre_num` gate (flat h across advance sequences + float-keyed
   duplicate detection — the closed h-accounting ledger's plateau by name).
   Full verdict: `docs/field-gaps-execution-0.26.md` §3.7. **Cost: 0.**
8. **Dynamic axioms**: no named board deficit charges to it. Defer.

Explicitly NOT on this list: deferred evaluation / h-economy — already
in-engine, dropped with proof in 0.25; the field read's "lazy GBFS"
ingredient is not a gap here.

---

## 4. What NOT to do

- **No code at the pathways/tpp metric-time walls before the Phase 3
  decode** (the recorded fence names those two; treating rovers the same is
  this memo's proposed extension). The 0.25 precedent: two real bugs fixed,
  still 0/30.
- **No temporal delete-relaxation variant, ever** (ledger closed at ten
  negatives). The §1d sitting is framed to exclude this exit in advance.
- **No model-train STN encoder revival**. Only §3.7's plan-then-schedule
  probe differs in mechanism; anything STN-interval-shaped is a re-buy.
  (**Update 2026-08-26**: the §3.7 read executed and its exit clause FIRED
  — see §3.7 — so this fence now has no live exception at all.)
- **No 2014-transport claims from L1–L3** (fenced in writing).
- **No naive novelty/driver promotion**: the §3.2 rung enters only behind
  its own decode, bounded, wall-sliced, old-binary refereed — and if its
  referee reads like 0.17's +7/−51, it dies the same day.
- **No anytime restarts for coverage** (−9 coverage for +4 quality);
  quality-only, existing boards only, no new tiers.
- **Stands refused**: org-synth hash-joins (lower-bound simulation),
  agricola grounding, markettrader, barman symmetry, caldera
  product-threshold routing (only the selectivity gate may reopen), the
  blanket 85% wall reserve, `FF_SAT_BRANCH` default, a third SAT band
  without a decode, 120 s classical tiers.
- **Tempting-but-wrong reads**: (i) "metric-time needs a faster engine" —
  the cliffs sit at instance 4 with sub-second solves and 14× unused
  headroom; (ii) "the 2023 ~20th placement is an engine verdict" — it is a
  60 s-vs-1800 s floor, and the 300 s twin already implies 51–52/140;
  (iii) ricochet's clean budget scaling is not a mechanism claim — it climbs
  with speed/budget or not at all; (iv) storage-tc's VAL SIGBUS rows are
  booked, not a scoring bug to chase.

---

## 5. The execution shape (adopted into 0.26, 2026-08-26)

1. **Phase 0 — the trucks/storage-time decode sitting**: one unnamed
   mechanism carrying −49 gross across three SGPlan tracks, +4 flips
   ipc5-time; instrumented probes, exit clause, temporal-relaxation exits
   pre-excluded. Committed report whatever it concludes.
2. **Phase 1 — fallback enrichment**: preferred ops + `FF_CLM` armed into
   the wBFS fallback (RED fixture: a settlers near-wall instance; new
   `FF_NO_*` restore; the h_cost guard interaction resolved by
   construction); prices the prop tails (+10–17) and the 2018
   +9-to-median in one sweep.
3. **Phase 2 — the cliff decode, then (only on its number) the
   forgetting/multi-heuristic rung**; the 2014 hiking-agile diagnosis and
   the +6-oracle config schedule ride the same boards.
4. **Phase 3 — build what the decodes priced**: metric-time
   (`charge_pre_num` hatch first with the workshop-economy fixture, AIBR if
   named) and transport L1–L3 widened-then-built — the 2008 overtake
   hypothesis tested with elevator's ~+3 memory fix alongside.
5. **Phase 4 — quality and memory**: quantum-layout anytime polish on
   existing entry boards, the folding/elevator memory sitting, the
   floor-tile no-code pricing probe, the YAHSP-lookahead probe — each with
   its own exit clause.

---

## Appendix — verification record

Three adversarial passes ran against the draft: **ledger** (anti-pot re-read
from roadmap-0.22–0.26 directly), **data** (≥8 claims re-derived from the
raws; found the 2014 union error — draft said 165/280/+16, truth 155/280/+6),
**feasibility** (every cost grade checked against `crates/ferroplan/src/`;
found the `search.rs:1404` h_cost/FF_CLM mutual exclusion, the
`temporal.rs` static-duration assumption, and the missing RED-fixture/
`FF_NO_*` gates, all now folded in). Confirmed exact by re-derivation: every
per-domain SGPlan delta in §1, the trucks 49-instance decomposition, the
constraints 28/120-zero-rejections state and the stale rankings row, the
188/220 like-for-like propositional rate, the 2023 60 s/300 s calibration
constant, and the transport/model-train/delete-relaxation fence texts.
