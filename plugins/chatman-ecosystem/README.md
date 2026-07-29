# Chatman Phase Engine

Ferroplan v26.7.29 provides the first lawful managed-world projection of the Chatman ecosystem into Claude Code.

> Claude Code is not the Chatman ecosystem. It is one managed-world projection of the ecosystem.

The plugin composes current Claude Code loader validation, bounded configuration validation, RDF-shaped observation, BCINR-CMCA allocation, persistent Ferroplan candidate planning, mechanically restricted agents, isolated manufacture, independent validation, structured protected-actuation intents, skill-triggered monitors, and canonical BLAKE3 receipts.

## Constitutional law

```text
A = μ(O*)
```

`O*` is admitted observation. `μ` is lawful manufacture. `A` is an artifact with standing.

```text
zero unreceipted actuation
```

The model may propose and supervise. It cannot manufacture execution evidence through prose.

## Combinatorial maximalism

The plugin defines orthogonal primitives and composition laws rather than one fixed workflow.

| Dimension | States |
|---|---|
| Epistemic | latent, observed, admitted |
| Allocation | unallocated, allocated |
| Planning | unplanned, candidate, validated |
| Actuation | sealed, manufacturing, receipted, publishable |
| Drift | stable, drifted, refused |
| Conformance | unknown, nonconformant, conformant |

The product contains 648 raw combinations. `profiles/phase-space.json` declares lawful transitions and invariants.

Pending repository observations project the effective state to:

```text
observed × unallocated × unplanned × sealed × drifted × unknown
```

The canonical snapshot remains a cache; it cannot override a pending frontier.

```sh
python3 "$CLAUDE_PLUGIN_ROOT/scripts/effective-phase.py" \
  --project "$CLAUDE_PROJECT_DIR"
```

## Authority graph

| Component | Maximum claim |
|---|---|
| Claude | model authoring and supervision |
| Claude Code loader | plugin load and installation conformance |
| Config validator | bounded modeled-surface conformance |
| RDF observer | bounded repository projection |
| BCINR-CMCA | bounded allocation |
| Ferroplan | deterministic candidate plan and suffix validity |
| Source manufacturer | reversible construction in an isolated worktree |
| Independent validator | exercised validation evidence |
| Admission tools | canonical evidence envelopes |
| Knowledge Hooks | observation and intent candidates |
| BRCE adapter | Claude-runtime protected-actuation admission |
| Receipt auditor | replay and maximum lawful standing |

No composition raises a component above its claim ceiling.

## Mechanical agent authority

Every non-manufacturing agent denies `Write`, `Edit`, and `NotebookEdit`.

The controller can route work and spawn only declared Chatman roles. It cannot directly edit source.

The source manufacturer is the single source actuator and declares:

```yaml
isolation: worktree
```

Worktree isolation establishes reversibility, not validation or publication.

## Installation

```text
/plugin marketplace add seanchatmangpt/ferroplan
/plugin install chatman-ecosystem@chatman-ecosystem --scope project
```

The distributed plugin is opt-in with `defaultEnabled: false`. Ferroplan's checked-in project settings explicitly enable it for trusted project sessions.

Optional user configuration:

- `ferroplan_root`: checkout used to launch the single combined `ferroplan` MCP server;
- `config_lsp_root`: checkout used only for explicit bounded config validation.

## Skills

| Skill | Purpose |
|---|---|
| `/chatman-ecosystem:self-host` | Run the complete dogfooding loop |
| `/chatman-ecosystem:phase-change` | Inspect or advance the product-state vector |
| `/chatman-ecosystem:compose` | Compose a capability from admitted primitives |
| `/chatman-ecosystem:configure` | Federate loader, ownership, and bounded config validation |
| `/chatman-ecosystem:observe` | Build the RDF-shaped repository world |
| `/chatman-ecosystem:allocate` | Run recursive bounded CMCA allocation |
| `/chatman-ecosystem:plan` | Retain or repair a persistent Ferroplan Session |
| `/chatman-ecosystem:manufacture` | Execute one reversible worktree step |
| `/chatman-ecosystem:validate` | Independently exercise the changed surface |
| `/chatman-ecosystem:admit` | Bind canonical plan and validator evidence |
| `/chatman-ecosystem:audit` | Replay receipts and determine standing |
| `/chatman-ecosystem:doctor` | Diagnose every projection surface |
| `/chatman-ecosystem:publish` | Explicitly derive a grant and perform protected publication |

`publish` cannot be invoked automatically by the model.

## One MCP process, multiple claim ceilings

The plugin starts one `ferroplan` stdio MCP process. It exposes stateless planning, persistent sessions, BCINR-CMCA allocation, and admission-envelope tools. One process does not imply one authority.

## Bounded configuration validation

The main plugin does not register `claude-code-config-lsp` as the global server for JSON, Markdown, TOML, and shell files. Claude Code's LSP dispatch is extension-based and lacks the path predicate needed to restrict common extensions to configuration files.

The standalone LSP remains available as a separate marketplace plugin or explicit CLI validator. `profiles/config-schema-epoch.json` records known loader/model differences. Known deltas cannot create false refusals; unknown disagreements remain `UNKNOWN`.

## Generated configuration ownership

Canonical owners include:

- `ontology/chatman-ecosystem.ttl`;
- `ontology/chatman-shapes.ttl`;
- `ontology/authority-graph.ttl`;
- `profiles/claude-projection.json`;
- `profiles/artifact-ownership.json`.

```text
admitted ontology/profile
→ ggen projection
→ source validation
→ Claude loader validation
→ bounded modeled validation
→ exact digest comparison
→ configuration receipt
```

Hand-editing a generated projection without changing its owner is refused.

```sh
python3 "$CLAUDE_PLUGIN_ROOT/scripts/validate-claude-projection.py" \
  --plugin-root "$CLAUDE_PLUGIN_ROOT"
```

## Knowledge Hooks

```text
hook event ≠ admitted truth
```

Per-tool events preserve exact mutation identity. `PostToolBatch` supplies one bounded summary. Configuration changes and worktree lifecycle events are recorded as separate candidates. The frontier must be admitted before advanced standing returns.

## Structured protected actuation

`scripts/actuation-intent.py` converts protected Bash requests into exact `ActuationIntent` objects binding operation, command digest, effective phase, required phase, pending frontier, predecessor receipt, and reversibility.

`scripts/grant-actuation.py` creates a matching `DerivedExecutionGrant` only after receipt verification and frontier closure. The older regex fence in `loop.py` remains as defense in depth.

A grant proves admission of the exact intent. It does not prove execution. The resulting tool event must later become execution evidence.

## Recursive CMCA

Every allocation remains exactly eight nodes with ten ordered factors. A selected node may root another admitted eight-node frontier. Recursive descent binds its parent allocation receipt and returns a consequence upward.

This provides multifractal scale without an unbounded global allocator.

## Live self-hosting world

The world is represented by `world/ferroplan-self-host-domain.pddl`, `scripts/project-world.py`, the observation ledger, the canonical phase snapshot, and the effective pending-state projection.

```sh
python3 "$CLAUDE_PLUGIN_ROOT/scripts/project-world.py" \
  --project "$CLAUDE_PROJECT_DIR" \
  --goal receipt \
  --output /tmp/ferroplan-live.pddl \
  --metadata /tmp/ferroplan-live.json
```

## Receipt chain

Receipts bind the observation frontier, RDF projection, eight CMCA candidates, CMCA output and BCINR revision, parent allocation receipt when recursive, PDDL commitments, candidate plan, independent validation, configuration projection, actuation intent and grant, and predecessor receipt.

## Standing

- `ALIVE`: exact runtime and replay evidence establishes the full claim.
- `PARTIAL_ALIVE`: a bounded subset is evidenced and remaining obligations are named.
- `BLOCKED`: an admitted dependency or authority prevents lawful progress.
- `BUILD_BROKEN`: an exercised surface failed.
- `UNKNOWN`: required evidence or execution was unavailable.
- `UNSUPPORTED`: the capability is outside the wired boundary.

Source presence, plans, schema validity, confidence, and prose do not establish `ALIVE`.

## Development checks

```text
/chatman-ecosystem:doctor
```

```sh
python3 plugins/chatman-ecosystem/scripts/validate-claude-projection.py \
  --plugin-root plugins/chatman-ecosystem
python3 -m compileall -q plugins/chatman-ecosystem/scripts
```

The crown check additionally requires loader validation, MCP protocol exercise, agent authority enforcement, worktree manufacture, independent validation, receipt replay, tamper refusal, and protected publication through an exact grant.
