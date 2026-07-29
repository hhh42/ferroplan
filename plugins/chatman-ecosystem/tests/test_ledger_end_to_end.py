"""CE-GALL-32's remaining gap: do the callers agree, not just the primitive?

`test_roots.py::test_project_key_is_identical_for_cwd_and_its_subdirectory`
proves `project_key`/`project_directory` hash a repository root and one of its
subdirectories identically. That is necessary but not sufficient: every
caller imports `project_directory` and calls it independently, so a caller
that keys off something else (a stale local variable, an import that shadows
`roots`, an argument computed before the anchor fix) would still diverge even
though the shared primitive is correct in isolation.

This file runs the actual scripts as subprocesses -- the way a Claude Code
hook or a terminal invokes them, not by importing their functions -- from two
different directories inside the same checkout (the repository root and
`plugins/chatman-ecosystem`), and checks that state written by one is visible
to another. It covers five of the six declared callers:

* `loop.py`     -- writes/reads `state.json` + `events.jsonl`
* `phase.py`    -- writes/reads `phase-state.json` + `phase-events.jsonl`
* `effective-phase.py` -- reads both of the above and projects them
* `event-summary.py`   -- writes `claude-events.jsonl` (no reader CLI)
* `actuation-intent.py` -- reads both state files to decide BRCE refusal

`grant-actuation.py` is the sixth. It additionally requires a live
ferroplan-mcp `verify_receipt` round trip to reach the code path that reads
`project_directory`, so it needs `needs_cargo` treatment and is deliberately
left for a follow-on rather than faked here.

The repository itself is the fixture: `project_root()` only anchors a
checkout that carries the Ferroplan marker (`crates/ferroplan-mcp/Cargo.toml`),
so a throwaway `git init` fixture does not exercise the anchor at all -- it
falls back to the pre-fix raw-realpath behavior instead of the code path this
file is supposed to cover. Running against the real checkout is what actually
exercises anchoring; `CLAUDE_PLUGIN_DATA` (set by the autouse `_isolate`
fixture) still keeps every write under `tmp_path`, so nothing here touches the
real `~/.claude` ledger.
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
    """`event-summary.py` has no reader CLI, so assert directly on the shared file."""
    root, subdir = _repo_root_and_subdir()

    payload = json.dumps({"hook_event_name": "SessionStart", "cwd": str(subdir)})
    summary = run_script("event-summary.py", stdin=payload, cwd=subdir)
    assert summary.returncode == 0, summary.stderr

    directory = roots.project_directory(str(root))
    events_path = directory / "claude-events.jsonl"
    assert events_path.is_file()
    lines = events_path.read_text(encoding="utf-8").splitlines()
    assert len(lines) == 1
    # The recorded `project` field is the raw observed cwd (the subdirectory);
    # what this test is actually proving is that the *file* landed in the one
    # ledger directory keyed by the anchored root, not a directory of its own.
    assert json.loads(lines[0])["project"] == str(subdir.resolve())


def test_actuation_intent_from_the_root_denies_on_a_pending_count_observed_in_the_subdirectory(
    run_script,
):
    """The BRCE fence must see the same pending frontier no matter which directory observed it."""
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
