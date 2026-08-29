# The field-gaps execution dossier — 0.26

Assembled 2026-08-26 from an eight-spec authoring pass plus a house-law
verification pass (nine agents; every file:line anchor, fixture path, raw-board
number and fence checked read-only against source and raws — the cut25 sweep
owned the box throughout, and nothing was built or run except the two no-code
reads whose results are recorded below). The program: `docs/field-gaps-0.26.md`,
adopted into 0.26 (`docs/roadmap-0.26.md`, "The field-gaps expansion", F0–F6).

Two findings from this pass change the record and are already folded back into
the memo:

1. **§3.7 (plan-then-schedule / model-train) is CLOSED — the exit clause fired
   during the read, on stronger grounds than anticipated.** The imagined core
   already exists: per-pre-state duration resolution has been in-engine since
   v0.10 (`4c3e4e7`, 2026-07-19) — `temporal.rs` classifies fluent-reading
   durations into a `dur_exprs` side table and resolves them against the
   expanding node's state; the `temporal.rs:507–535` initial-state evaluator
   governs only STATIC durations. model-train's 0/30 (27 pure timeouts, 3
   mem-caps) was measured WITH that core in the binary, so a plan-then-schedule
   build could never have claimed the mass. The residue re-routes to the F3
   `charge_pre_num` gate (flat h across advance sequences + float-keyed
   duplicate detection — the closed h-accounting ledger's plateau, by name).
2. **The transport-L3 receipts are ipc-2011 rows, not 2008.** Both independent
   spec agents traced `post-entries25.sh:117–126`: i4 (16.18 s) and i6
   (58.98 s) under `FF_NO_NOVLIGHT=1 FF_NO_LAMA=1` ran the 2011 corpus. The
   memo's §1c said 2008; corrected. Consequence: the 2008 tempo-sat overtake
   has NO direct L3 receipt on its own board yet — the F0(d) widening carries
   even more of the load.

## Amendments (binding over the spec bodies below)

The verification pass found 9 problems (2 major) and ruled on 4 open
cross-spec conflicts. Where a spec body below disagrees with this list, THIS
LIST WINS.

**Rulings on cross-spec conflicts:**

- **R1 — rung ownership at the `search.rs:1404` guard (F1 vs transport-L1).**
  F1's enrichment stays an engine-wide default inside the guarded block. If
  `FF_COSTH_FIRST` is ever promoted default-on for the transport boards, those
  boards' first-plan searches take the `h_cost` branch and LOSE enrichment —
  that disarm is hereby declared an accepted, REFEREED consequence, never a
  silent one: the L1 promotion A/B must run against the enriched default and
  the cut record must name "enrichment disarmed on these boards" with the
  measured delta. F4.1 (quantum-layout) is verified clear of the guard.
- **R2 — F1/F2 landing order and merged mechanics.** F1 lands first; F2
  rebases on the dual-heap core. Lookahead terminals evaluate via whichever
  evaluator the rung is running (relaxed_helpful under enrichment), insert
  into `norm_heap` only, and never carry a pref flag on lookahead edges. The
  determinism test covers the enrichment+lookahead combined path. The
  floor-tile build (if its probe opens it) joins the same merged core with
  disjoint diffs; `search_from`'s insert path has one owner-of-record: F1.
- **R3 — AIBR vs the enriched fallback.** `FF_AIBR=1` forces `pref_ops` off
  for that search (exactly one evaluator-and-queue discipline per search);
  the AIBR probe runs under `FF_NO_ENRICH=1` for clean attribution. The AIBR
  spec's v1 has no helpful-action set — this ruling closes the undefined
  pref-source state.
- **R4 — probe vehicles vs the crucible cut.** Pre-cut probes and A/Bs
  (transport widening, FF_LOOKAHEAD pairs, deadend probe) may use the
  ipc67.py/shell lineage — they are probes, not the cut. Anything ARMED AT
  THE CUT is a manifest-declared board env in crucible (crucible scrubs
  ambient env by construction — ten inherited names plus two injected; an
  ambient `FF_*` arm cannot ride the primary instrument, by design).

**Corrections to spec bodies (apply before executing the named step):**

- *fallback-enrichment*: anchors — `SearchCfg` at `search.rs:188` (not 151),
  `from_weights` ~290, lama's `expanded` vec ~136. All else verified.
- *yahsp-lookahead*: the design text assumes the pre-F1 single-heap fallback —
  read it under R2. Its board A/B is a PRE-CUT probe per R4.
- *crucible-cut26*: env allowlist is TEN inherited names + two injected
  (`FF_TIME_LIMIT`/`FF_MEM_BUDGET_GB`); `PassVerdict::FeatureAbsent` at
  `db/model.rs:418`; ipc-standings line 61's cell also carries
  "1 engine-reject/error" (the −6/+6 + line 52's −1/+1 still sum to −7/+7).
- *model-train read*: census is 10 durative actions (4 fluent-reading +
  4 constant-1 + 2 constant-10). The verdict is unaffected.
- *decode-sittings*: **drop `FF_NOV_LAZYH=1` from Sitting B's condition
  matrix** — the hatch is deliberately unimplemented (`novelty.rs:646`); a
  run under it is a silent no-op and would enter the report as false refusal
  evidence. `FF_NOV_R_CAP/PART/W2` carry the forgetting question. Also: the
  `[search] capped at N evals` phase-split line is gated on `FF_RES_DEBUG`
  (not `FF_WALL_DEBUG`) and prints only on capped returns; the spider fixture
  path needs the `instances/` component.
- *quality-memory*: storage-tc sizing — i8 is 8 crates/4 depots/4 hoists/
  2 containers, i7 is 7 crates; the ablation twin deletes `crate7` (not the
  non-existent crate8). The one-crate cliff claim survives.

**Verification coverage, for the record:** every fixture path, every quoted
raw-board row, every hatch name (13 proposed names all unused; 34 existing
FF_NO_*; 132 FF_* total), every fence (no pre-decode code at fenced walls, the
closed temporal ledger untouched, 2014-transport unclaimed, no
anytime-for-coverage, no new tiers, every armed change carries a restore and a
referee) — checked and confirmed. Size grades read honest throughout.

## Execution order (behind the 0.25 sweep + promote)

1. **F6 part 1** (crucible DB wiring — the kill -9 premise) and **F1**
   (fallback enrichment) — both ungated, build-first.
2. **F0 sittings** A–D (cheap box time, committed reports) + the floor-tile
   deadend probe + the storage-tc fold probe.
3. **F2** (lookahead, rebased per R2), **F5** (2014 reconciliation), **F4**
   (quality polish + memory sitting).
4. **F3 builds** strictly as F0's decodes open them (charge_pre_num → AIBR →
   transport L1–L3 → forgetting/multi-heuristic rung).
5. **F6 parts 2–4**: backfill, Linux cross-check, then the crucible cut26
   runbook (parity gates → sweep → mem-cap fix as its own commit).

---



# ═══ F1 — fallback enrichment ═══

## F1 — Fallback enrichment: preferred operators + landmark count armed into the complete wBFS fallback

**STATUS 2026-08-29: REFEREED on crucible — +8 ipc5-prop (band +10–17:
under-delivered), +4 ipc2018-sat, 14 gains / 2 losses, every gain a
fallback-note row; settlers i12 did NOT convert; default stays ON.** Record
and the shortfall hypotheses: docs/roadmap-0.26.md F1 bullet.
Earlier status (2026-08-28): BUILT, unit pin green, referee in flight on crucible —
record in docs/roadmap-0.26.md ("The field-gaps expansion", F1 bullet). Two
measured facts the spec did not predict: the landmark term alone is inert on
the pin's chain fixture (the deferred h already orders it), and the enriched
fallback crosses the visit-all 10×10 plateau (tests/novlight.rs) that the bare
one caps on. `f1-before` (ff 0.25.0) is sweeping; `f1-armed` follows the
0.26.0 build.

### Goal + evidence anchor

Bring the `lama.rs` dual-heap preferred-operator alternation and the landmark-count ordering term (`FF_CLM`, opt-in since 0.11) into `search_from`'s complete weighted best-first fallback, armed by default, scoped exactly to the plain classical fallback — the search that does most of the solving and today carries neither signal.

Numbers (all re-derived from the committed raws):

- **128 of 369 solved rows** on `benchmarks/ipc5-prop.jsonl` (450-row board) carry "EHC found no improving state; used weighted best-first" (`session.rs:1736/1830`) — the fallback is the workhorse and it is a bare single-queue wBFS (`search.rs:649`, `search_from`; the memo's `:729` anchor is this function — it now sits at 649 on HEAD).
- ipc5-prop like-for-like is **188/220 (85.5%) vs SGPlan 218/220**; per-domain deficit outside trucks: pathways 6, rovers 4, storage 4, tpp 1 (memo §1e). All failures are near-wall timeouts, zero mem-caps: storage-prop i27–30 unsolved having burned **57.6–59.2 s** of a 60 s wall; pathways-prop i17/18/19/21 at **56.6–58.0 s**; rovers-prop 35/38/39/40 and rovers-strips 35/37/38/39/40 at 57.7–60 s.
- 2018 rung-1 mass: settlers has **7 solves >30 s (max 59.93 s)** — i6 41.86, i7 59.62, i11 59.58, i13 59.87, i14 59.65, i15 59.64, i16 59.93, every one via the fallback note — with i12 unsolved between solved neighbors. Settlers' claimed share of rung 1 is +2–3.
- **Band: +10–17 on the ipc5-prop tails at ~2×-equivalent (memo §1e) plus the settlers/near-wall share of the 2018 +9-to-median (rung 1 names §3.1 + §3.4 jointly as the delivery vehicle — this spec claims the settlers slice, not the whole +9).** Trucks' 15 is explicitly NOT claimed — it belongs to the F0(a) decode.

### Gate status

**OPEN NOW.** F1 is listed "ungated" in `docs/roadmap-0.26.md` ("The field-gaps expansion", F1 bullet) and memo §5 Phase 1; no decode gate is declared for it. Two fences honored, stated here so the spec cannot read as fence-breaking:

1. Order of operations: nothing touches the box until the 0.25 cut sweep completes and promotes (roadmap-0.26, field-gaps preamble). Build and unit tests wait for the box; this spec is the pre-work.
2. The pathways/tpp anti-pot fences the **metric-time** walls (memo §4; roadmap-0.26 anti-pots). The pathways-**propositional** tail claimed here is a different track, classified by §1e as a genuine tail, not the fenced riddle. tpp-prop is +1 at most and rides along; no tpp-metric-time contact.

### Design

**The §3.1 guard interaction, resolved BY CONSTRUCTION — disjoint rungs.** The enrichment lives entirely inside the block that already scopes `FF_CLM`: `search.rs:1400–1418` in `plan_avoiding`, guarded by `cfg.h_cost.is_none() && !cfg.anytime`. The cost-augmented first-plan rung (§3.5; `relaxed_costed` at `heuristic.rs:1209`, plumbed via `SearchCfg.h_cost`, only setter `costs.rs:155`) takes the `h_cost = Some(_)` branch and by that same guard never sees enrichment. Justification for not lifting the guard: the fallback key is calibrated in length-h units (one h-unit = 1280 = 5·256, one g-step = 256, one landmark at the FF_CLM default 3.0 = 768 — the `SearchCfg` doc, `search.rs:178–186`), while `relaxed_costed` returns cost-scaled values whose magnitude varies per domain by orders of magnitude; `w_lm` and a pref/normal batch ratio have no domain-independent calibration against that scale. Lifting the guard means answering the key-rescale question with its own referee — not bought here. Rung assignment, explicit: the **plain-length classical fallback** (the rung that writes the 128 fallback notes) gets preferred ops + landmark count; the **h_cost rung, the anytime/metric B&B loops (`sat`/`closure`/`cost_fluent` callers), `len_anytime`, the `solve_subgoal*` partition wrappers, temporal, and the optimal ladder get nothing** — enforced by the cfg-default-off construction below, so every other caller is bit-identical without needing to know this feature exists.

Concrete changes, three files:

1. **`SearchCfg` (`search.rs:151`)** gains `pref_ops: bool`, default `false` in `from_weights` (`search.rs:~305`). Default-false makes every existing construction site bit-identical by construction.

2. **`plan_avoiding` (`search.rs:1147`), inside the guarded block at `search.rs:1400–1418`:**
   - `cfg.pref_ops = std::env::var("FF_NO_ENRICH").is_err();`
   - `w_lm` arming: if `FF_CLM` is set explicitly, honor it exactly as today (including under `FF_NO_ENRICH` — this preserves the historical opt-in path, which the referee's decomposition arm needs); else if enrichment is armed, default `cfg.w_lm = (3.0 * WEIGHT_SCALE) as i64` (768 — the existing FF_CLM parse-fallback weight).
   - `FF_RESLM` untouched. The block runs before the refill loop (`search.rs:1434–1453`), so `round_cfg` carries enrichment into every refill round.

3. **`search_from` (`search.rs:649`), when `cfg.pref_ops`** — the `lama.rs` shape transplanted, key formula untouched:
   - **Dual heaps**: `pref_heap` + `norm_heap` (the `lama.rs:204–217` pop loop; reuse `PREF_BATCH=192` / `NORM_BATCH=64` from `lama.rs:29–30` so the total batch stays 256 = today's `BATCH` and the wall-checkpoint/cap cadence at `search.rs:889–905` is unchanged). An `expanded: Vec<bool>` mirrors `lama.rs:141`'s rule — a node may sit in both heaps, expands once. Every insertion goes to `norm_heap`; `pref_heap` additionally holds successors reached via a parent's helpful op. Completeness is lama's argument verbatim: the normal queue holds everything, so open-list exhaustion still means exhaustion and `Unsolvable{capped:false}` semantics survive.
   - **Evaluator**: the popped batch evaluates via `relaxed_helpful` (`heuristic.rs:982`, returns `Option<(i32, Vec<u32>)>`) instead of `relaxed_to` — same `par_map_with` structure, h value identical (same relaxed plan; helpful is the applicability extraction on top), helpful set rides to expansion, where each successor carries `pref = helpful.contains(&(oi as u32))` (the `lama.rs` `Cand` shape).
   - **Key untouched**: `w_g·g + w_h·ph + w_lm·un` — the existing `lm_acc`/`clm_accept_into` machinery (`search.rs:365–375, 702–717, 1022–1060`) already computes the landmark term; deferred evaluation (parent h as priority) is preserved; goal check, caps, node-cap model, wall checkpoint, refill contract all untouched. Unlike the greedy LAMA rung (no g), the fallback keeps its `w_g` — alternation changes pop order only, never key values, so the weight semantics and the refill escalation (`w_h ×4`) are undisturbed.
   - **When `pref_ops` is false, the existing single-heap/`relaxed_to` code path runs literally** — the new code is an if/else at exactly three touch points (heap structure, pop loop, evaluator choice), so the hatch off-path is bit-identical.
   - **Determinism**: fixed 192/64 shares, order-preserving parallel h eval, serial insertion — the standing thread-count-independence contract (`lama.rs:17–19`, `search.rs` module doc).
   - **Memory model**: with `w_lm` now default-armed, `lm_acc` (`clm_words × 8` bytes/node) is allocated on every fallback node; `per_node_model_bytes` does not count it. The build adds the clm term to the model when `cfg.w_lm > 0` (off-path untouched) — the 0.19/0.22 mem-honesty precedent.

### RED fixture

Primary (2018 near-wall, per the memo's instruction):

- **`benchmarks/.ipc-corpus/ipc-2018/domains/settlers-sequential-satisficing/instances/instance-12.pddl`** (domain: `domain.pddl` beside `instances/`). Today, from `benchmarks/ipc2018-sat.jsonl`: **UNSOLVED at the 60 s budget**, while both neighbors solve at the wall via the fallback — i11 at 59.58 s and i13 at 59.87 s, each with the "EHC found no improving state; used weighted best-first" note (i14–i16 likewise at 59.64–59.93 s). Solo RED repro: `FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 ff` on that pair, quiet box. GREEN condition: i12 solves inside 60 s under the armed binary.

ipc5-prop tail instances the +10–17 band claims (all unsolved today having burned near-full wall, from `benchmarks/ipc5-prop.jsonl`):

- pathways: `benchmarks/.ipc-corpus/ipc-2006/domains/pathways-propositional/instances/instance-17.pddl` (paired `domains/domain-17.pddl`); also i18, i19, i21, i27 and strips i17–19, i21, i22 — 56.6–58.6 s burned.
- storage: `benchmarks/.ipc-corpus/ipc-2006/domains/storage-propositional/instances/instance-27.pddl` — i27–30 unsolved at 57.6–59.2 s, with i22–24 solved at 20.2/23.7/25.9 s all via the fallback note.
- rovers: `benchmarks/.ipc-corpus/ipc-2006/domains/rovers-propositional/instances/instance-35.pddl` — i35/38/39/40 plus strips i35/37/38/39/40 unsolved; solved neighbors i31/33/36 (strips) at 15.7–36.5 s all via the fallback note.

The settlers i12 fixture is the named must-convert; the ipc5-prop list is the band the sweep prices (not every instance is promised).

### Hatch

**`FF_NO_ENRICH`** (new; verified unique against the 34 existing `FF_NO_*` names). Restores the bare single-queue wBFS fallback — single heap, `relaxed_to` evaluator, `w_lm = 0` unless `FF_CLM` is explicitly set — bit-identical off-path by the three-touch-point construction above. `FF_CLM` keeps its exact historical opt-in semantics (explicit env overrides the default weight; works under the hatch), which is what makes the referee decomposition below possible with no second hatch.

### Referee + test plan

- **Which sweep arms it**: the change is default-armed in-engine, so the **0.26 cut sweep** arms it — specifically the `ipc5-prop prop-2006 60` and `ipc2018-sat sat-2018 60` boards of the standing 22-board list (`benchmarks/cut25-sweeps.sh:139,142`; the 0.26 successor runs on crucible per F6, `standings.py` alongside as the differential oracle). Rows staged in `benchmarks/air26*`; `ipc67.py`/crucible `classify()` reads them, and the fallback note on each converted row is the mechanism witness. Crucible's scrubbed-env + `env_json` records that the arm (and nothing else) was in force.
- **Old-binary referee** (roadmap-0.21:308 rule — this reallocates search order/budget, and "a hatch only tests what it gates"): a `backfill-air.sh`-shape leg — same box, CURRENT harness, `FERROPLAN_FF` pointing at the v0.25.0 tag's binary, same `--jobs 2 --mem-gb 6` — on ipc5-prop and ipc2018-sat. The claim is judged against that same-box old-binary board, not the hatch alone.
- **Hatch differential + decomposition**: new `hatch-differential.py` SPECS entry `"enrich"` (hatch `FF_NO_ENRICH`, from_boards ipc5-prop + ipc2018-sat, witnesses = the converted rows), plus a second arm `FF_NO_ENRICH=1 FF_CLM=3` (landmark-term-only, the 0.11 opt-in path) to attribute the pref-queue half vs the landmark half — this directly answers whether the 0.11 negative was the term or its boards.
- **Tests**: (i) determinism: enriched fallback returns the identical plan at threads 1 vs 8 on a fixture task; (ii) off-path pin, `ladder_wall.rs` style (env-child pattern): `FF_NO_ENRICH` run matches the pre-change binary's eval count byte-for-byte on a fixture, and a plateau fixture where the pref queue converts within an eval budget the single queue misses (the RED pin); (iii) existing FF_CLM plumbing and `ladder_wall.rs` note assertions stay green; (iv) VAL + plan-length watch on converted rows (dual-queue boosting can lengthen plans); (v) full pre-flight per `RELEASING.md`.
- **Measured-negative discipline**: if the board A/B reads like 0.17's +7/−51, the default flips off with the receipt recorded; the hatch is the off-path either way.

### Estimated size

**M.** The `search_from` edit is surgical (three touch points plus cfg plumbing, all shapes copied from `lama.rs`), but the referee load is real: two boards swept armed, the old-binary backfill leg, the differential, and the memory-model touch.

### Risks and interactions

1. **Declared adjacency — the 0.11 Phase 3 FF_CLM measured negative** (`docs/roadmap-0.11.md:122–132`): `FF_CLM=3` vs default read transport08 identical, visit-all no-fire, floor-tile worse-direction. What this item stands on: (a) that referee's boards are not this band's constituency, and 0.11's own conclusion says transport/floor-tile-class needs a different heuristic — the tails claimed here are near-wall fallback rows, a different shape; (b) the term now arrives paired with the preferred queue (the LAMA recipe), never measured in the fallback; (c) the arm is refereed on the claiming boards with the decomposition arm isolating the term. A floor-tile-class board regression re-records the negative and kills the default.
2. **Eval-cost tax**: `relaxed_helpful` pays helpful extraction per pop vs `relaxed_to`; the named warning is the 0.22 parking receipt (86 s cumulative worker time at 100k evals of per-pop `relaxed_helpful` in the h-guided novelty rung, `novelty.rs` module doc). Mitigation: extraction shares the RPG the h already builds; the wall checkpoint backstops; the referee prices the net.
3. **Refill interaction** (`search.rs:1434–1500`): rounds escalate `w_h ×4` while `w_lm` and the batch split stay fixed, so the enrichment's relative pull weakens in greedier rounds. Deliberate (escalation stays h-greedy); recorded as a designed asymmetry, not tuned here.
4. **Node-cap undercount**: default-armed `lm_acc` adds `clm_words×8` bytes/node the byte model doesn't count — fixed in this build (design point 3, memory model); the fix itself must not move the hatched path's caps.
5. **Landmark build cost at entry**: `landmarks_for` (`landmarks.rs:36`) is one RPG build per `search_from` call; the refill loop can re-enter up to 6 rounds → up to 6 rebuilds. Same cost FF_CLM pays today; trivial against a 60 s wall; noted.
6. **Orbit co-fire**: thinks pass `orbit` into the same fallback (`session.rs` think path; `search.rs:664` guard). Orbit changes visited keys only, the enrichment changes pop order only — mechanically orthogonal, but the determinism test must cover the combined path.
7. **Inert-by-construction surfaces, stated for the record**: anytime/metric B&B, `len_anytime`, `h_cost` rung, `solve_subgoal*`/partition cascade, temporal, optimal ladder — all keep `pref_ops=false` defaults and are bit-identical; the LAMA and novelty rungs above the fallback are untouched (no double-dipping — LAMA still runs bounded first, and its slice machinery is not this spec's business).
8. **No trucks claims** (F0(a) owns the trucks/storage-time decode; trucks' fast-or-never rows are structural, not tails) and **no 2014-transport claims** — both fences carried by reference.


# ═══ F2 — YAHSP-style lookahead ═══

All evidence is gathered. Here is the spec section.

## F2 — YAHSP-style relaxed-plan lookahead in the complete fallback (`FF_LOOKAHEAD`)

**STATUS 2026-08-29: CLOSED, measured negative — the flag left the tree.**
Parking differential 6/20 both arms (parking is solved by LAMA; the probe
was scoped to the fallback by a misread of the "used weighted best-first"
note, which is not a rung witness); 2018 witnesses −1 (data-network i7 lost).
Both exit-clause halves read against it. Code removed, receipts in
`benchmarks/cut26/lookahead-*.log`, record in docs/roadmap-0.26.md F2 bullet.
The LAMA-side lookahead is the deferred rider, unpriced.

### Goal + evidence anchor

At a popped node, greedily EXECUTE the relaxed plan's actions on the concrete state and hand the resulting deep state to the open list — the YAHSP2/3 mechanism (won IPC-2014 agile and temporal; memo §3.4: "the biggest speed payoff per the field evidence", no anti-pot adjacency, never tried in this engine). The constituency, re-read from the raws for this spec:

- **parking-2014**: 4/20 both configs, and every solve sits AT the wall — sat rows i2 59.52 s / i3 59.80 s / i5 59.63 s / i14 59.90 s (lengths 67/78/89/75), each noting "EHC found no improving state; used weighted best-first"; the agile board is the identical set at 59.51–59.66 s. Unsolved i1/i4/i6/i7/i8 all die at 59.65–59.86 s of 60 (`benchmarks/ipc2014-sat.jsonl`). This is a search that finds the plan and pays ~60 s of wall doing it — exactly the depth-per-eval shape lookahead buys.
- **tetris-2014**: 9/20 sat with solves i4 59.44 / i5 59.56 / i12 59.35 at the wall; unsolved i8 (59.83), i9 (59.73), i11 (59.93), i13 (59.83) are the adjacent band (agile additionally converts i14 at 52.94).
- **cave-diving-2014**: 4/20 — i6–i9 solve at 36.4–45.7 s (all length 33), the other sixteen sit at 54.5–59.9 s. Bimodal; claimed at 0–2, honestly.
- **2018 clusters** (`benchmarks/ipc2018-sat.jsonl`, shared mass with F1 — attribution fence below): settlers 15/20 with six solves at 59.58–59.93 s (i7/i11/i13/i14/i15/i16) and i12/i17–i20 unsolved; data-network unsolved i6–i10 at 58.8–59.3 s; flashfill unsolved i2–i5 at 58.95–59.71 s; spider i7 solves at 59.96 s; nurikabe i12 at 58.0 s.

Band: parking +2–6, tetris +2–4, cave-diving 0–2, 2018 boards +0–4 *marginal over F1* (the near-wall mass is shared; the referee attributes, never sums).

### Gate status

**Open now.** Roadmap-0.26 "The field-gaps expansion" lists F2 as *ungated; never tried in this engine: opt-in hatch first; parking-2014's four 59.5–59.9 s solves are the fixture class*. No decode is named and no decode-before-build gate is declared for this item. The temporal ledger fence is honored by scope: nothing here touches `temporal.rs` or any temporal search.

### Design

**Where the relaxed plan lives today (the load-bearing source fact).** The extraction at `crates/ferroplan/src/heuristic.rs:715` (`relaxed_extract`) produces **only a count** — `select` (heuristic.rs:1285) stamps `sc.selected[oi] = sc.gen` and increments `count`; the only op list materialized is `sc.helpful` (layer-0, applicable-now ops; Scratch field at heuristic.rs:133–135, consumed by EHC via `relaxed_helpful`, heuristic.rs:982). The plan's **ordered action sequence is NOT materialized anywhere**. Its order is recoverable for free: `sc.op_layer[oi]` (stamp-gated, valid post-eval) gives each selected op's RPG layer.

1. **New read-out, `heuristic.rs`** (~15 lines, modeled on `extraction_need_facts` at heuristic.rs:1136): `pub fn extraction_plan_ops(sc: &Scratch) -> Vec<u32>` — all ops with `sc.selected[oi] == sc.gen`, sorted ascending by `(sc.op_layer[oi], oi)` (layer INF-stamped entries keep op-id order). Valid immediately after a `relaxed_to`/`relaxed_helpful` on the same state, same contract as the existing read-outs. Read-only; no extraction change, so helpful sets and h values cannot move.

2. **Which rung: the complete wBFS fallback only** (`search_from`, search.rs:649; the bare single-queue heap at search.rs:729). Justification: (a) every parking/tetris/cave-diving solve row carries "EHC found no improving state; used weighted best-first" (emitted at api.rs:1084) — the fallback does the solving and eats the wall on this fixture class; (b) EHC already owns a lookahead of a different kind (`bfs_improve`, search.rs:1658 — breadth-first over helpful actions); grafting a second jump mechanism into that rung entangles a recorded behavior for no evidence; (c) scoping precedent: `FF_CLM` armed exactly this fallback (search.rs:1401–1418). The LAMA (lama.rs:204) and novelty (novelty.rs:33) dual-heap rungs are untouched. An EHC-side lookahead is a named deferred rider, priced only if the fallback receipt lands.

3. **Arming, `plan_avoiding`** (search.rs:1147, next to the FF_CLM block at 1401–1418): `FF_LOOKAHEAD=1` sets a new `SearchCfg.lookahead: bool` (default `false`; SearchCfg at search.rs:188), under the **same guard** `cfg.h_cost.is_none() && !cfg.anytime` — this makes F2 and the cost-augmented first-plan rung (§3.5) mutually exclusive by construction, per the memo's search.rs:1404 fence. Inside `search_from`, lookahead additionally requires `cost_fluent.is_none() && closure.is_none() && sat.is_none() && cfg.g_bound == usize::MAX && !cfg.len_anytime` — the plain classical fallback exactly. The refill loop (search.rs:1434–1538) re-enters with the flag intact.

4. **Per-pop mechanics** (inside the parallel h-eval closure, search.rs:862–875, worker-local `Scratch`): after `relaxed_to` succeeds for popped node `ni` with h > 0:
   - `rp = extraction_plan_ops(sc)`; state `s = nodes[ni].state.clone()`; `applied: Vec<u32>`.
   - Greedy passes: scan `rp` in order; apply the first-fit un-consumed op that is really applicable (`task.op_applicable`) and not `forbidden`; `s = task.apply(op, s)`; each op consumed at most once (ops selected with reps > 1 execute once — recorded simplification); stop a pass-loop when a full pass applies nothing, `task.goal_met_with` fires, or `applied.len() == rp.len()`.
   - Yield gate: `applied.len() >= 2`, else discard (a 1-step jump duplicates a normal successor).
   - Evaluate `h_la = relaxed_to(terminal)`; relaxed dead end discards. Goal-met terminal gets `h_la = 0`.
   - Closure returns `(h, Option<(terminal, applied, h_la)>)`. `par_map_with` preserves order, so determinism and thread-count independence hold as today.
5. **Open-list entry** (serial insert section, search.rs:977–1064): in popped order, dedup the terminal through the same `visited`/`khash` bucket path; if new, insert a node with `father = ni`, `g = nodes[ni].g + applied.len()`, `op = LA_SENTINEL` (`usize::MAX - 1`), and the edge recorded in a side table `la_edges: FxHashMap<u32, Box<[u32]>>` keyed by node index. Heap key: `cfg.w_g * g_la + cfg.w_h * h_la` plus the `lm/res` terms; when `FF_CLM` co-fires, `lm_acc` is built by running `clm_accept_into` (search.rs:365) over **each intermediate state** so path-accepted landmarks are not skipped. Because `h_la` is the terminal's TRUE h (not the parent's deferred h), the deep node competes fairly at its depth. A goal-met terminal returns `Plan` immediately after insertion (the plain path is first-improvement; serial, deterministic).
6. **Reconstruction** (`reconstruct`, search.rs:1083): on `op == LA_SENTINEL`, splice `la_edges[&idx]` instead of one op. Plans stay valid by construction — every edge op passed `op_applicable` on the concrete state (the existing VAL step referees anyway).
7. **Cost accounting**: each lookahead terminal h-eval counts `evaluated += 1` (honest against `max_eval` and the wall checkpoint at search.rs:888–905, whose cadence is unchanged); `g` advances by real applied steps, so `max_g` and length reporting stay truthful. Per-node byte model (search.rs:146): unchanged; the `la_edges` bytes (≤ h ops × 8 per lookahead node) are a recorded under-count, bounded and accepted for v1.

**Completeness**: lookahead only ADDS successors — every normal successor is still generated and inserted, so the fallback's completeness argument is untouched.

### RED fixture

`/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2014/domains/parking-sequential-satisficing/instances/instance-1.pddl` (28 cars, 15 curbs; shared `domain.pddl` beside it) — the **first unsolved parking instance**. Today's raw row (`benchmarks/ipc2014-sat.jsonl`): `{"ipc":"ipc-2014","variant":"parking-sequential-satisficing","instance":1,"solved":false,"time":59.74,"budget":60}` — dies at the wall while its neighbors i2/i3/i5/i14 solve at 59.52–59.90 s via the fallback. RED = unsolved at 60 s as shipped; GREEN = solved under `FF_LOOKAHEAD=1` at the same budget with a VAL-validated plan. Band witnesses named for the referee: parking i4/i6/i7/i8; tetris i8/i9/i11/i13; cave-diving i10–i13; 2018 settlers i12/i17–i20, data-network i6–i10, flashfill i2–i5 (paths under `benchmarks/.ipc-corpus/ipc-2018/domains/<variant>/instances/`).

### Hatches

- `FF_LOOKAHEAD=1` — opt-in probe (name verified unused in tree). Flag-off leaves every existing path byte-identical (the read-out is dead code; `SearchCfg.lookahead` defaults false).
- On promotion (only on a banked sweep receipt): default-on scoped to this fallback, shipping **`FF_NO_LOOKAHEAD`** as the named restore — bit-identical off-path by construction, per house law.

### Referee + test plan

1. **Unit pins**, new `crates/ferroplan/tests/lookahead.rs`: (a) synthetic serial-chain task — flag-on solves in one expansion round with the multi-op edge, plan replays valid; (b) flag-off pin — plan and eval count identical to today on an existing fixture; (c) `extraction_plan_ops` ordering pin (layer-sorted, deterministic).
2. **Armed differential** (the sweep that arms it — house law): a `lookahead` entry in `benchmarks/hatch-differential.py` `SPECS` over `("ipc-2014", "parking-sequential-satisficing")`, arms `FF_LOOKAHEAD=1` on one arm, both arms interleaved on one box at 60 s. Question: "do the wall-sitting solves come off the wall, and do i1/i4/i6–i8 convert?"
3. **Board A/B in the 0.26 cut chain**: `benchmarks/cut26-sweeps.sh` (post-cut25, per the standing queue discipline) runs `ipc67.py` pairs — ipc2014-sat, ipc2014-agile, ipc2018-sat — with `FF_LOOKAHEAD=1` vs unarmed, raws to `benchmarks/cut26/lookahead-{on,off}-<board>.jsonl`, read by `benchmarks/compare.py` per-instance. **Attribution fence vs F1**: the 2018 near-wall mass is claimed by both; the A/B must also run the F1-enriched binary ± `FF_LOOKAHEAD` so the marginal is attributed, never double-banked.
4. **Old-binary referee** (roadmap-0.21 rule — this is a search-order/budget-reallocation claim: lookahead spends h-evals differently inside the fallback): the promotion sweep re-runs the fixture class with `FERROPLAN_FF` pointed at the 0.25 tagged binary on the current harness, the backfill pattern.
5. **Exit clause**: if the parking differential converts none of i1/i4/i6/i7/i8 and the 2018 marginal reads ≤ 0, the negative is recorded and the flag leaves the tree — the named sweeps guarantee evidence either way (no-sweep-no-evidence law satisfied).

### Estimated size

**M.** ~15 lines heuristic.rs read-out; ~100–130 lines in `search_from` + `plan_avoiding` (closure extension, serial insert arm, sentinel reconstruction); tests + one SPECS entry + sweep-script arm.

### Risks and interactions

- **Per-pop cost up to 2×** (a second RPG build per popped node) — the exact tax the opt-in + A/B prices; on slow-h domains (the openstacks shape) it can cost near-wall rows, which is why no default flips without the board receipt.
- **Plan-length inflation**: greedy jumps lengthen plans (parking today: 67–89 steps); coverage dominates at 4/20, but the sat-board quality delta is read in the referee, not assumed away.
- **§3.5 mutual exclusion** by the shared `h_cost.is_none() && !cfg.anytime` guard — the memo's declared fence, resolved by construction.
- **`FF_CLM` co-fire** implemented (per-intermediate landmark acceptance) but untested until a sweep arms both — declared, mirroring the F3 co-fire declaration style.
- **Helpful-action pruning**: none in this rung (the fallback expands all ops); lookahead is additive, completeness untouched. EHC's own lookahead is deliberately not modified.
- **Novelty/LAMA rungs untouched**; the ladder order and slices (search.rs:1272–1393) are unchanged, so no 0.17-style rung-tax exposure.
- **Byte-model under-count** for `la_edges` (bounded ≤ h×8 bytes per lookahead node) — recorded; revisit only if a mem-cap row names it.
- **Reps>1 numeric achievers execute once** — recorded simplification; numeric soundness holds since real applicability is checked per step.
- **Temporal scope**: classical `search_from` only; nothing near `temporal.rs` — the closed temporal delete-relaxation ledger is not approached.


# ═══ F0 — the four decode sittings ═══

All evidence is gathered and verified; here is the spec section.

## F0 — the four decode sittings, as executable probe plans

**Goal + evidence anchor.** Four no-code decode sittings (memo `docs/field-gaps-0.26.md` §1a/§1d/§3.2/§1c; roadmap `docs/roadmap-0.26.md` F0 a–d) turned into runnable checklists: exact commands, exact instances, the introspection the engine already carries, a fixed probe budget, a committed-report path, and an exit clause each. The stakes, re-read from the raws on 2026-08-26 (fresh 60 s cut25 rows where banked, tracked raws elsewhere): (A) trucks-time 11/30 and storage-time 15/30 at 60 s with every solve ≤6.21 s / ≤1.93 s — the −49-gross trucks family, +4 flips ipc5-time; (B) rubiks-cube-agile 5/20 at 60 s AND 5/20 at 300 s (5× bought exactly +0), floor-tile-2011 7/20, spider 4/20 — the cliff class the forgetting/multi-heuristic rung claims; (C) pathways-metric-time 0/30 at 60 s (i1 now fails at 38.4 s UNDER the wall with a named note), tpp 8/40 (the 60 s tier move dissolved the i4 cliff into a budget tail through i8 — the riddle narrowed), rovers i3 converted at 45.13 s while i5 stays down; (D) transport L1–L3's +8–20-of-211 band rests on exactly two receipts. Sittings are the 0.25 Phase 4 mould: **a design read is a committed artifact, never a conversation.**

**Gate status.** Open now — F0 is itself the gate layer. Order of operations per the roadmap: nothing touches the box until the 0.25 cut sweep completes and promotes (`benchmarks/promote-air25.sh`). Two boards these sittings read are still mid-flight: `benchmarks/air25/ipc67-results.jsonl` has the 2008 half banked (300 rows) but not 2011 (180 rows, no transport yet, no `.done`), and the `ipc7-mco-*` boards are not staged. Each checklist therefore begins "re-read the fresh board row" and D's 2011 leg waits for its bank.

**Design — the shared machinery (all verified in source).**

- *Runner shape*: the `post-entries25.sh` §3b probe idiom — `FF_TIME_LIMIT=60 /usr/bin/time -o R.time env <HATCHES> ./target/release/ff -o DOM -f PRB --json >R.json 2>R.log`. Debug hatches print to stderr, so `2>R.log` is the capture. Solo and serial per the `hatch-differential.py` rule (a differential holds A-vs-B conditions constant, not the board's job count); quiet-box discipline (`wait_quiet`, ≥70% idle) before every leg. Receipts under a new `benchmarks/decode26/` (gitignored like air*/); reports commit under `benchmarks/metrics/` (precedent: `attribution-0.25.md`). Local-only, detached, on this box.
- *Introspection already in the engine* (the full survey — `features.rs` holds only the temporal-tier flags; the trace/stat hatches live at their use sites):
  - `FF_WALL_DEBUG=1` — wall/checkpoint narration: classical `search.rs:893-894` ("best-first checkpoint expired at {evaluated} evals"), `search.rs:917` (**the phase split**: "capped at {evaluated} evals: h {}ms, expand {}ms, insert {}ms, total {}ms" — plateau-vs-throughput in one line), `search.rs:1611-1613` (EHC slice exhausted, evals count); temporal `temporal.rs:2901-2903` (pass entry refused) and `temporal.rs:3170-3176` ("nodes {}, evaluated {}" at {}ms — open-list size + eval count at the wall).
  - `FF_RES_DEBUG=1` — structure narration: `[tsearch] pass start: prune/masked/words/fv/rel_fluents/tils/ops` (`temporal.rs:2907-2909` — grounded-op count, relevant fluents, **TIL count**), the relevance-mask/[TREL] reads, novelty-rung acceptance narration (`novelty.rs:200,253,280,442,664,754,841`), preference statics (`pddl3.rs:1183`).
  - `--json` `statistics.evaluated_states` (`api.rs:185`) — eval counts on every run, no hatch needed; and the unsolved temporal story note shipped in 0.25: "temporal ladder exhausted its budgets with {N} s of wall left" (`api.rs:1002-1005`) — already present on fresh raws (pathways i1).
  - *Deterministic slicing*: classical `--max-evaluated N` (CLI) and `FF_SEARCH_NODE_CAP`; temporal `FF_TEVAL_BUDGET` (`temporal.rs:936`) and `FF_TEMPORAL_NODE_CAP` (`temporal.rs:2810`) — run the same instance at 10k/30k/100k/300k evals and read where progress (h floor, goal-agenda depth, note text) stops moving: the poor man's h-collapse trace, machine-independent.
  - *Plan-side*: `introspect.rs::explain` (causal links on classical plans, `over all` invariant spans on temporal plans) and `trace.rs::trace` (classical state replay) — solved-neighbour instruments: read the last solved instance's structure to name what the first failing one must coordinate.
  - `FF_ORBIT_DEBUG=1` (`orbits.rs:335`, `temporal.rs:962,3047,3607`) — symmetry-group sizes where relevant.
- *Instrument limit, stated*: neither search prints a per-eval h series. The sittings work from eval-budget slicing + wall narration first; if a sitting cannot name its mechanism without one, a **local, uncommitted** eprintln patch on a scratch worktree is permitted, with the diff reproduced verbatim in the report. No committed engine code — these are the no-code sittings the anti-pot demands.
- *Hatch names*: sittings arm **no** change, so no new `FF_NO_*` ships here — the restore-hatch law binds the F3 builds these reports open (each F3 spec carries its own). The flag-evidence law is satisfied the other way around: a sitting's evidence is its committed report, and every condition below uses hatches that already exist.

---

### Sitting A — trucks/storage-time (memo §1d; report: `benchmarks/metrics/decode-trucks-storage-0.26.md`)

**Instances (RED set, exact paths, today's failing behavior from `benchmarks/air25/ipc5-time.jsonl`, budget 60):**
- `benchmarks/.ipc-corpus/ipc-2006/domains/trucks-time/instances/instance-12.pddl` — unsolved at 60 s; i11 solves in 6.21 s; i12 adds exactly ONE package (13→14, same 2 trucks/4 locations/3 truckareas — measured from `:objects`).
- `benchmarks/.ipc-corpus/ipc-2006/domains/storage-time/instances/instance-15.pddl` and `instance-17.pddl` — unsolved at 60 s while i14 solves in 1.21 s and **i16 in 0.12 s** (i15 adds one hoist over i14 at equal 15 storeareas/5 crates; i16 is BIGGER — 18/6 — and trivial: the cliff is not size-monotone).
- The serialization contrast pair, free in the fresh raw: `trucks-time-strips` is 13/30 and **non-monotone** — i11/i12 FAIL yet i13 solves in 22.21 s, i14 in 47.59 s, i18 in 0.94 s. Same mechanism family, different encoding; whatever kills the ADL variant at i12 is not plain scale.

**Fence, stated before the sitting (memo §1d, verbatim intent):** the temporal delete-relaxation ledger is CLOSED at ten negatives. An exit that lands on "temporal h accounting" is dead on arrival — so `FF_TRPG`/`FF_H_ENDGATE` are **not** in this matrix; the sitting instruments search shape, serialization, and compiled structure only.

**Checklist (run post-sweep, box quiet):**
```sh
D06=benchmarks/.ipc-corpus/ipc-2006/domains; OUT=benchmarks/decode26; mkdir -p $OUT
# 1. Shape read, both cliff edges + the last solves (6 instances × baseline):
for spec in trucks-time:11 trucks-time:12 trucks-time-strips:13 \
            storage-time:14 storage-time:15 storage-time:17; do
  v=${spec%%:*}; i=${spec##*:}
  FF_TIME_LIMIT=60 FF_WALL_DEBUG=1 FF_RES_DEBUG=1 /usr/bin/time -o $OUT/A-$v-i$i.time \
    ./target/release/ff -o $D06/$v/domain.pddl -f $D06/$v/instances/instance-$i.pddl \
    --json >$OUT/A-$v-i$i.json 2>$OUT/A-$v-i$i.log
done
# Read: [tsearch] pass start (ops/tils/rel_fluents — is the i11→i12 grounding jump
# linear?); the wall line's nodes/evaluated (open-list shape: a thin deep list is
# serialization, a fat flat one is a plateau); the story note; statistics.evaluated_states.
# 2. Eval-budget slices on the two RED instances (where does progress stop?):
for k in 10000 30000 100000 300000; do
  FF_TIME_LIMIT=60 FF_TEVAL_BUDGET=$k FF_WALL_DEBUG=1 ./target/release/ff \
    -o $D06/trucks-time/domain.pddl -f $D06/trucks-time/instances/instance-12.pddl \
    --json >$OUT/A-tt12-ev$k.json 2>$OUT/A-tt12-ev$k.log
done   # repeat for storage-time i15
# 3. Search-order conditions matrix on i12/i15/i17 (one hatch per run):
#    FF_NO_TSYMM=1 (temporal.rs:2982)  FF_TLIFO=1 (temporal.rs:2501)
#    FF_TEMPORAL_ABS_KEY=1 (temporal.rs:2986)  FF_NO_TLAMA=1 (temporal.rs:461)
#    FF_TDEMAND=1 / FF_NO_TDEMAND=1 / FF_NO_ESCALATE=1 (features.rs)  FF_TDECOMP=1
# 4. One 300 s solo leg on i12 and i15 (probe, not a tier): does 5x buy anything at all?
# 5. Serialization read on the SOLVED side: introspect::explain invariant spans on
#    trucks-time i11's plan (via a small --json + explain driver) — how many drive/load
#    intervals overlap? If the plan is fully serialized, name WHY (truckarea 'closer'
#    chain? hoist 'lifting' exclusivity?) against the domain text quoted in the report.
```
**Probe budget:** ~48 timed runs ≤60 s + two 300 s legs ≈ **2 h box time**, one sitting. **Exit clause:** budget exhausted ⇒ the report commits whatever is named — a mechanism (with the instance/number that shows it), or "not named; the −49 stays undiagnosed" — and no build opens. A named mechanism inside the closed temporal-h class also closes the item (recorded, DOA per the fence).

---

### Sitting B — the cliff decode that precedes the forgetting/multi-heuristic rung (memo §3.2; report: `benchmarks/metrics/decode-cliff-0.26.md`)

**STATUS 2026-08-29: EXECUTED — report `benchmarks/metrics/fieldgaps-B-cliff.md`
(165 rows, clean box, 0 starved). The rung AS SPECIFIED is refused; three
families name narrower mechanisms: rubiks needs the h-guided novelty rung the
slot's h-free driver replaced (i5/i6 solve in <1 s under `FF_NOVELTY_ONLY`,
i7 falls to nothing); spider's driver `|R|` cap binds (64 loses i9, 1024 solves
it with 26% fewer evals); labyrinth pays a 25 s ladder tax ahead of the driver.
floortile REFUSES (9–10M-eval flat plateau, nothing moves). Follow-up B2 (rubiks
board under `FF_NOV_OLD=1` at 5/10/30% slices; spider i1–i6 at R-cap 1024/4096)
is queued behind F5 — the two candidate builds are gated on its number.**

**Claim under test:** the Scorpion-Maidu ingredients (novelty-with-forgetting; alternation across DIFFERENT heuristics' queues). Verified in source before the sitting: forgetting is absent (`novelty.rs` clears its buffers per iteration — `r_true.clear()/r_new.clear()`, novelty.rs:809-810); dual pref/normal batch alternation WITHIN one heuristic already ships (`lama.rs:204-210`, `novelty.rs:33-35`). The deliverable is **the named mechanism forgetting/alternation would fix in THIS engine — or refusal**; the rung builds only on that number (0.24/0.25 SAT-wing lesson: field receipts do not price these walls).

**Instances (exact paths; failing behavior from the raws):**
- `benchmarks/.ipc-corpus/ipc-2023/domains/rubiks-cube-agile/instances/instance-{5,6,7}.pddl` — i5 solves 8.4 s @60 s / 32.78 s @300 s; i6–i20 burn both walls (`ipc2023-agile.jsonl` vs `ipc2023-agile-300s.jsonl` — the purest cliff on any board).
- `benchmarks/.ipc-corpus/ipc-2011/domains/floor-tile-sequential-satisficing/instances/instance-{7,9,10}.pddl` — fresh board 7/20 (i7 fails at 58.72 s beside i8's 8.22 s solve); the 0.22 plateau read (best_h flat, dedup 0.0%) stands to be re-confirmed with today's instrument.
- 2018/2023 residue: `ipc-2018/domains/spider-sequential-satisficing/instance-{1,9}.pddl` (fresh 4/20; i1–i6 all wall), `ipc-2023/domains/labyrinth-agile/instances/instance-1.pddl` (0/20), `slitherlink-agile/instances/instance-4.pddl` (3/20), `recharging-robots-agile/instances/instance-6.pddl` (5/20). Excluded by fence: folding (memory domain — F4's sitting), ricochet (anti-pot: climbs with budget or not at all).

**Checklist:**
```sh
C=benchmarks/.ipc-corpus; OUT=benchmarks/decode26
# 1. Wall-slice phase split, per instance (the search.rs:917 line is the instrument):
FF_TIME_LIMIT=60 FF_WALL_DEBUG=1 FF_RES_DEBUG=1 ./target/release/ff \
  -o $C/ipc-2023/domains/rubiks-cube-agile/domain.pddl \
  -f $C/ipc-2023/domains/rubiks-cube-agile/instances/instance-6.pddl \
  --json >$OUT/B-rubiks-i6.json 2>$OUT/B-rubiks-i6.log     # ... all 10 instances
# 2. Rung isolation (which rung owns the wall?): one condition per run —
#    FF_NOVLIGHT_ONLY=1 (search.rs:1235)   FF_NOVDRIVER_ONLY=1 (search.rs:1248)
#    FF_NOVELTY=1 / FF_NOVELTY_ONLY=1 (novelty.rs:30)   FF_NO_LAMA=1 (search.rs:1317)
#    FF_NO_NOVLIGHT=1 (search.rs:1293)    FF_NO_REFILL=1 (search.rs:1435)
# 3. The forgetting question, asked directly with existing ablations:
#    FF_NOV_R_CAP=64 / =1024 (novelty.rs:542 — does the |R| cap SATURATE? a cap-bound
#    plateau is the one mechanism forgetting addresses), FF_NOV_PART=0, FF_NOV_W2=0,
#    FF_NOV_LAZYH=1 (novelty.rs:646).
# 4. The alternation question: FF_CLM=3 (search.rs:1404-1410 — a SECOND ordering
#    signal in the fallback; if a landmark term moves a cliff instance at all, queue
#    alternation has something to alternate WITH; if nothing moves, refusal evidence).
# 5. Deterministic slices: --max-evaluated 30000/100000/300000 — is best-h moving?
#    (h floor per slice from the wall lines + evaluated_states.)
```
**Probe budget:** ~10 instances × ~7 conditions ≤60 s ≈ **2.5 h box time**. **Exit clause:** the report either names, per board family, a mechanism in the engine's own vocabulary ("|R|-cap saturation with h flat" ⇒ forgetting; "the fallback's single signal is exhausted while a second signal reorders" ⇒ alternation) with the instance and number that shows it — or refuses the rung. A refusal is a completed deliverable; the rung then stays un-built this cycle (standing width rule), and if a later referee reads like 0.17's +7/−51 it dies the same day.

---

### Sitting C — the metric-time widening: pathways/tpp + rovers i3/i5 (memo §1a; rides roadmap Phase 3; report: `benchmarks/metrics/decode-metrictime-0.26.md`)

**Fresh evidence the sitting starts from (this is already movement — record it first):** at the 60 s tier, `air25/ipc5-metric-time.jsonl` reads tpp 8/40 (i4 12.17 s … i8 41.53 s — **the "cliffs at i4" story is now a budget tail; the riddle is i9+**), pathways 0/30 with i1 failing at **38.4 s carrying the note "temporal ladder exhausted its budgets with 22 s of wall left"** (`api.rs:1002` — the 0.25 story plumbing paying off), rovers 6/40 with **i3 converted (45.13 s)** and i5/i6/i9+ still down. tpp-metric-time-constraints stays 0/30 (`air25/ipc5-constraints.jsonl`) — the empty-`(:constraints (and))` riddle rides along.

**Instances (exact paths):** `benchmarks/.ipc-corpus/ipc-2006/domains/pathways-metric-time/instances/instance-{1,2,3,5}.pddl`; `tpp-metric-time/instances/instance-{9,10}.pddl`; `tpp-metric-time-constraints/instances/instance-1.pddl`; `rovers-metric-time/instances/instance-{3,5,6,9}.pddl` (i3 = the solved-neighbour instrument for i5).

**Checklist:**
```sh
D06=benchmarks/.ipc-corpus/ipc-2006/domains; OUT=benchmarks/decode26
# 1. The pathways ladder autopsy: the note says budgets exhaust with wall LEFT — so
#    which rung eats it? Baseline + per-tier conditions on i1/i2/i3/i5:
#    (a) FF_WALL_DEBUG=1 FF_RES_DEBUG=1 (which passes run, [tsearch] ops/rel_fluents,
#        where each rung's budget dies);  (b) FF_NO_ESCALATE=1;  (c) FF_TDEMAND=1;
#    (d) FF_NO_TDEMAND=1 (pristine path);  (e) FF_NOREL=1 (relevance pruning OFF —
#        the [TREL] mask was one of 0.25's two bugs; is its conservative fallback
#        still eating ops?);  (f) FF_TDECOMP=1;  (g) FF_TEVAL_BUDGET slices.
FF_TIME_LIMIT=60 FF_WALL_DEBUG=1 FF_RES_DEBUG=1 ./target/release/ff \
  -o $D06/pathways-metric-time/domain.pddl -f $D06/pathways-metric-time/instances/instance-1.pddl \
  --json >$OUT/C-pw-i1.json 2>$OUT/C-pw-i1.log
# 2. rovers bimodality: same baseline + slices on i5/i6/i9; one 300 s solo probe each
#    (i3's 45.13 s conversion predicts part of this family is budget-shaped — split
#    the domain's deficit into budget-vs-mechanism with numbers).
# 3. tpp tail: i9/i10 baseline + 300 s solo probe (does the tail keep converting?).
# 4. tpp-mtc i1 vs tpp i1 A/B under FF_RES_DEBUG (what the empty block disarms
#    on its way through the PDDL3 wing — pddl3.rs:1183 statics narration).
```
**Probe budget:** ~30 timed runs + five 300 s solo probes ≈ **2 h box time**. **Exit clause & gate linkage:** this sitting IS the Phase 3 decode that the F3 metric-time builds are gated on. The `charge_pre_num` temporal arming (`ground.rs:3114` clears via `!stratified`; `FF_NO_NUMPRE` restore at `heuristic.rs:782`) opens ONLY if the report names a mechanism outside the closed temporal-h-accounting class, and its build spec must carry the workshop-economy quality negative (`packed.rs:132-139`) and declare the `FF_H_ENDGATE`/`FF_TRPG` co-fire untested; AIBR opens only if the report names Metric-FF-class relaxation blindness. No named mechanism ⇒ both stay shut, recorded. No code at the pathways/tpp/rovers walls before this report lands (standing fence, rovers by this cycle's extension).

---

### Sitting D — the transport L1–L3 probe widening (memo §1c; rides roadmap Phase 4; report: `benchmarks/metrics/decode-transport-0.26.md`)

**Provenance to settle, on the record:** the banked L3 receipts (`benchmarks/air25-entries/transport-L3-i{4,6}.json`) were produced by `post-entries25.sh` §3b against **`ipc-2011`**`/domains/transport-sequential-satisficing` (i4 16.18 s vs 59.59 s on-board; i6 58.98 s vs unsolved), while memo §1c cites them as "2008 i4/i6". The widening covers both boards and retires the ambiguity.

**Instance lists (full, exact paths) and conditions matrix:**
- **2011 classical** — `benchmarks/.ipc-corpus/ipc-2011/domains/transport-sequential-satisficing/instances/instance-{1..20}.pddl` (board today: 2/20, only i4/i5 at 59.59/59.88 s — `ipc67-default.jsonl`; re-read `air25/ipc67-results.jsonl` once its 2011 half banks). Conditions, one run each, serial solo: (1) default; (2) `FF_NO_NOVLIGHT=1 FF_NO_LAMA=1` (the L3 pair); (3) `FF_NO_NOVLIGHT=1` alone; (4) `FF_NO_LAMA=1` alone — which rung's wall slice (novelty-light's 10% at `novelty.rs:408`, LAMA's 25% at `search.rs:1324`) is the tax.
- **2008 classical** — `ipc-2008/domains/transport-sequential-satisficing-strips/instances/` — the ten fresh-board unsolved rows **i8, i9, i10, i18, i19, i20, i27, i28, i29, i30** (`air25/ipc67-results.jsonl`: 20/30) plus the three near-wall solves i6/i16/i17 (59.63/59.62/59.84 s) as speedup witnesses. Same four conditions.
- **2008 transport-numeric (tempo-sat)** — `ipc-2008/domains/transport-temporal-satisficing-numeric-fluents/instances/` — board 4/30 (i1, i2, i11, i12 only; fresh `air25/ipc67-temporal.jsonl`). The classical rung hatches do not reach this path, so this leg is **instrument-first**: baseline `FF_WALL_DEBUG=1 FF_RES_DEBUG=1` on i3–i6 and i13/i14 ([tsearch] ops/tils/rel_fluents; where the fuel-numeric structure lands), plus `FF_TDEMAND=1`, `FF_TDECOMP=1`, `FF_NO_TSYMM=1`, and `FF_TEVAL_BUDGET` slices. No temporal-relaxation exits (closed ledger).
- **mco** — no new runs: read the fresh `air25/ipc7-mco-t{2,4,8}.jsonl` when banked (0.24-era: t2 4, t4 5, t8 7 of 20 — cores enumerate a plateau).

**Pricing rule (the number the memo demands):** conversions per BOARD under the winning condition. The +8–20 band is aggregated across 2008/2011/mco with no per-board split priced; this sitting produces the split. The §1c overtake hypothesis is judged on the tempo-sat leg alone: it needs **~+10 on the 2008 tempo-sat board specifically** — classical-leg conversions do not count toward it. Fence carried verbatim: 2014 transport is NOT claimable (fresh `ipc2014-sat.jsonl` 0/20 confirms the 25-package wall; in writing before any code).

**Referee:** `FF_NO_NOVLIGHT`/`FF_NO_LAMA` conversions are driver/search-order claims — the roadmap-0.21 rule applies: re-run every converted instance on the 0.25.0 tagged binary (worktree build, pointed at via `FERROPLAN_FF` exactly as `ipc67.py:36-40` supports) before any is priced. **Probe budget:** ~(20+13)×4 + ~8×5 runs ≤60 s ≈ **3 h box time**. **Exit clause:** if aggregate conversions land under 8, or land off the 2008 boards, the 2008-overtake hypothesis is recorded refused and the band re-priced down from the receipts — measured negative, never papered over. A priced split opens the F3 transport build (which then ships its own RED fixture — a named converted instance — and its own `FF_NO_*` restore, armed at the 0.26 cut sweep with `standings.py`/crucible as referee).

---

**Referee + test plan (all four).** Sittings ship no code, so the referee is evidential: every number in a report traces to a receipt file under `benchmarks/decode26/` (json + time + stderr log) or a committed raw row cited by path; conditions per leg recorded (`contention.py --out` watcher on any leg longer than ~30 min); driver-order claims old-binary refereed (D, and B if a rung-isolation condition converts anything). Reports commit whatever they conclude — "not named" is a result. Each report ends with the gate verdict for its F3 dependent, one line, quotable by the cut record.

**Estimated size.** S per sitting as specs go (no code, ~9 h total box time across quiet windows, four reports); the four together are one M-sized commitment of box time and reading.

**Risks and interactions.** (1) The box: all runs queue behind the 0.25 cut sweep and promote — nothing here starts while a sweep owns the box (standing rule; phantom failures). (2) Two boards these sittings cite are mid-flight (2011 seq-sat half, mco) — checklists re-read fresh rows first so no probe measures against a stale baseline. (3) The 60 s tier move already moved C's ground truth (tpp i4–i8, rovers i3) — reports must quote the 60 s rows, not the memo's 30 s-era shapes, and say so. (4) Fence collisions are pre-declared: no temporal delete-relaxation exits (A, C — `FF_TRPG`/`FF_H_ENDGATE` excluded from matrices), no code at any probed wall pre-report, folding/ricochet excluded from B, 2014 transport excluded from D. (5) `FF_CLM` in B's matrix doubles as F1 evidence — F1's build spec must not cite B's probe rows as its armed-sweep evidence (the flag law needs the cut sweep itself). (6) The D provenance discrepancy (memo "2008 i4/i6" vs the 2011 receipts) is settled by measurement, not by editing the memo — the report records both readings and the true split.


# ═══ F3 — the gated builds ═══

## The gated builds — four specs armed in advance, so their decodes open doors instead of design sessions

Memo anchors: `docs/field-gaps-0.26.md` §1a (metric-time build candidates), §3.3 (AIBR), §3.5 (cost-sensitive first plan), §2 rung 2 (2014 reconciliation). Every part states its gate; nothing here is built before its gate opens. Verified against source 2026-08-26, with the cut25 sweep untouched (read-only sitting).

---

### 1. `FF_NUMPRE_TEMPORAL` — the charge_pre_num temporal hatch (one line, gated on the Phase 3 decode)

**Goal + evidence.** Arm the 0.21/0.22/0.24 numeric-precondition charge (a1 + damping + a2 chain) on temporal groundings, opt-in, aimed at the 2006 metric-time family: pathways-metric-time **0/30** with every failure UNDER the 30 s wall (raws: i1 0.02 s → i8 17.38 s, `benchmarks/ipc5-metric-time.jsonl`, budget stamp 30), tpp-metric-time **3/40** cliffing exactly at i4 (i3 0.34 s, i4 10.05 s), rovers-metric-time **5/40** bimodal (i7/i8 at 0.01/0.06 s beside i3/i5/i6 at the wall). The receipt that makes this worth a hatch: the a2 chained charge converted pathwaysmetric-2023n i2 from 948,388 dead evals to 173 (0.24 P6.3), and it is INERT on these boards today purely by the grounding-entry rule.

**Gate status.** **CLOSED.** Opens only if the 0.26 Phase 3 decode (pathways/tpp riddles, `docs/roadmap-0.26.md` Phase 3) names a mechanism **outside the closed temporal h-accounting ledger** (ten negatives, CLOSED at 0.22). Declared adjacency, verbatim from the record: the 0.22 scoping armed the *pre-a2* charge on temporal groundings and measured NEGATIVE — model-train plateau re-leveled 6→13 and stayed flat at 683,555 evals (roadmap-0.22 anti-pots). The distinction this build stands on: the a2 chain landed later with its RED fixture converted, and the constituency is 2006 metric-time, not the TMS/model-train plateau. A decode exit that lands on "temporal h accounting" keeps this gate shut — that is the anti-pot, not a loophole to argue with.

**Design.** One armed line. `ground.rs:3114` currently reads `charge_pre_num: !stratified,` (inside the packed-task constructor; `stratified` is the `ground_v` parameter at `ground.rs:1542` — the temporal snap/session entries `ground_stratified`/`ground_stratified_walled` pass true). Change to:

```
charge_pre_num: !stratified || std::env::var("FF_NUMPRE_TEMPORAL").is_ok(),
```

Everything downstream already exists and is untouched: the heuristic gate at `heuristic.rs:782-784` (`task.charge_pre_num && !task.pre_num.flat.is_empty() && FF_NO_NUMPRE unset`), the damping halves (`FF_NUMPRE_NODAMP/NOSKIP/NOSUM`), the chain (`FF_NUMPRE_DEPTH`, `FF_NO_NUMPRE_CHAIN`). The change also updates the two now-stale comments as part of the same commit: `packed.rs:132-139` ("FALSE on the temporal snap/session entries") and `heuristic.rs:955` ("the two passes never co-fire today").

**Naming decision (asked by the memo):** `FF_NUMPRE_TEMPORAL` opt-in, **keep `FF_NO_NUMPRE` untouched as the deep restore** — it already kills the charge at `heuristic.rs:784` regardless of how `charge_pre_num` was set, so the restore story is: flag unset = bit-identical off-path (the `!stratified` expression short-circuits identically); flag set + `FF_NO_NUMPRE=1` = still off. No collision: neither name exists in the current 130-hatch registry.

**RED fixture.** `benchmarks/.ipc-corpus/ipc-2006/domains/pathways-metric-time/instances/instance-2.pddl` — today unsolved at 0.12 s of a 30 s budget on the banked board (and still 0/30 after the 0.25 dur-0 + [TREL] fixes, per the post-entries25 30-row re-measure). If the decode names a different instance, the fixture follows the decode; pathways i2 is the standing default (the 2023n twin of the a2 receipt).

**The mandatory quality probe rider (memo condition, non-negotiable):** the probe carries the workshop-economy temporal fixture — `benchmarks/village/domain.pddl` (durative) + `benchmarks/village/workshop.pddl`, test `crates/ferroplan/tests/village.rs::workshop_forges_the_chisel_before_carving`. The recorded negative (`packed.rs:132-139`): the charge re-routed the 27-step carve plan to a 47-step chisel-sale plan when armed on temporal tasks. **Today's test asserts only solve + forge-before-carve — the 47-step re-route would pass it silently.** The build therefore adds a plan-length pin to the probe (workshop under `FF_NUMPRE_TEMPORAL=1` must stay at the carve-plan length), and the 27→47 shape recurring is a recorded negative that closes the item.

**Hatches.** `FF_NUMPRE_TEMPORAL` (opt-in arm), `FF_NO_NUMPRE` (existing deep restore), plus the standing attribution hatches for bill decomposition.

**Referee + test plan.** No standing sweep arms an opt-in flag, so: the Phase 3 post-decode probe runner (the `post-entries25.sh` pattern; receipts under `benchmarks/air26-probes/`) runs the metric-time constituency (pathways 30 + tpp 40 + rovers 40 rows) at the 30 s board budget with `FF_NUMPRE_TEMPORAL=1`, solo, post-cut-sweep. Referee reads: (a) coverage vs the banked `ipc5-metric-time.jsonl` baselines 0/30, 3/40, 5/40; (b) the temporal quality surfaces — workshop plan length (the pin above) and makespans on a solved-row sample from `ipc67-temporal.jsonl`/`ipc2014-tempo.jsonl`/`ipc5-time.jsonl` re-run armed; (c) flag-unset bit-identity is structural (env short-circuit) and spot-checked on one temporal board row. **Declared untested co-fire:** `FF_NUMPRE_TEMPORAL=1` with `FF_H_ENDGATE=1` or `FF_TRPG=1` — the endgate discount composes additively with the charge (`heuristic.rs:944-956`) and has only ever run with the charge cleared; the probe runs with both unset, and arming any two together requires its own referee first.

**Size: S** (one armed line, two comment updates, one length pin, one probe script).

**Risks/interactions.** The 0.22 negative class (plateau re-leveling without conversion) is the base rate; the workshop re-route is the quality risk; the endgate/TRPG co-fire is declared; the standing anti-pot "no code at the pathways/tpp walls before Phase 3's decode" is exactly why this ships as a spec now and code only after.

---

### 2. AIBR/subgoaling interval numeric h — module sketch (gated on the decode naming Metric-FF-class relaxation blindness)

**Goal + evidence.** A second numeric relaxation beside the FF-extension h, for walls where the extraction's linear/one-level machinery has no gradient. What exists today (`heuristic.rs`): monotone interval bounds are already propagated for **reachability** — `Scratch.lb/ub`, `widen` (:209), `eval_iv` (:255), `num_sat` (:304), `build_rpg` (:340), soundness-audited by `numeric_interval_audit` (:599) — but the **h value** comes from relaxed-plan extraction whose only numeric distance estimates are `numeric_achiever_linear` (:1366, `linearize`-only, `FF_NO_NUMH`) and the a1/a2 precondition charge (:782+). Non-linear comparisons, multi-fluent interactions beyond linearization, and repetition counts over op *sets* fall through to plateau. Constituency numbers: 2023-numeric satisficing is **251/400** with the walls concentrated in expedition 5/20, settlersnumeric 3/20, pathwaysmetric-numeric 5/20 (markettrader 1/20 stands refused); the metric-time family joins ONLY if its decode names relaxation blindness there. The field read found nobody publishing modern numbers on the 2006 numeric corpus — unclaimed territory, but the 0.24/0.25 SAT-wing lesson (field-priced +16–50, delivered +1/+0 twice) is why this is decode-gated, not field-priced.

**Gate status.** **CLOSED.** Opens only if the Phase 3 decode names **Metric-FF-class relaxation blindness** on a named wall — concretely: a wall instance whose best_h trace is flat under the existing extraction while an interval-subgoaling estimate provably is not. The decode names the RED fixture.

**Design (module-level, so a later session implements without re-deriving).**
- New module `crates/ferroplan/src/aibr.rs`. Own scratch (`AibrScratch`) with per-fluent `[lb, ub]` vectors over `task.rel_fluents` (the `Scratch.lb/ub` shape, deliberately NOT shared — the generation-stamp discipline in `Scratch` must not entangle two relaxations).
- **Propagation:** from the state's `fv/fdef`, iterate: for each relaxed-applicable op, apply numeric effects with additive-interval semantics (increase by a positive-interval amount widens `ub` with unbounded-repetition convex union; assign takes hull; the asymmetric scale-up/down cases follow the case analysis `numeric_interval_audit` already documents at :559-599) to fixpoint or a layer cap.
- **Subgoaling:** each unsatisfied comparison (task `goal_num`, plus selected ops' `pre_num`) decomposes into subgoals; per subgoal, estimate repetitions = ceil(gap / best-step) over the *set* of achiever ops with monotone contribution — generalizing `numeric_achiever_linear`'s single-op estimate via `eval_iv` on effect expressions, so non-linear effects get interval-valued steps. h = Σ subgoal repetitions + the propositional relaxed count (computed beside, not inside, `relaxed_to`).
- **Integration:** the h switch at `search.rs:868-871` (today `match cfg.h_cost { Some → relaxed_costed, None → relaxed_to }`) becomes a three-way selector (an `HKind` on `SearchCfg`, `None` variant bit-identical). First and only v1 consumer: the complete wBFS fallback (`search_from`, the bare single-queue at `search.rs:729`) on classical-numeric tasks, armed in `plan_avoiding` beside the `FF_CLM` block under the same `cfg.h_cost.is_none() && !cfg.anytime` scoping. EHC/LAMA/novelty keep h^FF (AIBR has no helpful-action set in v1). The temporal/metric-time session path is explicitly a *second* consumer with its own referee, only if its decode names it.
- **Mutual exclusions by construction:** AIBR replaces `relaxed_to` on its rung, so the a1/a2 charge and AIBR never co-fire on one evaluation; AIBR and `h_cost` are exclusive (one h per search); AIBR armed leaves the `search.rs:1404` guard condition untouched (it keys on `h_cost`), and the spec that changes that guard is Part 3's verifier constraint, not this one.

**RED fixture.** Named by the decode. Standing candidate to sample in the sitting: `benchmarks/.ipc-corpus/ipc-2023n/domains/expedition-numeric-satisficing/instances/instance-4.pddl` (today unsolved, 56.73 s of 60 — the first of a 15-instance wall).

**Hatches.** `FF_AIBR=1` opt-in arm; `FF_NO_AIBR` reserved now as the named restore if ever promoted default-on. Neither collides with the existing registry. Flag unset = the `HKind::None` path, bit-identical.

**Referee + test plan.** Armed at a 2023-numeric probe sweep (constituency rows solo, 60 s), refereed against the banked `ipc2023-numeric.jsonl` per-domain baselines above; unit pins in a new `tests/aibr.rs` following `tests/numh.rs`'s micro-fixture style (exact repetition arithmetic on a linear and a non-linear comparison); flag-off byte-identity pinned on one costed and one numeric fixture. No old-binary referee needed while opt-in probe-armed; promotion to any default would be a search-order change and buys one then.

**Size: L — priced honestly.** The landscape memo priced this class SIGNIFICANT; new relaxation semantics plus the audit-class soundness questions plus a second scratch is a multi-session build, and the gate exists precisely because field receipts (Scala-class results) have twice failed to price this engine's walls.

**Risks/interactions.** Soundness must be argued against the same case analysis as `numeric_interval_audit` or the estimate silently under/over-shoots; double-charging with a1/a2 excluded by construction (state it in the module doc); the h-switch triangle with Part 3 (below); the standing "no naive variant" anti-pot — this rung dies the 0.17 way if its referee reads +7/−51.

---

### 3. Transport L1 — cost-augmented FIRST plan (`FF_COSTH_FIRST`), gated on the widened probe's number

**Goal + evidence.** The 0.25 transport decode, mechanism 3 (`benchmarks/metrics/attribution-0.25.md`): "the first-plan search is cost-blind — `relaxed_costed` exists but only the post-hoc anytime sweep uses it; all 200–794 roads tie during search (2008 receipts: first plan ~2× the swept cost)." The machinery: `relaxed_costed` at `heuristic.rs:1209`, plumbed via `SearchCfg.h_cost` (`search.rs:198`), consumed at `search.rs:868-871`, whose **only setter today is the post-hoc cost sweep** (`costs.rs:152-155` via `with_cost_h`, `search.rs:322`). Boards today: transport-sequential-satisficing (2011) **2/20** — solving exactly i4/i5 at 59.59/59.88 s, the ~12–14-package line; 2008 `-strips` **20/30**; mco t2/t4/t8 **4/5/7 of 20**; near-wall timeouts throughout (i1 at 59.62–59.77 s). L3 receipts prove the family converts when the fallback gets wall: i4 16.18 s, i6 58.98 s under `FF_NO_NOVLIGHT=1 FF_NO_LAMA=1` — **provenance correction for the verifier: those receipts are IPC-2011 rows** (`benchmarks/post-entries25.sh:119-124` runs `ipc-2011/domains/transport-sequential-satisficing` i4/i6; the memo's §1c "2008 i4/i6" label is wrong — the script and `benchmarks/air25-entries/transport-L3-i*.json` are authoritative).

**Gate status.** **CLOSED until the widened transport probe reports a number** (roadmap-0.26 Phase 4: "two instances is not a lever — widen the probe before pricing it"). The widening this spec defines IS the gate-opener: all 20 of 2011-sat + the 2008 `-strips` unsolved ten + mco-t4's unsolved rows, solo at 60 s, cells {baseline, L1, L3, L1+L3}, receipts under `benchmarks/air26-probes/`, `post-entries25.sh:117-126` as the template. The probe's number prices the build; the fence stands in writing — **2014 sequential boards are NOT claimable** (0/20 everywhere, 25 packages vs the ~12–14 line).

**Design.** Integration point: `api.rs:1082` — the classical first-plan call `search::plan(&task, threads, opts.search_cfg(), ehc_first, orbit)`. The cost fluent is derivable right there, exactly as `api.rs:1097` already does post-plan (`costs::metric_fluent(problem).and_then(|d| task.fluent_id(&d))`, shapes per `costs.rs:45`). Change: under `FF_COSTH_FIRST=1` and a supported metric, the first-plan cfg becomes `opts.search_cfg().with_cost_h(cf)`; hoist the cf lookup above the call and reuse it for the sweep (no behavior change unset — flag unset must leave the constructed cfg byte-identical). **No domain recognition, ever** — arming is flag + metric-shape (the memo's own SGPlan domain-recognition note is the reason). v1 effect surface: `h_cost` changes only the fallback's evaluation (`search.rs:868-871`); EHC/LAMA/novelty call `relaxed_to`/`relaxed_helpful` directly and are untouched — which is the right rung anyway (`ehc_fell_back` on every non-trivial transport solve, attribution receipt).

**The FF_CLM mutual-exclusion, resolved and stated for the verifier to cross-check against the Phase 1 fallback-enrichment spec.** `search.rs:1404` guards `FF_CLM`/`FF_RESLM` wiring with `cfg.h_cost.is_none() && !cfg.anytime` — so an h_cost-armed first plan **silently disarms the landmark/resource ordering terms** on the same search. Constraint, binding on both specs: **(a)** the two levers take disjoint cells by construction — the widened probe runs L1 with `FF_CLM` unset, and no sweep arms both on one run; **(b)** any future co-arming must first lift the 1404 guard deliberately, answer the key-rescale question (h_cost's units are cost+steps vs `w_lm`'s `WEIGHT_SCALE`-pre-scaled counts), and carry its own referee — and that lift belongs to neither this spec nor Phase 1's. If the fallback-enrichment spec chooses to arm `FF_CLM` per-board by default, its board set and `FF_COSTH_FIRST`'s board set must be disjoint at the sweep registry level.

**RED fixture.** `benchmarks/.ipc-corpus/ipc-2011/domains/transport-sequential-satisficing/instances/instance-1.pddl` — today unsolved at 59.77 s of 60 (`ipc67-default.jsonl`), inside the package line (the failing-but-in-range class the decode named), on the exact variant the L3 receipts prove convertible.

**Hatches.** `FF_COSTH_FIRST` opt-in (no registry collision); `FF_NO_COSTH_FIRST` reserved as the restore name if the probe's number buys default-on for the 2008/2011/mco boards. Unset = bit-identical cfg.

**Referee + test plan.** The widened probe (above) is both gate and pricer. If priced: armed at the 0.26 cut sweep on 2008/2011/mco transport constituencies only, refereed against the banked raws (2/20, 20/30, 4-5-7/20). Promotion to any default board config is a search-order change: **old-binary referee** per the 0.21 rule (`docs/roadmap-0.21.md:308`, via `FERROPLAN_FF` pointing the unchanged harness at the previous tag). Tests: a costed micro-fixture in `tests/action_costs.rs` pinning that the armed first plan picks the cheap achiever; a pin that armed-plus-`FF_CLM` leaves `w_lm` at 0 (documenting the guard rather than discovering it later).

**Size: S–M.**

**Risks/interactions.** Quality-for-coverage inversion where costs are uniform is bounded by `relaxed_costed`'s cost+1-per-action shape (gradient survives free regions, per its own doc); the 1404 triangle with Phase 1 (constraint above); refill-loop rounds inherit `h_cost` unchanged (correct); the 2014 fence.

---

### 4. The 2014 config schedule + hiking diagnosis — with the config difference now READ, and it is not a config

**STATUS 2026-08-29: DIAGNOSIS EXECUTED, schedule REFUSED** —
`benchmarks/metrics/fieldgaps-F5-hiking.md`. hiking-agile i5/i6 are tails
(solve at 300 s), i7 an evaluation-cost wall (101 s of h in 143 s); tetris i14
spends 27–47 s of its 60 s wall before search (grounding), so the flip is a
grounding-time row. hiking-sat i16 (a 37 s board solve) did not solve solo on
the 0.26.0 candidate — flagged for the cut sweep.

**Goal + evidence, updated by this sitting's read (no planner run).** Memo rung 2: union 155/280, oracle +6 over sat's 149; hiking sat 20/20 vs agile 12/20, agile losses i5/6/7/13/14/18/19/20 at 59.79–60 s where the sat rows of the same numbers read 7.34–26.9 s. **The asked-for driver diff comes up empty by construction:** `benchmarks/ipc67.py` distinguishes the tracks ONLY by variant-name regex (`:141` `seq-sat-2014: r"sequential-satisficing"`, `:142` `seq-agile-2014: r"sequential-agile"`); both boards sweep at 60 s (every cut script since cut19: `run_board ipc2014-sat seq-sat-2014 60` / `ipc2014-agile … 60`; both raws stamp `budget: 60`), same env, no `--mode`. **The difference is the corpus, not the config:** in `benchmarks/.ipc-corpus/ipc-2014/domains/`, hiking's domain.pddl is byte-identical across the pair but **all 20 instance files differ** (md5), and the agile set is bigger — i3: 5 cars/4 tents/4 couples/3+3 people (agile) vs 3/3/3/2+2 (sat); i5: 5 cars/4 couples vs 3 cars/3 couples. openstacks likewise 0/20 identical (agile i6 is a re-generated, ~15 % smaller file). **tetris and parking are 20/20 byte-identical across the pair** — and their solved-set deltas are opposite near-wall flips on identical inputs: parking i14 sat-solved 59.9 s / agile-unsolved 60 s; tetris i14 agile-solved 52.94 s / sat-unsolved 60 s.

**Consequences (memo corrections for the verifier).** (i) The "agile ordering dies on hiking" hypothesis dissolves: the agile losses are 5-car/4-couple instances the engine has never solved on any board — a scaling wall (the car/couple assignment plateau, the transport-capacity family shape), not an ordering kill. (ii) The +6 oracle decomposes as openstacks i6–i10 (+5, **different instance files** — cross-set comparison, unreachable by any config schedule) plus tetris i14 (+1, identical file — a wall-edge variance/contention question, the class `contention.py` exists for). (iii) The per-instance-number union arithmetic behind 155/280 mixes non-comparable rows on 2 of the 4 divergent domains; rung 2's honest schedule oracle on truly-shared instances is **≤ +2** (tetris i14, parking i14), not +6.

**Gate status.** The diagnosis run is **OPEN now** (no code, queues behind the cut25 sweep like every probe). The schedule build is **gated on the diagnosis report's number** — and with a ≤ +2 corrected oracle against a split tax paid on all 260 other rows, a recorded negative is the likely and acceptable outcome.

**The diagnosis run (committed artifact, receipts under `benchmarks/air26-probes/`).** Solo, post-sweep: (a) hiking-agile i5/i6/i7 at `FF_TIME_LIMIT=60` with `FF_WALL_DEBUG=1` — which rung eats the wall (the ladder narrates per `search.rs` rung gates); (b) the same three at 300 s — cliff or tail; (c) grounding/eval stats (`--json` statistics) for hiking-agile i5 vs hiking-sat i16 (the 37 s slowest sat solve) — does grounded_actions scale with cars×couples; (d) tetris i14 determinism check: 3 solo reps of the identical file against the original sweeps' conditions files for a contention verdict on the two boards' i14 windows. Exit clause: one fixed probe sitting; the report is committed whatever it concludes.

**The schedule design (only if the diagnosis leaves it alive).** A sequential config schedule within 60 s is exactly the shape the refill loop already implements in-engine (`search.rs`, re-enter greedier ×4 under remaining wall), so the design question the diagnosis must answer first is what a driver-side schedule adds that the refill loop does not. If built: driver-side two-phase cell in the sweep registry (config A then B under one 60 s wall), no engine change (no engine hatch needed; the cell is the switch) — and it is a search-order/budget-reallocation claim, so **old-binary refereed** per `docs/roadmap-0.21.md:308`, priced only after that referee (the memo's own warning stands: a naive time split will not keep even the oracle, and the oracle is now ≤ +2).

**RED fixture.** Diagnosis: `benchmarks/.ipc-corpus/ipc-2014/domains/hiking-sequential-agile/instances/instance-5.pddl` — unsolved 59.79 s/60 on `ipc2014-agile.jsonl`. Schedule (if built): tetris-sequential-satisficing i14 — `benchmarks/.ipc-corpus/ipc-2014/domains/tetris-sequential-satisficing/instances/instance-14.pddl`, the identical-file flip (unsolved on sat at 60 s, solved on agile at 52.94 s).

**Hatches.** Diagnosis: none (no code). Schedule: the sweep-registry cell; any engine-side scheduling that emerges instead ships its own named `FF_NO_*` then.

**Referee + test plan.** Diagnosis receipts committed as the artifact; schedule refereed old-binary on the full 2014 sat board (not the four divergent domains alone — the split tax lands everywhere), against the banked 149/280.

**Size: diagnosis S; schedule M, priced after the referee.**

**Risks/interactions.** All probes queue behind the cut25 sweep (no concurrent CPU — house rule); the tetris flip may be a contention ghost, which is why the conditions files are read before any mechanism claim; the memo's rung-2 arithmetic correction should propagate to `docs/field-gaps-0.26.md`'s next revision alongside the §1c transport-receipt year correction (Part 3).

---

**Cross-cutting note for the verifier.** Three specs touch the same h-selection site (`search.rs:868-871`) and the same guard (`search.rs:1404`): Part 2's `HKind`, Part 3's `h_cost` arming, and Phase 1's `FF_CLM` enrichment. The invariant all three must hold: exactly one h per search, ordering terms only where their key units are answered, and any guard change is its own refereed change — never a side effect of landing one of these.


# ═══ F4 — quality + memory ═══

All evidence gathered from the committed raws and source (no planner, no cargo — read-only throughout). The spec section follows.

---

## F4 — the quality + memory wing (memo §2 rungs 3–4, §1c elevator; three parts)

Provenance: every number below re-derived read-only from the committed raws (`benchmarks/*.jsonl`, `benchmarks/air25-entries/*.jsonl`) and the source tree during the cut25 sweep — nothing was run.

---

### F4.1 — quantum-layout plan-length polish: spend the wall the first plan leaves behind

**STATUS 2026-08-29: BUILT, MEASURED NEGATIVE, REMOVED.** Solo on the candidate:
i13 212 → 212, i19 87 → 87, i20 129 → 129 with every rung running its full
deadline; the rows are novelty-rung solves, and a default-on polish would drag
every metric-free solved row's `time` to the wall on the boards. Record in
docs/roadmap-0.26.md (F4 bullet); the 0.9 opt-in `FF_LEN_SWEEP_EVALS` is
untouched.

**Goal + evidence anchor.** The entries sat board (`benchmarks/air25-entries/ipc2023-sat.jsonl`, 60 s, 36/140 solved) carries quantum-layout 20/20 — over half the board's solves in one domain — at **mean quality 0.714** against the vendored bounds (`benchmarks/.ipc-corpus/ipc-2023/bounds.json`, `sat/quantum-layout/pNN.pddl` keys, upper bound = best known; the memo's 0.72-vs-board-0.79). Worst 3 by ratio, re-derived:

| inst | our len | bound hi | q | time (60 s board) | time (300 s board) |
|---|---|---|---|---|---|
| i13 | 212 | 110 | **0.519** | 21.65 s | 81.79 s |
| i20 | 129 | 73 | **0.566** | 4.07 s | 4.03 s |
| i19 | 87 | 50 | **0.575** | 2.02 s | 1.98 s |

(p13's bound is `[0, 110]` — no proven lower bound; the agile twin `agl/quantum-layout/p13.pddl` reads `[0, 114]`.) Every solve returns with wall unspent — i13 leaves ~38 s of 60 and ~218 s of 300 on the table; 14 of the 20 rows carry "EHC found no improving state; used weighted best-first" (the wBFS fallback found the plan). The domain is metric-free (no `:metric`, no `total-cost` — `metric: null` on every row), so plan LENGTH is the quality currency and `length == cost`.

**Gate status: OPEN NOW.** No decode gate. Fenced by the standing anti-pot: quality only, existing boards only (the 300 s agile entry board + the 60 s boards), **no new tiers**, and no anytime-restarts-for-coverage (the recorded −9-coverage/+4-quality receipt).

**Design.** The machinery exists, default-off, in three recorded pieces:

- `costs::improve` (`crates/ferroplan/src/costs.rs:119-205`) — the metric B&B sweep, `anytime: true` at costs.rs:154; **inapplicable here** (needs a cost fluent; quantum-layout has none).
- `costs::improve_length` (`costs.rs:238-313`) — iterated-weight restarts (w_h = 3, 2, 1, incumbent `g_bound` pruning, plain `search_from` per rung), opt-in `FF_LEN_SWEEP_EVALS`, **measured negative at 0.9** *at eval-proportionate budgets* ("2M evals — ~28× the p01 solve — buys 226 → 222"); its own doc records the next ideas.
- `len_anytime` (`search.rs:217-229`, `FF_LEN_ANYTIME` armed at search.rs:1398), **measured negative at 0.10** *as an in-search drain at the 60 s wall* (−9 coverage, sokoban −7).

Neither negative tried the shape the raws now expose: **a wall-denominated post-first-plan sweep** — the budget is the wall the solve already left unspent, not an eval multiple. The build: in `improve_length`, replace the eval-slice budget with a per-rung `SearchCfg.deadline` (the 0.24 Phase 5 per-search deadline, checked at the batch boundaries with the teardown reserve, search.rs:265-278) computed from remaining `FF_TIME_LIMIT` wall minus the report reserve; enter only when ≥25% of the wall remains; keep the w_h 3/2/1 ladder and `g_bound` pruning; `max_eval` stays at the cfg default. Default-ON for metric-free satisficing solves at the existing call site `api.rs:1131-1146` (the branch already runs — `optimize = !--satisfice` defaults true and the runner passes no `--satisfice`; today `FF_LEN_SWEEP_EVALS` unset makes it a no-op). `FF_LEN_SWEEP_EVALS` keeps its legacy meaning as an explicit override.

**The search.rs:1404 guard, stated:** `cfg.h_cost.is_none() && !cfg.anytime` scopes `FF_CLM`/`FF_RESLM` arming inside `plan()`'s ladder only. The polish rungs call `search_from` directly from costs.rs — they never pass through that arming block, so `w_lm` stays 0 in polish rungs regardless; and `cfg.anytime` is set only by the metric B&B (costs.rs:154), which cannot arise on a metric-free task. **The guard does not bite this item.** The real interaction is F1: an `FF_CLM`-enriched fallback changes the FIRST plan on exactly these rows (14/20 are fallback plans) — land F4.1 after F1 and re-baseline the quality column.

**RED fixture.** `benchmarks/.ipc-corpus/ipc-2023/domains/quantum-layout-satisficing/domains/domain-13.pddl` + `instances/instance-13.pddl` (per-instance domains in this corpus). Today (raws): plan length 212 at 21.65 s of 60, q = 0.519 vs bound 110, ~38 s dies unspent. GREEN: the armed build returns a strictly shorter plan inside the same 60 s budget. Board-level success: quantum-layout mean q +0.02 or better with coverage byte-held.

**Hatch.** `FF_NO_LEN_POLISH=1` — restores first-found output byte-identically (the polish is post-plan; the first search is untouched either way).

**Referee + test plan.** Armed by the standing 0.26 cut sweep (F6, crucible; shell fallback drivers) re-running `ipc2023-sat` (60 s) and `ipc2023-agile-300s` (300 s) — default-ON, so no opt-in-orphan violation. Quality read: `standings.py` `bounds_quality` (standings.py:982-1006; `MODERN_Q` rows "2023 seq-sat" → `("2023-sat","-satisficing")` and "2023 classical" → `("2023-agl","-agile")`). **Instrument gap found in this read:** the "2023 agile ENTRY (300s)" row prints a fixed note instead of `bounds_quality` (standings.py:1035-1037) — add it to the quality render as its own instrument-only commit, else the 300 s half of the referee is blind. Coverage-neutrality is the hard clause: 36/140 and 52/140 must hold exactly; any lost row is the 0.10 verdict class — record the negative, the flag stays opt-in, the hatch stays. Not a driver/search-order claim (first plan bit-identical under the hatch), so no old-binary column owed; a unit test pins hatch-off = today's plan on a small fixture.

**Size: S–M.** **Risks:** wall-reserve regression at the 300 s budget (the reserve math is shared with the wall checkpoint — reuse, don't fork); F1 re-baselining; the polish burning wall on domains where q is already 1.0 (guard: skip when the first plan already meets its bound? — no, bounds are referee-side only; the ≥25%-wall gate and deadline bound the spend); the 0.9/0.10 adjacency is declared above and distinguished by mechanism (wall-denominated, post-plan, coverage-guarded).

---

### F4.2 — the folding/elevator memory-profile sitting (footprint work only)

**STATUS 2026-08-29: EXECUTED — the memory build is REFUSED; the mechanism is
GROUNDING** (`benchmarks/metrics/fieldgaps-F42-memory.md`). folding i9/i15 spend
all 300 s in binding enumeration (RSS 1.2–3 GB, never reach search); elevator
2008-strips i29 / 2011 i10 overrun the 60 s wall inside `ground::ground_v`
(stack-sampled), a too-coarse grounding checkpoint on the temporal path; the
numeric twin solves in 47 s. The or-aware-hoist rider's gate condition is met
by this ledger; the checkpoint granularity is a small fix with a fixture —
**landed 2026-08-29**: `ground.rs` `WallTick::STRIDE` 8,192 → 256, receipt
elevator i29 @60 s returns at 60 s with the no-verdict note (was 80.7 s under
the guard with nothing checkpointed).

**Goal + evidence anchor.** The two-domain memory problem, from the committed raws:

- **folding**, `benchmarks/ipc2023-agile-300s.jsonl` (300 s, MEMGB=6, jobs=2, threads=1): 2/20 solved (i1 104.17 s len 146, i14 269.67 s len 114); **10 mem-caps at t = 12.02–18.54 s** (i9–i12, i15–i20); **2 `engine-exit--9` at 171.19/194.10 s** (i6/i7); 6 wall timeouts. At 60 s folding is 0/20 on both the agile board and the sat entries board — the 300 s board is where the mem fixes pay.
- **elevator 2008**, `benchmarks/ipc67-temporal.jsonl`: `elevator-temporal-satisficing-strips` 27/30, **3 mem-caps: i28 11.65 s, i29 8.47 s, i30 8.96 s** (the memo's 8.5–11.7 s, ~+3). The numeric-fluents twin solves 30/30 — the inversion (wider states, no caps) is itself a probe datum.
- **elevator 2011**, same file: `elevator-temporal-satisficing` 7/20 with **7 mem-caps at 9.46–13.73 s** (i7, i9–i13, i20) — the up-to-+7 on a different board, not nettable against the 2008 gap.

The +3–10 band is ACROSS these boards (2008 ~+3, 2011 ≤+7, folding-300s ≤+10 optimistic); the sitting's deliverable includes the honest per-board split.

**Gate status: sitting OPEN NOW** (post-sweep — nothing runs until the cut25 chain completes and the box is free). **Any build inside it is gated on the sitting's attribution number** (decode-before-build, declared here).

**Design — what to measure, and the mechanism map already in source.** The boards ran with `FF_TIME_LIMIT={60,300}`, `FF_MEM_BUDGET_GB=6` (ipc67.py:425-431), threads 1. Engine side: retained-state budget = min(8 GiB `NODE_CAP_TARGET_BYTES`, 6 GiB × 60% share) = **3.6 GiB** (search.rs:52-57, 95-127); classical node cap = budget / `per_node_model_bytes` = `words*8 + fv0*8 + fdef0 + 128` (search.rs:146-148); temporal path takes the same budget via the `node_bytes` parameter plumbed through `temporal::solve_from` (temporal.rs:1672 → 1844-1954). Runner side: Darwin cannot enforce RLIMIT_AS, so the **RSS watchdog** kills at 6 GB resident (ipc67.py:84-91, 484-494); "mem-cap" without the "self-inflicted: node byte target raised" suffix means the kill came **without** the refill re-entry's ×2/×4 raise narration (search.rs:1452-1530, stderr line at 1526). Every folding/elevator cap row is the plain kind — **RSS reached 6 GB while the model believed retained < 3.6 GiB**, i.e. ≥2.4 GB is either model undercharge or un-modeled memory (open-list heap, visited `FxHashMap`, temporal Kind/InvMap tables, grounding CSRs, scratch). That attribution is the whole sitting:

1. **Solo re-runs** of folding `instance-9` (capped 16.2 s) and `instance-15` (12.1 s) under `/usr/bin/time -l`, `FF_MEM_BUDGET_GB=6 FF_TIME_LIMIT=300`, stderr captured: peak RSS at death, whether an internal capped return ever fires, any raise narration.
2. **RSS-at-forced-cap attribution** — the recorded 0.19 Phase 4 method (quoted in the search.rs:69-76 doc): `FF_SEARCH_NODE_CAP=10000 / 100k / 1M` runs give (a) the non-search floor (task tables + grounding) at the 10k point and (b) the true bytes-per-node slope, compared against `per_node_model_bytes` for folding's dims.
3. Same two probes on elevator-2008-strips `instance-29` and 2011 `instance-10` through the temporal path; one extra read on why the numeric-fluents twin (strictly wider `fv`) never caps — if state COUNT not state WIDTH drives it, the model's per-node constant is not the culprit there.
4. Deliverable: a per-domain RSS ledger (model-charged vs actual, slope, non-node residue), the per-board split of +3–10, committed whatever it says.

**Named build candidate the shape already suggests** (post-sitting, on its number): the engine converts memory into a capped return + greedier refill only when its MODEL trips — on these boards the watchdog trips first and 50–290 s of wall dies with the process. Make the internal trip fire first: correct the model's undercharge, or add an actual-RSS checkpoint at the existing batch cadence (Darwin has no `/proc` — `task_info` is a new platform surface; the sitting prices it). Hatch reserved now: `FF_NO_MEMTRIP=1` restores model-only behavior. Search-order untouched (a capped return is already a defined outcome), but the refill re-entry it enables IS a driver-visible change — the referee below carries the old-binary column per the 0.21 rule if it lands.

**Fences carried.** The `engine-exit--9` labeling belongs to F6's mem-cap classification fix (the −7/+7 movement) — this sitting reads those rows, does not re-own the label. **org-synth stays refused** (hash-join lower-bound simulation, twice) — footprint work only, no grounding-join code. **The or-aware-hoist rider's gate, quoted from the record:** 0.24 — "folding p01 honestly not cleared (a different mechanism — the or-aware hoist is sized, not taken)"; 0.25/0.26 deferred list — "the or-aware hoist for folding p01 (sized, not taken — and folding's 300 s face is a MEMORY ceiling, 10 mem-caps + 2 engine kills, not a time wall)". No numeric size is recorded anywhere in the tree — "sized, not taken" is the record's own sizing sentence. The rider enters ONLY if the ledger attributes folding RSS to grounding tables; note folding p01 grounds <5 s solo (0.24) and SOLVES today (i1, 104.17 s) — it is not a coverage lever on the current face.

**RED fixture** (for the eventual build; the sitting itself is no-code): `benchmarks/.ipc-corpus/ipc-2023/domains/folding-agile/instances/instance-9.pddl` (domain.pddl beside `instances/`) — today: mem-cap at 16.2 s of 300; and `benchmarks/.ipc-corpus/ipc-2008/domains/elevator-temporal-satisficing-strips/instances/instance-29.pddl` — today: mem-cap at 8.47 s of 60. GREEN: the row converts to solved or to an honest full-wall exit.

**Referee + test plan.** The standing 0.26 cut sweep re-runs `ipc67-temporal` and `ipc2023-agile-300s`; referee = coverage columns plus the mem-cap note counts per variant (standings.py failure classes); old-binary column mandatory if the refill-visible build lands. Exit clause: fixed budget (~one sitting day, ≤10 solo runs); "model honest, space genuine" is a recorded negative and the band dies honestly.

**Size: sitting S, build M.** **Risks:** probe runs must wait for the cut25 chain (the no-concurrent-CPU rule); `task_info` portability; double-counting with F6's reclassification; folding's residue may be search-shaped even after the memory fix (the 60 s boards say the domain is hard regardless — the band claims only the capped rows' upside).

---

### F4.3 — storage-tc i8–i10: why the at-end fold stalls, smallest probe first

**STATUS 2026-08-29: PROBED, OPEN** (`benchmarks/metrics/fieldgaps-F43-storage.md`):
i8 is a per-node-cost wall (6,078 temporal nodes in 42.7 s); the crate-ablation
twin (7 crates on i8's layout) still stalls at ~21 ms/node — the layout, not the
count, so hypothesis (a) is refuted; `FF_NO_TRAJ_END` chokes in 2 s as
predicted. Needs a temporal-evaluation time-split instrument before (b)/(c)
can be separated; +3 unpriced.

**Goal + evidence anchor.** `benchmarks/ipc5-constraints.jsonl`, `storage-time-constraints` (60 s): i1–i7 solved (0.01–11.22 s, makespans 6.0–25.0), **i8/i9/i10 unsolved at the full 60 s wall with `notes: null`** — clean timeouts, zero rejects (the whole board is 28/120 with zero rejection notes; the VAL-SIGBUS rows elsewhere in this domain are booked per the memo §4 — not chased here). The corpus decade structure, read from the instance files: bases 1–10 appear three times at rising constraint tiers — decade 1 (i1–i10) carries ONLY `(forall (?c - crate) (at end (exists (?d - depot) (in ?c ?d))))`; decade 2 adds a `sometime-before` chain + `within`; decade 3 adds `at-most-once` + `always-within 3.5`. Solved: i11–i16 and i21–i22; the tiers fail progressively earlier (i17+, i23+). **The isolated bonus is decade 1's tail: instance-8 = 9 crates / 5 depots / 5 hoists / 3 containers with `(:goal (and))` EMPTY** — the at-end fold IS the entire goal — while i7 (8 crates / 5 depots) solves in 8.23 s. The cliff is one crate wide.

**Gate status: probe OPEN NOW (no engine code); any build gated on the probe's verdict.**

**Design — the machinery, and where the stall could live.** Path: `constraints::gate` → `expand` → `simplify_static` (constraints.rs:697) → `compile`/`compile_timed` (constraints.rs:1008/1020); an `(at end φ)` hard constraint lowers to a **transition-free ACC latch on the forced-terminal `TRAJ-END` op** (module doc constraints.rs:46-91; `END_ACTION` constraints.rs:111; `strip_end` at 924) — compiled goal = `TRAJ-ENDED ∧ TRAJ{i}-ACC`, all positive literals, precisely to avoid the REACH-GOAL DNF product (the doc's own storage receipt: 3^10 = 59,049 ops at 0.7). Temporal route: constrained tasks skip partitioning and solve monolithically (`tresolve.rs:141-155`) with the monitor audit. Heuristic side, verified in source: the RPG fires conditional adds once conditions are relaxed-reached (heuristic.rs:386-405), and extraction charges the **cheapest disjunct's** condition facts through `queue_cond_for` (heuristic.rs:1253-1282) — so each crate's ACC latch does charge `(in ?c ?d)` for its best depot. Guidance exists; the stall is therefore NOT self-evidently h-opacity, and the spec ranks three hypotheses: (a) a plain decision-epoch search cliff at 9 crates × 5 depots — the §1d storage-family wall wearing a constraints costume; (b) `TRAJ-END`-lowering overhead (the goal reachable only through the terminal op interacting badly with the temporal search's pruning); (c) tie-flattening across depots in the latch charge.

**The smallest probe** (scratchpad fixture edits + solo runs, post-sweep; committed report either way):

1. **Crate-ablation twin:** copy `benchmarks/.ipc-corpus/ipc-2006/domains/storage-time-constraints/instances/instance-8.pddl` to the scratchpad, delete `crate8` from `:objects`/`:init` (the `forall` constraint tracks automatically), run at 60 s. Solves fast like i7 ⇒ smooth scaling wall — **route i8–i10 to the Phase-0/§1d storage-time decode and close F4.3 as a recorded negative here** (the memo already suspects one mechanism under storage-time's own i15/i17 wall). Still stalls ⇒ the compile/shape is implicated locally.
2. **Eval-rate read:** i7 vs i8 with `FF_WALL_DEBUG=1` and the `--json` stats — evals, max_g, evals/s: plateau (flat h, high rate) vs slow-eval (few evals at the wall).
3. **`FF_NO_TRAJ_END=1` on i8** (the 0.7 goal-side shape): expected to choke on the re-opened DNF product — but if it SOLVES, the END construction is directly implicated, and the fix target is precise.

A build follows only the probe's number; if it lands heuristic- or lowering-side, it ships with hatch **`FF_NO_ACCFOLD`** (name reserved; every armed change carries one per house law), the RED fixture below, and — if it changes expansion order — the old-binary referee column.

**RED fixture.** `benchmarks/.ipc-corpus/ipc-2006/domains/storage-time-constraints/instances/instance-8.pddl` (+ the domain's single `domain.pddl`). Today (raws): unsolved, 60 s, notes null; i7 beside it solves in 8.23 s. GREEN: i8 solves inside 60 s; i9/i10 (10–11 crates, 6 depots) are the follow-through and the +3 the memo prices.

**Referee + test plan.** The standing 0.26 cut sweep re-runs `ipc5-constraints` (60 s); referee = the storage-tc row block (target 18/30 from 15/30) with the rest of the board byte-held; the probe report cites the exact twin files and stderr. No new constraints machinery — the memo's §1b fence.

**Size: probe S, build S–M.** **Risks:** the likely outcome is hypothesis (a) — then this item's honest product is a routed decode, not a win here (record it; the +6-over-SGPlan domain lead stands either way); scratchpad twins must never enter the corpus dir; decade-2/3 rows stay out of scope (they ride the same base wall plus enforcement, and enforcement is proven working by i11–i16/i21–i22).


# ═══ F4 probe — floor-tile dead-end lever (read half EXECUTED) ═══

## The floor-tile irreversible-consumption dead-end lever — probe spec (with the read-side pricing executed)

**Provenance of the lever, recovered exactly.** Named in `benchmarks/metrics/attribution-0.25.md:50-77` (the Phase 4 floor-tile sitting) and carried by `docs/roadmap-0.25.md:506-509` and `docs/field-gaps-0.26.md:205-208` (§2 rung 5: "run the probe, build only on its number"). The lever: **a sound dead-end test for irreversible consumption — prune states where an unpainted goal tile is not `clear` (or unreachable-clear)**. The attached pricing probe, verbatim from the record: *"needs no search change: instrument the fraction of expanded nodes failing the test on a solved row."* That is a counter inside a planner run — it CANNOT execute during the sweep. The pure-read half of the pricing I executed now; the instrumented half is specced below for post-sweep.

### Goal + evidence anchor (numbers — re-derived from the raws today)

- **−177/220 re-derives exactly**: 43/220 solved across the eleven floor-tile board-rows in the raws — `ipc67-default` 7/20, `ipc7-mco-t2/t4/t8` 7/20 each, `ipc67-temporal` 4/20, `ipc-opt-2008-11` 2/20, `ipc2014-{sat,agile,mco-t4,opt}` 2/20 each, `ipc2014-tempo` 1/20. Sum solved 43; 220−43 = 177.
- **Every failure is full-budget search churn, zero early exits**: all 177 failing rows burn 58.0–59.4 s of the 60 s budget (e.g. `benchmarks/ipc67-default.jsonl` i7–i20 all 58.03–59.39 s; `ipc2014-sat.jsonl` i2–i17,i19,i20 all 58.6–59.4 s). This is exactly the profile dead-end pruning targets.
- **The cliff, sharpened from instance parsing**: 2011 solves i1–i6,i8 (15–24 tiles) and fails from i7 (24 tiles) up; 2014 solves only its two 15-tile instances (i1, i18) and fails every 24+-tile instance. The record's "~28 tiles" is really **a 24-tile cliff with one marginal pair**: 2011 i7 (24 tiles) FAILS while i8 (24 tiles) SOLVES in 7.15 s — the single most flippable row in the family.
- **Irreversibility verified in the domain source** (`benchmarks/ipc/costs/floortile11/domain.pddl`, identical structure in `.ipc-corpus/ipc-2011/domains/floor-tile-sequential-satisficing/domain.pddl`): `paint-up`/`paint-down` delete `(clear ?y)`; the ONLY adder of `clear` is a movement action vacating a tile; movement ONTO a tile requires `(clear ?y)`. So painted ⇒ never enterable ⇒ never vacated ⇒ `clear` never re-added: irreversible, QED. Soundness caveat found in the same read: a robot-occupied tile is not-clear but recoverable — the test must exempt tiles with `robot-at`. The temporal variant shares the delete (`floor-tile-temporal-satisficing/domain.pddl:34-35`) but is fenced out (below).
- **The overlap question the probe must answer** (found in `search.rs`, this session): the fallback wBFS already drops popped nodes whose h is `None` before expansion (`live` filter, "expand non-dead-end popped nodes", search.rs:~946-950) — and relaxed reachability from s DOES treat painted tiles as walls (clear(t) and robot-at(·,t) adders are mutually dependent, both false ⇒ neither relaxed-reachable). So both halves of the named test (wrong-color-painted goal tile; unreachable-clear goal tile) are in principle caught by the existing `relaxed_to` `None` verdict — **but only after paying a full relaxed-graph eval per popped dead node, and dead successors are still inserted, held in the arena/heap, and popped first** (deferred evaluation: priority is the parent's h — search.rs:724-728). The 0.22 sitting measured `b_blocked 0` (attribution-0.25.md:57-59). The lever's real claims are therefore: (a) O(|goals|) bit-check replacing evals on dead pops, (b) insertion-time filtering keeping dead subtrees out of the arena/heap entirely. The ordering/corridor dead ends the README names ("painting tiles behind") are invisible to BOTH tests — if the probe's fraction is small, the plateau is finite-h states and the lever is dead.

### Gate status

Open for the probe now-after-sweep: the probe IS the decode (no prior decode gates a measurement). The BUILD is gated on the probe's number — inviolable, per field-gaps §2 rung 5: "run the probe, build only on its number."

### Design — the probe (post-sweep)

Instrumentation behind opt-in env `FF_DEADEND_PROBE=1`, counters + one stderr summary line, bit-identical search behavior even when armed:

- **Test** (probe scope; schema-recognized at grounding for `painted`/`clear`/`robot-at` — measurement only, generalized only at build time): state s is DEAD iff ∃ goal atom `painted(t,c)` ∉ s with `clear(t)` ∉ s and no `robot-at(·,t)` ∈ s.
- **Counter 1 — pop-time**: in `search_from`'s main loop, immediately after the pop batch and goal check (search.rs:757-776), test each popped state; count `dead_pops / total_pops`. This is the record's asked-for number: fraction of expanded nodes failing the test.
- **Counter 2 — cross-tab**: zip against the `hs: Vec<Option<i32>>` eval results (search.rs:862-875) — count dead-by-test ∧ `h.is_some()` (states the cheap test condemns that `relaxed_to` did NOT). Expected ~0 by the derivation above; a nonzero value is a finding about `relaxed_to`.
- **Counter 3 — generation-time**: in the expansion `par_map` (search.rs:~952-972, after `task.apply(oi, st)`), test each successor; count `dead_generated / total_generated` — prices the insertion-time filtering claim.
- **Output**: one line at return, alongside the existing `dbg` summary (search.rs:908-923): `[deadend-probe] pops N dead P (x%) h-missed M gen G dead D (y%)`.
- **Runner (which sweep arms it / what referee reads it)**: no sweep arms this — it runs via a new named script `benchmarks/probes/floor-tile-deadend-probe.sh` modeled on `post-entries25.sh` (idle-gate, refuse-under-load, receipts to a benchmarks dir), executed after cut25 banks. Rows: solved — 2011 i1–i6,i8 (`.ipc-corpus/ipc-2011/domains/floor-tile-sequential-satisficing/instances/instance-{1..6,8}.pddl`) and 2014 i1; unsolved at cap — 2011 i7, i9, i11 and 2014 i2, 60 s budget, default pipeline (the burn site is the wBFS fallback — every solved row's notes read "EHC found no improving state; used weighted best-first"). Referee artifact: a COMMITTED report `benchmarks/metrics/floor-tile-deadend-0.26.md` (house rule from 0.25: a read is a committed artifact, never a conversation).
- **Exit clause, numeric**: dead-pop fraction <10% on all probe rows ⇒ recorded negative, lever closed, nothing built. 10–30% ⇒ economy-only rider (insertion filter for memory/heap pressure), no coverage claim. >30% ⇒ the build rung opens.

### Design — the build (only if the probe opens it)

Generalized, not floor-tile-hardcoded: at grounding, detect mutually-dependent false-fact adder cycles (every adder of f requires g, every adder of g requires f — `clear(t)`/`robot-at(·,t)` pairs fall out automatically); at successor generation (search.rs:~961), drop successors where a goal atom's support enters such a dead cycle. Armed default-on in the classical pipeline. Constituency beyond floor-tile per the record: "likely sokoban corners."

### RED fixture

Probe fixtures (measurement): the row set above, failing behavior from the raws cited per-row (e.g. 2011 i7: unsolved, 58.58 s/60 in `ipc67-default.jsonl`). Build RED fixture: `/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2011/domains/floor-tile-sequential-satisficing/instances/instance-7.pddl` — fails today at 58.58 s/60 while same-size i8 solves in 7.15 s; secondary: `.../ipc-2014/domains/floor-tile-sequential-satisficing/instances/instance-2.pddl` (24 tiles, fails at 58.77 s/60, `ipc2014-sat.jsonl`).

### Hatches

Probe: `FF_DEADEND_PROBE` (opt-in, print-only; unset ⇒ bit-identical, no counter reads). Build: armed with `FF_NO_DEADEND` restore (bit-identical off-path: the check sits in a new branch guarded at successor generation).

### Referee + test plan

Probe: the committed metrics report; no old-binary referee needed (no behavior change). Build: **old-binary referee mandatory** (pruning changes expansion order — a search-order claim under the roadmap-0.21 rule), A/B on the six floor-tile boards plus no-regression on the standing corpus; cut26 sweeps (`ipc67-default`, `ipc7-mco-*`, `ipc2014-sat/agile/mco-t4`) read the armed default. A unit fixture pinning soundness: a hand-built state with a robot ON an unpainted goal tile must NOT test dead.

### Size

Probe: **S** (~40 lines: env gate, three counters, one print, one script). Build: **M**.

### Board movement — what the lever could claim, and what it cannot

- **Could claim** (only if the probe prices >30%): the 24–28-tile band — 2011 i7/i9/i10 on four boards (default + three mco) and 2014 i2/i7/i14 on sat/agile/mco-t4: an honest ceiling of **+4 to +12 of the 177**, concentrated exactly at the cliff. Plus eval-economy on solved rows and a possible sokoban rider.
- **Cannot claim**: the deep tail (35–56 tiles — the plateau there is finite-h ordering states BOTH tests are blind to; three guidance transfers measured negative; "nothing on record has ever moved the family"); the optimal faces (`ipc2014-opt`, `ipc-opt-2008-11` — the 0.22 sitting's 594k expansions for the 5×3 is proof burn, not dead-end churn); the temporal boards (same delete structure exists in the durative domain, but the 0.25 read assigns floor-tile-t's wall to the temporal layer/LPG-td class, and durations evaluate against the initial state — `temporal.rs:507-535`); and any coverage number without the old-binary referee.

### Risks and interactions

- The probe's likeliest outcome is the honest negative: cross-tab counter 2 ≈ 0 and a low dead-pop fraction would mean `relaxed_to` already condemns everything the test sees and the plateau is elsewhere — that is a RESULT, recorded per house law, and it retires the lever cheaply.
- The build interacts with deferred evaluation (dead successors currently carry finite parent-h priorities); with the i11 casualty verdict (novelty rung caps at 400k pops — pruning shrinks pop counts and could shift that boundary: `FF_NOV_LAZYH` remains the pre-registered fallback, dockets-0.23); and it must NOT be conflated with the §3.1 fallback-enrichment rung touching the same `search_from` loop — disjoint diffs, separate referees.
- Soundness risk is the robot-occupancy exemption (verified necessary above) and, at build time, cycle detection over-firing on domains where the cycle is escapable via a third adder — the grounding-time detection must require the cycle to be adder-EXHAUSTIVE.

Files cited: `/Users/harold/ferroplan/benchmarks/metrics/attribution-0.25.md`, `/Users/harold/ferroplan/docs/field-gaps-0.26.md`, `/Users/harold/ferroplan/docs/roadmap-0.25.md`, `/Users/harold/ferroplan/crates/ferroplan/src/search.rs`, `/Users/harold/ferroplan/benchmarks/ipc67-default.jsonl`, `/Users/harold/ferroplan/benchmarks/ipc2014-sat.jsonl`, `/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2011/domains/floor-tile-sequential-satisficing/{domain.pddl,README.md,instances/instance-7.pddl}`, `/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2014/domains/floor-tile-sequential-satisficing/instances/instance-2.pddl`.


# ═══ §3.7 — model-train plan-then-schedule read (EXECUTED — CLOSED) ═══

## §3.7 read — plan-then-schedule vs model-train's duration expressions: EXIT CLAUSE FIRES (the core it would build already shipped in v0.10)

**Verdict up front.** The exit clause fires — and on stronger grounds than the memo anticipated. The question "can a post-hoc scheduler compute each duration from the known pre-state without threading evolving state through `temporal.rs`'s grounding core" is moot: **the engine already does exactly this, in-search, since commit `4c3e4e7` (2026-07-19, v0.9.0-18 — in every release since v0.10)**. The memo's feasibility caveat ("the needed core does not exist") is stale: `temporal.rs:507-535`'s initial-state assumption governs only the *static* branch. Nothing is priced; no `temporal.rs` edit is proposed. The one deliverable is the record correction below.

### Goal + evidence anchor

- **Duration expressions (step 1 of the read).** `benchmarks/.ipc-corpus/ipc-2008/domains/model-train-temporal-satisficing-numeric-fluents/domain.pddl`: 9 durative actions. Four (`advance-head-to-switch`, `advance-head-to-next-train`, `advance-tail-to-switch-for-leading-train`, `advance-tail-to-switch-for-trailing-train`) have durations reading `(head-segment-position ?t)` / `(tail-segment-position ?pred)` / `(tail-segment-position ?t)` — fluents **assigned by other actions** (every advance `increase`s them at start; every `update-*-segment` action `assign`s them to 0). Evolving, confirmed anti-pot shape. The other five are constants (`= ?duration 1` ×4, `= ?duration 10` ×2). All four advance actions also use `?duration` inside at-start `increase` effects.
- **Source (step 2).** `crates/ferroplan/src/temporal.rs`:
  - `build_kind` (line 1292; classification at 1345-1377): when a grounded duration expression reads any `modified_fluents` fluent (1471-1487 — model-train's positions qualify), the grounded `NExpr` goes to a `dur_exprs` side table and the start is marked `Kind::Start { dur: 0.0, dexp }` — explicitly "resolved per expansion; no init positivity gate". Only *static* durations take `eval_duration` (line 1369 → 523, the init-state evaluator the memo cites).
  - Expansion (≈3252-3267): the duration is resolved "against THIS node's state", skipped if unresolved/negative. This **is** durations-from-the-pre-state, computed during search — no evolving state crosses the grounding core; grounding stores an unevaluated expression.
  - `validate` (3863; dur_check at ≈3944-3985 and 4025-4060): state-dependent bounds are checked "against the simulation state (init would be wrong)". `reconcile_durations` (986; fixpoint at ≈1080-1105) re-derives emitted durations from each step's own simulated pre-state.
  - The compile-path skip at 276-279 does **not** hit model-train: its `?duration` uses are start-side only (`end_uses` false), so all nine actions compile with correct start-side substitution.
  - `tsched.rs::reschedule` (line 34) is irrelevant here: it is the crew-domain actor repacker (bails without ≥2 actor-typed objects); model-train never engages it.
- **Raws (step 3).** `benchmarks/ipc67-temporal.jsonl`: model-train 0/30 — 27 pure timeouts at exactly 60 s (`notes: null`), 3 mem-caps (i14 24.65 s, i25 15.92 s, i26 13.93 s). Zero parse/ground errors; even instance-1 (2 trains, 5 segments, 4 switches) times out in search. **Provenance kills the probe's premise:** the jsonl is dated 2026-08-21, the 0.24.0 cut day (`79ec55b` committed the matching `ipc67-temporal.md` row `0/30`), and `4c3e4e7` is an ancestor of `79ec55b` — today's 0/30 was measured **with per-pre-state durations already in the binary**. A plan-then-schedule build could not claim this mass.

### Gate status

Closed by this read (the read item's own exit clause, roadmap-0.26 F4: "exit clause fires before any `temporal.rs` edit"). No decode reopens it; anything STN-interval-shaped stays fenced (§4: "No model-train STN encoder revival"). This is a *different* firing than the 0.25 encoder probe's (`state-dependent durations (the STN needs fixed interval lengths)` on i1/i2, `benchmarks/air25-entries/`) — that one declined an encoder; this one finds the imagined scheduler core already in-engine and the failure elsewhere.

### Design

None — no build. The residual model-train mass re-routes to where the source points: the wall is **search guidance, not duration semantics**. The numeric gates that pace the domain (`= (head-segment-position ?t) (SEGMENT-LENGTH ?s-old)` guarding every segment crossing) are invisible to the relaxation because `charge_pre_num` is deliberately cleared on temporal groundings (`ground.rs:3114` via `!stratified`; `packed.rs:132-139` records why; `FF_NO_NUMPRE` restore at `heuristic.rs:782`), so h is flat across each advance sequence, and float-valued position fluents in state keys gut duplicate detection. That is the model-train/TMS plateau by name — the 0.22 pre-a2 temporal charge was measured NEGATIVE on exactly it (one of the ten closed-ledger negatives), and the only lawful reopen is the roadmap's existing F3 item (a2-chained `charge_pre_num` on temporal groundings, strictly post-Phase-3-decode, workshop-economy fixture mandatory). This read adds no new build path and none should be added under §3.7's banner.

### RED fixture (for the record, should F3 ever open)

`benchmarks/.ipc-corpus/ipc-2008/domains/model-train-temporal-satisficing-numeric-fluents/instances/instance-1.pddl` — smallest of the family (2 trains, 5 segments, 4 switches; goals propositional: both trains visit `seg-goal` and return). Today: unsolved, 60 s timeout in search (jsonl instance 1), no parse/ground failure. Any future claim on this family must convert it.

### Hatch name(s)

None — no code changes. (Existing relevant hatches, unchanged: `FF_NO_NUMPRE`, `FF_TEMPORAL_NODE_CAP`, `FF_TEVAL_BUDGET`.)

### Referee + test plan

No armed change, so no sweep evidence is owed. The family's standing referee, for whoever reopens under F3: `benchmarks/ipc67.py` → `ipc67-temporal.jsonl` → `standings.py` → `ipc67-temporal.md` (cut25-sweeps.sh line 149 runs `ipc67-temporal tempo-sat 60`). VAL-validated per `bench_temporal.py` conventions.

### Estimated size

**0 (read only).** Memo correction: field-gaps-0.26 §3.7's caveat sentence and §1c's "priced-zero" footnote should carry this finding — the pre-state duration core exists since v0.10 (`4c3e4e7`); model-train's 13-instance share of the 2008 tempo-sat gap is a heuristic-plateau/memory problem inside the closed temporal h-accounting ledger, reachable only through the F3 gate.

### Risks and interactions

- The main risk of NOT recording this: a future cycle re-buys §3.7 believing the duration core is missing, or a plan-then-schedule build claims credit the 0.24.0 raws already refute. This section is the antidote.
- The mem-cap trio (i14/i25/i26, 13.9-24.7 s) is consistent with node-accumulation on a flat h over float-keyed states, but grounding-blowup on the larger boards (i14: 8 trains × 23 segments × 16 switches → ~10⁶ grounded snap pairs) is not excluded by this read; whoever runs the F3 probe should log ground-vs-search split first (`FF_WALL_DEBUG` exists in `temporal.rs`).
- No concurrency hazard was found that would ever force an STN: all nine actions' at-end effects are pure adds (`idle`, `switch-available`, `visited-segment`) — sequential execution is monotone-safe — but this is moot given the verdict and stays as a note, not a lever.


# ═══ F6 — crucible and the cut ═══

All evidence is in hand. Writing the spec now.

---

## Crucible finishes, and the 0.26 cut sweeps on it

**Provenance note for the implementer:** the crucible source lives on the `crucible` branch (tip `9473d5e`, code commit `8b0cbb5`; `main` has only the gitignored `crucible/target/`). Every `crucible/...` path below is a branch path; line numbers are at `9473d5e`. Finishing this item includes merging `crucible` to `main` under the finish-in-main working agreement, after `crucible/preflight.sh` is green.

**Goal + evidence anchor.** Phase 5's own closing words: "`SweepRunner::attempt` still holds its rows in memory and judges cleanliness from a before/after sample pair. `db/` is implemented and tested and `sched::resume` implements the per-sample window intersection — they are simply not wired together. So resumption survives a killed BOARD and not a killed PROCESS, and surviving `kill -9` is the premise." (docs/roadmap-0.26.md:442-449). The cost being bought back: the 0.25 entries sweep took five passes and ~37 hours to bank ten boards, pass 1 burning thirteen hours for one board (roadmap:141-145); the whole-board retry alone cost ~nine board-hours on 2026-08-21 (crucible/crates/crucible-core/src/sched/resume.rs:4-11). The decision this spec executes: field-gaps-0.26.md:6-8 — "the 0.26 cut sweep runs on crucible."

**Gate status.** Open now for parts 1-2 (design work; the roadmap explicitly classifies part 1 as "wiring rather than design"). Parts 3-4 and all `cargo`/build/test execution are **queued behind the cut25 sweep draining** (the no-concurrent-CPU law). No engine decode gates apply: this item touches no `crates/ferroplan` code. The gates that stand in for a decode here are the byte-parity gates already banked (roadmap:302-354): `standings --check` byte-identical on both documents, "314 agree, 0 MISMATCH (42,356 rows classified, 144 boards)", the 26-track selector equality over 292 variant dirs, 43,186-row byte round-trip, and `sweep --set cut25 --dry-run` printing 6,366.

---

### Part 1 — the DB wiring: resumption survives `kill -9` of the process (M)

**STATUS 2026-08-28: LANDED** — `4336c35` on `crucible`, merged to `main` as
`8cae317`, `crucible/preflight.sh` green on the merged tree. Record:
docs/roadmap-0.26.md Phase 5, "Recorded — the gap is closed". The RED fixture
(`kill9_resume.rs`) and the agreement test (`gate_agreement.rs`) exist as
specified; the `--no-db` hatch is `CRUCIBLE_NO_DB=1`'s twin. Part 3 was
attempted the same day and is BLOCKED (no `x86_64-linux-gnu-gcc` for the
bundled SQLite build — see the roadmap record); parts 2 and 4 stand open.

**What is true today, precisely.**

- `crucible/crates/crucible/src/sweep.rs:32-43` — `Board { clean: BTreeSet<String>, rows: BTreeMap<String, RawRow> }`, both in memory only. `attempt` (lines 322-399) judges cleanliness from a before-sample (line 346, `self.sample()`) plus an after-sample (line 376) plus `m.clock_jump.is_zero()` (line 377) — a two-point probe, not a window intersection. `write_artifacts` (lines 229-274) rewrites `stage/{id}.jsonl`/`.md`/`.done` after every attempt, so the *rows* survive a kill — but `SweepRunner::new` (lines 117-193) starts every board with empty `clean`/`rows` and never reads the stage back, so a restarted process re-measures everything.
- `grep -rn 'db::|resume::' crucible/crates/crucible/src/` returns **nothing**: neither `crucible_core::db` nor `sched::resume` is referenced by the driver crate. Both are built and tested: `db/writer.rs:89` (`WriterHandle::run` — a run row is never batched, own transaction, caller waits for the commit), `db/read.rs:245-292` (`Reader::window_gate` — the per-sample intersection as a SQL range query, `Cleanliness::{Clean,Dirty,Uncovered}`, fail-closed on `Uncovered` and on NULL `competitors_total`), `db/read.rs:106` (`export_rows`, canonical order, latest-done-attempt rule), `db/rebuild.rs:79` (`rebuild_from_artifacts`), `db/lock.rs:52-121` (`DirLock`), schema tables `run`/`sample`/`sample_process`/`board_pass`/`live_child`/`throttle_window`/`event` (db/schema.rs:172,364,376,325,303,388,400).
- Three adjacent unwired facts the wiring must close or it does not work:
  1. **No engine stamp on measured rows.** `crucible-core/src/sweep.rs:130-155` builds the `RawRow` with `extra: Default::default()`, and the driver constructs `SweepEngine { path, ver }` (crucible/src/sweep.rs:510-514), discarding `engine.blake3`. The resume gate refuses any unstamped row by design (`sched/resume.rs:103` `ENGINE_KEY = "engine"`; `judge` at :432 returns `EngineUnstamped`). Crucible's own output is currently un-resumable under crucible's own gate.
  2. **No persistent timeline.** `sample()` results are consumed and dropped; `sample`/`sample_process` never receive a row, so `window_gate` has nothing to intersect.
  3. **`live_child` is never written**, so the startup orphan reap (`exec/orphan.rs:223 reap`, identity-verified via `db/model.rs:399 LiveChild::identity`) has no input — and a `kill -9` parent leaves `SIGSTOP`'d children stopped forever (the spec's own edge case, crucible-spec.md §14).
  - Small rider: `SetSpec.requires_version` (crucible-publish/src/manifest.rs:429) is parsed and never read; only the CLI `--require-version` gates.

**Design.**

1. **Open the database at sweep start.** `sweep::run` (crucible/src/sweep.rs:479) calls `Db::open(&cfg.db_dir)` (db/mod.rs:160). New config field `Config.db { dir: PathBuf }`, default `~/.crucible/db/` beside the existing `worktree_dir` default `~/.crucible/worktrees` (crucible/src/config.rs:120). `DirLock` gives "one crucible per queue" for free; `LockError::Busy` is the operational answer, not a fault.
2. **Carry the engine identity through.** Add `blake3: String` to `crucible_core::sweep::Engine` (crucible-core/src/sweep.rs:41); `measure()` stamps `row.extra.insert(resume::ENGINE_KEY, blake3)` before returning. Driver passes `engine.blake3` at crucible/src/sweep.rs:510-514. This single change makes both the DB rows and the exported JSONL self-identifying under the gate that already exists.
3. **Persist the timeline.** A watcher thread (spawned in `sweep::run`, joined on exit) calls the existing `SweepRunner::sample()` every `Contention.sample_interval_secs` (default 20, crucible/src/config.rs:103 — equals `resume::DEFAULT_INTERVAL_SECS`) and sends `writer.sample(SampleRec::of(&s))` (db/model.rs:347; `pass_id: None` = box-wide, exactly what `window_gate`'s `pass: None` reads). Batched by the writer (BATCH_MAX 64 / 200 ms, db/writer.rs:37-41); telemetry is allowed to be lost, run rows are not — the asymmetry is already built.
4. **Per instance, inside `attempt`:** once per board, `writer.resolve(BoardKey/BoardFacts from BoardSpec + board_cfg, EngineKey { blake3, ver }, EngineFacts)` (db/writer.rs:148) → `(board_id, engine_id)`. Around `measure`: `writer.child_spawned(...)` / `writer.child_gone(pid)` (requires plumbing pid/pgid out of `exec::run`'s `RunOutcome` — the fields exist in `Measured` (db/model.rs:287) and the `run` table; `RunOutcome` at exec/mod.rs:69 must expose them). After `measure`: build `RunRecord { attempt, state: Done, timing (from step 5), val_reason, row, measured }` → `writer.run(rec)` — the immediate-commit call that IS the kill -9 receipt.
5. **Replace the before/after pair with the window intersection.** After `writer.run` + `writer.flush()`: `reader.window_gate(row.start_ts, row.end_ts, interval, None)` → `Clean` banks the instance (`timing_quality='clean'`), `Dirty`/`Uncovered` keeps the row and re-runs later (`'dirty'`/`'unknown'`). The admission-time sample and `throttle.on_sample` (crucible/src/sweep.rs:346-357) survive *for scheduling only* — admission and quiet-hours behavior unchanged; the verdict moves to the persisted per-sample rule. This is precisely "the per-sample window intersection, wired": same threshold constant (`SAMPLE_CLEAN_PCPU`, monitor/sample.rs:35), same fail-closed directions the Python fails closed in (db_roundtrip.rs header, bullet 5).
6. **Startup resumption.** In `SweepRunner::new`, per board: `resolve` → `reader.export_rows(bid, eid)` seeds `Board.rows`; a new small reader query `clean_instances(board_id, engine_id)` (`SELECT v.ipc, v.name, i.label FROM run ... WHERE state='done' AND timing_quality='clean'`, latest-attempt rule copied from `export_rows`) seeds `Board.clean`. Then `write_artifacts` regenerates the stage — **the JSONL becomes a pure export**, the roadmap's exact phrase. Rows in the DB from a different engine hash simply don't resolve to this `(board_id, engine_id)` and are invisible: the blake3 gate at DB granularity. Rows measured by non-crucible tools (ipc67.py raws) are NOT silently adopted: importing goes through `db::rebuild_from_artifacts`, which marks timing `unknown` (never `clean` — db/rebuild.rs:11-27), so they re-run. Fail closed; a needless re-run costs sixty seconds.
7. **Startup orphan reap.** Before the first pass: `reader.live_children()` → `exec::orphan::reap(...)` (SIGCONT-before-SIGKILL, stranger-detection already tested in tests/orphan_reaping.rs) → `writer.child_gone` per reaped row.
8. **Board pass provenance.** On each board attempt completion, `writer.board_pass(BoardPassRec { verdict, ran, reused, source_path: None, ... })` (db/writer.rs:140; `''` = live pass identity). The `.done` file stays as the artifact-level marker; the DB row carries what a zero-byte file cannot.
9. **`requires_version` rider:** `sweep::run` defaults `require_version` from `SetSpec.requires_version` when the CLI flag is absent.

**RED fixture.** New integration test `crucible/crates/crucible/tests/kill9_resume.rs`, in the `fakeff` pattern of crucible-core/tests/sweep_rows.rs: run a two-board sweep against `CARGO_BIN_EXE_fakeff` with a spawn-counting side file; construct a second, fresh `SweepRunner` over the same stage+db (the restart); assert the second run re-spawns **zero** instances that banked clean. Real-corpus anchors, cited: the timeline evidence is `crucible/tests/fixtures/conditions/timeline-numeric-opt.json` (real file, whole-run verdict `clean` with 53 over-threshold samples — the per-sample care point) and the row corpus is `benchmarks/ipc2014-opt.jsonl` (256 real committed rows, already the db_roundtrip fixture). **Today's failing behavior:** the fresh `SweepRunner::new` (crucible/src/sweep.rs:167-172, `clean: Default::default(), rows: Default::default()`) owes every instance again — the test fails with a full re-measure count. A second RED assertion: a `judge()` call over crucible's own exported row currently returns `Reject::EngineUnstamped` (no `"engine"` key in any row `measure()` writes) — passes after step 2.

**Hatches.** `crucible sweep --no-db` (env twin `CRUCIBLE_NO_DB=1`): restores today's in-memory path bit-identically — no `Db::open`, no watcher thread, no engine stamp in `row.extra`, before/after-pair cleanliness — so the off-path artifacts are byte-identical to the current binary's. The outer restore hatch for the whole instrument is named below in Part 4 (the Python driver lineage stays runnable).

**Referee + test plan.** `crucible/preflight.sh` is the gate (fmt, clippy -D warnings, `cargo test --all`, fixture `extract.py --check`, `verify-manifest.py`, the oracle differential, the 43,186-row round-trip) — run **after the cut25 sweep drains**, never concurrently. New tests: `kill9_resume.rs` (above); a `window_gate`-vs-`sched::resume::judge` agreement test over the four `timeline-*.json` fixtures (two implementations of one rule is the shape of half the incidents — hold them against each other the way `the_two_conditions_readers_agree` already does for the readers); a live-then-export byte test extending db_roundtrip's two-lap rule to rows produced by `measure()` with the engine stamp. No engine claims are made, so the roadmap-0.21 old-binary referee is N/A — stated, not skipped.

**Size: M** (~5 files touched in the driver, 2 in crucible-core, 2 new test files; no schema change — every column named above already exists).

---

### Part 2 — `crucible backfill`: drive what repo.rs already proves (M)

**What exists, undriven** (crucible/crates/crucible/src/repo.rs): `Engine::probe` (:58 — blake3 + `--mode` capability probe from the binary's own `--help`, so it works against a tag whose tree is not checked out), `require_version` (:84), `supports_mode` (:99 — feature-absent, never a board of zeroes; empty probe assumes capable), `worktree_for` (:156, `#[allow(dead_code)]` — distinct prefix under `cfg.repo.worktree_dir` (default `~/.crucible/worktrees`) so GC can never eat the operator's hand-made `~/ferroplan-backfill-*` checkouts), all with tests (:162-240). The rule the driver must keep, already in the module doc (:143-154) and in `benchmarks/backfill-air.sh`: **the instrument is ALWAYS the working tree's; only the ENGINE comes from the tag** — checking out the old `benchmarks/` "would vary the INSTRUMENT as well as the engine, and then the delta means nothing."

**Design.** New subcommand in the `Cmd` enum (crucible/src/main.rs:38): `Backfill { tag: String, set: String, stage: Option<PathBuf>, dry_run: bool, max_passes: Option<u32> }`. Steps, each mirroring backfill-air.sh's proven shape:

1. `git rev-parse -q --verify refs/tags/<tag>` in `cfg.repo.local`; refuse a missing tag by name.
2. `git worktree add --detach {worktree_for(&cfg.repo.worktree_dir, tag)} <tag>` if absent; build `cargo build --release -p ferroplan-cli` inside it (worktrees get their own `target/`, so the candidate binary at `repo/target/release/ff` is untouched).
3. `Engine::probe(worktree/target/release/ff)`; set `Engine.tag = Some(tag)`; **skip** `require_version` (a backfill is exactly the case where the version is old); record the engine in the DB with `EngineFacts { tag, .. }` so history queries can find it.
4. Reuse `SweepRunner` unchanged with `Setup { capable: &|m| engine.supports_mode(m), .. }` — the capability gate is already honored at crucible/src/sweep.rs:146-154 (SKIP with zero rows). Additionally write a `board_pass` row with `PassVerdict::FeatureAbsent` (db/model.rs:409; schema CHECK already includes `'feature-absent'`) so the skip has provenance, not just an absence.
5. Stage defaults to `benchmarks/air-<ver>/` (the existing `air-0.18.0/`, `air-0.19.0/`, `air-0.21.0/` convention); manifest and corpus are the working tree's, never the tag's.
6. Worktree GC: keep `cfg.repo.keep_tags` (default 5), delete only paths under the `worktree_dir` prefix — the property `worktrees_live_under_their_own_prefix` (repo.rs:217) already pins.

**RED fixture.** `crucible backfill --tag v0.18.0 --set cut25 --dry-run` — **today clap exits with "unrecognized subcommand"**; after, it prints the board plan with the capability skips named. Real-corpus anchor: tag `v0.18.0` exists (the operator's `~/ferroplan-backfill-0.18.0` worktree and the committed `benchmarks/air-0.18.0/` raws prove it was backfilled by hand), and per backfill-air.sh:56-60 a pre-0.19 engine has no `Mode::Optimal` — so the dry run must list `ipc2014-opt`, `ipc2026-opt`, `ipc-opt-2008-11` as `SKIP ... feature-absent, not zero coverage`, which no code path can produce today.

**Referee.** Structure, not timings: the backfill's exported raws must carry the same denominators, the same row shape, and classifier/coverage agreement with the committed Python-produced `benchmarks/air-0.19.0/` and `air-0.21.0/` sets (the two full tracked backfill sets the roadmap names as hermetic goldens) via `benchmarks/crucible-differential.py --only air-0.19` extended to the new stage. Re-measured times will differ; coverage deltas are reported through `crucible diff`, with losses named individually. Runs only post-sweep.

**Size: M** (~250-350 lines: subcommand + git/build shelling + feature-absent pass rows + tests; the load-bearing rules are already written and tested).

---

### Part 3 — arming the Linux cross-check (S)

`crucible/preflight.sh` already contains the gate and self-reports its disarmed state: if `rustup target list --installed` lacks `x86_64-unknown-linux-gnu` it prints `SKIPPED: rustup target add x86_64-unknown-linux-gnu to arm this gate`; armed, it runs `cargo check --target x86_64-unknown-linux-gnu -p crucible-core -p crucible-publish` — the proof that no macOS-only `libproc` call escaped `trait Platform` (platform/generic.rs exists on the branch for exactly this). **Post-sweep action, two commands, in order:** `rustup target add x86_64-unknown-linux-gnu`, then re-run `crucible/preflight.sh` and confirm the step reports checked, not skipped. Nothing runs now — `rustup target add` is a download but the check is a cargo invocation, and both wait for the box. No hatch needed (read-only gate); the referee is the preflight transcript in the cycle record. **Size: S.**

---

### Part 4 — the cut-26-on-crucible runbook (S to write; the sweep itself is the cycle's tail)

**Preconditions, all green before the first spawn, in this order:**

1. **Crucible merged to main** (branch `crucible` → `main`, fast-forward per the working agreement), `crucible/preflight.sh` fully green on the merged tree — including the newly-armed Linux step (Part 3) and the Part 1 kill-9 test.
2. **Byte-parity gates re-verified against the 0.26 candidate's repo state:** `crucible --repo . standings --doc all --check` → both `ok ... matches` lines; `python3 benchmarks/crucible-differential.py` → `0 MISMATCH` (the banked baseline: 314 boards agree, 42,356 rows); round-trip loop green.
3. **The manifest grows `[[set]] cut26`** via `crucible/tools/gen-manifest.py` after cut25 promotion updates the registries (the generator transcribes, never invents — `verify-manifest.py` must agree): `name = "cut26"`, `stage = "benchmarks/air26"`, `requires_version = "0.26"`, boards = the 32-board like-for-like table Phase 6 names (the standing 22 of `cut25` + the 9 of `entries25` + `ipc5-complex-pref` of `post-entries25` — one stage now, because from 0.26 on the 32 IS the instrument).
4. **Corpus-enumeration gate:** `crucible sweep --set cut26 --require-version 0.26 --dry-run` must print the instance total equal to the promoted STANDINGS.md denominator — expected `set cut26: 8444 instances` (roadmap Phase 6: 4,743/8,444). Any other number stops the cut; the board registry, selectors, and corpus walk must agree with the standing table without being told the answer.
5. **The certificate gate,** exactly as cut25-chain.sh runs it: `benchmarks/opt-differential.py --board-budget` green on the 0.26 candidate with a fresh `--out benchmarks/cut26/opt-differential-board.jsonl`; `^REGRESSION`-anchored grep (the unanchored-grep incident is on the record).
6. Box law: no concurrent cargo/CPU (MEMORY), local detached process only (MEMORY: ferroplan runs local-only).

**The sweep command line:**

```
nohup ./crucible/target/release/crucible --repo /Users/harold/ferroplan \
  sweep --set cut26 --require-version 0.26 \
  > benchmarks/cut26-sweep.log 2>&1 &
```

What each guarantee comes from, on the record: **budgets** from the manifest boards (60 s everywhere, `ipc2023-agile-300s` at 300 s; mco boards `--threads 2/4/8` with jobs forced to 1 by the wall-clock rule; jobs 2 / 6 GiB defaults; `--mode optimal` on the proof boards) — no shell-side budget knobs exist to get wrong. **Env scrubbing** is structural: `exec/env.rs::build` inherits only the eleven-name allowlist, drops all 132 ambient `FF_*` hatches, injects `FF_TIME_LIMIT`/`FF_MEM_BUDGET_GB`, applies the board's declared `env` last and records it — a row can no longer be measured under a hatch nobody can name. **The blake3 gate** is the resume identity: rebuilding `ff` mid-sweep changes the hash, and every row from the old build refuses to stitch (which is the reason not to rebuild mid-sweep, not a reason to override anything). No `--max-passes`: resident behavior, stall-guard back-off, still running in the morning. `kill -9` at any point loses nothing (Part 1's premise, now tested).

**The Python oracle beside it, as differential — kept permanently, never retired:** after the sweep drains, (a) `crucible standings --doc all --check` against tables regenerated by `python3 benchmarks/standings.py` — byte agreement in both directions; (b) `python3 benchmarks/crucible-differential.py` re-run so the fresh `benchmarks/air26/` raws join the row-by-row classify/coverage/select corpus; (c) `crucible diff benchmarks/air25 benchmarks/air26` with losses named individually. The oracle costs ~2 s per cut; every incident in its comment corpus is one implementation drifting from another unobserved.

**The mem-cap classification fix, sequenced (roadmap Phase 2 + the recorded Phase 5 decision "port the bug, prove the port, then fix it as its own named change"):**

1. **Port the bug** — already done: crucible's `classify()` matches `"mem-cap"` and `"spawn-fail"` by exact equality, mirroring `standings.py:262/264` against `ipc67.py:493`'s labelled variant `"mem-cap (self-inflicted: node byte target raised)"` (both verified on main's disk today).
2. **Prove parity** — the 314-board / 42,356-row zero-mismatch differential and the byte-identical standings ARE the proof; the cut26 sweep and its table regeneration run with the bug in place.
3. **THEN fix, as its own commit** — widen the match to a prefix test in crucible's typed classifier **and** in `standings.py:262` (mem-cap) and `:264` (spawn-fail) in the same commit, so the differential stays 0-mismatch across the fix; regenerate the goldens; record the movement in the cut record: `benchmarks/ipc-standings.md` line 61 (2023 numeric) `6 early-exit, 1 mem-cap` → `0 early-exit, 7 mem-cap`, line 52 (2014 seq-mco t4) `2 early-exit, 1 mem-cap` → `1 early-exit, 2 mem-cap`, plus the two rows in ipc2014-mco-t8 swept and awaiting promotion. Coverage moves zero; attribution moves −7/+7 (−9/+9 once mco-t8 promotes). After the fix the drift is structurally impossible: the runner's note is a typed variant and the classifier matches the variant.

**Instrument restore hatch, named:** the retired shell-driver lineage (`benchmarks/cut25-sweeps.sh` + `ipc67.py` + `contention.py` + `standings.py`) stays committed and runnable; a `cut26-sweeps.sh` clone of the cut25 driver with the 32 boards is the fallback instrument if crucible is refused at the gate — and choosing it is recorded as a measured negative for the harness, never papered over.

**Size: S** (a runbook section in the cut record + one manifest set + the gen-manifest transcription).

---

### Risks and interactions

- **The `"engine"` extra key changes new-row bytes.** Committed raws are untouched; new air26 raws carry one extra key, which `RawRow.extra`/`extra_json` and the round-trip test already model (schema.rs:267 "forensics rather than export"). The `--no-db` hatch omits it, keeping the off-path bit-identical.
- **Two cleanliness judges during transition.** `sched::resume::judge` (artifact side) and `Reader::window_gate` (DB side) implement one rule; the Part 1 agreement test over the four real timeline fixtures is the anti-drift hold, in the exact shape of `the_two_conditions_readers_agree`.
- **`synchronous=NORMAL` loses the WAL tail only on power cut** — accepted by design (db/mod.rs:199-206): the committed `.jsonl` is the durable record and `db::rebuild` exists for exactly this.
- **Pre-0.20 multipart raws refuse to import** (`DbError::DuplicateInstance`, db/mod.rs:111-131 — the 320-rows-under-288-keys collapse). Backfill/import paths must surface the refusal, never work around it.
- **The Ctl channel is still unused in the sweep path** (crucible/src/sweep.rs:330): mid-run SIGSTOP/demote from the throttle is not wired to live children. Deliberately out of scope — the Phase 5 gap list names three things and this is not one of them; the admission gate plus dirty-row policy carries the cut. Record it as the known residue so it is not "discovered".
- **Jobs > 1 is admission-time only** — `attempt` measures serially. The single-writer DB thread is indifferent either way; parallel slots are TUI-era work, not cut-blocking.
- **A second crucible against the live queue** is refused by `DirLock` (tested, db/mod.rs:240); the stale-lock-on-crash case is handled by flock semantics (lock dies with the process).
- **Killing processes:** any orphan reap the operator triggers by hand still falls under the confirm-before-killing house rule; crucible's own reap is identity-verified (`ProcIdentity`) and reports strangers rather than signalling them.
