# The re-entrant engine — a product track beside the 0.31 cycle

*Drafted 2026-10-31 on the operator's ask: "run it, get a partial result
back, recall it with a tick, from a game engine." Three tiers, each a
shippable step, each measured in milliseconds per call and never in
board rows. The board cycle (`roadmap-0.31.md`) does not wait on this and
this does not claim rows from it.*

## What already exists (read 2026-10-31, `session.rs`, `search.rs`, `optimal.rs`)

- **`Session`** is the world model: `apply_start`, `observe`, `elapse`,
  `set_fact` / `set_fluent` / `set_goal`, `fork`, `plan_still_valid`,
  `restrict_ops`. A game already has the loop it needs around it.
- **`Session::think(&ThinkBudget) -> Think`** is a budgeted one-shot:
  `max_evaluated` (the deterministic unit), `wall_ms`, `memory_mb`; the
  verdict is typed `Solved | Capped | Exhausted` and a capped think never
  reads as unsolvable. `spent_ms` / `spent_evals` ride on the report.
- **`CallBudget`** (thread-local in `search.rs`) carries a deadline and a
  cancel `AtomicBool`; every rung polls it. Cancellation from another
  thread is already a one-line affair.
- **Sprint-resume** in `optimal.rs` keeps the h^max sprint's A* state
  across the handover to LM-cut — the one place the engine already
  suspends a search and continues it. The resumable tier generalises
  exactly this.
- The three satisficing loops own their state as function locals:
  `search_from` (two heaps, `expanded`, the visited hash → arena index
  table, the node arena, `Scratch`, the memory wall, timers, the orbit
  key closure); `lama::search_cfg` (preferred + normal heaps, `pop_lazy`,
  the landmark tables); `novelty::search` / `search_driver` (their heaps
  and novelty tables); EHC (`bfs_improve`'s local queue, the committed
  prefix). Nothing is shared between rungs except through return values.

## Tier 1 — the actor (days; no search change)

```rust
let h: ThinkHandle = session.spawn_think(budget);   // worker thread
match h.poll() {                                     // from the game tick
    Pending(Progress { evals, elapsed_ms, rung, best_h, prefix }) => …,
    Done(Think { .. }) => …,
}
h.cancel();                                          // CallBudget's flag
```

`Progress` is a snapshot the search writes once per batch into a shared
cell (an `AtomicUsize` for evals, a small `Mutex<Snapshot>` for the rest);
`prefix` is EHC's committed plan prefix when the ladder is in EHC, empty
otherwise — the honest "partial result" today. `Session` is `fork`ed
into the worker so the game keeps mutating its own; `plan_still_valid`
reconciles when the think returns. Byte-identical results to `think`
(same code path, same budget); the only new fixture is the snapshot's
consistency under cancel.

**Measure:** ms from `spawn_think` to the first `Progress`; ms from
`cancel` to the thread's return (the kill: > one batch).

## Tier 2 — the resumable `Thinker` (weeks; the engine rework)

```rust
let mut t = Thinker::new(&session, cfg);      // owns the ladder position + rung state
loop {
    match t.step(StepBudget { evals: 2_000, wall_ms: Some(4) }) {
        Step::Progress(p) => { /* tick */ }
        Step::Solved(plan) => break,
        Step::Exhausted => break,
    }
}
```

The locals of each rung move into an enum of rung states —
`Bfs { heap, pref_heap, expanded, visited, arena, scratch }`, `Lama { .. }`,
`Novelty { .. }`, `Ehc { current, prefix, improve_queue }` — and the
ladder's "which rung, how much of the wall is left" becomes a field. Each
rung's loop body becomes a `fn step_n(&mut self, n)`; the existing
functions become `Thinker::new(..).run_to_end()`, so the CLI, the boards
and the whole test suite stay **byte-identical** (that is the hatch and
the referee: `cargo test --all --release` unchanged, `compare` on a
subset sweep flat).

The parts that resist: the memory wall is armed per call (it becomes per
`Thinker`); `CallBudget` is per call (it becomes per step, re-armed on
entry); the orbit key closure borrows the task (the `Thinker` borrows or
`Arc`s the `PackedTask`); the ladder's slice fractions are wall-clock
and must be accounted across steps (evals as the deterministic currency,
wall as the cap). Determinism: a step budget in evals gives identical
expansion order to the one-shot; a wall-only step does not, and the API
says so.

What it buys the boards, honestly: a rung could *resume* after another
rung's slice instead of restarting. The matrix simulation priced that at
+7–12 cells on 936 — a receipt, not a lane. The product is the reason.

**Measure:** ms per `step` at a fixed eval budget on a six-cell panel (one
per board family); the one-shot's byte-identity.

## Tier 3 — real-time search (after tier 2; the different engine)

`Session::act(budget) -> Option<Step>`: bounded lookahead from the
current world (tier 2's `step` is the lookahead), pick the frontier state
with the best `f`, back up `h` over the local search space (LSS-LRTA*,
Koenig & Sun 2009; RTAA*, Koenig & Likhachev 2006), **commit the first
action** on the path, `apply_start` it. The agent always has a next
action inside `budget`; the learned `h` table lives in the `Session` and
survives across acts, so it cannot loop forever on a finite task without
a dead end. `observe` / `elapse` between acts are the world changing
under it; `fork` + `plan_still_valid` are the repair rule.

**Measure:** ms per act at a fixed budget; actions-to-goal against the
one-shot plan's length (the suboptimality price); stuck rate on the
corridor fixtures (`tests/ladder_wall.rs`'s shapes). **Kill for the
tier:** a budget at which actions-to-goal exceeds 3× the plan on the
panel, or any act that commits an inapplicable action (a bug, not a
price).

## Order and where it lands

Tier 1 beside 0.31 (small, product-visible, no risk to the cycle); tier 2
as 0.32's engine rework, gated on byte-identity; tier 3 after it, as the
feature that makes the engine different rather than five percent better.
Micro-optimisation (`build_rpg`'s incremental form, the 0.30 Lane 6
mechanism) belongs to this track's latency measure, not to the boards.
