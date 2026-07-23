<p align="center">
  <img src="https://raw.githubusercontent.com/hhh42/ferroplan/main/assets/logo.svg" alt="ferroplan" width="360">
</p>

# ferroplan

[![CI](https://github.com/hhh42/ferroplan/actions/workflows/ci.yml/badge.svg)](https://github.com/hhh42/ferroplan/actions/workflows/ci.yml)
[![docs](https://img.shields.io/badge/docs-mdbook-blue)](https://hhh42.github.io/ferroplan)
[![live demo](https://img.shields.io/badge/live_demo-try_in_browser-6c5ce7)](https://hhh42.github.io/ferroplan/demo/index.html)
[![license](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)

A fast, data-parallel **PDDL planner** in Rust — a deterministic planning core for
the age of AI.

**The bet:** an LLM should be the *author and supervisor* of a planner, not its
runtime. The same reason you don't ask a model to add a column of numbers — you have
it emit code that does the arithmetic deterministically, and for free — applies one
level up: don't ask an LLM to *be* the planner for a whole village of agents. Have it
author a PDDL domain that then plans deterministically, cheaply, and inspectably at
scale, and let it only *nudge* that domain at runtime. PDDL is the auditable interface
between your intent, the model's authoring, and a fast solver — and `ferroplan` is
that solver.

**Why PDDL, not prompt-spaghetti:**

- **Cost** — a solved domain plans essentially for free; an LLM call per decision per agent does not.
- **Determinism** — same problem, same plan; you can regression-test it.
- **Inspectability** — you can read a domain and an axiom; you cannot read a model's weights.
- **Scale** — a village of agents each replanning is tractable for a fast solver, not as a wall of LLM calls.

> **[▶ Try it live in your browser](https://hhh42.github.io/ferroplan/demo/index.html)** —
> pick a built-in example or paste your own PDDL; it plans entirely client-side via
> WebAssembly, no install. There's also a
> [browser visualizer + block editor](https://hhh42.github.io/ferroplan/gui/index.html).

`ferroplan` is a from-scratch reimplementation of the FF family of planners with a
data-oriented core (bitset states, structure-of-arrays / CSR operator tables),
**enforced hill-climbing** (EHC) with a best-first fallback, parallel grounding
and parallel heuristic evaluation, plus an SGPlan-style partition-and-resolve mode,
PDDL3 preference/metric optimization, and **PDDL2.1 temporal** planning (durative
actions). It ships both a **library** (with a structured, JSON-serializable API)
and the **`ff`** command-line binary — a drop-in for Metric-FF's
`ff -o domain -f problem`.

On classical and ADL benchmarks it runs within ~1.4× of the heavily-optimized C
Metric-FF (EHC reaches goals in dozens of evaluations, not thousands); numeric
trails and IPC-5 preference quality is competitive-not-winning — see
[Benchmarks](#benchmarks).

> Status: **v0.14.0** — `ferroplan` + `ferroplan-cli` are on [crates.io](https://crates.io/crates/ferroplan). APIs may shift before 1.0.

> **What's new in 0.14.0 — the living-bazaar cycle.** The population
> runs. The tick loop is driven end-to-end (`bazaar_live`): N
> actor-scoped minds (**`Session::restrict_ops`** — a mind plans only
> its OWN actions; rivals arrive as drift) in one authoritative world,
> byte-deterministic, with exact conflict attribution — and the
> measured lesson that naive pursuit in a contended one-way economy is
> mutually destructive, while loop-side CLAIMS (mask what a rival's
> plan still needs) drop conflicts to zero at ~18× less search.
> Worlds carry a SCHEDULE: **`set_timed_fact`** plants clock-relative
> events thinks must beat — or wait through — and **`elapse`** fires
> them. And the at-rest fence is gone: **`apply_start`** puts a
> durative action in flight, thinks happen MID-INTERVAL, and `elapse`
> fires interval ends itself, retiring the mirror-the-end-effects
> idiom — all with zero search-engine changes. Plus
> **`Session::goal_met()`**, the bazaar in the
> [browser demo](https://hhh42.github.io/ferroplan/demo/bazaar-live.html),
> and a refreshed classical scoreboard.
> The research extension closed a real temporal soundness gap —
> `over all` invariants are now enforced on EVERY happening, not just
> interval endpoints (a delete + re-add between them used to slip
> through and fail VAL; the kiln-gap fixture pins it) — and added
> **object-symmetry orbits** (`FF_NO_ORBIT=1` reverts):
> interchangeable objects and goal pairs collapse to one canonical
> visited state, breaking machine-shop's 8.7×10⁸-fold symmetry wall
> and rescuing turn-and-open under the sound semantics.
> `replan_following` learned the temporal path, and the
> ⌈demand/capacity⌉ landmark rung is recorded as the fourth precise
> guidance negative (`FF_RESLM` hatch). Full record:
> [`docs/roadmap-0.14.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.14.md),
> [`STATUS.md`](https://github.com/hhh42/ferroplan/blob/main/STATUS.md).

> **What's new in 0.13.0 — the many-minds cycle.** One world, a
> population of planners. **`Session::set_goal`** retargets a mind —
> any ground conjunction over the interned fact space, no regrounding,
> honest errors for desires the world cannot express (and it flushed
> out a latent mirror-sync bug in `set_fact`, now fixed).
> **`Session::fork`** makes minds cheap: the grounded payload shares
> behind `Arc`, so 12 bazaar NPCs cost one ~1.9 s grounding plus
> ~0.4 KB of private state each — forks diverge freely with no
> cross-mind interference and thread-count determinism intact.
> **`Session::replan_following`** keeps NPCs steady under drift:
> replay the broken plan's surviving prefix, search only the tail
> (measured: churn 1 at 3 evals where the unbiased rethink churns 16
> at 2,899). The game track gains its own scoreboard —
> [`benchmarks/bazaar-thinks.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/bazaar-thinks.md):
> solo trade chains are heuristic-transparent (11 hops, sub-ms,
> every tick), contended chains show honest budget-exhaustion curves.
> The temporal search gains agenda symmetry reduction (canonical
> pending-interval order + redundant-copy skip, `FF_NO_TSYMM=1`
> reverts). Full record:
> [`docs/roadmap-0.13.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.13.md),
> [`STATUS.md`](https://github.com/hhh42/ferroplan/blob/main/STATUS.md).

> **What's new in 0.12.0 — the game cycle.** `Session` learns TEMPORAL
> domains: ground a durative world once, then every think is a bounded
> call returning a timed, genuinely-concurrent plan from the current
> at-rest state — the temporal path gains a real eval budget spanning
> its whole pass ladder (it had only node caps), memory targets, and
> per-think duration rebuilds. **`plan_still_valid`** replays a plan's
> remaining suffix for free, so drifting worlds only spend a think when
> the plan actually breaks (the scripted fixture: exactly two thinks
> across follow / helpful-drift / breaking-drift). **Fixpoint
> grounding** — the `Session`'s grounding entry — enumerates from
> reached atoms instead of typed products: elevator-11 p04 grounds in
> 6.9 s at 48.8 MB where the stratified path spends 31.6 s at 5.7 GB
> (~117× less transient, identical task); the corpus solve paths keep
> stratified grounding, whose tie-breaks the scoreboard baselines pin.
> The vendored bazaar fixtures classify the game's any-for-any barter
> economy as dense-reachable: ground once (5.5 s), think forever. Plus
> self-relative quality scoring in the corpus runner and a precise
> diagnosis for the parc-printer-t plateau. Full record:
> [`docs/roadmap-0.12.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.12.md),
> [`STATUS.md`](https://github.com/hhh42/ferroplan/blob/main/STATUS.md).

> **What's new in 0.11.0 — the guidance cycle and the think API.** The
> game-embedding surface lands: **`Session::replan_budgeted`** makes a
> think a *bounded call* — an eval budget (deterministic, never wall
> clock) plus a retained-memory target — on a ground-once world, with
> an honest budget-exhausted verdict and thread-count-independent
> results (its new determinism test immediately caught and fixed a
> real EHC budget leak); `examples/game_think.rs` walks the episodic
> think → follow → drift → rethink loop. On the scoreboard side this
> was the honest cycle: three principled guidance transfers (a
> temporal LAMA rung, a lax helpful-set fallback, a classical
> landmark-count term) were built, measured at the baselines, and
> concluded **negative** — each ships opt-in with its diagnosis
> recorded, and the finding stands: the remaining walls need a
> genuinely different heuristic, not reweightings of what exists.
> Default-path behavior is unchanged, so the 0.10.0 scoreboards remain
> current. Full record:
> [`docs/roadmap-0.11.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.11.md),
> [`STATUS.md`](https://github.com/hhh42/ferroplan/blob/main/STATUS.md).

> **What's new in 0.10.0 — the walls fall where they can.** Three
> grounder truths land: **fact-space compaction** (elevator-08-t p22
> minted 2.35 M fact ids for ~7 k live facts — 287 KB per state, 8 GB
> RSS, dead at any budget; it now solves in ~26 s), **stratified END
> grounding** (snap ENDs enumerate only over bindings their STARTs
> produce), and **DNF static resolution** (the 2^k `forall (imply
> (static…) …))` conjunct explosion collapses — **openstacks-ADL
> 6/30 → 30/30, its temporal twins swept 30/30 + 30/30**). Temporal
> search gains **shift-invariant visited keys** (retimed permutations
> of one state finally dedup: sokoban-t +5, floor-tile-t +3,
> turn-and-open's first-ever solve, and a suite repro proving the
> decision-epoch scheme has **no required-concurrency semantics gap**),
> **PDDL2.1 `?duration` in expressions with state-dependent durations**,
> and a **byte-aware node cap**. The portfolio is now **budget-aware**
> (ladder to its natural end first — coverage ≥ default by
> construction), the corpus runner **VAL-validates temporal plans**
> (which caught and fixed a real same-instant numeric-mutex bug) and
> caps per-job memory, and every unmoved wall carries a measured
> diagnosis (storage/TMS/transport/model-train: guidance, not
> semantics, memory, or grounding). Full record:
> [`docs/roadmap-0.10.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.10.md),
> [`STATUS.md`](https://github.com/hhh42/ferroplan/blob/main/STATUS.md).

> **What's new in 0.9.0 — the IPC6/IPC7 arc opens.** ferroplan learns the
> IPC-2008/2011 satisficing objectives: **real action costs** (the metric
> is replayed, never estimated, and an anytime sweep trades length for
> cost — elevators08 p01 goes 100 → 54) and **net benefit /
> oversubscription** (`maximize` normalizes onto the minimize B&B; the
> empty plan is a legal candidate — the vendored subset reports the
> benefit on **16/16**). A **LAMA-style landmark rung** (first-achiever
> landmarks + preferred-operator boosting) runs bounded between EHC and
> the complete fallback on BOTH execution paths — barman11 solves for
> the first time (**0/4 → 4/4**) — and two grounder walls fell (a
> type-cycle hang on domains redeclaring `object`; join-style static
> pruning, 91.6 s → 2.8 s) taking tidybot11 **0/4 → 4/4**. The vendored
> costs subset goes **35/54 → 54/54** at a 240 s library budget, every
> solved plan externally VAL-validated where available. A **sequential
> portfolio mode** (`--mode portfolio`) time-slices four complementary
> configurations under one deterministic eval pool. Full record:
> [`docs/roadmap-0.9.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.9.md),
> [`STATUS.md`](https://github.com/hhh42/ferroplan/blob/main/STATUS.md).

> **What's new in 0.8.0 — Pay the Costs.** 0.7 enforced trajectory
> constraints and wrote down the bill; 0.8 pays it. Hard-monitor
> acceptance now rides one forced-terminal END action instead of an
> exponential goal-DNF product (the recorded storage hard fixture drops
> **59,969 grounded ops → 921**), and the monitor transition block —
> byte-identical across every ground action — grounds **once** instead of
> per op: both recorded 15 GB grounding OOMs are gone (storage
> qualitative p07 grounds in **313 ms at 109 MB**; p08 in 676 ms at
> 174 MB), and those two instances produce their **first-ever metrics
> (200 / 261), reported == verified exact**. ESPC now engages on real
> once-only achievement structure instead of monitor artifacts, so the
> storage tail runs on **pure defaults** — qualitative coverage rises
> from 36/40 to **38/40**, every remaining gap still named on the
> [scoreboard](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-qualitative-scoreboard.md). A
> deterministic search memory backstop (byte-model node cap, t1 ≡ t8 by
> construction) guards the wide-state passes. Every change keeps a
> restore hatch; constraint-free inputs are byte-identical
> ([0.8 roadmap](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.8.md)).

> **What's new in 0.7.0 — Trajectories: enforce the constraint, price the
> preference.** The oldest fence is retired: PDDL3 `(:constraints ...)`
> blocks — rejected-by-design since 0.4.1 — are now ENFORCED on the
> classical path. The six untimed modal operators compile into monitor
> automata riding every action; hard constraints become goal conjuncts,
> soft `(preference name ...)` constraints are priced through the existing
> metric stack with **zero optimizer changes**, and the independent
> verifier folds the ORIGINAL constraint semantics over every replay (now
> grounding quantified bodies — exact on 5 of 6 simple-preferences domains
> too). Measured proof: the IPC-5 *qualitative-preferences* track is
> vendored (5 domains × 8 instances) — **36/40 instances produce a plan
> and a metric** (reported == verified exact on all 11 oracle-checked
> instances), every gap has a named reason, and quadratic `forall`-constraints ground tractably via
> constraint-side static simplification
> ([qualitative scoreboard](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-qualitative-scoreboard.md),
> [0.7 roadmap](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.7.md)). Timed operators and the temporal
> path still reject by name; `FF_CONSTRAINTS_REJECT=1` restores the
> blanket rejection.

> **What's new in 0.6.0 — Selection: solve the choice, then plan to it.**
> Plan forensics ([the write-up](https://github.com/hhh42/ferroplan/blob/main/docs/forensics-tpp.md)) proved the remaining
> quality gap on preference domains was a *selection* problem, not a search
> problem — so ferroplan now solves the preference-subset choice **exactly**
> (`selection.rs`: mutex-variable end states, branch-and-bound, an admissible
> bound that can *prove* optimality) and plans to it as a target, and keeps
> init-satisfied "trap" preferences visible to the guidance. Results on pure
> defaults vs SGPlan5, the IPC-5 winner: **storage becomes an 8/8 domain
> sweep** (totals 234 vs 547), **tpp p06 an exact tie**, rovers' totals lead
> widens to 4862 vs 5632, and the suite tally reaches **19W/16T/13L**
> ([scoreboard](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md)) — three domains led under
> both quality conventions, deterministic at any thread count, every default
> change with a restore hatch, every dead end recorded
> ([0.6 roadmap](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.6.md)).

> **What's new in 0.5.0 — closing on first.** On the vendored IPC-5
> simple-preferences suite, **pure defaults** (one configuration, no env vars,
> deterministic at any thread count) now **lead SGPlan5 — the IPC-5 winner —
> under BOTH quality conventions on three of the six domains**: openstacks
> (wins p04–p08), storage (wins p01–p07), and rovers (wins p04/p06/p07/p08,
> exact ties p01/p05) — with trucks ahead on the domain total and a suite-wide
> instance tally of **19W/14T/15L** ([scoreboard](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md)).
> Under the hood: the ESPC penalty loop **graduated to a deterministically
> budgeted default**, both B&B loops gained **anytime in-sweep tightening + a
> diversified restart ladder** (which broke the storage/tpp plateaus), and
> folded numeric metrics **route through the exact-closure optimizer** (the
> rovers flip). Every default change keeps a restore hatch
> ([tuning reference](https://hhh42.github.io/ferroplan/tuning.html));
> negative results are recorded, not hidden (two seeding levers measured
> neutral, shipped opt-in). See the [CHANGELOG](https://github.com/hhh42/ferroplan/blob/main/CHANGELOG.md) and the executed
> [0.5 roadmap](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.5.md).

> **What's new in 0.4.0** — the PDDL3 preference-metric release, measured
> against the official IPC-5 winner on the vendored simple-preferences suite
> ([scoreboard](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md)): ferroplan now **leads SGPlan5
> on two of the six domains** — openstacks (opt-in `FF_ESPC=1` partitioned
> penalty loop: 19/23/17/**16/21/22/66/87** vs 13/16/12/26/36/33/67/123) and
> storage (plain defaults: **3/5/6/9/46**/145/200/263 vs 5/8/14/17/87/…, up
> from 2/8 coverage) — is **ahead on the trucks total**, and **ties SGPlan5 on
> every tpp and pathways p01–p04 instance**. Under the hood: an
> **exact-closure metric optimizer** (search real states, close the compiled
> preference bookkeeping with a provably-optimal phase tail), **static
> preference simplification** (storage's 62k-instance quadratic forall
> collapses ~97% at compile), barrier-free full-DNF guidance, and a
> **budget-escalating B&B** whose deterministic eval budget
> (`FF_PREF_EVAL_BUDGET`) is a real quality dial. Every change has a restore
> hatch. See the [CHANGELOG](https://github.com/hhh42/ferroplan/blob/main/CHANGELOG.md) for the full breakdown, including
> 0.3.0's temporal solver depth (65 → 73/75 corpus, default goal-relevance
> pruning, escalation ladder, `Session` API), the animator's transport bar and
> Gantt view, and the move to **Bevy 0.19**.

## Features

- **EHC + best-first** — enforced hill-climbing with helpful actions (the FF
  speed default), falling back to weighted best-first when it stalls. Selectable
  per solve (`--search auto|ehc|best-first|…`).
- **FF heuristic** — delete-relaxation relaxed-plan heuristic over a
  data-oriented task, deferred evaluation, tunable `g`/`h` weights.
- **Data parallelism** — parallel grounding and parallel batch heuristic
  evaluation (`std::thread`); the plan found is identical for any thread count.
- **PDDL coverage** — STRIPS, typing, negative/disjunctive preconditions,
  numeric fluents (Metric-FF style), **ADL** (conditional effects,
  `forall`/`exists`, equality), and **derived predicates / axioms** (`:derived`,
  static/stratified — closed into the initial state via a datalog fixpoint).
- **PDDL3 preferences** — soft goal preferences (incl. `forall`-quantified and
  precondition preferences) compiled away, with anytime branch-and-bound metric
  optimization. *(Exact-optimal on small/medium instances; best-found, flagged,
  on the largest — see [Limitations](#limitations).)*
- **PDDL2.1 temporal** — `:durative-action`s with `at start`/`over all`/`at end`
  conditions & effects, **constant or parameter-dependent durations**, and
  required concurrency, via a decision-epoch forward search; output in the IPC
  temporal plan format (`t: (action) [dur]`) with a makespan.
- **SGPlan-style partitioning** — an optional partition-and-resolve mode.
- **Robust** — a published library shouldn't crash: malformed/pathological PDDL
  (incl. deeply-nested forms) returns a typed error, never a panic.
- **Structured output** — the library returns typed, `serde`-serializable
  results; the CLI emits classic FF text **or** JSON.

## GUI

[`ferroplan-bevy`](https://github.com/hhh42/ferroplan/tree/main/crates/ferroplan-bevy) is a Bevy app that visualizes a
domain+problem as a typed graph, animates the plan, and edits both problems and
domains in a Blockly-style block editor (`cargo run -p ferroplan-bevy`).

![ferroplan-bevy visualizing a delivery problem as a typed graph](https://raw.githubusercontent.com/hhh42/ferroplan/main/book/src/images/graph.png)

## Install / build

```sh
# install the `ff` CLI from crates.io
cargo install ferroplan-cli    # puts `ff` on your PATH

# …or build from a clone
cargo build --release          # produces target/release/ff
cargo run --release --bin ff -- -o domain.pddl -f problem.pddl
```

As a library dependency: `cargo add ferroplan` (see [Library](#library) below).

## CLI (`ff`)

```sh
# drop-in: classic Metric-FF text output
ff -o domain.pddl -f problem.pddl

# structured JSON solution
ff -o domain.pddl -f problem.pddl --json

# pick a mode / search strategy
ff -o domain.pddl -f problem.pddl --mode partition
ff -o domain.pddl -f problem.pddl --search best-first --weight-h 3

# temporal (durative actions) — auto-detected; prints the IPC temporal plan
ff -o temporal-domain.pddl -f problem.pddl --mode temporal

# decompose a too-big temporal goal into ordered, individually-solved contracts
# (the "LLM authors, planner decomposes" bet, made inspectable — text or --json)
ff -o temporal-domain.pddl -f problem.pddl --mode temporal --decompose

# self-contained JSON job: {"domain": "...", "problem": "...", "options": {...}}
ff --json-request job.json
```

Run `ff --help` for all flags (`--search`, `--weight-g/--weight-h`,
`--max-evaluated`, `--satisfice`, `--threads`, …).

## Library

```rust
use ferroplan::{solve, Options};

let domain  = std::fs::read_to_string("domain.pddl")?;
let problem = std::fs::read_to_string("problem.pddl")?;

// Syntax-check before solving (no grounding/solving) — fast authoring feedback.
let report = ferroplan::parse(&domain);
assert!(report.ok, "{:?}", report.error);

let solution = ferroplan::solve(&domain, &problem, &Options::default())?;
if let Some(plan) = solution.plan {
    for step in &plan.steps {
        println!("{} {}", step.action, step.args.join(" "));
    }
    println!("metric: {:?}", plan.metric);
}
# Ok::<(), ferroplan::SolveError>(())
```

The public, `serde`-serializable surface: **`solve`** (plan a domain+problem),
**`decompose`** (split a too-big temporal goal into validated contracts),
**`parse`** (syntax-check + summarize PDDL without solving),
**`Session`** (ground once, replan many — for a live loop that re-solves the same
world every tick), and **`plan::validate_plan`** (independently check a plan). See
[`examples/`](https://github.com/hhh42/ferroplan/tree/main/crates/ferroplan/examples) for `solve`, `parse`, `json_api`, and
`replan` (`Session` vs. re-solving from scratch, with timings).

## Configuration

Every solver knob lives on one `Options` struct (library-first, `serde`-
serializable). The CLI flags and JSON job options map to the same fields; omitted
JSON fields fall back to the defaults shown.

```rust
ferroplan::solve(&domain, &problem, &ferroplan::Options {
    mode:            Mode::Auto,        // auto | ff | partition | pddl3 | temporal
    search:          Search::Auto,      // auto | ehc | best-first | ehc-then-best-first
    helpful_actions: true,              // helpful-action pruning (EHC)
    weight_g:        1.0,               // best-first path-length weight
    weight_h:        5.0,               // best-first heuristic weight  (1·g + 5·h)
    threads:         0,                 // 0 = auto
    max_evaluated:   None,              // search node cap
    optimize:        true,              // PDDL3: optimize metric vs. satisfice
    ..Default::default()                // every field is optional
})?;
```

CLI equivalents: `--mode`, `--search`, `--no-helpful`, `--weight-g/--weight-h`,
`--max-evaluated`, `--satisfice`, `--threads`. Via JSON:
`{"domain": "...", "problem": "...", "options": {"search": "best-first"}}`.

## Workspace layout

| crate | what |
|---|---|
| [`ferroplan`](https://github.com/hhh42/ferroplan/tree/main/crates/ferroplan) | the library: engine + modes + `solve` / `decompose` / `Session` API |
| [`ferroplan-cli`](https://github.com/hhh42/ferroplan/tree/main/crates/ferroplan-cli) | the `ff` binary (clap + JSON) |
| [`ferroplan-mcp`](https://github.com/hhh42/ferroplan/tree/main/crates/ferroplan-mcp) | an MCP server exposing `solve` / `validate` / `decompose` over stdio — so an LLM agent can author PDDL and drive the planner |
| [`ferroplan-bevy`](https://github.com/hhh42/ferroplan/tree/main/crates/ferroplan-bevy) | Bevy app: visualize, inspect & animate a domain+problem (`cargo run -p ferroplan-bevy [domain.pddl problem.pddl]`) |
| [`ferroplan-wasm`](https://github.com/hhh42/ferroplan/tree/main/crates/ferroplan-wasm) | WebAssembly binding behind the client-side [browser demo](https://hhh42.github.io/ferroplan/demo/index.html) — `solve` a domain+problem entirely in-page |
| [`ferroplan-py`](https://github.com/hhh42/ferroplan/tree/main/crates/ferroplan-py) | Python binding (`pip`-installable extension module) exposing `solve` for embedding in Python tools |

## Examples

[`examples/`](https://github.com/hhh42/ferroplan/tree/main/examples) collects worked domains that exercise the full feature set
— see the [examples index](https://github.com/hhh42/ferroplan/blob/main/examples/README.md) for a feature-by-feature map and a
suggested reading order. Highlights:

- [`rpg`](https://github.com/hhh42/ferroplan/tree/main/examples/rpg) — the clean intro: durative actions with renewable
  (workers) and consumable resources, gather → craft → build.
- [`rpg-world`](https://github.com/hhh42/ferroplan/tree/main/examples/rpg-world) — a ~120-action crafting/economy domain
  (durative actions, numeric resources, renewable capacities, a reachability
  axiom) with a corpus of validated contracts, a flavor-×-scale [`suite/`](https://github.com/hhh42/ferroplan/tree/main/examples/rpg-world/suite),
  an adversarial [`hard/`](https://github.com/hhh42/ferroplan/tree/main/examples/rpg-world/hard) batch, and an
  [industrial-city](https://github.com/hhh42/ferroplan/tree/main/examples/rpg-world/industrial-city) showcase that runs a whole
  metal/stone/wood industry as a pipeline of contracts.
- [`cabin`](https://github.com/hhh42/ferroplan/tree/main/examples/cabin) — deep numeric build plus a durative "crew" twin
  (makespan vs. crew size, skill-gated scheduling).
- [`reachability`](https://github.com/hhh42/ferroplan/tree/main/examples/reachability) — the worked **derived-axiom**
  (`:derived`) example: static transitive-closure reachability.
- [`village`](https://github.com/hhh42/ferroplan/tree/main/examples/village) — a full-ADL stress test (`when`, `forall`+`when`,
  `or`, negation) over durative + numeric state.
- [`villagers`](https://github.com/hhh42/ferroplan/tree/main/examples/villagers) — a data-driven recipe planner with numeric
  **PDDL3 metric** optimization; the "embed in a game" model.
- [`logistics`](https://github.com/hhh42/ferroplan/tree/main/examples/logistics) — transshipment: per-location goods, trucks
  with capacity, a train line.
- [`jobshop`](https://github.com/hhh42/ferroplan/tree/main/examples/jobshop) — scheduling with machine-exclusion (scales to 100
  concurrent jobs).
- [`BORDERS.md`](https://github.com/hhh42/ferroplan/blob/main/examples/BORDERS.md) — a measured map of where one-shot planning
  solves vs. where a goal must be decomposed into contracts. The **`decompose` API /
  `ff --decompose`** acts on that border: it splits a too-big temporal goal into
  ordered, individually-solved contracts and stitches them into one validated plan
  (e.g. `hard/order-8` → 8 named contracts), falling back to a monolithic solve when
  a goal can't be split.

## Benchmarks

Classical and ADL coverage/speed are measured against the C **Metric-FF**; the
IPC-5 preference quality is measured against **SGPlan5** (the IPC-5 winner), over
a subset of the IPC contest suites. Headline (native Metric-FF, EHC default):

| category | ferroplan solved | speed vs Metric-FF |
|---|---:|---|
| STRIPS | 40/40 | 0.71× (~1.4× slower) |
| ADL | 23/24 | 0.77× (~1.3× slower) |
| numeric | 36/40 | 0.22× |

The IPC-5 preference scoreboard (vs SGPlan5):
[`benchmarks/ipc5-scoreboard.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md); classical/numeric
detail: [`benchmarks/results.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/results.md) (and the
[project site](https://hhh42.github.io/ferroplan)). The oracles are not bundled
(GPL / non-commercial licences) — reproduce per
[`benchmarks/COMPARING.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/COMPARING.md).

**Profiling & perf tracking:** [`PROFILING.md`](https://github.com/hhh42/ferroplan/blob/main/PROFILING.md) — a deterministic
metrics harness (`benchmarks/perf.py run`/`compare` against a committed baseline,
so improvement/regression is measurable across machines) plus the samply /
flamegraph / criterion-baseline workflow for finding and tracking hotspots.

## Limitations

- **Numeric** trails Metric-FF: EHC's helpful-action lookahead stalls on some
  numeric domains and falls back to (complete, slower) best-first.
- **IPC-5 preferences**: compiled away, then optimized by an **exact-closure
  metric optimizer** with anytime sweeps, a diversified restart ladder, and the
  deterministically-budgeted ESPC penalty loop — all defaults. Coverage is
  **full (48/48)** on the vendored simple-preferences suite and ferroplan
  **leads SGPlan5 under both quality conventions on three of the six domains**
  (openstacks, storage, rovers), with trucks ahead on totals — see the
  [scoreboard](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-scoreboard.md). The tpp/pathways p05–p08 tails
  still trail (best-found, flagged *not proven optimal*, measured
  direction-bound); the design record for the remaining work is in
  [`docs/espc-preferences-spec.md`](https://github.com/hhh42/ferroplan/blob/main/docs/espc-preferences-spec.md) and
  [`docs/roadmap-0.5.md`](https://github.com/hhh42/ferroplan/blob/main/docs/roadmap-0.5.md).
- **PDDL3 trajectory constraints** (`(:constraints ...)`): the six untimed
  modal operators (`always`, `sometime`, `at-most-once`, `sometime-after`,
  `sometime-before`, `at end`) are **enforced on the classical path** — compiled
  into monitor automata and cross-checked by the independent verifier. Hard
  constraints latch a forced-terminal END action (linear in monitors — the
  0.8 construction; goal-side compilation via `FF_NO_TRAJ_END=1`); soft
  `(preference name ...)` constraints are **priced through the PDDL3 metric
  machinery** like native goal preferences (the IPC-5
  *qualitative-preferences* suite is vendored and scored at **38/40** — see
  [`benchmarks/ipc5-qualitative-scoreboard.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc5-qualitative-scoreboard.md)).
  The timed operators (`within`, `hold-during`, `hold-after`,
  `always-within`) and the temporal path are still **rejected by name**
  rather than silently dropped (`FF_CONSTRAINTS_REJECT=1` restores the
  pre-0.7 blanket rejection).
- **Temporal**: durative actions with constant, parameter-dependent, or
  state-dependent durations and required concurrency are supported, and **every
  solved plan on the full IPC-2008/2011 tempo-sat corpus is VAL-validated**
  (403/630 at 30 s, 403/403 valid — see
  [`benchmarks/ipc67-temporal.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc67-temporal.md)). Coverage on
  the remainder is search-limited: the recorded walls (machine-shop, storage,
  model-train) are guidance problems, not semantics — and since the 0.14
  extension, `over all` invariants are enforced on every happening, with
  object-symmetry orbits collapsing interchangeable-object state blowups
  (match-cellar 10/20 → 20/20, turn-and-open off zero). Duration
  *inequalities* (`(>= ?duration L)` / `(<= ?duration U)` / `and` ranges) are
  supported — the search commits to the shortest feasible duration — as are
  **timed initial literals** (`(at <time> <literal>)` in `:init`) and
  `?duration` inside numeric effect expressions (duration-dependent effects).
  Continuous (`#t`) effects are not yet supported.
- **Derived predicates** (`:derived`): static/stratified axioms are supported
  (closed into the initial state); *dynamic* derived predicates (bodies over
  changing facts) are not yet.

## Acknowledgments

This project is built in deep respect for the planners that came before it.

**SGPlan** (SGPlan5 / SGPlan6), by Chih-Wei Hsu and Benjamin W. Wah at the
University of Illinois, has been the standard to beat in this corner of automated
planning for the better part of two decades — the IPC-winning system whose
constraint-partitioning and extended-saddle-point penalty-coordination ideas still
define the state of the art for satisficing planning with preferences and with
temporal/resource constraints. I've followed that line of research for many years,
and to build something that even comes *close* to it on a slice of the benchmarks
is, genuinely, an honor. Enormous credit to that team for the depth, rigor, and
sheer durability of the work — ferroplan is in no small part an attempt to learn
from it, in Rust.

Equal thanks to Jörg Hoffmann's **FF / Metric-FF**, whose relaxed-plan heuristic
and enforced hill-climbing are the backbone of this engine; to the IPC organizers
and domain authors whose benchmarks make progress measurable; and to Derek Long and
Maria Fox's **VAL**, used here to independently validate the temporal plans.

## License

Dual-licensed under either of [MIT](https://github.com/hhh42/ferroplan/blob/main/LICENSE-MIT) or [Apache-2.0](https://github.com/hhh42/ferroplan/blob/main/LICENSE-APACHE),
at your option.
