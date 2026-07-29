#!/usr/bin/env python3
"""PreToolUse hook: fence write-shaped Bash commands for non-manufacturing agents.

Gall Checkpoint 3 ("Mechanical Agent Authority") requires that only
`source-manufacturer` can edit source. Frontmatter `tools:` allow-lists
(ontology-generated, see `generate.py`) already make `Write`/`Edit`/
`NotebookEdit` structurally absent from the other 7 agents' tool schemas.
But those same 7 agents still legitimately need `Bash` for read-only
status/build/test commands, and `tools:` is a named-tool allow-list, not a
command-shape policy — it cannot express "Bash, but only for reads". That
gap was found live: `rdf-observer` (Bash but no Write/Edit) successfully
executed `echo ... > file` via Bash despite its role prose forbidding edits.

This hook closes that specific gap mechanically: it reads the hook
payload's `agent_type` field (a genuine harness-provided identifier, e.g.
"chatman-ecosystem:rdf-observer" — not a self-reported tool list), looks
up that agent's own `tools:` frontmatter line, and denies the Bash call if
`bash_classify.is_mutation` says the command mutates and the agent's
allow-list does not grant `Write`/`Edit`. Agents with no recognized
`agent_type` (the primary session, or an agent outside this plugin) are
not fenced by this hook.

Reuses `bash_classify` — the module's own docstring records that three
independent copies of this exact classification once disagreed with each
other, so a fourth copy here would repeat that anti-pattern rather than
learn from it.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bash_classify import is_mutation  # noqa: E402

AGENTS_DIR = Path(__file__).resolve().parent.parent / "agents"


def read_hook_input() -> dict:
    raw = sys.stdin.read()
    if not raw.strip():
        return {}
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as error:
        raise SystemExit(f"invalid hook input: {error}") from error
    if not isinstance(value, dict):
        raise SystemExit("hook input must be a JSON object")
    return value


def agent_name(payload: dict) -> str | None:
    raw = payload.get("agent_type")
    if not isinstance(raw, str) or not raw:
        return None
    return raw.rsplit(":", 1)[-1]


def agent_tools(name: str) -> set[str] | None:
    agent_path = AGENTS_DIR / f"{name}.md"
    if not agent_path.exists():
        return None
    text = agent_path.read_text(encoding="utf-8")
    match = re.search(r"^tools:\s*(.+)$", text, re.MULTILINE)
    if not match:
        return None
    return {tool.strip() for tool in match.group(1).split(",") if tool.strip()}


def deny(reason: str) -> int:
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": reason,
                }
            }
        )
    )
    return 0


def main() -> int:
    payload = read_hook_input()
    if payload.get("hook_event_name") != "PreToolUse":
        return 0
    if payload.get("tool_name") != "Bash":
        return 0

    name = agent_name(payload)
    if name is None:
        return 0

    tools = agent_tools(name)
    if tools is None:
        # No tools: frontmatter for this agent (or an agent outside this
        # plugin) — nothing to enforce against.
        return 0
    if "Write" in tools or "Edit" in tools:
        # This agent (source-manufacturer) is already granted direct edit
        # tools; Bash writes are not a privilege escalation for it.
        return 0

    tool_input = payload.get("tool_input")
    command = tool_input.get("command") if isinstance(tool_input, dict) else None
    if not isinstance(command, str) or not is_mutation(command):
        return 0

    return deny(
        "BASH_WRITE_FENCE: "
        f"agent '{name}' declares tools: {', '.join(sorted(tools)) or '(none)'} "
        "— no Write/Edit grant — and this Bash command mutates. "
        "Gall Checkpoint 3 makes source-manufacturer the sole source editor; "
        "route this change through an allocation receipt, a candidate plan "
        "step, and source-manufacturer instead of writing through Bash."
    )


if __name__ == "__main__":
    raise SystemExit(main())
