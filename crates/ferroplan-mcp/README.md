# ferroplan-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) server that exposes the
[`ferroplan`](https://github.com/seanchatmangpt/ferroplan/tree/main/crates/ferroplan) PDDL planner to an LLM agent. This is the README's bet
made operational: the agent is the **author and supervisor** of a planner, not its
runtime — it writes PDDL, calls a tool, reads a structured, deterministic result, and
iterates.

There are two ways to use it. **One-shot** tools take the PDDL they need and answer.
**Session** tools ground a world once and then take incremental updates — which is the
shape a running simulation actually has, and the only shape in which re-planning is
cheap.

## One-shot tools

| tool | what it does |
|---|---|
| `solve` | Plan a domain + problem; returns the structured `Solution` (typed steps, makespan/metric, statistics). Mode is auto-detected (STRIPS / typing / ADL / numeric / derived axioms / PDDL3 preferences / PDDL2.1 temporal). |
| `parse` | Syntax-check a single PDDL string (domain *or* problem, auto-detected) and return a structure summary — name, requirements, counts — without grounding or solving. Fast feedback while authoring. |
| `validate` | Independently check a plan against a domain + problem under ferroplan's own semantics (classical or temporal); returns valid / invalid-with-reason. |
| `decompose` | Split a temporal goal too big for one-shot search into ordered, individually-solved contracts and stitch them into one validated plan; returns the inspectable `Decomposition` (each contract's named sub-goal + sub-plan + offset). Falls back to a monolithic contract when a goal can't be split. |

`solve` and `decompose` accept an optional `options` object (the same fields as the
library `Options`: `mode`, `search`, `weight_g`, `weight_h`, `threads`,
`max_evaluated`, `optimize`); omitted fields use defaults. The crate enables
ferroplan's `schema` feature, so `options` is advertised as a **typed** JSON Schema
with its real knobs rather than an opaque object.

## Session tools — a world kept open

Grounding is the expensive part of planning, and re-sending a whole domain on every
step throws that work away. A **session** grounds once and stays open: you tell it what
changed and ask it to rethink.

| tool | what it does |
|---|---|
| `session_open` | Ground a domain + problem and keep it live; returns a `session_id` plus the memory split. |
| `session_close` / `session_list` | Free a session; list the open ones with `goal_met` and their memory split. |
| `session_fork` | Fork a second **mind** over the *same* grounded world — the many-minds primitive. |
| `session_set` | Edit the world in one call: flip facts, set fluents, schedule exogenous timed facts, and/or replace the goal. |
| `session_observe` | Tell a mind what it now sees; get back only the **surprises** — sightings that contradicted its beliefs. |
| `session_elapse` | Advance the clock by `dt`, firing due timed facts and the ends of in-flight durative actions. |
| `session_apply_start` | Commit to starting an action; a durative action goes in flight and the mind can rethink while it runs. |
| `session_replan` | Rethink from the current state, optionally budgeted (`max_evaluated` / `memory_mb`). |
| `session_state` | Read back `goal_met`, the memory split, and any facts/fluents you name. |

The loop is: `session_open` once, then repeat *(tell it what changed)* →
`session_replan`. Because the grounded world is shared by every fork, a second actor
costs one **mind**, not one world — `world_bytes` vs `mind_bytes` in `session_state`
report exactly that split.

```jsonc
// after session_open returns {"session_id": "s1", ...}
{"name": "session_set",    "arguments": {"session_id": "s1",
                                         "facts": [["(at v1 field)", true]]}}
{"name": "session_replan", "arguments": {"session_id": "s1"}}
```

Session handles live in the server process: they do not survive a restart, and an
unknown handle comes back as a readable tool error rather than a protocol failure.

## Build & run

```sh
cargo build --release -p ferroplan-mcp     # -> target/release/ferroplan-mcp
```

The server speaks MCP over **stdio** (newline-delimited JSON-RPC 2.0). Point any MCP
client at the binary. For Claude Code / Claude Desktop, add it to your MCP config:

```json
{
  "mcpServers": {
    "ferroplan": {
      "command": "/path/to/ferroplan/target/release/ferroplan-mcp"
    }
  }
}
```

Then ask the agent to author a domain and `solve` it, or to `decompose` a goal that
overruns the one-shot search (see [`../../examples/BORDERS.md`](https://github.com/seanchatmangpt/ferroplan/blob/main/examples/BORDERS.md)).

## Notes

- Built on [`rmcp`](https://crates.io/crates/rmcp), the official MCP Rust SDK, so
  framing, capability negotiation, tool-schema derivation and the error conventions
  come from the SDK rather than a hand-rolled loop. Tool input schemas are derived
  from the Rust parameter types, so they cannot drift from the code.
- **This crate's MSRV is 1.88** (rmcp's), not the workspace's 1.74. The library keeps
  the wider MSRV — an MCP server is a tool you run, not a dependency you compile into
  something old.
- Requests are served **concurrently**, and the two expensive calls — `session_open`
  (grounding) and `session_replan` (search) — run off the async runtime, so one deep
  search cannot stall other sessions. Ordering dependent calls is the client's job, as
  in any JSON-RPC service: wait for `session_open` to return its handle before using it.
- Failures stay readable: a failing tool returns an `isError` result (so the agent sees
  the message and can fix its PDDL) rather than a protocol error, and `solved: false`
  is a normal answer, not an error. As of the rmcp migration the server enforces the
  MCP lifecycle — `initialize` must precede `tools/call`, per spec.
