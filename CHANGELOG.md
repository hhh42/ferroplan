# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.21.0] - 2026-08-04 — The numeric cycle, and the ladders that pay their own way

The cycle that took the sailing wall down, closed a temporal debt
carried since 0.18, and repaired the −26 coverage regression the v0.19
backfill exposed in 0.20 — while keeping every win 0.20 had bought.
Full record:
[`docs/roadmap-0.21.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.21.md).

### Where this leaves the standings

**53% coverage across 13 IPC boards** (2,153/4,076), of which **354 are
certified optima** — on the optimal tracks coverage IS proof rate.
At-a-glance: [`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md);
per-track detail: [`benchmarks/ipc-standings.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc-standings.md).

Against 0.19.0 — re-measured on the SAME machine, so the comparison is
engine-to-engine — the twelve comparable boards move **1,943 → 2,132,
+189**:

| board | 0.19 | 0.20 | **0.21** |
|---|---|---|---|
| 2026 numeric | 124 | 121 | **165** |
| seq-opt (08/11) ⚖️ | 235 | 250 | **275** |
| 2023 numeric | 193 | 194 | **229** |
| 2014 seq-agile | 114 | 103 | **142** |
| 2014 seq-sat | 115 | 110 | **138** |
| seq-sat (08/11) | 472 | 473 | **486** |
| 2018 seq-sat | 63 | 53 | **70** |
| 2014 tempo-sat | 65 | 66 | **70** |
| 2023 classical | 30 | 27 | **32** |
| 2023 agile ENTRY (300 s) | 49 | 48 | **51** |
| tempo-sat (08/11) | 419 | 419 | 416 |
| 2014 seq-opt ⚖️ | 64 | 56 | 58 |

Two boards remain behind 0.19 and are not netted away: **tempo-sat −3**
(within the ±4 band re-measurement showed on this box) and **2014
seq-opt −6**, which is entirely `city-car` — the one domain where the
optimal root gate does not recover what 0.20's unconditional
quarter-budget sprint cost. Both are 0.22 work.

**Every board in this release was measured under recorded conditions.**
This box is a laptop, and contention only ever depresses coverage — so
it invents regressions and hides gains. Each board now carries a
`conditions.json` (median idle, load, swap, and the competing processes
by name); a board measured below 65% median idle is refused rather than
banked, and the driver re-measures it at the next quiet window. All 13
boards here are verdict `clean`, 67.8–74.2% median idle. Two apparent
regressions in the first pass (tempo-sat −19, the 300 s entry −3)
turned out to be contention and vanished on clean measurement.

- **The numeric-precondition charge** (Phase 3): extraction now
  charges a selected op's unsatisfied numeric preconditions through
  the existing achiever machinery — sailing-numeric i1 goes from a
  5,000,048-eval cap-out to a 174-step solve at 29,203 evals;
  block-grouping i1 (a 0/20 domain) solves in 24 evals via the new
  one-sided Eq charge. Hatch `FF_NO_NUMPRE`; numeric novelty lands
  opt-in behind `FF_NUMNOV`; temporal groundings deliberately keep
  0.20's heuristic. The capped-search text no longer claims "proven
  unsolvable".
- **The optimal ladder learns the clock** (Phase 4): under an armed
  `FF_TIME_LIMIT`, a root informativeness gate decides whether LM-cut
  earns the remaining wall or h^max keeps the full budget, and the
  h^max sprint is time-boxed (`FF_OPT_SPRINT_FRAC`, default 0.4).
  scanalyzer-08 i4: PROVEN cost 24 inside the wall vs 0.20's 60 s
  kill mid-sprint. No armed wall ⇒ bit-identical to 0.20. Hatch
  `FF_OPT_NO_ROOTGATE`; h-memo on re-opened states kept (−4.6%
  evaluated, expansions identical).
- **The static-fluent fold** (Phase 6): defined-static, irrelevant
  fluents fold to constants and the fluent tables compact out of
  every stored node — data-network i12 drops 3,683 → 209 bytes/node
  (17.6×), tpp i12 24,418 → 4,672 (5.2×) — with plans, eval counts
  and expansion order byte-identical (hatches `FF_NO_FLUENT_FOLD`,
  `FF_NO_FLUENT_COMPACT`). The session `set_fluent` contract is
  pinned with a fixture whose teeth are proven. `FF_MEM_BUDGET_GB`
  tells the engine its byte budget on kernels without a workable
  RLIMIT_AS (macOS), so the retained-state cap trips internally and
  the refill loop spends the wall the RSS watchdog used to eat.
- **The ladder tax** (Phase 5): under an armed budget, EHC and
  novelty-light get wall-denominated slices (`FF_EHC_WALL_FRAC` 0.25,
  `FF_NOVLIGHT_WALL_FRAC` 0.10) instead of op-scaled/fixed-pop
  budgets — the repair for the −26 the v0.19 backfill exposed.
  hiking-2014 i6: 55.5 s (half a second inside the kill line) →
  20.3 s, same plan; openstacks i1 keeps its EHC-direct solve. No
  armed budget ⇒ byte-identical. Hatch `FF_NO_EHC_WALLCAP`; rung
  narration under `FF_WALL_DEBUG`.
- **Temporal emission is sound on the witness** (Phase 7): the two
  same-slot bubble repairs become one per-slot topological order
  with cross-kind guard edges — map-analyzer's three VAL-RED rows
  (the only temporal VAL failures on the twelve boards, 0.20's
  honest negative) go GREEN: solo referee 13/13 VAL-valid.
- **The h-surgery bet dies its pre-registered death** (Phase 8): the
  end-gated interval credit probe landed, priced a snap pair as one
  unit (pinned), and BOTH reads failed — the village stool contract
  still dies at 200k evals, and TMS's best_h floor re-levels
  110→174 without breaking. Fifth negative on this wall; the ledger
  line dies with a sharper localization; the probe stays dormant
  behind `FF_H_ENDGATE`.
- **Harness**: the IPC-2026 -opt pairs get a proof-track board
  (`ipc2026-opt`, cut21-sweeps.sh + promote-air21.sh); multipart
  instance names keep their full identity in the JSONLs; the
  early-exit class is closed (the classifier's timeout line moved to
  the refill loop's 90% re-entry floor).

## [0.20.0] - 2026-08-01 — The guidance cycle, cut on new silicon

The cycle that set out to improve search GUIDANCE — and then had to move
house mid-cut. Phases 1–5 landed on the old cloud container; the cut
itself, and every board in it, was run on an M5 MacBook Air. That
migration is not a footnote: **every scoreboard number in this release
was re-measured from scratch on the new machine**, and none of them may
be read against a 0.19 number. Faster silicon inflates coverage at a
fixed time budget, so a cloud→Air "improvement" would be hardware, not
progress. Full record: [`docs/roadmap-0.20.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.20.md)
and [`docs/roadmap-0.21.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.21.md).

### Where this leaves the standings

**48% coverage across 12 re-baselined IPC boards** (1,917/4,016), of
which **306 are certified optima** — on the optimal tracks coverage IS
proof rate. seq-sat 473/580 (82%), tempo-sat 419/630 (67%), 2023
numeric 194/400, seq-opt 250/550. At-a-glance:
[`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md); per-track detail:
[`benchmarks/ipc-standings.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc-standings.md).

Two boards deserve calling out. **482 of 485 temporal plans validate**
under VAL across the IPC-6/7 boards (419/419 and 473/473 green); the
only three failures are the map-analyzer rows this cycle already
recorded as an honest negative. And the **IPC-2026 numeric corpus gets
its first board — 121/320, with ZERO engine-rejects across 16 domains
the planner had never seen.**

### Spend the whole wall (Phase 1)

- The runner records elapsed wall for UNSOLVED rows, and the standings
  classifier now separates a graceful exit AT an armed `FF_TIME_LIMIT`
  from a true fast reject. The old columns overstated rejects and
  understated timeouts on every budget-armed board.
- **The refill loop**: after ladder exhaustion with >10% of a declared
  wall remaining, the search re-enters GREEDIER (w_h ×4, max_eval ×4,
  at most 6 rounds). An engine holding a time limit should not return
  unsolved with double-digit budget unspent. Hatch: `FF_NO_REFILL=1`.

### LM-cut, and an admissibility bug it uncovered (Phase 2)

- **A 0.19 soundness repair first.** h^max iterated only unconditional
  adds, so a goal reachable only through a `(when ...)` effect was
  labelled unreachable — an OVERestimate, and A* certified wrong optima
  (pinned witness: "PROVEN cost 100" where the optimum is 11). The
  relaxation now runs over an achiever list. The differential says the
  252 shipped 0.19 certificates were not corrupted in practice: the bug
  was real, its bite was not.
- **LM-cut** (Helmert & Domshlak 2009) over the achiever graph, as a
  two-rung ladder — an h^max sprint on a quarter of the node budget,
  then LM-cut on the full one. The PROVEN note names its prover.
  Hatches: `FF_NO_LMCUT`, `FF_NO_HMAX_SPRINT`.
- **Priced honestly, by differential.** 13 certificates carry the LM-cut
  prover label, but re-running exactly those instances with
  `FF_NO_LMCUT=1` on the same box shows four fall to the h^max sprint
  anyway — so LM-cut's UNIQUE contribution is **9 of 306 certificates
  (2.9%)**. No instance is lost by running it (`hatch-only 0`), so the
  two-rung ladder costs nothing. Against the phase's 554-instance
  ambition that is a small pot; it is also real, free and correctly
  wired, which is a different verdict from "does not pay".

### The novelty-LIGHT rung (Phase 3)

visit-all-2014 — the canonical width-2 domain, dispatched in
milliseconds by BFWS-class planners — took 35 s here, and forcing the
existing novelty rung changed nothing. The decode: that rung IS
BFWS-shaped, and spent all 35 s on per-pop `relaxed_helpful` calls a
width-1 structure never needed. So: `novelty::search_light`, IW(1) +
goal count with ZERO heuristic evaluations. **visit-all-2014 i1 35 s →
under 1 s**, and the domain now scores 20/20. Cap priced at 300k pops
(~1 s ladder tax). `FF_NOVLIGHT` / `FF_NO_NOVLIGHT` / `FF_NOVLIGHT_ONLY`.
The cycle also named what it did NOT expect to move — transport,
parking, cave-diving — and all three duly stayed at 0/20.

### Retained-state compression (Phase 4)

The visited structures stored a full StateKey per inserted node,
duplicating what the node arena already held. They are now hash → node
index buckets, with collisions settled exactly against the arena.
Dedup verdicts and expansion order byte-identical. RSS at an identical
forced cap: city-car 133.9 → 113.2 MB (−15%), block-grouping-numeric
169.9 → 124.2 MB (−27%).

### The debt basket (Phase 5)

- **tpp-numeric's early exhaustion, closed**: every probed instance is
  a node-cap trip, not open-list exhaustion — no completeness hole.
- **drone-numeric's 16 VAL-RED rows, attributed to VAL**: its parser
  fails on any drone problem with two objects of the location type,
  before a plan is judged. The runner now records `val: null`
  (validation unavailable), which is not the verdict "plan rejected".
- **The sailing class, named**: sailing / markettrader / pathwaysmetric
  share a genuine numeric-reachability wall (interval/AIBR-class), on
  the record for a numeric cycle. Confirmed on the Air, and again on
  IPC-2026's sailing-wind-sat (0/20) — instances this cycle never saw.
- **An honest negative**: the ε-separation START-vs-provider surgery
  landed a same-slot pin, but the three map-analyzer VAL-RED rows are
  NOT that shape — the repair belongs in the temporal emission layer.
  Carried forward with a sharper decode.

### The MCP server grows a memory (`session_*`, on rmcp)

The library has had a rich `Session` API since the many-minds cycle — fork,
observe, elapse, timed facts, budgeted rethink — and the MCP server exposed
none of it. An agent could ask `solve` a question but could not keep a world
open: every step re-sent the whole domain and paid grounding again. That is now
fixed, and the server moved onto [`rmcp`](https://crates.io/crates/rmcp), the
official MCP Rust SDK, to do it.

- **Ten session tools.** `session_open` grounds a world ONCE and returns a
  handle; then `session_set` (facts / fluents / scheduled timed facts / goal, in
  one call), `session_observe` (returns only the SURPRISES — sightings that
  contradicted belief), `session_elapse`, `session_apply_start`,
  `session_replan` (optionally budgeted), `session_state`, `session_list`,
  `session_close`. The loop is: open once, then *tell it what changed* →
  *rethink*.
- **`session_fork` — the many-minds primitive over the wire.** A fork shares the
  grounded world and owns its beliefs and goal, so two minds can disagree about
  whether they are done. `session_state` reports `world_bytes` (shared, paid
  once) against `mind_bytes` (what one more fork costs) — pinned by a test that
  moves the fork, checks the parent did not move, and asserts both still report
  the same world.
- **On rmcp.** Framing, capability negotiation, tool-schema derivation and the
  error conventions now come from the SDK; tool input schemas are DERIVED from
  the Rust parameter types and cannot drift from the code. This is where the
  `schema` feature below pays off end to end: `solve`'s `options` advertises its
  real knobs instead of an opaque object, pinned by
  `protocol.rs::solve_advertises_a_typed_options_schema`.
- **Behaviour changes worth naming.** The server now enforces the MCP lifecycle
  — `initialize` must precede `tools/call`, per spec, where the hand-rolled loop
  was permissive. Requests are served concurrently and the two expensive calls
  (grounding, search) run off the runtime, so one deep search cannot stall other
  sessions; ordering dependent calls is the client's job, as in any JSON-RPC
  service. And **this crate's MSRV is now 1.88** (rmcp's), overridden locally so
  the LIBRARY keeps the workspace's 1.74 — an MCP server is a tool you run, not
  a dependency you compile into something old.
- The stateless four (`solve` / `parse` / `validate` / `decompose`) answer
  exactly as before, including `solved: false` as a normal answer and tool
  failures as readable `isError` results. 13 protocol/session tests drive the
  real binary over stdio.

### Uptake from downstream (thanks, Sean Chatman)

Two self-contained improvements adopted from
[seanchatmangpt/ferroplan](https://github.com/seanchatmangpt/ferroplan), which
runs this planner as the deterministic core of a Claude Code agent control
plane and pushed hard on the surfaces below. Credit to Sean for both the
patches and the pressure-testing.

- **`schema` cargo feature** (off by default) derives
  `schemars::JsonSchema` on `Options`, `Mode`, and `Search`, so MCP servers
  and other tooling get a *typed* configuration schema instead of an opaque
  `Value`. `schemars` is an optional dep: default builds — and
  `ferroplan-wasm`/`-cli`/`-bevy` — pull nothing new. Defended by
  `tests/api.rs::schema_feature_types_the_options_surface`.
- **Three more wasm bindings** on `WasmSession`: `set_timed_fact` (schedule an
  exogenous flip `dt` from now), plus `world_bytes` / `mind_bytes` for the
  shared-world vs per-fork memory split the bazaar demo wants.

### The move to new hardware, and three bugs it exposed

Porting the harness to macOS/ARM was supposed to be paperwork. It found
three things that would each have ruined a sweep:

- **`RLIMIT_AS` cannot be set on macOS at all.** It reports INFINITY and
  rejects every `setrlimit` with EINVAL. Raised inside a `preexec_fn`,
  that surfaced as a spawn failure — and the runner's retry then booked
  **every instance** as `spawn-fail`. The twelve-board sweep would have
  burned ~5.6 hours producing 4,016 garbage rows that looked like
  environmental fork failures. Now probed once, side-effect-free.
- **The per-job memory cap got a new instrument**: a 0.25 s RSS
  watchdog, since the address-space cap is unavailable. On this path the
  mem-cap column measures RESIDENT bytes — a different instrument
  reading the same column, recorded wherever it is used.
- **The IPC-2026 corpus lost three instances to its own normalizer**: a
  0-indexed `p000.pddl` collapsed to an empty instance number, which the
  runner then died on mid-listing, taking the board with it. Fixed at
  source; the runner now skips un-numbered files loudly rather than
  crashing.

Also: `benchmarks/get-val.sh` builds again (CMake 4.x removed the
pre-3.5 compatibility VAL's CMakeLists declares).

### VAL's other refusal, and 15 instances it was hiding

VAL has more than one way to decline a domain. 0.19 taught the runner
`"Parser failed"`; `data-network-2018` and `factory-robot-2026` instead
say `"Problem in domain definition!"` — and say it against an EMPTY
plan, so VAL never judged our plans at all. Those rows arrived as
`val: false`, and since the standings drop a rejected plan from
coverage, **the standings table read 15 instances lighter than the
boards beside it** (2018-sat 46 vs 53; 2026-numeric 113 vs 121). One
sweep, two artifacts, disagreeing.

`val_check` now tests a list of unavailability signatures, and a VAL
*timeout* returns `null` rather than `false` for the same reason.
[`benchmarks/val-availability.py`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/val-availability.py)
probes every domain and currently names four VAL cannot ingest.

### Release notes you can actually read

The front page had accumulated **sixteen "What's new" blockquotes —
~308 of 684 lines, 45% of the README** — so a visitor met a year of
history before learning what the planner is. The changelog had reached
22 releases and 1,919 lines.

- [`scripts/release-notes-roll.py`](https://github.com/hhh42/ferroplan/blob/main/scripts/release-notes-roll.py)
  keeps `[Unreleased]` plus the newest two releases in both places;
  older changelog sections move verbatim to
  [`CHANGELOG-ARCHIVE.md`](https://github.com/hhh42/ferroplan/blob/main/CHANGELOG-ARCHIVE.md). `publish.sh` reads
  release notes from BOTH files, so archiving never breaks
  `--release-only <old-version>`.
- **[`STANDINGS.md`](https://github.com/hhh42/ferroplan/blob/main/STANDINGS.md)** is new: every track banded and
  sorted, proof tracks marked, cloud-era boards held separate and
  excluded from the headline. Generated by `standings.py`, which
  patches the README headline in the same run so the shop window cannot
  drift from the boards.
- `benchmarks/standings-history.json` banks per-release numbers, each
  tagged with the BOX it ran on. Improvements are only ever computed
  between snapshots from the same hardware; where no comparable
  predecessor exists the table says "baseline" instead of inventing a
  delta.

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (21 earlier releases, 0.1.0–0.19.0).
