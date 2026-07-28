# Chatman Phase Engine

A phase-changing Claude Code plugin that makes Ferroplan its first managed world.

It composes:

- current Claude Code loader validation;
- `claude-code-config-lsp` diagnostics, completion, semantic tokens, and Declare conformance;
- RDF/PROV/SHACL-shaped repository observation;
- BCINR semantic admission and receipts;
- the Chatman Multifractal Cascade Allocator (CMCA);
- stateless and persistent Ferroplan planning;
- reversible manufacturing agents;
- independent validation;
- protected hooks, monitors, and canonical BLAKE3 receipt chains.

## Design law

The plugin follows **design for combinatorial maximalism**. It does not encode one fixed workflow. It defines orthogonal primitives and laws for composing them.

The live operating state is a product of six dimensions:

| Dimension | States |
|---|---|
| Epistemic | latent, observed, admitted |
| Allocation | unallocated, allocated |
| Planning | unplanned, candidate, validated |
| Actuation | sealed, manufacturing, receipted, publishable |
| Drift | stable, drifted, refused |
| Configuration | unknown, nonconformant, conformant |

This produces 648 raw combinations. `profiles/phase-space.json` declares the transitions and invariants that admit only lawful combinations. The active agents, skills, and capabilities are the set union associated with the current vector.

Repository mutations deliberately collapse affected dimensions back to:

```text
observed × unallocated × unplanned × sealed × drifted × unknown
```

Nothing re-enters an advanced phase without a receipt.

## Authority graph

| Component | Maximum claim |
|---|---|
| Claude | model authoring and supervision |
| Claude Code loader | plugin load/install conformance |
| claude-code-config-lsp | conformance for its modeled schema epoch |
| RDF observer | bounded semantic projection |
| BCINR | semantic admission/oracle result |
| CMCA | bounded allocation |
| Ferroplan | deterministic candidate plan and suffix validity |
| Source manufacturer | reversible source construction |
| Independent validator | exercised validation result |
| Admission MCP | canonical evidence envelope |
| Hooks | observation and protected-command refusal |
| Receipt auditor | replay and maximum lawful standing |

No composition raises a component above its claim ceiling.

## Installation

From Claude Code:

```text
/plugin marketplace add seanchatmangpt/ferroplan
/plugin install chatman-ecosystem@chatman-ecosystem --scope project
```

The repository also declares the marketplace and plugin in `.claude/settings.json`, so trusted project sessions can enable the plugin at project scope.

At enable time, the plugin can accept optional checkout locations for:

- `claude-code-config-lsp`;
- BCINR.

If omitted, resolvers first use an installed binary and then look for a sibling checkout beside Ferroplan. They never install from the network automatically.

## Main skills

| Skill | Purpose |
|---|---|
| `/chatman-ecosystem:self-host` | Run the complete dogfooding loop |
| `/chatman-ecosystem:phase-change` | Inspect or advance the product-state vector |
| `/chatman-ecosystem:compose` | Manufacture a new capability from existing primitives |
| `/chatman-ecosystem:configure` | Federate loader and config-LSP conformance |
| `/chatman-ecosystem:observe` | Build the RDF-shaped repository world |
| `/chatman-ecosystem:allocate` | Run CMCA and bind allocation evidence |
| `/chatman-ecosystem:plan` | Retain or replan a persistent Ferroplan Session |
| `/chatman-ecosystem:manufacture` | Execute one reversible plan step |
| `/chatman-ecosystem:validate` | Independently exercise the changed surface |
| `/chatman-ecosystem:admit` | Bind canonical plan and validator evidence |
| `/chatman-ecosystem:audit` | Replay receipts and determine standing |
| `/chatman-ecosystem:doctor` | Diagnose every plugin surface |
| `/chatman-ecosystem:publish` | Explicitly perform protected publication |

`publish` cannot be invoked automatically by the model.

## MCP servers

The plugin starts two independent stdio authorities:

- `ferroplan`: stateless parse/solve/validate/decompose, persistent `Session`
  (observation, suffix replay, bounded replanning, CMCA), and Chatman
  admission (canonical digest, allocation envelope, plan envelope, receipt
  verification) — all 16 tools in one process, one `rmcp` server;
- `bcinr`: semantic admission, PDDL/POWL, capability, and receipt tools.

## Live self-hosting world

The repository world is represented by:

- `world/ferroplan-self-host-domain.pddl`;
- `scripts/project-world.py`;
- the hook ledger;
- the current phase vector.

Generate a live problem:

```sh
python3 "$CLAUDE_PLUGIN_ROOT/scripts/project-world.py" \
  --project "$CLAUDE_PROJECT_DIR" \
  --goal receipt \
  --output /tmp/ferroplan-live.pddl \
  --metadata /tmp/ferroplan-live.json
```

The problem is deterministic for the same ledger and phase state. Ferroplan therefore plans against its actual observed repository standing rather than a static example.

## Configuration schema epochs

`claude-code-config-lsp` is ontology-generated and valuable, but its current ontology models an earlier Claude Code plugin schema. `profiles/config-schema-epoch.json` records known differences, including:

- optional commit-SHA plugin versions;
- object marketplace sources;
- plugin dependencies;
- experimental monitors;
- user configuration;
- expanded hook types;
- plugin-root agent and skill locations.

The current Claude loader and `claude plugin validate` govern loadability. The LSP governs only the surfaces represented in its ontology. Known epoch differences cannot create false refusals. Unknown disagreements remain `UNKNOWN`.

## Receipt chain

Allocation and plan envelopes bind canonical forms of:

- observation frontier;
- eight CMCA candidates;
- CMCA result and BCINR revision;
- PDDL domain and problem commitments through the Session receipt;
- candidate plan;
- independent validator result;
- predecessor receipt.

The admission server uses recursively key-sorted JSON, length-framed inputs, and BLAKE3. Verification recomputes both the payload digest and receipt.

## Protected actuation

Hooks deny protected Bash operations when repository observations are ahead of the admitted receipt frontier. Protected surfaces include publication, destructive git operations, package publishing, recursive forced deletion, and state-changing HTTP requests.

A source change is allowed only as reversible manufacturing. It immediately becomes a new observation and seals advanced actuation until the loop closes again.

## Standing

- `ALIVE`: exact runtime and replay evidence establishes the complete stated claim.
- `PARTIAL_ALIVE`: a bounded subset is evidenced and the remaining obligations are named.
- `BUILD_BROKEN`: an exercised build, validation, or execution surface failed.
- `UNKNOWN`: the required executor or evidence was unavailable.

Source presence, plans, confidence, and prose do not establish `ALIVE`.

## Development check

Run the plugin doctor inside Claude Code:

```text
/chatman-ecosystem:doctor
```

The doctor checks loader validation, LSP resolution, Python syntax, shell resolvers, Rust binaries, MCP startup, live PDDL projection, phase invariants, and receipt replay.
