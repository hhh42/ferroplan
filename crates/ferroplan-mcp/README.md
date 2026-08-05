# ferroplan-mcp

`ferroplan-mcp` is a single Model Context Protocol server that exposes Ferroplan as a deterministic planning, persistent-mind, evidence, diagnosis, composition, and operator-experience platform.

The server advertises **42 typed tools** over MCP stdio. Tool schemas are derived from Rust types. Semantic resources are generated from the repository's Turtle ontologies at build time.

## Authority map

| Plane | Tools | Purpose |
|---|---|---|
| Planning | `solve`, `parse`, `validate`, `decompose` | Author, solve, inspect, and independently validate PDDL plans. |
| Session lifecycle | `session_open`, `session_observe`, `session_set_goal`, `session_think`, `session_advance`, `session_status`, `session_close` | Keep a grounded planning world open and update it incrementally. |
| Persistent control | `session_list`, `session_state`, `session_set`, `session_fork`, `session_replan`, `session_checkpoint`, `session_restore`, `session_verify_checkpoint`, `session_history`, `session_compare`, `session_restrict_ops`, `session_schedule_fact`, `session_apply_start`, `session_elapse` | Operate many independent minds over shared grounding with authority, recovery, time, and receipts. |
| Allocation | `cmca_allocate`, `cmca_allocate_recursive` | Execute the pinned Chatman Multifractal Cascade Allocator. |
| Admission | `canonical_digest`, `bind_allocation_receipt`, `bind_plan_receipt`, `verify_receipt` | Bind canonical identities and verify receipt envelopes. |
| DX | `dx_manifest`, `dx_compose` | Make the server self-describing and compile desired outcomes into minimal tool sequences. |
| Doctor | `doctor_scan`, `doctor_explain` | Diagnose server/session standing and turn refusals into typed recovery guidance. |
| Wizard | `wizard_bootstrap`, `wizard_recipe` | Manufacture ready planning minds and compile high-level operator intents into inspectable recipes. |
| QoL | `qol_snapshot`, `qol_batch` | Collapse many reads into one snapshot and many compatible writes into one atomic transaction. |
| Telco | `telco_envelope`, `telco_verify` | Manufacture and verify transport-neutral integrity envelopes without performing network delivery. |
| Vision | `vision_lattice` | Enumerate bounded combinatorial capability reachability and blocked frontiers. |

## The persistent-mind model

A `Session` separates immutable grounded columns from mutable mind state. Forking a session therefore creates another independent mind without re-grounding the world.

Each managed mind carries:

- semantic state and a BLAKE3 state fingerprint;
- goal, retained plan, and cursor;
- optimistic-concurrency epoch;
- parent and generation lineage;
- allowed and denied operator prefixes;
- scheduled world events and in-flight durative ends;
- bounded canonical event history;
- receipt-chain head;
- shared-world and private-mind memory measurements.

The control plane preserves these laws:

1. A stale `expected_epoch` is refused before mutation.
2. `session_set` and `qol_batch` stage changes on a fork and commit only after complete validation.
3. A failed batch cannot expose partial state.
4. Operator restrictions alter the planner's search mask rather than filtering a finished plan.
5. Checkpoint restore is explicit and lineage-bound.
6. Search, memory, history, lattice, payload, and TTL boundaries have mechanical ceilings.
7. Every accepted session mutation extends the canonical receipt chain.

## Fast paths

### Bootstrap a ready planning mind

`wizard_bootstrap` combines grounding, optional goal replacement, operator scope, bounded search, session insertion, diagnosis, and receipt binding into one transaction.

```json
{
  "name": "wizard_bootstrap",
  "arguments": {
    "session_id": "factory-1",
    "domain": "(define ...)",
    "problem": "(define ...)",
    "goal": "(and (verified) (released))",
    "allowed_prefixes": ["VERIFY", "RELEASE"],
    "plan": true,
    "max_evaluated": 50000
  }
}
```

### Read the operating state in one round trip

`qol_snapshot` returns identity, lineage, selected facts and fluents, plan standing, authority scope, memory, history tail, and doctor findings.

```json
{
  "name": "qol_snapshot",
  "arguments": {
    "session_id": "factory-1",
    "facts": ["(verified)", "(released)"],
    "fluents": ["(remaining-budget)"],
    "history_tail": 16
  }
}
```

### Commit a heterogeneous transaction once

`qol_batch` supports fact writes, fluent writes, goal replacement, operator-scope replacement, temporal scheduling, durative starts, elapsed time, and one final bounded replan. The replan operation, when present, must be last.

```json
{
  "name": "qol_batch",
  "arguments": {
    "session_id": "factory-1",
    "expected_epoch": 3,
    "operations": [
      {"op": "set_fact", "fact": "(verified)", "value": true},
      {"op": "set_goal", "goal": "(released)"},
      {"op": "replan", "max_evaluated": 50000}
    ]
  }
}
```

## Self-describing composition

`dx_manifest` returns the modeled contract for every tool: category, required atoms, provided atoms, mutation behavior, reversibility, receipt behavior, latency class, and summary.

`dx_compose` performs bounded breadth-first search over those contracts. Given admitted starting atoms and desired outcome atoms, it returns a minimal deterministic tool sequence or an explicit missing frontier.

`vision_lattice` enumerates bounded reachable atom sets, minimal atom depth, tool dependency edges, blocked capabilities, and theoretical subset capacity. Its limits prevent combinatorial exploration from becoming unbounded execution.

## Doctor and wizard

`doctor_scan` evaluates one session or the server's complete live session set. Findings use typed codes such as missing receipt chain, absent plan, exhausted plan, invalid retained suffix, active operator scope, and memory imbalance.

`doctor_explain` deterministically classifies common protocol and tool failures, including unknown sessions, stale epochs, identity collisions, bounded-search refusals, non-finite inputs, missing checkpoints, invalid plans, ungrounded facts, expired envelopes, and integrity mismatches.

`wizard_recipe` converts supported operator intents into explicit tool recipes with preflight, rollback, and receipt checkpoints. Recipes never bypass tool schemas or session authority.

## Transport-neutral telco envelopes

`telco_envelope` binds sender, recipient, channel, issue and expiry times, correlation, causation, predecessor, idempotency, canonical payload, and BLAKE3 identities.

`telco_verify` checks schema, payload identity, envelope identity, recipient expectation, predecessor expectation, issue time, and expiry.

The boundary is deliberate:

- the tools perform **no network operation**;
- BLAKE3 establishes canonical identity and tamper evidence;
- authentication is reported as `UNSUPPORTED` rather than inferred from integrity;
- delivery, retries, authorization, and external actuation remain downstream broker responsibilities.

## RDF-owned semantics

MCP resources use the unified URI form:

```text
ferroplan://tools/<tool-name>
```

Planning, session, allocation, and admission semantics come from `plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl`.

The operator-experience plane comes from `plugins/chatman-ecosystem/ontology/ferroplan-experience.ttl`. That graph defines the experience plane, capability contracts, composition atoms, mutation/reversibility/receipt properties, telco non-actuation law, integrity-versus-authentication distinction, all eleven experience tools, and SHACL constraints.

`build.rs` extracts the tool comments into generated `OUT_DIR` constants. Runtime resource descriptions therefore cannot silently diverge from the admitted ontology source.

## Build and run

```sh
cargo build --release -p ferroplan-mcp
./target/release/ferroplan-mcp
```

The server speaks MCP over stdio using `rmcp` and Tokio.

Example MCP configuration:

```json
{
  "mcpServers": {
    "ferroplan": {
      "command": "/path/to/ferroplan/target/release/ferroplan-mcp"
    }
  }
}
```

## Verification

The permanent repository crown executes the built server through real MCP stdio and includes:

- planning and protocol tests;
- admitted session lifecycle tests;
- persistent-control tests;
- Vision 2030 experience-plane tests;
- exact 42-tool and 42-resource catalogue checks;
- ontology extraction and resource provenance checks;
- RDF parsing and SHACL validation;
- strict Clippy with warnings denied;
- plugin, Luna, live-harvest, receipt, replay, projection, and clean-tree boundaries.

A tool refusal remains a tool-level error with a readable message. `solved: false` remains a normal bounded planning result. Neither is promoted into success by the experience plane.
