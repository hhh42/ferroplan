#!/usr/bin/env python3
"""Claude Code hook ledger for the Chatman ecosystem plugin.

The ledger is not an execution authority. It records bounded observations of
repository mutation, tracks the admitted event frontier, and blocks publication
or session completion when the latest observations have not been reconciled
with a Ferroplan session receipt.
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import os
import re
import sys
import tempfile
import time
from collections.abc import Iterator
from pathlib import Path
from typing import Any

try:
    import fcntl  # type: ignore
except ImportError:  # pragma: no cover - Windows fallback
    fcntl = None

sys.path.insert(0, str(Path(__file__).resolve().parent))
# `_standing` is generated and imports only `enum`, so it is safe on the hook
# path, which must stay standard-library only -- the hooks are this plugin's
# only mechanical authority.
from _standing import DEFAULT as STANDING_DEFAULT  # noqa: E402
from _standing import Standing  # noqa: E402
from bash_classify import is_mutation as bash_is_mutation  # noqa: E402
from bash_classify import is_protected as bash_is_protected  # noqa: E402
from mcp_client import McpClient, McpToolError, tool_structured_result  # noqa: E402
from plugin_data import plugin_data_root as resolve_plugin_data_root  # noqa: E402
from roots import project_directory  # noqa: E402

STATE_SCHEMA = "urn:chatman:claude-code-loop-state:v1"
EVENT_SCHEMA = "urn:chatman:claude-code-observation:v1"

def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode(
        "utf-8"
    )


def sha256(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def plugin_data_root() -> Path:
    return resolve_plugin_data_root()


project_dir = project_directory


def default_state(cwd: str) -> dict[str, Any]:
    return {
        "schema": STATE_SCHEMA,
        "project": os.path.realpath(cwd),
        "event_count": 0,
        "admitted_event_count": 0,
        "plan_receipt": None,
        "plan_digest": None,
        "session_id": None,
        "standing": "UNKNOWN",
        "updated_at_unix_ms": 0,
    }


def load_state(directory: Path, cwd: str) -> dict[str, Any]:
    path = directory / "state.json"
    if not path.exists():
        return default_state(cwd)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return default_state(cwd)
    if value.get("schema") != STATE_SCHEMA:
        return default_state(cwd)
    return value


def atomic_write(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + "\n"
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=path.parent, delete=False
    ) as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())
        temporary = Path(handle.name)
    os.replace(temporary, path)


@contextlib.contextmanager
def state_lock(directory: Path) -> Iterator[None]:
    directory.mkdir(parents=True, exist_ok=True)
    lock_path = directory / "ledger.lock"
    with lock_path.open("a+b") as handle:
        if fcntl is not None:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        try:
            yield
        finally:
            if fcntl is not None:
                fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def read_hook_input() -> dict[str, Any]:
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


def bash_command(payload: dict[str, Any]) -> str:
    tool_input = payload.get("tool_input")
    if not isinstance(tool_input, dict):
        return ""
    command = tool_input.get("command")
    return command if isinstance(command, str) else ""


def is_mutation(payload: dict[str, Any]) -> bool:
    tool = payload.get("tool_name")
    if tool in {"Write", "Edit", "NotebookEdit"}:
        return True
    if tool != "Bash":
        return False
    command = bash_command(payload)
    # Local self-exemption, deliberately not in the shared module: the ledger's
    # own read-only subcommands must not be recorded as repository mutations.
    if "loop.py" in command and re.search(r"\b(?:admit|pending|status|monitor)\b", command):
        return False
    return bash_is_mutation(command)


def is_protected(payload: dict[str, Any]) -> bool:
    # No self-exemption here, by design.
    return payload.get("tool_name") == "Bash" and bash_is_protected(bash_command(payload))


def bounded_tool_observation(payload: dict[str, Any], sequence: int) -> dict[str, Any]:
    tool = str(payload.get("tool_name", ""))
    tool_input = payload.get("tool_input")
    if not isinstance(tool_input, dict):
        tool_input = {}

    surface: dict[str, Any] = {}
    if tool in {"Write", "Edit", "NotebookEdit"}:
        path = tool_input.get("file_path") or tool_input.get("notebook_path")
        if isinstance(path, str):
            surface["path"] = path
        for key in ("content", "old_string", "new_string", "new_source"):
            value = tool_input.get(key)
            if isinstance(value, str):
                surface[f"{key}_bytes"] = len(value.encode("utf-8"))
                surface[f"{key}_digest"] = hashlib.sha256(value.encode("utf-8")).hexdigest()
    elif tool == "Bash":
        command = bash_command(payload)
        surface["command_digest"] = hashlib.sha256(command.encode("utf-8")).hexdigest()
        description = tool_input.get("description")
        if isinstance(description, str):
            surface["description"] = description[:256]

    event = {
        "schema": EVENT_SCHEMA,
        "sequence": sequence,
        "session_id": payload.get("session_id"),
        "tool_use_id": payload.get("tool_use_id"),
        "hook_event": payload.get("hook_event_name"),
        "tool": tool,
        "surface": surface,
        "duration_ms": payload.get("duration_ms"),
        "failed": payload.get("hook_event_name") == "PostToolUseFailure",
        "observed_at_unix_ms": int(time.time() * 1000),
        "transport_digest_algorithm": "sha256",
    }
    event["transport_digest"] = sha256(event)
    return event


def append_event(payload: dict[str, Any]) -> None:
    cwd = str(payload.get("cwd") or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    directory = project_dir(cwd)
    with state_lock(directory):
        state = load_state(directory, cwd)
        sequence = int(state.get("event_count", 0)) + 1
        event = bounded_tool_observation(payload, sequence)
        with (directory / "events.jsonl").open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(event, sort_keys=True, separators=(",", ":")) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
        state["event_count"] = sequence
        state["standing"] = "PARTIAL_ALIVE"
        state["updated_at_unix_ms"] = int(time.time() * 1000)
        atomic_write(directory / "state.json", state)


def current_state(payload: dict[str, Any]) -> tuple[Path, dict[str, Any]]:
    cwd = str(payload.get("cwd") or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    directory = project_dir(cwd)
    with state_lock(directory):
        state = load_state(directory, cwd)
        if not (directory / "state.json").exists():
            atomic_write(directory / "state.json", state)
    return directory, state


def hook() -> int:
    payload = read_hook_input()
    event = str(payload.get("hook_event_name", ""))
    _, state = current_state(payload)
    pending = int(state.get("event_count", 0)) - int(state.get("admitted_event_count", 0))

    if event == "SessionStart":
        context = (
            "Chatman ecosystem control loop is active. Before source actuation, invoke "
            "the chatman-ecosystem:self-host skill. Persistent Ferroplan sessions preserve "
            "valid plan suffixes; CMCA allocates "
            "the admitted eight-node work frontier. Hook observations are proposals until "
            "their latest event frontier is bound to a BLAKE3 session receipt."
        )
        print(
            json.dumps(
                {
                    "hookSpecificOutput": {
                        "hookEventName": "SessionStart",
                        "additionalContext": context,
                    }
                }
            )
        )
        return 0

    if event == "PreToolUse" and is_protected(payload):
        if pending > 0 or not state.get("plan_receipt"):
            reason = (
                "BRCE_REFUSED: protected actuation is ahead of the admitted observation "
                f"frontier ({pending} pending event(s)). Consume the hook ledger, update "
                "the persistent Ferroplan session, run CMCA when selecting work, call "
                "session_think, then record its BLAKE3 receipt with loop.py admit."
            )
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

    if event in {"PostToolUse", "PostToolUseFailure"} and is_mutation(payload):
        append_event(payload)
        return 0

    if event == "Stop" and pending > 0 and not payload.get("stop_hook_active"):
        print(
            json.dumps(
                {
                    "decision": "block",
                    "reason": (
                        f"OBSERVATION_NOT_ADMITTED: {pending} repository mutation event(s) "
                        "occurred after the latest planning receipt. Read loop.py pending, "
                        "feed the observations to session_observe, retain or replan with "
                        "session_think, and bind the returned receipt using loop.py admit."
                    ),
                }
            )
        )
    return 0


def resolve_cli_project(project: str | None) -> tuple[str, Path]:
    cwd = os.path.realpath(project or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    return cwd, project_dir(cwd)


def pending(project: str | None) -> int:
    cwd, directory = resolve_cli_project(project)
    with state_lock(directory):
        state = load_state(directory, cwd)
        admitted = int(state.get("admitted_event_count", 0))
        events: list[dict[str, Any]] = []
        path = directory / "events.jsonl"
        if path.exists():
            for line in path.read_text(encoding="utf-8").splitlines():
                try:
                    value = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if int(value.get("sequence", 0)) > admitted:
                    events.append(value)
        print(
            json.dumps(
                {
                    "schema": "urn:chatman:claude-code-pending:v1",
                    "project": cwd,
                    "admitted_event_count": admitted,
                    "event_count": int(state.get("event_count", 0)),
                    "events": events,
                },
                sort_keys=True,
                indent=2,
            )
        )
    return 0


def verify_receipt_envelope(envelope_path: str, receipt: str) -> None:
    """Verify `receipt` against a real MCP envelope via `verify_receipt`.

    `envelope_path` of `-` reads the envelope JSON from stdin instead of a
    file, so a tool result already in hand can be piped straight in rather
    than transcribed to a throwaway file first (the gap CLAUDE.md calls out:
    a workaround this script forced was a defect in this script, not a
    permanent hand step).

    Raises SystemExit (same message style as the existing hex-format check)
    if the envelope cannot be read/parsed, the declared receipt does not
    match the envelope's own `receipt` field, the MCP call errors, or the
    server reports the receipt invalid. A format-valid but unverified receipt
    is never treated as success.
    """
    try:
        raw = sys.stdin.read() if envelope_path == "-" else Path(envelope_path).read_text(
            encoding="utf-8"
        )
        envelope = json.loads(raw)
    except OSError as error:
        raise SystemExit(f"cannot read --envelope {envelope_path}: {error}") from error
    except json.JSONDecodeError as error:
        raise SystemExit(f"--envelope {envelope_path} is not valid JSON: {error}") from error

    declared_receipt = envelope.get("receipt") if isinstance(envelope, dict) else None
    if not isinstance(declared_receipt, str) or declared_receipt.lower() != receipt.lower():
        raise SystemExit(
            "--receipt does not match the `receipt` field declared in --envelope"
        )

    try:
        with McpClient() as client:
            result = client.call_tool("verify_receipt", {"envelope": envelope})
    except McpToolError as error:
        raise SystemExit(f"receipt verification failed: {error}") from error

    verification = tool_structured_result(result)
    if not isinstance(verification, dict) or not verification.get("valid"):
        raise SystemExit(f"receipt verification failed: {json.dumps(verification)}")


def admit(args: argparse.Namespace) -> int:
    if not re.fullmatch(r"[0-9a-fA-F]{64}", args.receipt):
        raise SystemExit("receipt must be a 64-character hexadecimal BLAKE3 digest")
    if args.plan_digest and not re.fullmatch(r"[0-9a-fA-F]{64}", args.plan_digest):
        raise SystemExit("plan digest must be a 64-character hexadecimal digest")
    verify_receipt_envelope(args.envelope, args.receipt)
    cwd, directory = resolve_cli_project(args.project)
    with state_lock(directory):
        state = load_state(directory, cwd)
        state["admitted_event_count"] = int(state.get("event_count", 0))
        state["plan_receipt"] = args.receipt.lower()
        state["plan_digest"] = args.plan_digest.lower() if args.plan_digest else None
        state["session_id"] = args.session
        state["standing"] = args.standing
        state["updated_at_unix_ms"] = int(time.time() * 1000)
        atomic_write(directory / "state.json", state)
        print(json.dumps(state, sort_keys=True, indent=2))
    return 0


def status(project: str | None) -> int:
    cwd, directory = resolve_cli_project(project)
    with state_lock(directory):
        state = load_state(directory, cwd)
        state["pending_events"] = int(state.get("event_count", 0)) - int(
            state.get("admitted_event_count", 0)
        )
        print(json.dumps(state, sort_keys=True, indent=2))
    return 0


def monitor(project: str | None) -> int:
    cwd, directory = resolve_cli_project(project)
    previous: tuple[int, int] | None = None
    while True:
        with state_lock(directory):
            state = load_state(directory, cwd)
        current = (
            int(state.get("event_count", 0)),
            int(state.get("admitted_event_count", 0)),
        )
        if current != previous:
            print(
                json.dumps(
                    {
                        "project": cwd,
                        "event_count": current[0],
                        "admitted_event_count": current[1],
                        "pending_events": current[0] - current[1],
                        "standing": state.get("standing"),
                    },
                    sort_keys=True,
                ),
                flush=True,
            )
            previous = current
        time.sleep(1.0)


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    sub = root.add_subparsers(dest="command", required=True)
    sub.add_parser("hook")
    for name in ("pending", "status", "monitor"):
        command = sub.add_parser(name)
        command.add_argument("--project")
    command = sub.add_parser("admit")
    command.add_argument("--project")
    command.add_argument("--session", required=True)
    command.add_argument("--receipt", required=True)
    command.add_argument(
        "--envelope",
        required=True,
        help="Path to the JSON admission envelope returned by bind_plan_receipt/"
        "bind_allocation_receipt, whose `receipt` field must equal --receipt. "
        "Pass `-` to read the envelope JSON from stdin instead of a file. "
        "Verified against the ferroplan-mcp verify_receipt tool before admission.",
    )
    command.add_argument("--plan-digest")
    command.add_argument(
        "--standing",
        # Projected from ontology/chatman-ecosystem.ttl via scripts/generate.py.
        # This list was four values while the checkpoint doc used seven, so
        # BLOCKED and UNSUPPORTED could be claimed but never recorded. A
        # standing that cannot be recorded in the ledger is not a standing.
        choices=tuple(Standing),
        default=str(STANDING_DEFAULT),
    )
    return root


def main() -> int:
    args = parser().parse_args()
    if args.command == "hook":
        return hook()
    if args.command == "pending":
        return pending(args.project)
    if args.command == "admit":
        return admit(args)
    if args.command == "status":
        return status(args.project)
    if args.command == "monitor":
        return monitor(args.project)
    raise SystemExit(f"unsupported command: {args.command}")


if __name__ == "__main__":
    raise SystemExit(main())
