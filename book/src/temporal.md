# Temporal planning

ferroplan supports **PDDL2.1 durative actions**. Temporal problems are
auto-detected (any `:durative-action` in the domain) and routed to a
decision-epoch forward search; the CLI prints the IPC temporal plan format.

## What's supported

- `:durative-action` with `at start`, `over all`, and `at end`
  **conditions** and **effects**.
- **Durations** that are constants *or* **parameter-dependent**, e.g.
  `:duration (= ?duration (/ (distance ?a ?b) (speed ?v)))` — evaluated per
  grounded action against the initial state, or **per expansion against the
  current state** when the duration reads a fluent some action modifies
  (state-dependent durations, since 0.12). `?duration` is also accepted
  inside numeric *effect* expressions (duration-dependent effects like
  `(increase (energy ?x) (* ?duration (recharge-rate ?x)))`, since 0.10).
- **Duration inequalities** — `(>= ?duration L)` / `(<= ?duration U)` and `and`
  ranges; the search commits to the shortest feasible duration.
- **Timed initial literals** — `(at <time> <literal>)` in `:init`; each becomes a
  synthetic exogenous applier fired from a pre-seeded agenda at its time (so a goal
  reachable only via a TIL is not pruned as a dead end).
- **Required concurrency** — actions whose intervals must overlap (the classic
  "match / mend-fuse": the fuse can only be mended while the match is lit).

## How it works

Each durative action is compiled into two instantaneous **snap-actions** so the
existing grounder and relaxed-plan heuristic can be reused:

- `A-START` takes the `at start` condition (plus the `over all` invariant) and
  applies the `at start` effects plus a `(RUNNING-A …)` token;
- `A-END` requires the `at end` condition, the invariant, and that token; it
  applies the `at end` effects and drops the token.

The duration and the `over all` invariant live in a side table the temporal
search consumes: a decision-epoch search advances time over an agenda of pending
end-events, only letting `A-END` fire `duration` after its matching `A-START`,
and checking the invariant at both happenings. Since 0.13 the pending-interval
agenda is **symmetry-reduced** (canonical ordering + redundant identical-interval
elimination, `FF_NO_TSYMM=1` reverts) — same-epoch starts of interchangeable
intervals no longer multiply the visited space.

## Output

Plans are rendered in the IPC temporal format, `start: (action args) [duration]`,
with the overall **makespan**:

```
0.000: (fly plane1 city-a city-b) [3.000]
3.000: (fly plane1 city-b city-c) [4.000]
```

From the library, temporal solutions carry `time` on each `Step` and a
`makespan` on the `Plan`.

## Usage

```sh
ff -o temporal-domain.pddl -f problem.pddl            # auto-detected
ff -o temporal-domain.pddl -f problem.pddl --mode temporal --json
```

## Resource scheduling (renewable + consumable)

Durative actions over numeric fluents give you **resource allocation over time**
for free — the case that matters for scheduling crews, machines, tools, power, or
mana. Model a **renewable** resource as a pool that is taken at start and returned
at end, guarded by an at-start check:

```pddl
(:functions (workers))
(:durative-action chop-tree
  :duration (= ?duration 3)
  :condition (at start (>= (workers) 1))
  :effect (and (at start (decrease (workers) 1))     ; held over the interval…
               (at end   (increase (workers) 1))     ; …released at the end
               (at end   (increase (wood) 1))))
```

Because the decrement persists until the matching end fires, the decision-epoch
search holds the resource across the whole `[start, end]` interval: a pool of 1
forces tasks to serialize, a larger pool lets them overlap. **Consumable**
resources (materials) are the same idea without the release — produced and
consumed by a crafting chain (`wood → planks`, `planks + stone → house`).

See [`examples/rpg/`](https://github.com/hhh42/ferroplan/tree/main/examples/rpg)
for a full gather → craft → build example; the same problem with `(= (workers) 1)`
vs `3` plans serially (makespan ~19) vs in parallel (~13). Plans are satisficing,
not makespan-optimal — a good plan fast, suited to an agent that plans, acts, and
replans as the world changes.

## Validation against VAL

Plans are validated with [VAL](https://github.com/KCL-Planning/VAL), the IPC plan
validator. On the full IPC-2008/2011 tempo-sat corpus (630 instances, 30 s each),
ferroplan solves **388 — and every one of the 388 plans is VAL-valid** under
PDDL2.1 continuous-time semantics, confirming the snap-action compilation,
`over all` invariants, required concurrency, and ε-separation are correct.
(Testing against VAL is what surfaced the ε-separation requirement in the first
place; since 0.10 the pass totally ε-orders execution, so same-instant mutexes —
conditional-effect ones included — are impossible by construction.) Coverage on
the unsolved remainder is **search-limited**: the recorded walls are guidance
problems, not semantics. See
[`benchmarks/ipc67-temporal.md`](https://github.com/hhh42/ferroplan/blob/main/benchmarks/ipc67-temporal.md).

## Not yet supported

**Continuous** (`#t`) effects are not handled (discrete duration-dependent
effects via `?duration` are — see above). PDDL3 trajectory constraints
(`(:constraints …)`) are enforced on the *classical* path (untimed operators,
since 0.7) but not on the temporal path — a durative-action domain that
declares them is rejected rather than silently ignored. Temporal **search
guidance** on the recorded wall domains (machine-shop, storage, model-train,
turn-and-open) is the main open work item.
