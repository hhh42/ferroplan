# ferroplan reference

Lookup detail for the `ff` CLI, the library, and PDDL feature support. The
`SKILL.md` file holds the *how-to-think*; this is the *how-to-look-up*.

## CLI flags (`ff --help`)

| flag | meaning |
|---|---|
| `-o, --domain <FILE>` | domain PDDL |
| `-f, --problem <FILE>` | problem PDDL |
| `--json-request <FILE>` | read a JSON job `{domain, problem, options}` (or `-` for stdin) |
| `--json` | emit a structured JSON solution instead of FF text |
| `--mode <MODE>` | `auto` (default) · `ff` · `partition` · `pddl3` · `temporal` |
| `--search <S>` | `auto` (default) · `ehc` · `best-first` · `ehc-then-best-first` |
| `--no-helpful` | disable helpful-action pruning (EHC) |
| `--weight-g <N>` | best-first path-length weight (default 1) |
| `--weight-h <N>` | best-first heuristic weight (default 5) |
| `--max-evaluated <N>` | cap on evaluated states |
| `--satisfice` | PDDL3: return a satisficing plan over hard goals (skip optimization) |
| `--threads <N>` | worker threads (0 = auto) |
| `--ipc` | IPC time-stamped plan format (classic text mode) |
| `--validate <FILE>` | replay a plan FILE under ferroplan's own semantics; prints `Plan valid` / `Plan invalid: <reason>`, exit 0/1 |

`--validate` auto-detects classical (`step N: NAME ARGS`) vs temporal (`t: (name args)
[dur]`) from the domain and reuses the engine's `apply`/`op_applicable`/`goal_met`
(via `verify::verify` / `temporal::validate`), so "valid" means "valid under the
semantics that produced it". It applies happenings sequentially and does **not**
impose VAL's strict PDDL2.1 concurrent-numeric mutex — so it accepts ferroplan's
resource-parallel temporal plans that VAL rejects. There is **no `--debug`** flag.
Library entry point: `ferroplan::plan::validate_plan(&dom_src, &prob_src, &plan_src)`.

## Output formats

- **Classic FF text** (default): `ff: found legal plan as follows` then `step N: ACTION ARGS`
  (upper-cased). Footer reports metric / "plan length assumed."
- **Temporal** (`--mode temporal`): IPC timed plan — `t: (action args) [duration]` per line,
  plus `plan makespan: N`.
- **JSON** (`--json`): a typed, `serde`-serializable solution (plan steps, metric, stats).

## Library

```rust
let solution = ferroplan::solve(&domain_str, &problem_str, &ferroplan::Options {
    mode:            Mode::Auto,        // Auto | Ff | Partition | Pddl3 | Temporal
    search:          Search::Auto,      // Auto | Ehc | BestFirst | EhcThenBestFirst
    helpful_actions: true,
    weight_g:        1.0,
    weight_h:        5.0,               // best-first key = 1·g + 5·h
    threads:         0,                 // 0 = auto
    max_evaluated:   None,
    optimize:        true,              // PDDL3: optimize metric vs satisfice
    ..Default::default()
})?;
if let Some(plan) = solution.plan { /* plan.steps, plan.metric, plan.makespan */ }
```

`solve` returns a typed `SolveError` on bad input (it never panics on malformed PDDL).

## Requirements / feature support

| requirement token | supported? | notes |
|---|---|---|
| `:strips`, `:typing` | ✅ | baseline |
| `:negative-preconditions`, `:disjunctive-preconditions` | ✅ | |
| `:equality` | ✅ | `(= ?a ?b)`; `(not (= ...))` works under `:negative-preconditions` |
| `:conditional-effects` | ✅ | enables `(when c e)` |
| `:universal-preconditions` / `:quantified-preconditions` | ✅ | `(forall ...)`, `(exists ...)` |
| `:adl` | ✅ | umbrella for the ADL set above; also the token to declare for `:derived` rule bodies |
| `:numeric-fluents` | ✅ | `(:functions ...)`, `>=`/`<`/… preconds, `increase`/`decrease`/`assign` |
| `:preferences` | ✅ | needs a `(:metric minimize (is-violated <name>))` to take effect |
| `:durative-actions` | ✅ | `at start`/`over all`/`at end`; constant or param-dependent duration — fixed `(= ?duration …)` **or** inequalities `(>= …)`/`(<= …)`/`and`-ranges (search commits to the shortest feasible) |
| `:timed-initial-literals` | ✅ | `(at <time> <literal>)` in `:init` (incl. `(not …)`); fires as an exogenous happening at its absolute time |
| `:derived-predicates` | ❌ **token rejected** | declare `:adl` instead; ferroplan still parses `(:derived …)` — see below |

## Derived predicates (`:derived`)

- Declare `:adl` (NOT `:derived-predicates`). Write rules as
  `(:derived (HEAD ?x - t) <body>)`.
- **Static / stratified only**: bodies must reference predicates that no action changes.
  ferroplan computes the closure once, folds it into `init`, then clears the rules.
- A rule body that reads an action-changed predicate → error
  ("only static derived predicates supported so far"). No numeric comparisons or
  preferences in rule bodies.

## Not yet supported

- Dynamic (action-changed) derived predicates.
- Temporal: continuous (`#t`) effects. (Duration inequalities and timed initial
  literals are now supported.)
- Coverage on the largest instances is search-limited — see `../../../examples/BORDERS.md`
  for where to decompose instead.
