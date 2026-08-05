#!/usr/bin/env python3
"""Record bounded Claude lifecycle candidates and summarize parallel tool batches."""

from __future__ import annotations

import contextlib
import hashlib
import json
import os
import sys
import time
from collections.abc import Iterator
from pathlib import Path
from typing import Any

try:
    import fcntl  # type: ignore
except ImportError:  # pragma: no cover
    fcntl = None

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bash_classify import is_mutation as bash_is_mutation  # noqa: E402

try:
    from plugin_data import plugin_data_root as resolve_plugin_data_root
except ImportError:
    resolve_plugin_data_root = None
from roots import project_directory  # noqa: E402

SCHEMA = "urn:chatman:claude-code-lifecycle-candidate:v1"


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode(
        "utf-8"
    )


def digest(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def plugin_data_root() -> Path:
    if resolve_plugin_data_root is not None:
        return resolve_plugin_data_root()
    configured = os.environ.get("CLAUDE_PLUGIN_DATA")
    if configured:
        return Path(configured)
    return Path.home() / ".claude" / "plugins" / "data" / "chatman-ecosystem"


@contextlib.contextmanager
def lock(directory: Path) -> Iterator[None]:
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / "claude-events.lock").open("a+b") as handle:
        if fcntl is not None:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        try:
            yield
        finally:
            if fcntl is not None:
                fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def read_input() -> dict[str, Any]:
    raw = sys.stdin.read()
    if not raw.strip():
        return {}
    value = json.loads(raw)
    if not isinstance(value, dict):
        raise SystemExit("hook input must be an object")
    return value


def bounded_tool_call(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        return {"tool_name": "", "tool_use_id": None, "mutating": False}
    tool_name = str(value.get("tool_name", ""))
    tool_input = value.get("tool_input")
    if not isinstance(tool_input, dict):
        tool_input = {}
    mutating = tool_name in {"Write", "Edit", "NotebookEdit"}
    surface: dict[str, Any] = {}
    if tool_name == "Bash":
        command = tool_input.get("command")
        if isinstance(command, str):
            surface["command_digest"] = hashlib.sha256(command.encode("utf-8")).hexdigest()
            mutating = bash_is_mutation(command)
    else:
        path = tool_input.get("file_path") or tool_input.get("notebook_path")
        if isinstance(path, str):
            surface["path_digest"] = hashlib.sha256(
                os.path.realpath(path).encode("utf-8")
            ).hexdigest()
    return {
        "tool_name": tool_name,
        "tool_use_id": value.get("tool_use_id"),
        "mutating": mutating,
        "surface": surface,
    }


def candidate(payload: dict[str, Any]) -> dict[str, Any]:
    event = str(payload.get("hook_event_name", ""))
    project = str(payload.get("cwd") or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    result: dict[str, Any] = {
        "schema": SCHEMA,
        "event": event,
        "project": os.path.realpath(project),
        "session_id": payload.get("session_id"),
        "observed_at_unix_ms": int(time.time() * 1000),
    }
    if event == "PostToolBatch":
        calls = [bounded_tool_call(item) for item in payload.get("tool_calls", [])]
        result["tool_calls"] = calls
        result["tool_count"] = len(calls)
        result["mutating_tool_count"] = sum(1 for item in calls if item["mutating"])
    elif event == "ConfigChange":
        source = payload.get("source") or payload.get("config_file") or payload.get("file_path")
        if isinstance(source, str):
            result["config_path_digest"] = hashlib.sha256(
                os.path.realpath(source).encode("utf-8")
            ).hexdigest()
    elif event in {"WorktreeCreate", "WorktreeRemove"}:
        path = payload.get("worktree_path") or payload.get("path")
        if isinstance(path, str):
            result["worktree_path_digest"] = hashlib.sha256(
                os.path.realpath(path).encode("utf-8")
            ).hexdigest()
    result["candidate_digest"] = digest(result)
    return result


def append(value: dict[str, Any]) -> None:
    directory = project_directory(str(value["project"]))
    with lock(directory):
        with (directory / "claude-events.jsonl").open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")
            handle.flush()
            os.fsync(handle.fileno())


def main() -> int:
    payload = read_input()
    value = candidate(payload)
    append(value)
    if value["event"] == "PostToolBatch":
        mutating = int(value.get("mutating_tool_count", 0))
        context = (
            f"Chatman batch candidate {value['candidate_digest']}: "
            f"{value.get('tool_count', 0)} tool call(s), {mutating} mutation candidate(s). "
            "Mutation candidates remain observations until the frontier is admitted."
        )
        print(
            json.dumps(
                {
                    "hookSpecificOutput": {
                        "hookEventName": "PostToolBatch",
                        "additionalContext": context,
                    }
                }
            )
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
