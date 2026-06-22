---
name: ferroplan
description: >-
  Author, debug, and run PDDL for the ferroplan planner. Use when writing or fixing
  PDDL domains/problems for ferroplan — choosing which feature (STRIPS, typing, ADL,
  numeric fluents, derived axioms, PDDL3 preferences, durative/temporal) fits,
  invoking the `ff` CLI or the library, interpreting plans, or decomposing a large
  goal into solvable contracts.
---

# Authoring PDDL for ferroplan

ferroplan is a fast PDDL planner (a from-scratch FF-family reimplementation in Rust).
The discipline this skill enforces: **author a domain, run it, and confirm a real
plan comes out — never present PDDL you have not run.** Models are fluent in Python
but shaky in PDDL; the cure is the loop below plus the verified examples in
`examples/`.

## Run it

```sh
ff -o domain.pddl -f problem.pddl                  # classic FF text output (default `auto`)
ff -o domain.pddl -f problem.pddl --json           # structured JSON solution
ff -o domain.pddl -f problem.pddl --mode temporal  # durative actions -> IPC timed plan
ff -o domain.pddl -f problem.pddl --mode pddl3     # optimize soft preferences
```

Build once: `cargo build --release -p ferroplan-cli` → `target/release/ff`.
Modes: `auto` (routes by problem features — the default) · `ff` (classical/numeric) ·
`pddl3` (preferences/metric) · `temporal` (durative) · `partition` (SGPlan-style).
**There is no `--validate` or `--debug` flag** — "did it work" = a plan printed.
Other flags: `--search`, `--weight-g/--weight-h`, `--max-evaluated`, `--satisfice`,
`--threads`, `--ipc` (see `reference.md`). Library: `ferroplan::solve(&domain,
&problem, &Options)`.

## The loop (every time)

1. **Pick the smallest feature set** that models the situation (table below).
2. **Write** the domain + problem.
3. **Run** `ff` and **read the output**: a plan (`step N:` or `t: (a) [d]`) = valid;
   a parse error, "goal can be simplified to FALSE", or "no temporal plan" = fix it.
4. On failure, **localize** (parse line, unreachable goal, missing requirement) and
   iterate. Do not hand over PDDL you have not run.
5. For independent trust, validate the produced plan with **VAL** if available.

## Which feature, when

| feature | add to `:requirements` | reach for it when | example |
|---|---|---|---|
| STRIPS + typing | `:strips :typing` | discrete boolean state, no time/numbers | `examples/strips-typing/` |
| numeric fluents | `:numeric-fluents` | a consumable/continuous quantity gates actions (HP, gold, stock) | `examples/numeric/` |
| ADL | `:conditional-effects :universal-preconditions :negative-preconditions` | one action's effect depends on per-object state; collapse action fan-out | `examples/adl/` |
| derived axioms | `:adl` (**not** `:derived-predicates`) | a fact is a *consequence* of static state (reachability, connectivity) | `examples/axioms/` |
| PDDL3 preferences | `:preferences` **+** a `(:metric minimize (is-violated p))` | a goal is *preferred*, not required | `examples/preferences/` |
| temporal / durative | `:durative-actions` (run `--mode temporal`) | actions take time and must overlap; over-all invariants | `examples/temporal/` |

Each `examples/<feature>/` is a minimal domain+problem **verified to solve** against
this `ff`. Read the matching one before authoring that feature. For rich, integrated
domains see the repo `examples/` (rpg-world, logistics, jobshop).

## Engine gotchas (verified against this `ff`)

- **Output is upper-cased** (`PICK R1 B1`) though PDDL is case-insensitive — match plan
  steps case-insensitively when parsing.
- **0-arity fluents are written `(fuel)`**, declared under `(:functions ...)`; requires
  `:numeric-fluents`.
- **No `(:metric ...)` → "plan length assumed"**: the numeric value is not optimized and
  any `(preference ...)` is *ignored*. To honor a preference add
  `(:metric minimize (is-violated <name>))` with a name matching the `(preference <name> ...)`.
- **`(when ...)` without `:conditional-effects` is a parse error** (not silently dropped).
  Likewise declare `:universal-preconditions` for `(forall ...)` and
  `:negative-preconditions` for `(not ...)`.
- **Axioms are static-only**: ferroplan rejects the `:derived-predicates` requirement
  token; it computes the derived closure once into `init`, then clears the rules. A
  `:derived` body that reads any action-changed predicate errors ("only static derived
  predicates supported"). No numeric comparisons or preferences in rule bodies.
- **Temporal**: use `--mode temporal`; durations via `(= ?duration N)`; simultaneous
  starts are ε-separated (a start at `3.001` keeps its `over all` condition strictly
  inside the interval).
- **Empty types are tolerated** — a type with no objects grounds to zero instances
  (lets a problem use a subset of the domain).

## Using it properly at scale — decomposition

ferroplan solves one problem in one shot up to a coverage border; past it, a goal must
be **decomposed into contracts** that share a stockpile. The measured rule (see
`../../../examples/BORDERS.md`):

> The delete-relaxed heuristic keeps a gradient on linear/accumulative work and goes
> **flat the instant ≥2 contributions must converge onto one goal quantity.**

Hand a single contract whole if it is one accumulating chain (≤~2000 ops) **or** every
goal quantity in it receives ≤1 converging contribution. **Split** when a converging
recipe needs ≥2 fresh sub-chains (stage all-but-one input first), a depth≥2 chain is
conjoined with a sibling (one deliverable per contract), or you exceed the op budget.
`../../../examples/rpg-world/industrial-city/` is a worked whole-city decomposition;
`../../../examples/BORDERS.md` is the full ruleset.
