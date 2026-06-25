# Roadmap — the road to v0.2.0 ("The Bridge")

ferroplan v0.1 is a mature FF-family engine whose README makes one central bet:
**an LLM should be the author and supervisor of a planner, not its runtime.** Today
that bet is an argument, not an artifact — the engine is deep, but the *bridge* the
thesis promises (natural-language goal → decomposed, solvable contracts → an agent
running the author→run→read→fix loop over a real tool) isn't built yet.

v0.2.0 closes that gap. It hardens what's already proven, adds one engine-depth win,
**builds the bridge**, and ships it. The four phases are ordered so each derisks the
next: cheap hardening first (compounds, low risk), then a credibility win, then the
bridge that depends on a solid engine underneath, then distribution last so we
publish something already solid.

```
Phase 1: Graduate proven flags ──► Phase 2: Temporal depth (TILs) ──┐
         (hardening, low risk)              (credibility)            │
                                                                     ▼
                                            Phase 3: Decomposer ──► Phase 4: MCP + publish
                                                     (the bridge)            (reach)
```

Why "0.2.0" and not "1.0": the README reserves 1.0 for API stability ("APIs may
shift before 1.0"). This release adds a public `decompose` surface and changes
default heuristics — exactly the kind of churn 0.x exists to absorb. 1.0 comes after
0.2's APIs have settled in real use.

---

## Phase 1 — Graduate the proven opt-in flags to defaults

**Progress:** `FF_TDEMAND` graduated (the numeric half). It split into tiers rather
than a blind flip: a blind default regressed makespan on renewable-resource
concurrency domains (the `crew` pool serialized ~5→~10), because the
predicate-goal-threshold seeding reads a net-zero pool guard as accumulation demand.
The default is now the `Numeric` tier (numeric-goal demand only — the full measured
+8, RPG suite+hard 26→34/39, **no regression**); the predicate/structural half +
relevance pruning rides explicit `FF_TDEMAND` (`Full`); `FF_NO_TDEMAND` opts out
(bit-identical to 0.1.0). The override layer is now tri-state (`features::DemandMode`
/ `clear_overrides`). **Still to do in Phase 1:** ESPC's latency trade (its outer
loop is wall-clock-bounded, so "default on" is not free on deadline-structured
domains — decide always-on-where-it-bites vs. a smaller default budget), and confirm
the temporal landmark term needs nothing further (it is already always-on).

**Why:** `FF_TDEMAND`, ESPC, and the temporal landmark term are all measured wins
(`FF_TDEMAND`: RPG 26→34/39, all validated; ESPC: openstacks p08 608→227) carrying
"bit-identical when off" guarantees. They sit behind env vars, which means (a) the
default planner is needlessly weaker than the engine actually is, and (b) the
Phase-3 decomposer would otherwise have to know which magic vars to set. Turning
them on by default is the cheapest high-value work in the release.

**Scope:**
- Flip the defaults in `crates/ferroplan/src/features.rs` (the `tdemand()` /
  `tdecomp()` / `tconc()` getters and `set_overrides`) and the ESPC entry in
  `pddl3.rs:791`. Keep an **escape hatch** — invert each to an `FF_NO_*` opt-*out*
  so anything depending on the old default can recover it, and so we keep the
  byte-identical regression baseline reachable.
- Decide per-flag whether "default on" means *always* or *only when the cheap
  applicability check fires* (TDEMAND is already inert on domains without the
  converging-DAG shape; ESPC is inert without make-deadline structure — so "always
  on" is really "on where it does anything").
- Run the full corpus (`benchmarks/`, `examples/rpg-world/suite/`, `hard/`) under
  the new defaults and confirm: no coverage regressions, all plans still validate.

**Acceptance:** default-mode coverage ≥ current opt-in coverage on every suite;
every produced plan validates (in-crate + VAL where available); `FF_NO_*` recovers
the old byte-identical behavior; CHANGELOG documents the default change.

**Touches:** `features.rs`, `pddl3.rs`, `temporal.rs`, the tests in
`tests/tdemand.rs` / `tests/espc.rs` (flip set-var → assert-default + opt-out test),
`benchmarks/`, `CHANGELOG.md`, `README.md` (Limitations section).

---

## Phase 2 — Temporal depth: timed initial literals + duration inequalities

**Why:** the highest-credibility engine addition, and far more tractable than
continuous `#t` effects. TILs (`(at <t> (fact))` in the init) and duration
inequalities (`(<= ?duration N)` rather than `(= ?duration N)`) unlock a real slice
of the IPC temporal suite ferroplan currently can't even express. Pairs naturally
with attacking the decision-epoch search timeout, since both live in
`temporal.rs` / `tsched.rs` / `tresolve.rs`.

**Scope:**
- **Parser:** accept timed initial literals in `:init` and `<=`/`>=` duration
  constraints in `:durative-action` (`lexer.rs` / `parser.rs`).
- **Search:** TILs become scheduled exogenous events on the decision-epoch
  timeline; duration inequalities turn a fixed-duration commitment into a bounded
  choice the scheduler resolves (start with shortest-feasible, the FF default).
- **Validation:** extend the in-crate temporal validator (`temporal::validate`) to
  honor TILs and variable durations; keep VAL cross-checks green.
- **Search timeout (stretch):** profile the large temporal instances that time out
  today; cheapest win is likely tighter relevance pruning or making the Phase-3
  decomposer the default temporal path for over-budget instances.

**Acceptance:** parse + solve + validate a TIL domain and a duration-inequality
domain from the IPC temporal suite that fail today; no regression on the 44/45
already-valid temporal plans; new coverage numbers in
`benchmarks/temporal-results.md`.

**Touches:** `lexer.rs`, `parser.rs`, `temporal.rs`, `tsched.rs`, `tresolve.rs`,
`features.rs` (a `--temporal` capability surface), `benchmarks/temporal-results.md`,
README Limitations.

---

## Phase 3 — The bridge: the contract decomposer

**Why:** this is the thesis. `examples/BORDERS.md` is already a *measured, precise*
ruleset for when a single contract is solvable whole vs. must be split, and how to
split it (op-count ceiling ≈2000; converging-contributions ceiling; per-shape split
rules). So this is "encode a known spec," not "do research." The subproblem-maker is
currently a human reading a table — make it an actual tool.

**Scope:**
- A `decompose(domain, problem) -> Vec<Contract>` surface in the library: given a
  goal that exceeds the borders, emit a **sequenced** list of solvable sub-contracts
  (each a `(domain, sub-problem)` whose goal is within the borders), with the
  staging/ordering dependencies between them.
- Drive each contract through the engine to verify it actually solves; stitch the
  partial plans into one plan against the original goal and validate the whole.
- Encode the `BORDERS.md` rules as the splitting policy (op-count ceiling, converging
  ≥2-input joins → stage all-but-one input, jobshop-by-jobs-never-by-machine, etc.).
  `FF_TDECOMP` (the existing partition-and-resolve path) is the engine-level
  primitive this builds on; this phase adds the goal-level, cross-contract planner.
- CLI: `ff --decompose -o domain -f problem` prints the contract sequence + the
  stitched plan. Library: typed, serde-serializable `Contract` / `Decomposition`.

**Acceptance:** a goal from `BORDERS.md`'s "MUST SPLIT" column that one-shot search
fails on is solved end-to-end via the decomposer, with the stitched plan validating
against the original problem; the decomposition is deterministic and inspectable.

**Touches:** new `decompose.rs` module, `api.rs`, `partition.rs`/`resolve.rs`
(reuse), `ferroplan-cli`, `examples/BORDERS.md` (link the implementing module),
new `examples/` showcase, README ("make the thesis real").

---

## Phase 4 — The bridge, shipped: MCP server + publish

**Why:** the MCP server is the artifact that makes the whole bet *usable by an
agent* — `solve` / `validate` / `decompose` as tools an LLM calls in the
author→run→read→fix loop the `ferroplan` skill already describes. Packaging
(crates.io + PyPI) is the same "distribution" story, so ship them together as one
release: the engine is published *and* an agent can drive it the moment 0.2.0 lands.

**Scope:**
- **MCP server** (likely a new `crates/ferroplan-mcp` or a mode of the CLI) exposing
  `solve`, `validate`, `decompose`, and the feature table from the skill, with the
  structured JSON the library already returns.
- **crates.io:** publish `ferroplan` + `ferroplan-cli` (resolve the workspace
  version bump to 0.2.0, doc-test the public API, dependency audit).
- **PyPI:** publish the `ferroplan-py` wheel (maturin; it already builds standalone).
- Release mechanics per `RELEASING.md`; tag `v0.2.0`; CHANGELOG release notes that
  tell the "bridge is real" story.

**Acceptance:** `cargo install ferroplan` / `pip install ferroplan` work from a clean
machine; an agent can `decompose` then `solve` a too-big goal end-to-end through the
MCP server; `v0.2.0` tagged and released.

**Touches:** new `crates/ferroplan-mcp` (or CLI mode), `Cargo.toml` (version,
publish metadata), `crates/ferroplan-py`, `RELEASING.md`, `CHANGELOG.md`, README
(install + MCP quickstart).

---

## The 0.2.0 story

> v0.1 proved the engine. **v0.2 makes the bet real:** the proven heuristics are on
> by default, temporal coverage goes deeper, a goal too big for one-shot search is
> *automatically decomposed* into solvable contracts, and the whole thing is
> installable and drivable by an agent over MCP. The README's thesis — LLM as author
> and supervisor, PDDL as the auditable interface, a fast solver as the runtime —
> stops being an argument and becomes a tool you can `pip install`.
