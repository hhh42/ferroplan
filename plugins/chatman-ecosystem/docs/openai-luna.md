# GPT-5.6 Luna execution host

This is an **alternative host projection**, not a replacement for Ferroplan,
MuStar, or OntoStar authority.

```text
MuStar MotionObligation
        │
        ▼
executor projection: agent:claude-code → agent:openai-luna
        │                         (original obligation is unchanged)
        ▼
GPT-5.6 Luna Responses tool loop
        │
        ├── ferroplan MCP  ── deterministic PDDL/session/CMCA planning
        ├── ontostar MCP   ── semantic validation and admission
        └── extra MCP      ── optional bounded workspace/source actuation
        │
        ▼
A2A run trace + positive OntoStar witness + digest
```

## Fences

- The model ID is exactly `gpt-5.6-luna`. The `gpt-5.6` alias is refused.
- MuStar must select `ImplementAcceptedDelta`; every other obligation stops
  before an OpenAI request.
- The authoritative MuStar obligation and its BLAKE3 hash are preserved. The
  adapter records a separate host-specific executor projection.
- Ferroplan and OntoStar MCP servers are mandatory.
- A successful model response is not completion. At least one configured
  OntoStar admission/validation tool must return a positive result.
- OntoStar A2A is used for optional discovery/coordination. MCP remains the
  authoritative tool and admission path.
- No implicit filesystem or shell authority is introduced. To operate as a
  coding-agent replacement, attach an explicitly bounded workspace MCP server.

## Run

```bash
export OPENAI_API_KEY='...'
export FERROPLAN_ROOT=/path/to/ferroplan
export ONTOSTAR_ROOT=/path/to/open-ontologies
export CHATMANGPT_ROOT=/path/to/chatmangpt

python3 plugins/chatman-ecosystem/scripts/openai_luna.py \
  --project /path/to/target-repository \
  --mustar-project /path/to/control-workspace \
  --target ferroplan \
  --receipt .chatmangpt/openai-luna-trace.json \
  'Implement the MuStar-authorized accepted delta.'
```

Attach a repository MCP server when the task includes source mutation:

```bash
python3 plugins/chatman-ecosystem/scripts/openai_luna.py \
  --mcp workspace=/absolute/path/to/run-workspace-mcp.sh \
  'Implement the accepted delta, verify it, then obtain OntoStar admission.'
```

The host discovers all MCP schemas with `tools/list`, projects them into
namespaced OpenAI function tools (`ferroplan__*`, `ontostar__*`,
`workspace__*`), dispatches calls back to the owning stdio server, and feeds
structured outputs into the next Responses turn.

## Result standing

- `ALIVE`: this invocation observed an authorized MuStar obligation, completed
  the Responses/MCP loop, and observed a positive OntoStar witness.
- `BLOCKED`: a law boundary refused progression (wrong obligation/model,
  missing server/key, failed tool, exhausted loop, or missing admission).
- Live OpenAI, local binaries, and repository-specific workspace actuation must
  still be exercised in the target environment; unit tests use deterministic
  fake Responses and MCP surfaces.
