"""CE-GALL-32's remaining gap: do the callers agree, not just the primitive?

This file runs the actual scripts as subprocesses from two different directories
inside the same checkout and checks that state written by one is visible to
another. It covers loop.py, phase.py, effective-phase.py, event-summary.py,
and actuation-intent.py. grant-actuation.py remains a needs-cargo follow-on.
"""

from __future__ import annotations

import json

import roots


def _mutation_payload(cwd, command: str = "git commit -am wip") -> str:
    return json.dumps(
        {
            "hook_event_name": "PostToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": command},
            "cwd": str(cwd),
        }
    )


def _repo_root_and_subdir():
    repo_root = roots.project_root()
    assert repo_root is not None, "this checkout must resolve a project root"
    subdir = repo_root / "plugins" / "chatman-ecosystem"
    assert subdir.is_dir()
    return repo_root, subdir


def test_loop_and_phase_hooks_fired_from_the_subdirectory_are_visible_at_the_root(run_script):
    root, subdir = _repo_root_and_subdir()
    loop_hook = run_script("loop.py", "hook", stdin=_mutation_payload(subdir), cwd=subdir)
    assert loop_hook.returncode == 0, loop_hook.stderr
    phase_hook = run_script("phase.py", "hook", stdin=_mutation_payload(subdir), cwd=subdir)
    assert phase_hook.returncode == 0, phase_hook.stderr
    loop_status = run_script("loop.py", "status", "--project", str(root), cwd=root)
    assert loop_status.returncode == 0, loop_status.stderr
    loop_state = json.loads(loop_status.stdout)
    assert loop_state["event_count"] == 1
    assert loop_state["pending_events"] == 1
    phase_status = run_script("phase.py", "status", "--project", str(root), cwd=root)
    assert phase_status.returncode == 0, phase_status.stderr
    phase_state = json.loads(phase_status.stdout)
    assert phase_state["transition_count"] == 1
    assert phase_state["vector"]["drift"] == "drifted"


def test_effective_phase_read_from_the_root_sees_events_written_from_the_subdirectory(run_script):
    root, subdir = _repo_root_and_subdir()
    loop_hook = run_script("loop.py", "hook", stdin=_mutation_payload(subdir), cwd=subdir)
    assert loop_hook.returncode == 0, loop_hook.stderr
    effective = run_script("effective-phase.py", "--project", str(root), cwd=root)
    assert effective.returncode == 0, effective.stderr
    projection = json.loads(effective.stdout)
    assert projection["event_count"] == 1
    assert projection["pending_event_count"] == 1
    assert projection["effective_vector"]["drift"] == "drifted"


def test_event_summary_from_the_subdirectory_writes_where_loop_reads_from_the_root(run_script):
    root, subdir = _repo_root_and_subdir()
    payload = json.dumps({"hook_event_name": "SessionStart", "cwd": str(subdir)})
    summary = run_script("event-summary.py", stdin=payload, cwd=subdir)
    assert summary.returncode == 0, summary.stderr
    directory = roots.project_directory(str(root))
    events_path = directory / "claude-events.jsonl"
    assert events_path.is_file()
    lines = events_path.read_text(encoding="utf-8").splitlines()
    assert len(lines) == 1
    assert json.loads(lines[0])["project"] == str(subdir.resolve())


def test_actuation_intent_from_the_root_denies_on_a_pending_count_observed_in_the_subdirectory(
    run_script,
):
    root, subdir = _repo_root_and_subdir()
    loop_hook = run_script("loop.py", "hook", stdin=_mutation_payload(subdir), cwd=subdir)
    assert loop_hook.returncode == 0, loop_hook.stderr
    intent_payload = json.dumps(
        {
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": "git push origin main"},
            "cwd": str(root),
        }
    )
    intent = run_script("actuation-intent.py", stdin=intent_payload, cwd=root)
    assert intent.returncode == 0, intent.stderr
    decision = json.loads(intent.stdout)
    reason = decision["hookSpecificOutput"]["permissionDecisionReason"]
    assert "1 observation event(s) remain unadmitted" in reason
    directory = roots.project_directory(str(subdir))
    intents = list((directory / "intents").glob("*.json"))
    assert len(intents) == 1
