# ferroplan-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) server that exposes the
[`ferroplan`](../ferroplan) PDDL planner to an LLM agent. This is the README's bet
made operational: the agent is the **author and supervisor** of a planner, not its
runtime — it writes PDDL, calls a tool, reads a structured, deterministic result, and
iterates.

## Tools

| tool | what it does |
|---|---|
| `solve` | Plan a domain + problem; returns the structured `Solution` (typed steps, makespan/metric, statistics). Mode is auto-detected (STRIPS / typing / ADL / numeric / derived axioms / PDDL3 preferences / PDDL2.1 temporal). |
| `parse` | Syntax-check a single PDDL string (domain *or* problem, auto-detected) and return a structure summary — name, requirements, counts — without grounding or solving. Fast feedback while authoring. |
| `validate` | Independently check a plan against a domain + problem under ferroplan's own semantics (classical or temporal); returns valid / invalid-with-reason. |
| `decompose` | Split a temporal goal too big for one-shot search into ordered, individually-solved contracts and stitch them into one validated plan; returns the inspectable `Decomposition` (each contract's named sub-goal + sub-plan + offset). Falls back to a monolithic contract when a goal can't be split. |

`solve` and `decompose` accept an optional `options` object (the same fields as the
library `Options`: `mode`, `search`, `weight_g`, `weight_h`, `threads`,
`max_evaluated`, `optimize`); omitted fields use defaults.

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
overruns the one-shot search (see [`../../examples/BORDERS.md`](../../examples/BORDERS.md)).

## Notes

- No async runtime — a blocking stdio loop keeps the dependency surface to
  `serde`/`serde_json`, matching the rest of the workspace.
- Robust by construction: malformed JSON-RPC returns an error response, a failing
  tool returns an `isError` tool result (so the agent sees the message and can fix its
  PDDL), and the loop continues until stdin closes. The server does not panic on input.
