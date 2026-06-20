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

## Not yet supported

Duration **inequalities** (`(<= ?duration …)`), **timed initial literals**, and
**continuous** (`#t`) numeric effects are not handled yet.
