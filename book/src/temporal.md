# Temporal planning

ferroplan supports **PDDL2.1 durative actions**. Temporal problems are
auto-detected (any `:durative-action` in the domain) and routed to a
decision-epoch forward search; the CLI prints the IPC temporal plan format.

## What's supported

- `:durative-action` with `at start`, `over all`, and `at end`
  **conditions** and **effects**.
- **Durations** that are constants *or* **parameter-dependent**, e.g.
  `:duration (= ?duration (/ (distance ?a ?b) (speed ?v)))` — evaluated per
  grounded action against the initial state (the static fluents temporal
  durations usually read).
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
and checking the invariant at both happenings.

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

See [`examples/rpg/`](https://github.com/haroldhhersey/ferroplan/tree/main/examples/rpg)
for a full gather → craft → build example; the same problem with `(= (workers) 1)`
vs `3` plans serially (makespan ~19) vs in parallel (~13). Plans are satisficing,
not makespan-optimal — a good plan fast, suited to an agent that plans, acts, and
replans as the world changes.

## Validation against VAL

Plans are validated with [VAL](https://github.com/KCL-Planning/VAL), the IPC plan
validator, on real IPC temporal domains (2002–2014). **44 of 45 produced plans
are VAL-valid** under PDDL2.1 continuous-time semantics — confirming the
snap-action compilation, `over all` invariants, required concurrency, and
ε-separation are correct. (Testing against VAL is what surfaced the ε-separation
requirement in the first place.) Coverage is currently **search-limited**: at a
short budget many instances time out or the decision-epoch search exhausts. See
[`benchmarks/temporal-results.md`](https://github.com/haroldhhersey/ferroplan/blob/main/benchmarks/temporal-results.md).

## Not yet supported

Duration **inequalities** (`(<= ?duration …)`), **timed initial literals**,
**continuous** (`#t`) / duration-dependent numeric effects, and ε-separation of
*conditional*-effect mutexes are not handled yet. Temporal **search performance**
(coverage on large instances) is the main open work item.
