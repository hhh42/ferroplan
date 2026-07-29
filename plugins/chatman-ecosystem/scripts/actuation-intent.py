#!/usr/bin/env python3
"""Manufacture structured intents for protected Claude Code Bash actuation."""

from __future__ import annotations

import hashlib
import json
import os
import re
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))
try:
    from plugin_data import plugin_data_root as resolve_plugin_data_root
except ImportError:
    resolve_plugin_data_root = None
from roots import project_directory  # noqa: E402

SCHEMA = "urn:chatman:actuation-intent:v1"
GRANT_SCHEMA = "urn:chatman:derived-execution-grant:v1"
RECEIPT_RE = re.compile(r"^[0-9a-f]{64}$")
COLLAPSED_VECTOR = {
    "epistemic": "observed",
    "allocation": "unallocated",
    "planning": "unplanned",
    "actuation": "sealed",
    "drift": "drifted",
    "conformance": "unknown",
}
REQUIRED_VECTOR = {
    "epistemic": "admitted",
    "allocation": "allocated",
    "planning": "validated",
    "actuation": "publishable",
    "drift": "stable",
    "conformance": "conformant",
}
OPERATIONS: list[tuple[str, re.Pattern[str], str]] = [
    ("git-push", re.compile(r"(?:^|[;&|]\s*)git\s+push\b", re.I), "conditionally-reversible"),
    ("git-merge", re.compile(r"(?:^|[;&|]\s*)git\s+merge\b", re.I), "conditionally-reversible"),
    ("git-rebase", re.compile(r"(?:^|[;&|]\s*)git\s+rebase\b", re.I), "conditionally-reversible"),
    ("git-reset-hard", re.compile(r"(?:^|[;&|]\s*)git\s+reset\s+--hard\b", re.I), "conditionally-reversible"),
    ("git-clean-force", re.compile(r"(?:^|[;&|]\s*)git\s+clean\s+-[^\n;&|]*f", re.I), "irreversible"),
    ("pull-request-create", re.compile(r"(?:^|[;&|]\s*)gh\s+pr\s+create\b", re.I), "reversible"),
    ("pull-request-merge", re.compile(r"(?:^|[;&|]\s*)gh\s+pr\s+merge\b", re.I), "conditionally-reversible"),
    ("cargo-publish", re.compile(r"(?:^|[;&|]\s*)cargo\s+publish\b", re.I), "irreversible"),
    ("npm-publish", re.compile(r"(?:^|[;&|]\s*)npm\s+publish\b", re.I), "irreversible"),
    ("recursive-forced-delete", re.compile(r"(?:^|[;&|]\s*)rm\s+-[^\n;&|]*r[^\n;&|]*f", re.I), "irreversible"),
    (
        "state-changing-http",
        re.compile(
            r"(?:^|[;&|]\s*)curl\b[^\n;&|]*(?:-X\s*(?:POST|PUT|PATCH|DELETE)|--request\s*(?:POST|PUT|PATCH|DELETE))",
            re.I,
        ),
        "conditionally-reversible",
    ),
]


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


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


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


def read_input() -> dict[str, Any]:
    raw = sys.stdin.read()
    if not raw.strip():
        return {}
    value = json.loads(raw)
    if not isinstance(value, dict):
        raise SystemExit("hook input must be an object")
    return value


def bash_command(payload: dict[str, Any]) -> str:
    tool_input = payload.get("tool_input")
    if not isinstance(tool_input, dict):
        return ""
    command = tool_input.get("command")
    return command if isinstance(command, str) else ""


def classify(command: str) -> tuple[str, str] | None:
    for operation, pattern, reversibility in OPERATIONS:
        if pattern.search(command):
            return operation, reversibility
    return None


def phase_state(directory: Path) -> tuple[dict[str, str], int, dict[str, Any], dict[str, Any]]:
    phase = load_json(directory / "phase-state.json")
    loop = load_json(directory / "state.json")
    canonical = phase.get("vector")
    if not isinstance(canonical, dict):
        canonical = dict(COLLAPSED_VECTOR)
    canonical = {str(key): str(value) for key, value in canonical.items()}
    event_count = int(loop.get("event_count", 0))
    admitted = int(loop.get("admitted_event_count", 0))
    pending = max(0, event_count - admitted)
    effective = dict(COLLAPSED_VECTOR if pending else canonical)
    return effective, pending, phase, loop


def valid_grant(grant: dict[str, Any], intent_digest: str, now_ms: int) -> bool:
    if grant.get("schema") != GRANT_SCHEMA or grant.get("granted") is not True:
        return False
    if grant.get("intent_digest") != intent_digest:
        return False
    receipt = grant.get("receipt")
    if not isinstance(receipt, str) or not RECEIPT_RE.fullmatch(receipt):
        return False
    expires = grant.get("expires_at_unix_ms")
    if expires is not None and (not isinstance(expires, int) or expires < now_ms):
        return False
    return True


def deny(reason: str, intent: dict[str, Any], path: Path) -> None:
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": (
                        f"BRCE_REFUSED: {reason}. Structured intent "
                        f"{intent['intent_digest']} recorded at {path}. Complete validation, "
                        "receipt audit, and grant derivation; do not bypass the fence."
                    ),
                }
            }
        )
    )


def main() -> int:
    payload = read_input()
    if payload.get("hook_event_name") != "PreToolUse" or payload.get("tool_name") != "Bash":
        return 0
    command = bash_command(payload)
    classified = classify(command)
    if classified is None:
        return 0
    operation, reversibility = classified
    project = os.path.realpath(
        str(payload.get("cwd") or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    )
    directory = project_directory(project)
    effective, pending, phase, loop = phase_state(directory)
    created = int(time.time() * 1000)
    body: dict[str, Any] = {
        "schema": SCHEMA,
        "project": project,
        "session_id": payload.get("session_id"),
        "tool_use_id": payload.get("tool_use_id"),
        "operation": operation,
        "command_digest": hashlib.sha256(command.encode("utf-8")).hexdigest(),
        "target_digest": None,
        "effective_phase": effective,
        "required_phase": REQUIRED_VECTOR,
        "pending_event_count": pending,
        "predecessor_receipt": phase.get("receipt") or loop.get("plan_receipt"),
        "reversibility": reversibility,
        "created_at_unix_ms": created,
    }
    body["intent_digest"] = digest(body)
    intent_path = directory / "intents" / f"{body['intent_digest']}.json"
    atomic_write(intent_path, body)
    grant = load_json(directory / "grants" / f"{body['intent_digest']}.json")
    if pending:
        deny(f"{pending} observation event(s) remain unadmitted", body, intent_path)
        return 0
    if effective != REQUIRED_VECTOR:
        deny("effective phase is below the protected-actuation requirement", body, intent_path)
        return 0
    if not valid_grant(grant, str(body["intent_digest"]), created):
        deny("no valid derived execution grant matches this exact intent", body, intent_path)
        return 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
