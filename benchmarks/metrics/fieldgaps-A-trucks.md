# Field-gaps Sitting A — trucks/storage-time decode (F0a, 0.26)

Sat 2026-08-27, post cut25-sweep ("0.25 CUT SWEEP ALL DONE 2026-08-27 07:53:16",
`benchmarks/cut25-sweep.log`). Binary: `target/release/ff` 0.25.0 (the promoted
cut binary, not rebuilt). Box quiet throughout (≥89% idle at start, no builds,
one planner process at a time, probes strictly serial). Receipts:
`benchmarks/metrics/probes-0.26/A-trucks/` (referenced below as `R/`).
Probe ledger: 41 timed runs — 6 baseline shape reads, 1 strips redo, 8
eval-budget slices, 22 condition-matrix runs, 2×300 s solo legs, plus 2 solved
in-run (~34 min box time, inside the ~2 h envelope). Amendment honored:
pass-start/stats narration arrived under `FF_RES_DEBUG`, wall lines under
`FF_WALL_DEBUG`, as the binding Amendments section states.

## VERDICT — mechanism NAMED: decomposer starvation by a self-duplicating escalation ladder, over an h-plateau the monolithic search cannot cross

The −49-gross trucks/storage failure at the 60 s tier is not grounding blowup,
not memory, not symmetry, and not plain scale. It is a **routing/serialization
failure**: the temporal escalation ladder spends the entire 60 s wall running
the monolithic decision-epoch search up to eight times — of which only ~2 are
distinct searches, the rest **byte-identical recomputation** — and the
partition-and-resolve decomposer, the one strategy that actually solves these
boards (in 0.03 s on trucks-time i12), enters only after the wall is spent,
where its contract searches are "pass entry refused" and falsely labeled
UNSOLVABLE.

Three strands, each with receipts:

### S1 — the decomposer solves what the monolithic ladder cannot (primary)

| run | condition | result | receipt |
|---|---|---|---|
| trucks-time i12, 60 s | default | FAIL (wall) | `R/A-trucks-time-i12.log` |
| trucks-time i12, 60 s | `FF_TDECOMP=1` (decompose-first) | **SOLVED 0.03 s**, 66 steps, makespan 11894.57 | `R/A-trucks-time-i12-tdecomp.{json,log,time}` |
| trucks-time i12, 60 s | `FF_TDEMAND=1` (ambient=Full ⇒ ladder runs monolithic once, decomposer inherits the tail) | SOLVED 59.99 s (same 66-step plan) — a hair under the wall | `R/A-trucks-time-i12-tdemand.*` |
| storage-time i15, 60 s | default | FAIL | `R/A-storage-time-i15.log` |
| storage-time i15, 60 s | `FF_TDECOMP=1` | **SOLVED 51.89 s**, 20 steps, makespan 19.02 (shorter than solved i14's 24.02) | `R/A-storage-time-i15-tdecomp.*` |
| storage-time i17, 60 s | `FF_TDECOMP=1` | FAIL (needs > 60 s of contract merges) | `R/A-storage-time-i17-tdecomp.log` |
| trucks-time i12, 300 s | default | **SOLVED 155.5 s** — 8 node-capped monolithic passes burn ~155 s (36.7+13.7+14.1+13.6 s ambient + 35.5+13.8+13.9+13.8 s Full, `cap hit` lines 36–283), then the decomposer solves instantly | `R/A-trucks-time-i12-300s.*` |
| storage-time i15, 300 s | default | **SOLVED 117.8 s**, same shape: ladder burns out, `[TDECOMP] 5 initial contracts` at log line 285, 3 merges, solve | `R/A-storage-time-i15-300s.*` |

So "does 5× buy anything?" — yes, +2, but **only because the wall finally
outlasts the ladder's fixed ~155 s / ~100 s monolithic burn and reaches the
decomposer**. The monolithic passes are node-capped (400k), not wall-capped,
so their total cost is wall-independent: any wall above the burn solves, any
wall below it fails. That is the cliff, and it is why the cliff is not
size-monotone (i16, bigger than i15, happens to fall to the first helpful
pass in 0.12 s; board row in `benchmarks/air25/ipc5-time.jsonl`).

### S2 — the ladder's wall is mostly verbatim recomputation

The four-pass ladder (helpful/sound → full+tight → full+sound → full+unmasked,
`temporal.rs` pass closure) degenerates here because the relevance masks keep
EVERYTHING and demand is empty:

- `[TREL] sound 1504/1504 tight 1504/1504` (i12), `1692/1692` (i15),
  `3744/3744` (i17); `[TDEMAND] w=3 total=0 resources=[]` on every rung —
  receipts `R/A-*-i1{2,5,7}.log`.
- Hence full+tight ≡ full+sound ≡ full+unmasked: storage-time i15 ran
  `evaluated 588991` with **identical stats three times in a row**
  (`R/A-storage-time-i15.log` lines 71–142), and the Full-tier escalation —
  identical task, since demand total=0 — re-ran the whole quartet again
  (`evaluated 589433` twice, lines 178–214). ~40 s of i15's 60 s wall (~70%)
  is verbatim re-search; on i12 ~25 s (~42%) plus a refused tail
  (`R/A-trucks-time-i12.log`: passes 2 and 3 node-identical, 13.3 s + 11.8 s).

### S3 — why the monolithic search itself cannot win: a total h-plateau

- **trucks-time (ADL)**: best_h is **84 at 10k, 30k, 100k, 300k and 1.41M
  evals** — the heuristic signal is perfectly flat from the first 10k evals
  (`R/A-trucks-time-i12-ev{10000,30000,100000,300000}.log`,
  `R/A-trucks-time-i12.log` line 37). The plateau nodes churn permutations of
  pending END events at a fixed epoch: `popped: time 626.9 g 14–16 agenda 6–8
  [1x DRIVE-END …, 1x DELIVER-END PACKAGE-x …]` for hundreds of thousands of
  pops. `avg_helpful 0.0–0.1` — helpful-op yield collapses under the ADL
  `forall/imply` truckarea encoding (`load`'s `(forall (?a2) (imply (closer
  ?a2 ?a1) (free ?a2 ?t)))`, domain lines 18–28). The contrast receipt: the
  STRIPS encoding of the same family runs `avg_helpful 0.6` and **i13 solves
  in 14.76 s in a single pass** (`R/A-trucks-time-strips-i13.*`) — the
  encoding, not the scale, decides the helpful-action yield. (84 = 6×14
  packages is suggestive arithmetic only; recorded, not relied on.)
- **storage-time**: best_h flat at 7 (helpful pass reaches 4 and stalls) across
  all slices (`R/A-storage-time-i15-ev*.log`); the churn is near-goal:
  doomed 111,839 (invariant-blocked agenda heads — the hoist `lifting`
  exclusivity: `lift` deletes `available` at start, `drop` restores it at end,
  domain lines 19–37), b_blocked 33,722, tie_rescue 7,884
  (`R/A-storage-time-i15.log` line 37). The i14→i15 cliff is exactly one added
  hoist (`:objects` diff) yet grounding grows 1128→1692 ops (+50%) and the
  extra-hoist interleavings drown the fixed 400k-node pass.
- Search-order hatches move nothing: `FF_NO_TSYMM`, `FF_TLIFO`,
  `FF_TEMPORAL_ABS_KEY`, `FF_NO_TLAMA`, `FF_TEMPORAL_NODE_CAP=0` (one
  uncapped deep pass), `FF_NO_ESCALATE`, `FF_NO_TDEMAND` all FAIL on
  i12/i15/i17 with best_h unmoved (`R/A-*-i1{2,5,7}-{notsymm,tlifo,abskey,
  notlama,nodecap0,noesc,notdem}.*`; abskey shifts i15's h landscape to
  best_h 1 but still fails). `deduped 0` everywhere — orbit dedup contributes
  nothing on these boards.

### Side finding — TDECOMP conflates timeout with unsolvability

At the 60 s baseline the decomposer enters after the wall is spent; every
contract search returns via "pass entry refused" and TDECOMP records
`contract 0 UNSOLVABLE from current state`, merging 14→1 (i12) / 5→1 (i15)
(`R/A-trucks-time-i12.log` lines 118–222, `R/A-storage-time-i15.log` lines
247–294). The verdict text is false — the contracts are solvable in
milliseconds with wall (S1). Any future logic that trusts contract verdicts
(F3 builds, F6 crucible notes) must treat a refused-entry UNSOLVABLE as
"unpriced", not "impossible".

### Serialization read (leg 5, solved side)

- trucks-time i11 (`R/A-trucks-time-i11.json`): 63 steps, makespan 8431.75,
  **max concurrency 2** (both trucks driving; 8 overlapping drive pairs;
  truck1 19 drives vs truck2 5). Within a truck the plan is serialized by the
  truckarea closer-chain quoted above; across trucks concurrency is real but
  thin. No deadlines exist to compress against: i12's goal is bare
  `(delivered …)` ×14, **no TILs** (`tils=0` on every pass-start line) — the
  memo's "compiled-deadline/TIL structure" hypothesis is ruled out for this
  family; makespan pressure is entirely drive-time serialization.
- storage-time i14 (`R/A-storage-time-i14.json`): 20 steps, makespan 24.02,
  max concurrency 3 (two hoists + move). The decomposed i15 plan reaches
  makespan 19.02 with the third hoist.

### Instrument caveats (recorded, not chased)

- `FF_TEVAL_BUDGET` runs on storage-time i15 burned 46–54 s of *real* time
  after exhausting 10k–300k eval budgets (`R/A-storage-time-i15-ev*.time`) —
  the post-budget wall is unattributed by current narration. Flagged for the
  F6 instrumentation backlog; slice h-readings themselves are unaffected.
- Leg-1's `A-trucks-time-strips-i13` first attempt hit a bad path (this
  variant keeps per-instance domains under `domains/domain-N.pddl`); the
  empty-JSON receipt was overwritten by the corrected run.

### Fence compliance

The temporal delete-relaxation ledger stays closed: no `FF_TRPG` /
`FF_H_ENDGATE` runs, and this report's exit is **not** a temporal-h-accounting
claim. The flat-h observations in S3 are descriptive of the plateau the ladder
sits on; the named, actionable mechanism is ladder routing + duplicate-pass
serialization + ADL helpful-yield collapse — search shape and serialization,
inside this sitting's charter.

## Priced band

- **Measured, 60 s tier, existing opt-in hatch (`FF_TDECOMP=1`), zero code:**
  +2 of the RED set (trucks-time i12 at 0.03 s; storage-time i15 at 51.89 s —
  near-wall, fragile). storage-time i17: no at 60 s.
- **Measured, 300 s tier, default engine, zero code:** +2 (i12 at 155.5 s,
  i15 at 117.8 s).
- **Build-shaped upside (F3 candidate this decode opens):** stop the wall
  waste named in S2 — skip mask-degenerate duplicate passes (sound≡tight≡all
  ⇒ run the complete pass once) and skip the Full-tier re-run when demand
  total=0 (identical task by construction). That alone returns ~25–40 s of a
  60 s wall to later rungs; on the receipts above the decomposer needs <1 s
  (i12) / ~15 s of contract work (i15) once entered, so the two 60 s flips
  should hold **without** arming decompose-first globally. Band for the
  trucks/storage share of the −49: **+2 firm, +3–6 plausible** (i17 plus the
  unswept trucks-time i13–i18 / storage-time i18 tail — unmeasured, not
  claimed). The full −49 is NOT claimed: trucks-time-strips' non-monotone
  track (i11/i12 fail while i13/i14/i18 solve) shows the ADL helpful-yield
  strand (S3) needs its own pricing before the family sweeps.

Per the exit clause: mechanism named with instance and number ⇒ this report
commits; the F3 gate question (ladder dedup + decomposer routing, restore
hatch and referee per the flag-evidence law) goes to the roadmap.
