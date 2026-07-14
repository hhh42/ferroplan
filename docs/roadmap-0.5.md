# Roadmap — the road to v0.5 ("First Place")

> **Status (0.5.0 shipped): executed, honest outcome "closing on first."**
> Phase 1 ✅ (ESPC graduated: deterministic eval budget, default-on, the old
> opt-in row reproduced exactly on defaults). Phase 2 ✅ (anytime sweeps —
> measured neutral alone — plus the diversified restart ladder, which broke
> the storage/tpp/pathways plateaus: storage flips to 7/8). Phase 3 ⚙
> (partitioned closure seed BUILT and measured NEUTRAL on finals — the
> anytime+ladder loop dominates composition-as-seeding; ships opt-in
> `FF_PREF_SEED3`, kept as the λ-pricing substrate). Phase 4 ✅ beyond its
> gate (sub-lever (c): numeric-closure routing flips **rovers to a full
> domain lead** — the 0.4.0 churn verdict was a first-improvement artifact).
> Phase 5 ✅ (defaults-only remeasure). Final ledger: **3 of 6 domains led
> under BOTH conventions** (openstacks, storage, rovers), trucks led on
> totals/drawn on instances, tpp+pathways to SGPlan5; suite-wide instance
> tally 19W/14T/15L. The 4/6 first-place bar was NOT met — per this plan's
> own protocol, 0.5.0 ships the work with the "closing on first" verdict.
> The 0.6 headline: the tpp/pathways tails (direction-bound, ladder- and
> composition-resistant) and per-stage λ pricing on the seed3 substrate.
>
> Successor to the archived [0.2.1 roadmap](roadmap.md). Ground truth for the
> numbers below: [`benchmarks/ipc5-scoreboard.md`](../benchmarks/ipc5-scoreboard.md)
> (2026-07, post-0.4.1) and the [ESPC spec](espc-preferences-spec.md).

0.4.0 took ferroplan from a distant quality 2nd on the IPC-5 simple-preferences
suite to a **strong 2nd**: full 48/48 coverage, two domain-level leads, and
small-instance parity nearly everywhere. **0.5 has one headline: take first
place — on the defaults.** Everything in this plan either closes a measured
per-domain gap or makes the claim legitimate (single configuration,
deterministic budgets).

---

## Where we stand (the ledger)

IPC-5 ranked per domain: **coverage first, then plan quality**. Both ferroplan
and SGPlan5 cover 48/48, so quality decides each domain. "Quality" can be
read two ways, and the two conventions disagree about who leads what today:

| domain | per-instance (W/T/L for ferroplan) | totals (ferroplan vs SGPlan5) | leader by instances | leader by totals |
|---|---|---|---|---|
| openstacks¹ | 5/0/3 (wins p04–p08) | 271 vs 326 | **ferroplan** | **ferroplan** |
| storage | 5/0/3 (wins p01–p05) | 677 vs 547 | **ferroplan** | SGPlan5 |
| trucks | 2/3/3 | 29 vs 31 | SGPlan5 | **ferroplan** |
| tpp | 0/4/4 | 594 vs 489 | SGPlan5 | SGPlan5 |
| pathways | 0/4/4 | 64.1 vs 47.4 | SGPlan5 | SGPlan5 |
| rovers | 2/0/6 | 5680.1 vs 5132.5 | SGPlan5 | SGPlan5 |

¹ with the opt-in `FF_ESPC` partitioned penalty loop — see "Eligibility" below.

Read it honestly: **only openstacks is an unambiguous lead.** Storage flips on
totals (SGPlan5 keeps the three largest instances), trucks flips on instance
wins (we lose p03/p06/p08 by 1/6/4 points). First place that survives scrutiny
means **≥ 4 of 6 domains led under BOTH conventions**, on default settings.

### The arithmetic per domain

- **openstacks, secure the sweep** — p01–p03 trail 19/23/17 vs 13/16/12
  (+18 total). The per-order partition grain is too coarse on small instances;
  the **polish B&B is the binding mechanism** (scoreboard item 1 residual).
- **storage, win the totals too** — p06–p08 trail 145/200/263 vs 124/160/132
  (+192 on the tail; we are −62 ahead on p01–p05, so **−130 net** flips the
  sum). Plateaus at the 2M eval budget under first-improvement tightening.
- **trucks, flip the three instance losses** — p03 1→0, p06 6→0, p08 10→≤6.
  Small absolute counts; the closest domain flip on the board.
- **pathways** — tail gaps 8.5/12.9/12.5/20.2 vs 6.5/10/8/12.9 (+16.7). Small
  absolute, 30–57% relative.
- **tpp** — tail gaps 97/116/131/146 vs 79/101/100/105 (+105). The largest
  pure-preference tail gap.
- **rovers** — p01–p06 trail by +7.9…+206.9 (+547.6 total incl. our p07/p08
  wins). A different problem class: numeric metric, subset selection (which
  preferences are worth their forced traverse cost). Prefix-cost ordering is a
  **measured dead end** (`w_c`, 0.4.0); the lever must price the *completion*.

**Minimum winning set:** hold openstacks, secure storage, flip trucks, flip
one of pathways/tpp ⇒ 4/2. rovers is the stretch (research-grade); tpp and
pathways ride the same mechanisms, so the plan targets both and needs one.

### Eligibility (the claim has to be legitimate)

The openstacks lead currently rides `FF_ESPC=1 FF_ESPC_TIME_MS=90000` — an
opt-in env var with a **wall-clock** outer budget. A competition entry is one
configuration, and this codebase's contract is determinism (same problem, same
plan, any thread count, any machine). Two consequences for 0.5:

1. ESPC must engage **by default** where its trigger structure exists (it is a
   verified no-op on the other five domains — no deadline pairs).
2. Its outer budget must become a **deterministic eval count**, exactly the
   conversion `FF_PREF_EVAL_BUDGET` already made for the B&B in 0.4.0.

Without these, "first place" is an asterisk. With them, the scoreboard's
default row IS the entry.

---

## Phases

```
Phase 1: ESPC graduation ──► Phase 2: Tightening upgrade ──► Phase 3: Partitioned closure
        (eligibility,                (trucks flip;                  (tpp/pathways/storage
         low risk)                    tails; openstacks p01–p03)     tails — the big lever)
                                                                          │
                              Phase 4: rovers completion pricing ◄────────┤ (independent, gated)
                                                                          ▼
                                                            Phase 5: Measure everything + ship
```

Ordering rationale: Phase 1 is cheap and makes every later measurement a
*default-settings* measurement. Phase 2 is the highest expected value per line
of code (one loop, three domains touched). Phase 3 is the named next lever in
both the scoreboard and the ESPC spec, and the largest build. Phase 4 is
explicitly gated — 4/2 is reachable without rovers.

---

## Phase 1 — Graduate ESPC: deterministic budget, default-on where it bites

**Why:** eligibility (above), plus the 0.2.1 roadmap's unfinished Phase-1 item
("ESPC's latency trade… decide always-on-where-it-bites vs. a smaller default
budget"). The loop already terminates by stall/saddle well inside its budget
(worst case ~58 s on p04), and `features::espc()` / the deadline-pair trigger
already scope it to where it does anything.

**Scope:**
- Convert the outer-loop budget in `espc.rs` from `Instant`/`FF_ESPC_TIME_MS`
  to a deterministic evaluated-state budget shared with (or sized like)
  `FF_PREF_EVAL_BUDGET`; keep wall-clock as an optional *additional* cap for
  interactive use, never the primary contract.
- Flip `features::espc()` to default-on when the compiled task carries
  deadline pairs; add `FF_NO_ESPC` opt-out (house rule: byte-identical old
  behavior reachable). Keep `FF_ESPC_MONO` as-is.
- Re-verify the no-op claim on the other five domains (locked results, t1≡t8)
  and re-measure openstacks 3× per instance on the new budget.

**Acceptance:** openstacks default row ≥ the current opt-in row (19/23/17/16/
21/22/66/87 or better); identical plans across 3 runs and across thread
counts; the other five domains byte-identical with ESPC auto-on; `FF_NO_ESPC`
restores 0.4.1 behavior.

**Touches:** `espc.rs`, `features.rs`, `pddl3.rs` (entry), `tests/espc.rs`,
scoreboard.

---

## Phase 2 — Tightening-loop upgrade: better-than-first-improvement

**Why:** scoreboard item 4 names the plateau mechanism precisely: the
remaining tails (tpp/pathways p05–p08, storage p06–p08) "plateau at the 2M
default budget with **greedy first-improvement tightening**." Each B&B
iteration (`metric_optimize_closure`, and the legacy loop's shared budget
logic in `pddl3.rs`) accepts the FIRST plan under the incumbent bound and
immediately re-bounds. The same loop is the **polish B&B** that binds
openstacks p01–p03. One mechanism, four domains.

**Scope:**
- *Exhaust-then-pick:* when a probe finds an improvement early in its
  iteration cap, keep spending the cap and take the BEST plan found under the
  bound before re-bounding (best-improvement steps descend faster per
  iteration than first-improvement re-probes).
- *Diversified restarts:* on a capped no-improvement probe, before the
  all-remaining-budget escalation, retry with diversified deterministic
  tie-breaking / successor ordering (vary by iteration index, never by RNG
  seeded from time) — the current single escalation re-treads a deterministic
  prefix by design; diversification is the cheap way to make the retry explore
  *differently*.
- Both under the existing deterministic eval accounting; both with restore
  hatches (`FF_PREF_FIRST_IMPROVEMENT=1`, `FF_PREF_NO_DIVERSIFY=1` or
  similar). Measure per domain per change; keep what wins, document what
  doesn't (the `w_c` precedent).

**Acceptance (the trucks flip is the gate):** trucks p03→0, p06→0, p08 ≤ 6
(domain led under both conventions); storage p06–p08 net −130 or better;
openstacks p01–p03 and tpp/pathways tails materially narrowed; **no won
instance regresses**; t1≡t8 everywhere.

**Touches:** `pddl3.rs` (both loops), `search.rs` (`solve_closure_bounded` /
`solve_subgoal_bounded` return-best-under-bound variant), tests, scoreboard.

---

## Phase 3 — Partitioned closure search (ESPC increment 3)

**Why:** the second lever named by scoreboard item 4, and the natural
generalization the ESPC spec has been building toward: increment 2 proved
that λ-scheduled **partitioned composition** beats the monolithic loop where
preference interactions decompose (openstacks 42/…/227 → 19/…/87), but it only
engages on deadline-pair structure. tpp, storage, trucks, and pathways have no
deadline pairs — their tails are still optimized **monolithically** by the
closure B&B, even though their preferences decompose by construction (markets/
goods, crates/depots, packages, pathways).

**Scope:**
- One closure-search stage per preference-interaction component
  (`partition::interaction_partition_of` over the real goal + preference phis;
  the invariant-synthesis mutex groups from `invariants.rs` supply the shared
  resource variables to exclude from edges, as increment 2 did with
  `stacks-avail`).
- Shared/global variables priced by the existing per-trigger λ schedule
  (`espc.rs`), generalized from deadline pairs to cross-partition constraint
  violations; leftover budget to the (Phase-2, improved) monolithic polish
  B&B bounded by the incumbent — the same shape that already works.
- Default path when >1 component is found and the metric is pure-preference;
  monolithic closure otherwise. Restore hatch (`FF_PREF_MONO=1`).
- This unifies `espc.rs` and `metric_optimize_closure` rather than adding a
  third optimizer — increment 2 stays as the deadline-pair specialization.

**Acceptance:** tpp p05–p08 total gap +105 → ≤ 0 *or* pathways tail +16.7 →
≤ 0 (one of the two flips, both attacked); storage p06–p08 improves further;
openstacks unchanged (locked results); no coverage loss anywhere; components
and λ trajectories inspectable in `--json` diagnostics.

**Touches:** `pddl3.rs`, `espc.rs`, `partition.rs`, `invariants.rs`
(consumers), possibly a new `closure_partition.rs`, tests, scoreboard.

**Risk:** highest in the plan (the spec's "open/ambiguous" items — penalty
schedule generalization, grain selection). Mitigation: Phase 2 alone may
already flip trucks + storage; Phase 3 needs to deliver only ONE of
tpp/pathways for 4/2.

---

## Phase 4 — rovers completion pricing (gated stretch)

**Why:** scoreboard item 5. The residual p01–p06 gap is subset selection over
a **numeric** metric (folded traverse costs). 0.4.0 measured the obvious lever
dead: prefix-cost open-list ordering (`w_c`) collapses quality at every weight
— cost only grows along a path, so cost-ordering buries goal-reaching
prefixes. The working lever must price what a preference **still costs to
complete**, not what the path has already paid.

**Scope (in expected-value order, each measured independently):**
- *Forgo-aware seeding:* seed the B&B with incumbents that deliberately forgo
  the preferences whose estimated completion cost exceeds their violation
  weight (a knapsack over relaxed-plan cost estimates), instead of always
  seeding from the all-collect EHC plan.
- *Cost-aware completion heuristic:* extend the relaxed-plan extraction to
  carry the numeric traverse cost of each preference's completion, and use it
  only in the acceptance/closure test (not open-list ordering — that's the
  measured dead end).
- *Numeric closure (exploratory):* extend the exact-closure optimizer to
  folded-numeric metrics so rovers leaves the legacy compiled-goal B&B; 0.4.0
  measured closure-churn here, so this rides only if the Phase-2 iteration
  change alters that verdict.

**Gate:** if none of the three measures a win by the time Phases 1–3 are
merged, rovers ships as-is (documented, like `w_c`) and moves to 0.6 — it is
not on the minimum winning path.

**Touches:** `heuristic.rs`, `pddl3.rs`, `search.rs`, scoreboard.

---

## Phase 5 — Measure everything, then ship

**Scope:**
- Full preference suite: 6 domains × p01–p08 × 3 runs × {1, 8} threads, on
  **defaults only** — the scoreboard's headline table becomes the default
  row, with opt-outs demoted to footnotes.
- Full regression: classical/ADL/numeric (`benchmarks/results.md`), temporal
  (`temporal-results.md`), `examples/rpg-world/suite` + `hard/`, perf harness
  (`benchmarks/perf.py compare`) — the closure/ESPC changes must not move
  anything outside the PDDL3 path.
- Scoreboard rewrite: the ledger table above, re-computed, under **both**
  conventions, with the verdict stated in IPC-5's own coverage-first terms.
  The claim ships only if the ledger shows ≥ 4/6 under both readings —
  otherwise 0.5 ships the same work with an honest "closing on first" verdict.
- Release mechanics per `RELEASING.md` / `publish.sh` (crates.io + tag +
  GitHub Release); README/book/CHANGELOG retell the story; carry-over item:
  the still-unpublished `ferroplan-py` PyPI wheel (optional, non-blocking).

**Version:** 0.5.0, not 0.4.2 — the preference-metric default path changes
again (ESPC auto-on, best-improvement tightening, partitioned closure), the
same reasoning that made 0.4.0 a minor bump. Every default change keeps a
restore hatch.

---

## Explicitly NOT in 0.5

- **PDDL3 trajectory constraints** (`:constraints` — `always`, `sometime`,
  `within`, …): 0.4.1 made them a clean *rejection*; enforcing them is the
  IPC-5 **qualitative**-preferences track, a different (larger) build. It is
  the natural 0.6 headline once first place on simple preferences is banked.
- Dynamic derived predicates, continuous (`#t`) effects, the temporal
  decision-epoch timeout, the numeric-domain EHC gap vs Metric-FF — all real,
  all tracked in README Limitations, none of them move the IPC-5 SP ledger.

---

## The 0.5 story

> 0.4 proved ferroplan could take domains off the IPC-5 winner. **0.5 takes
> the suite**: the penalty loop and the closure optimizer become one
> partitioned, deterministically-budgeted default path — no magic env vars,
> same plan on any machine at any thread count — and the scoreboard's default
> row leads SGPlan5 on four of six domains, coverage-first, under either
> reading of quality. Second place was the honest verdict of 0.4; 0.5 exists
> to retire it.
