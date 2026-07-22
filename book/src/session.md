# Game embedding (the `Session`)

`ferroplan::solve` re-parses and re-grounds on every call. A **`Session`**
is the embedding API for callers that re-solve the *same world* every
tick — a game's villagers, a simulation loop, an agent runtime. It
parses, compiles axioms, and grounds **once**, then holds the current
world state; every "think" afterward pays only the search.

Everything below is deterministic and thread-count independent: the same
session state, budget, and goal produce byte-identical plans at any
`threads` setting.

```rust,no_run
use ferroplan::{Options, Session};
# let (domain_src, problem_src) = (String::new(), String::new());
let mut s = Session::new(&domain_src, &problem_src, &Options::default())?;
let first = s.replan();                       // plan from the problem's :init
s.set_fact("(at vera field)", true)?;         // the world moved
s.set_fluent("(grain)", 3.0)?;
let next = s.replan();                        // replan from HERE
# Ok::<(), String>(())
```

Classical, numeric, and ADL domains are supported — and **temporal
(durative-action) domains** since 0.12: thinks return timed,
genuinely-concurrent plans with per-step `time`/`duration` and a
`makespan`.

## Bounded thinks

`replan_budgeted(max_evaluated, memory_mb)` is the real-time surface: an
**eval budget** (the deterministic unit — never wall clock) and a memory
target. A think either returns a plan or an honest budget-exhausted
`solved: false` the game can react to — think again later, escalate, or
pick a fallback behavior.

## Follow before you rethink

`plan_still_valid(&plan, from_step)` replays a plan's remaining suffix
against the current state — **no search, no budget**. An agent whose
world drifted irrelevantly keeps following its plan for free; only a
broken suffix warrants a real rethink. When one does break,
`replan_following(&plan, from_step, budget, memory_mb)` biases the
rethink toward the broken plan: it replays the still-applicable prefix
and searches only for a new tail, so plan **churn** stays confined to
what drift actually broke (measured: a deep break re-plans in 3 evals
with churn 1 where an unbiased rethink spends 2,899 evals churning 16
steps). If no tail exists, it falls back to an unbiased rethink — the
bias can cost budget, never completeness.

## One world, changing desires

`set_goal("(and (has a0 item5) …)")` retargets a session with **zero
regrounding** — any ground conjunction (atoms, negated atoms with
grounded mirrors, numeric comparisons) over the already-interned fact
space. Desires the world cannot express are an honest error, not a
silent unsolvable. `goal_met()` is the pure state test ("is it done
*now*") — distinct from a think, which answers "could I still plan."

## One grounding, a population of minds

`fork()` clones a mind over the **same grounded world**: the grounded
payload (operator tables, names, indexes) is shared behind `Arc`, so N
minds cost one grounding plus kilobytes of private state each — measured
on the vendored bazaar, 12 NPCs fork in ~0 ms with **+0 MB** RSS.
Forks start from the parent's current state and goal, then diverge
freely; no fork's writes or tie-breaks ever touch a sibling.

`restrict_ops(|display| …)` scopes a mind to **its own actions** — the
many-minds correctness primitive. A rival's moves reach it as
`set_fact` drift, never as plan steps, and loop-side policies (like
masking exchanges a rival's plan has claimed) compose on top of it.
The [`bazaar_live` example](https://github.com/hhh42/ferroplan/blob/main/crates/ferroplan/examples/bazaar_live.rs)
drives the whole tick loop — and the
[browser demo replays a real run](./demo/bazaar-live.html).

## The scheduled world

`set_timed_fact(dt, "(power)", false)` plants a **clock-relative**
event: in `dt` time units, the fact flips. Pending events ride into
every temporal think — plans beat closing windows or fail honestly, and
can *wait through* an outage whose repair is scheduled — and into
`plan_still_valid` replays. `elapse(dt)` advances the schedule as the
game's clock moves, firing due events. (Absolute-clock TILs stay
rejected at construction: session time is always relative to *now*.)

The domain contract: a scheduled fact must be **dynamic** — touched by
some action in the domain. A truly static fact is compiled into the
grounded operators, so flipping it at runtime could not soundly change
behavior; model exogenous-changeable facts (market-open, power) with an
action that touches them.

## In-flight intervals

`apply_start("(fire urn)")` begins a durative action **now**: its start
effects apply, and its end joins the session's in-flight set. Thinks
happen **mid-interval** — plans cover what remains, never restart the
running action, and are valid *through* every pending end (a think can
even be pure waiting: zero steps, makespan = the pending end's moment).
`elapse(dt)` fires due interval ends with their own at-end effects; an
end whose preconditions drift broke is reported, its effects dropped —
the game decides what a ruined firing means.

## Fences (why some writes are errors)

Honesty over silent wrongness, everywhere:

- **Static facts** are rejected by `set_fact`/`set_timed_fact`: grounding
  enumerates operators against statics, so flipping one could require
  operators that were never enumerated.
- **`RUNNING-*` tokens** (the temporal compiler's interval bookkeeping)
  are managed by `apply_start`/`elapse`, never by hand.
- **ADL goal connectives** in `set_goal` (or goals over never-interned
  atoms) error with the reason — recreate the session for an ADL goal.
- **PDDL3 constraints/preferences** and **absolute-clock TILs** are
  rejected at construction with pointers to the per-solve API.

`world_bytes()` / `mind_bytes()` report the shared-vs-private memory
split (flat-byte floors) for embedders budgeting a population.
