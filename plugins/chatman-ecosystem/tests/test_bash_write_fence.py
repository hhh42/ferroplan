"""End-to-end tests for the Bash-write fence (CE-GALL-35).

`bash-write-fence.py` is a standalone hook script (hyphenated name, so it is
not importable as a module) invoked exactly the way Claude Code invokes it:
as a subprocess with a JSON `PreToolUse` payload on stdin. These tests
exercise it that way rather than importing internals, so a refactor that
keeps the external contract intact cannot break them for the wrong reason.

Covers all 7 non-manufacturing agents individually -- a live nested-session
probe only checked 2 of them (`rdf-observer`, `config-law-architect`) before
this test existed; that manual-probe gap is what this file closes.
"""

from __future__ import annotations

import json

import pytest

NON_MANUFACTURING_AGENTS = [
    "cmca-allocator",
    "config-law-architect",
    "ecosystem-controller",
    "ferroplan-planner",
    "independent-validator",
    "rdf-observer",
    "receipt-auditor",
]


def _deny_reason(proc):
    if not proc.stdout.strip():
        return None
    emitted = json.loads(proc.stdout)
    return emitted.get("hookSpecificOutput", {}).get("permissionDecisionReason")


@pytest.mark.parametrize("agent", NON_MANUFACTURING_AGENTS)
def test_non_manufacturing_agent_is_denied_a_bash_write(agent, hook_event, run_script, tmp_path):
    target = tmp_path / "f.txt"
    payload = hook_event(
        "PreToolUse",
        tool_name="Bash",
        agent_type=f"chatman-ecosystem:{agent}",
        tool_input={"command": f"echo hi > {target}"},
        cwd=str(tmp_path),
    )
    proc = run_script("bash-write-fence.py", stdin=payload)
    assert proc.returncode == 0, proc.stderr
    reason = _deny_reason(proc)
    assert reason is not None and reason.startswith("BASH_WRITE_FENCE:"), agent
    assert not target.exists()


def test_source_manufacturer_is_not_fenced(hook_event, run_script, tmp_path):
    """Positive control: the sole source editor is not caught by this fence."""
    target = tmp_path / "f.txt"
    payload = hook_event(
        "PreToolUse",
        tool_name="Bash",
        agent_type="chatman-ecosystem:source-manufacturer",
        tool_input={"command": f"echo hi > {target}"},
        cwd=str(tmp_path),
    )
    proc = run_script("bash-write-fence.py", stdin=payload)
    assert proc.returncode == 0, proc.stderr
    assert _deny_reason(proc) is None


def test_read_only_bash_is_not_denied(hook_event, run_script, tmp_path):
    """The fence must not break the read-only commands these agents need."""
    payload = hook_event(
        "PreToolUse",
        tool_name="Bash",
        agent_type="chatman-ecosystem:rdf-observer",
        tool_input={"command": "git status --porcelain 2>&1 | head -50"},
        cwd=str(tmp_path),
    )
    proc = run_script("bash-write-fence.py", stdin=payload)
    assert proc.returncode == 0, proc.stderr
    assert _deny_reason(proc) is None


def test_unrecognized_agent_type_is_not_fenced(hook_event, run_script, tmp_path):
    """No agent_type (primary session) or an agent outside this plugin: no-op."""
    target = tmp_path / "f.txt"
    payload = hook_event(
        "PreToolUse",
        tool_name="Bash",
        tool_input={"command": f"echo hi > {target}"},
        cwd=str(tmp_path),
    )
    proc = run_script("bash-write-fence.py", stdin=payload)
    assert proc.returncode == 0, proc.stderr
    assert _deny_reason(proc) is None
