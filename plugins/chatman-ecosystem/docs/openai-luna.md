# OpenAI-hosted Star execution

This is an alternative execution host for the existing Chatman architecture. It does not replace OStar, Ferroplan, or OntoStar authority.

```text
OStar MuStar / SigmaStar
  └─ provisional build order + POWL + diagram + candidate artifact
             │
             ▼
OpenAI Responses tool loop
  ├─ Ferroplan MCP ─ deterministic PDDL/session/CMCA planning and replay
  ├─ OntoStar MCP  ─ CTQ/work-order/workflow admission and receipts
  └─ workspace MCP ─ optional bounded repository actuation
             │
             ▼
sealed trace: Star proposal + Ferroplan witness + OntoStar witness
```

## Preserved Star calculus

The adapter uses the actual OStar contracts:

- `MuStarPlanner`: problem and constraints to build order, POWL model, and sequence diagram.
- `MuStarExecutor`: the admitted strategy to a provisional candidate artifact.
- `SigmaStarDecomposeSignature`: a large objective to a bounded list of MuStar tasks.

It deliberately does **not** call `MuStarAgent.forward()` or `SigmaStarAggregator.solve()`. Those methods execute generated artifacts internally. In this host, Star agents are proposers only; source mutation and command execution must occur through an explicitly attached MCP authority.

## Fences

- The model ID is exactly `gpt-5.6-luna`; the unsuffixed alias is refused.
- Star output must declare `provisional=true`, `authority=proposer`, and `internal_actuation=false`.
- Ferroplan and OntoStar MCP servers are mandatory.
- A model response alone cannot establish completion.
- `ALIVE` requires all of:
  1. a valid OStar Star proposal;
  2. a positive configured Ferroplan planning or replay witness;
  3. a positive configured OntoStar admission witness;
  4. non-empty final OpenAI output.
- OntoStar A2A is optional discovery and coordination only. MCP remains the admission path.
- No filesystem or shell authority is implicit. Attach a bounded workspace MCP server for repository mutation.
- Tool discovery, call count, result size, MCP message size, response rounds, and Star task count are independently bounded.
- MCP pagination cycles, duplicate tool identities, reused OpenAI call IDs, oversized results, malformed JSON, and untyped transport faults are refused.
- Common credential fields are redacted from trace projections while their original values remain represented only by cryptographic digests.

## Run

```bash
export OPENAI_API_KEY='...'
export OSTAR_ROOT=/path/to/ostar
export FERROPLAN_ROOT=/path/to/ferroplan
export ONTOSTAR_ROOT=/path/to/open-ontologies

python3 plugins/chatman-ecosystem/scripts/openai_luna.py \
  --project /path/to/target-repository \
  --target ferroplan \
  --receipt .chatmangpt/openai-star-trace.json \
  'Implement the requested change through the Star planning and admission chain.'
```

Use SigmaStar decomposition by changing the profile:

```json
{
  "star": {
    "mode": "sigma-star",
    "domain": "SYSTEM_DESIGN",
    "max_tasks": 8
  }
}
```

Attach a repository MCP server when the task includes source mutation:

```bash
python3 plugins/chatman-ecosystem/scripts/openai_luna.py \
  --mcp workspace=/absolute/path/to/run-workspace-mcp.sh \
  'Plan, implement through bounded tools, verify, and obtain OntoStar admission.'
```

The host discovers MCP schemas with `tools/list`, projects them into namespaced OpenAI function tools (`ferroplan__*`, `ontostar__*`, `workspace__*`), dispatches calls to the owning stdio server, and feeds structured results into the next Responses turn.

## Verification ladder

The suite keeps each form of evidence distinct. A lower rung cannot impersonate a higher rung.

| Form | Executable surface |
|---|---|
| Legacy regression | Original adapter and MCP pagination witnesses |
| Unit | Profile, envelope, registry, result projection, refusal, and redaction laws |
| Contract | Committed profile, OpenAI payload, trace schema, witness, and digest contracts |
| OStar contract | Real `MuStarPlanner`, `MuStarExecutor`, SigmaStar decomposition, and OpenAI DSPy binding seams |
| Property/fuzz | Deterministic randomized tool names, canonicalization, digest invariance, and nested verdicts |
| Replay | Deterministic trace reproduction, resealing, and tamper falsifiers |
| Mutation sentinels | Removal or inversion of every crown condition must block standing |
| Integration | Real stdio MCP subprocesses, Star launcher subprocess, and A2A HTTP probe |
| End-to-end | Black-box CLI through local Responses HTTP, Star, Ferroplan MCP, OntoStar MCP, and atomic receipt |
| Security | Prompt-injection resistance, unadvertised-tool refusal, and credential redaction |
| Chaos | MCP crash, malformed output, timeout, OpenAI partition, A2A partition, and round exhaustion |
| Stress | 2,000-tool discovery, 100 repeated host runs, and 256 parallel trace seals |
| Benchmark | Bounded registry-discovery and large-trace sealing latency guards |
| Coverage | Branch-aware coverage over protocol, runtime, MCP, verifier, and OStar seams |
| Compatibility | The focused ladder runs on Python 3.11, 3.12, and 3.13 |

Run the complete focused ladder and produce receipts:

```bash
cd plugins/chatman-ecosystem
python -m pytest -q \
  --junitxml=openai-luna-junit.xml \
  tests/test_openai_luna*.py \
  tests/test_openai_ostar_star.py

python scripts/verify_openai_luna.py \
  --junit openai-luna-junit.xml \
  --output openai-luna-verifier.json \
  --static-check ruff \
  --static-check compileall \
  --static-check shell-syntax \
  --static-check coverage
```

The verifier report is sealed with the SHA-256 of its unsigned canonical JSON and includes the SHA-256 of the JUnit receipt. GitHub Actions uploads both files for every supported Python version.

## Claim boundaries

The black-box E2E test is complete for the host protocol but uses local deterministic fake servers. It proves subprocess, HTTP, MCP, response-loop, admission, receipt, and shutdown composition without consuming external credentials.

Live OpenAI service behavior, installed OStar dependencies, the production Ferroplan/OntoStar binaries, and repository-specific workspace mutation remain `UNKNOWN` until executed in the target environment. No mocked or local test upgrades those external surfaces to `ALIVE`.

## Standing

- `ALIVE`: this invocation observed the complete bounded chain described above.
- `BLOCKED`: a required law boundary refused progression or a witness was absent.
- `BUILD_BROKEN`: an executable verifier failed.
- `UNKNOWN`: the surface was not executed in the relevant environment.
