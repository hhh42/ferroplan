# Lawful Claude Projection

## Status

This document defines the Claude Code projection of the Chatman ecosystem for Ferroplan v26.7.29.

The projection is a managed-world adapter. It is not the constitutional center of the ecosystem.

## Constitutional boundary

```text
                         CHATMAN CONSTITUTION
                  A = μ(O*) · zero unreceipted actuation
                                   │
             ┌─────────────────────┴─────────────────────┐
             │                                           │
       Observation law                            Actuation law
        star-toml / RDF                                BRCE
             │                                           │
             ▼                                           ▼
       admitted O*                               protected DO broker
             │                                           ▲
             ├───────────────┐                           │
             │               │                           │
             ▼               ▼                           │
         Graphlaw          CMCA                          │
      semantic closure   work allocation                 │
             │               │                           │
             └───────┬───────┘                           │
                     ▼                                   │
                   MFW / POWL v2                         │
             planning and admission law                  │
                     │                                   │
                     ├──────────────┐                    │
                     ▼              ▼                    │
                Ferroplan       other planners           │
             candidate plans   comparative rails         │
                     │                                   │
                     ▼                                   │
                 Manufacture ─────────────────────────────┘
                     │
                     ▼
          unit → integration → end-to-end
          → chaos → stress → benchmark
                     │
                     ▼
            validator / VAL / Lean
                     │
                     ▼
                BLAKE3 receipt
                     │
                     ▼
              replay and standing
```

Claude Code attaches as an adapter:

```text
Claude Code plugin
├── projects the current admitted world
├── routes to bounded authorities
├── invokes planner and allocator tools
├── performs reversible manufacture
├── records observation candidates
└── submits protected actions to BRCE
```

## Responsibility matrix

| Component | Lawful responsibility | Claude projection |
|---|---|---|
| star-toml / O*.toml | Canonical admitted observation carrier | Project and session configuration |
| Graphlaw | RDF, SHACL, and bounded semantic derivation | Configuration and repository diagnostics |
| BCINR-CMCA | Bounded multifractal allocation | `cmca_allocate` MCP tool |
| MFW / POWL v2 | Planning constitution, promotion, admission, and planner choice | Planner-routing authority |
| Ferroplan | Deterministic candidate planning and persistent sessions | Planning MCP server |
| VAL | Independent PDDL validation | External validator evidence |
| Lean / mfact | Kernel proof for modeled claims | Crown proof rail |
| ggen | Graph-to-filesystem manufacturing | Claude projection generation |
| BRCE | Exclusive protected-actuation broker | PreToolUse intent and grant adapter |
| Knowledge Hooks | Observation and intent manufacture | Claude lifecycle events |
| Receipt system | Evidence binding and replay | BLAKE3 envelopes and predecessor chains |
| Claude Code | Interactive coding runtime | Plugin host and reversible manufacturer |

## Authority graph

| Authority | Maximum claim | Explicit exclusion |
|---|---|---|
| Claude | Model authoring and supervision | No execution proof |
| Claude Code loader | Plugin load and installation conformance | No semantic correctness |
| Config validator | Modeled configuration conformance | No global file ownership |
| RDF observer | Bounded repository projection | No admission or actuation |
| CMCA | Bounded allocation | No planning |
| Ferroplan | Candidate plan and suffix validity | No independent validation |
| Source manufacturer | Reversible source construction | No validation or publication claim |
| Independent validator | Exercised evidence | No repair or manufacture |
| Admission service | Canonical evidence envelopes | No consequence execution |
| Knowledge Hooks | Observation and intent candidates | No truth collapse |
| BRCE adapter | Protected-operation admission | No claim beyond Claude runtime |
| Receipt auditor | Replay and maximum standing | No publication |

No composition raises a component above its claim ceiling.

## Product-state calculus

The operating state is the product of six orthogonal dimensions:

```text
epistemic × allocation × planning × actuation × drift × conformance
```

The raw state space has 648 combinations. `profiles/phase-space.json` declares allowed transitions and invariants.

Pending repository mutation projects the effective state to:

```text
observed × unallocated × unplanned × sealed × drifted × unknown
```

This projection does not rewrite the canonical receipt-bound snapshot. It prevents stale advanced standing from being used while observations remain pending.

## Event sourcing law

```text
phase-events.jsonl = canonical phase-event source
state.json          = observation-frontier cache
phase-state.json    = derived phase snapshot
receipt chain       = admission evidence
phase-space.json    = transition law
```

Startup and status operations must distinguish:

- canonical snapshot;
- pending observation frontier;
- effective projected vector;
- admitted transition history;
- replay mismatch.

A replay mismatch is a typed refusal, not an invitation to trust the newest file.

## Agent authority law

Only `source-manufacturer` has direct source-edit tools.

Every other role denies `Write`, `Edit`, and `NotebookEdit`. Roles that do not require shell execution also deny `Bash`.

The controller can spawn only declared Chatman agents. It cannot directly manufacture source.

The manufacturer uses:

```yaml
isolation: worktree
```

The worktree boundary provides reversible construction. It does not establish validation, receipt closure, or publication.

## Configuration law

The current Claude Code loader is the runtime load/install authority.

`claude-code-config-lsp` is a bounded modeled-surface validator. The main Chatman plugin does not register it against broad extensions because Claude Code's LSP dispatch is extension-based and lacks a path predicate suitable for limiting it to Claude configuration files.

The standalone LSP marketplace entry remains available for explicit opt-in use.

Known loader/model deltas are enumerated in `profiles/config-schema-epoch.json`. A known delta cannot become a false refusal. An unknown disagreement remains `UNKNOWN`.

## Generated-artifact law

All Claude projection files have a canonical owner in `profiles/artifact-ownership.json`.

The lawful manufacturing chain is:

```text
admitted ontology/profile
→ deterministic projection
→ syntax validation
→ loader validation
→ modeled-surface validation
→ exact digest comparison
→ configuration receipt
```

Generated output may not be hand-edited. A change begins at the owner and propagates to every dependent projection.

## Protected actuation law

A protected Bash request becomes an `ActuationIntent` before execution.

The intent binds:

- session and actor;
- operation class;
- command digest;
- target digest when available;
- pending event frontier;
- current effective phase;
- required phase;
- reversibility;
- predecessor receipt.

A `DerivedExecutionGrant` binds the intent digest to a verified receipt and a closed frontier.

The executor result must later become an `ExecutionAttestation`. The Claude adapter currently implements the intent and grant Gall checkpoint. Cross-runtime execution attestation remains a BRCE milestone.

## Recursive CMCA law

Each allocation is exactly eight nodes. A selected node may become the root of another eight-node frontier.

This preserves bounded local computation while allowing multifractal scale:

```text
root frontier
└── selected node
    └── local frontier
        └── selected node
            └── local frontier
```

Every recursive descent must bind:

- parent allocation receipt;
- local observation frontier;
- projection law;
- BCINR revision;
- local allocation result;
- return consequence.

## Planner separation

```text
planner implementation ≠ planning constitution
```

MFW/POWL v2 chooses and admits planning rails. Ferroplan supplies deterministic candidate plans. VAL supplies independent PDDL validation where required. Receipts bind the identities, versions, inputs, outputs, and predecessor evidence.

## Crown falsifier

The projection reaches `ALIVE` only when the exact release commit demonstrates:

- strict loader validation;
- clean installation;
- complete MCP protocol exercise;
- agent authority enforcement;
- worktree-isolated manufacture;
- replayable phase and observation state;
- independent validation;
- receipt tamper refusal;
- protected actuation refusal before closure;
- protected actuation acceptance after a verified grant;
- draft pull-request publication;
- complete receipt-chain replay.

Source completeness is `PARTIAL_ALIVE`, not `ALIVE`.
