# Changelog

All notable changes to this project are documented here.

## [Unreleased]

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

## [0.19.0] - 2026-07-31 — The contest cycle

Improve the standings on every entered track and enter the one the
project always fenced off — by direct request (cycle record in
`docs/roadmap-0.19.md`).

### The reject audit (~120 instances back from the front door)

- **Negative number literals** (`(= (d p0) -370)`) now lex; the
  sailing/fo-sailing/fo-counters reject cluster parses and searches.
- **Implicit `(total-cost) = 0`** — the PDDL 3.1 `:action-costs`
  convention: agricola, flashfill, and settlers (60 IPC-2018
  instances that silently returned zero facts) ground and solve.
- **Named verdicts**: an unsolvable-at-grounding result now says WHY
  in `Solution.notes` ("goal fact (X) is unreachable: no surviving
  grounded action adds it").
- Reject columns: 2018-sat **60 → 0**, 2023-numeric **60 → 1**.

### The optimal tracks, entered (`Mode::Optimal`)

- A* + admissible cost-labeled h^max over the same packed task,
  **proof-or-nothing**: a plan is returned only with an optimality
  certificate; caps are inconclusive, exhaustion certifies
  UNSOLVABLE past the delete relaxation. Constant and static-fluent
  action costs; the rest reject by name. `--mode optimal`.
- First entries: **2008 seq-opt 114/270, 2011 seq-opt 90/280, 2014
  seq-opt 48/256 — 252 certified optima**, every plan VAL-green,
  costs cross-checked against the independent cost-sweep oracle and
  literature. The h^max walls (floor-tile, parking, barman) are
  named; classical LM-cut is the recorded next bet.

### The numeric-heuristic swing (+52/−1)

- Linear numeric goals (`(>= (+ (* 2 (x)) (y)) (d))` — the 2023
  numeric track's staple) now get a repetition-counting gradient:
  `linearize` + ⌈gap / combo-delta⌉ charges, running only where the
  old bare-fluent path punted. **2023-numeric 129 → 181 solved
  (valid 113 → 165)**: farmland +17, fo-farmland +17, counters +8.
  One named casualty (tpp-metric-time i4, `FF_NO_NUMH` hatch).

### Ladder, memory, and emission

- **Novelty by default under a budget**: with `FF_TIME_LIMIT`
  declared, the width-1 novelty rung runs by default (0.18's gated
  +4/−0 referee; `FF_NO_NOVELTY` opts out; budget-less behavior
  byte-identical). At the cut this compounded to **+16/−0 on
  2018-sat** (30 → 50 valid over the cycle) and **+11 on the
  580-instance seq-sat flagship** (441 → 452, its first movement in
  three cycles).
- **The node cap can now see the memory limit**: the retained-bytes
  target clamps to 60% of the actual `RLIMIT_AS` — tiny-state
  numeric searches stop dying to the OOM killer before the internal
  cap fires (the numeric board's 105-row mem-cap class, attributed
  to search-state growth, NOT grounding).
- **Emitted-duration reconciliation**: final plans replay and clamp
  state-dependent durations to their domain expressions at emitted
  start times (never half-correcting). The map-analyzer witnesses
  refused the fix and decoded the debt one level deeper (ε-shifted
  starts also precede propositional providers) — named 0.20 work.

## [0.18.0] - 2026-07-29 — The living-village cycle

Correctness debt paid first, then the village made live and visible,
with the budget-aware ladder as the cycle's engine bet (cycle record
in `docs/roadmap-0.18.md`).

### The ε-emission order inversion, fixed (0.17's named debt)

- `epsilon_separate` now repairs SAME-SLOT end groups by invariant
  relation before emission — if one end's deletes hit another's
  invariant-positives, the protected end emits first; cycles defer to
  the existing STN-consistency veto, and zero-slack geometries keep
  the recorded raw-times fallback. Fixture:
  `benchmarks/bench/eps-cross-*` (minimized match-cellar shape) pinned
  as a unit test on the emission pass itself.
- **match-cellar-2014: VAL 0/20 → 20/20** — the whole red cluster
  green, coverage and plans byte-stable. The 630-instance
  2006/2008/2011 tempo board: **zero movement instance-by-instance**.
  2014 tempo-sat standing: valid **42 → 62 of 200**.
- map-analyzer's 3 reds survived and REFUTED the 0.17 hypothesis —
  solo-check decoded them as **state-dependent duration drift**
  (duration expressions read fluents; an ε-shifted start crosses a
  fluent write; VAL fails the duration constraint). Named 0.19 debt
  with witnesses.

### The village, alive (tick loop + screens)

- **`examples/village_live.rs`**: the tick-loop economy over
  `benchmarks/village/` — one authoritative world `Session`, workers
  HIRED by goal contract (fork + restrict + `set_goal`), validity as
  the free suffix replay on a probe fork carrying the worker's own
  contract, dispatch via in-flight durative starts, interval ends
  firing from `elapse`, and a mid-run theft forcing a drift rethink.
  Measured: two workers, three contracts, one theft survived —
  `benchmarks/village-live.md`.
- **`web/village-live.html`**: the same loop LIVE in the browser —
  map, economy sparklines, contracts and visible intentions per
  worker, theft/till disruption buttons — over new `WasmSession`
  verbs (`apply_start`, `elapse`, `set_fluent`/`fluent`,
  `restrict_contains`, `plan_valid_json`).
- **Plan introspection** (`introspect` module + the solver demo's
  "Explain this plan"): causal links (last-achiever replay over the
  solver's own grounding), invariant spans (`over all` conditions
  from the original schema, arguments substituted), preference
  breakdown (final-state goal prefs + verify-oracle trajectory
  prefs).

### A seven-cycle-old corpse, found by the new smoke test

- On wasm32, `NODE_CAP_TARGET_BYTES = 8 << 30` silently wrapped to
  ZERO (32-bit usize; shl drops high bits) — every default-cap wasm
  solve (all of temporal, the classical best-first fallback) had been
  dead since 0.8, invisible behind EHC-solvable demos and the
  explicit budgets of Session thinks. Fixed with a width-guarded
  2 GiB 32-bit ceiling (64-bit byte-identical); the wasm demo's
  temporal examples went unsolved → solved.
  `crates/ferroplan-wasm/smoke.js` (headless-Chromium page smoke) is
  now part of the cut drill.

### The budget-aware ladder (the novelty referee's next idea)

- `FF_TIME_LIMIT=<secs>` tells the engine its REAL wall budget; a
  bounded classical rung (LAMA, novelty) is entered only while more
  than 40% of the budget remains, so late-ladder rungs stop starving
  the complete fallback near the budget edge — the mechanism behind
  the novelty referee's −51. Unset ⇒ byte-identical to 0.17.
  `benchmarks/ipc67.py` passes its per-instance timeout
  automatically; `FF_WALL_DEBUG=1` narrates the gate's verdict.
- **The referee, re-run at the cut** (all eight gate-touched classical
  boards): base boards neutral within noise (the 580-instance flagship
  variant-for-variant identical; every casualty solo-verified as
  contention noise, not gate tax), and **the novelty rung under the
  gate scores +4/−0** where 0.17's ungated verdict was +7/−51 — the
  tax is gone when the budget is declared. `FF_NOVELTY` stays opt-in;
  default-on-under-`FF_TIME_LIMIT` is the recorded 0.19 candidate.

---

Older releases: [`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) (19 earlier releases, 0.1.0–0.17.0).
