#!/usr/bin/env python3
"""PreToolUse hook enforcing the Chatman generated-artifact ownership registry.

This is a bounded worktree guard, not a cryptographic generation receipt. It
refuses direct edits to registered projections unless at least one declared
canonical owner is already changed in the same repository worktree. Exact
generator identity and source/projection digests remain validation and receipt
obligations.
"""

from __future__ import annotations

import fnmatch
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

REGISTRY_PATH = "plugins/chatman-ecosystem/profiles/artifact-ownership.json"
LEGACY_GENERATED = {
    "crates/ferroplan-wasm/src/lib.rs": [
        "plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl"
    ]
}


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


def load_registry(root: Path) -> list[tuple[str, list[str]]]:
    path = root / REGISTRY_PATH
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise SystemExit(f"cannot read artifact ownership registry {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise SystemExit(f"invalid artifact ownership registry {path}: {error}") from error

    entries: list[tuple[str, list[str]]] = []
    artifacts = document.get("artifacts") if isinstance(document, dict) else None
    if not isinstance(artifacts, list):
        raise SystemExit("artifact ownership registry must contain an artifacts array")
    for item in artifacts:
        if not isinstance(item, dict):
            continue
        pattern = item.get("path")
        owner = item.get("owner")
        if not isinstance(pattern, str) or not pattern:
            continue
        owners = [owner] if isinstance(owner, str) else []
        extra = item.get("additional_owners")
        if isinstance(extra, list):
            owners.extend(value for value in extra if isinstance(value, str))
        if owners:
            entries.append((pattern, owners))
    entries.extend((path, owners) for path, owners in LEGACY_GENERATED.items())
    return entries


def changed_paths(root: Path) -> set[str]:
    commands = [
        ["git", "diff", "--name-only", "--relative", "HEAD"],
        ["git", "diff", "--name-only", "--relative", "--cached"],
        ["git", "ls-files", "--others", "--exclude-standard"],
    ]
    changed: set[str] = set()
    for command in commands:
        try:
            result = subprocess.run(
                command,
                cwd=root,
                check=False,
                capture_output=True,
                text=True,
                timeout=5,
            )
        except (OSError, subprocess.TimeoutExpired):
            continue
        if result.returncode == 0:
            changed.update(
                line.strip().replace("\\", "/")
                for line in result.stdout.splitlines()
                if line.strip()
            )
    return changed


def matching_owners(target: str, entries: list[tuple[str, list[str]]]) -> list[str]:
    owners: list[str] = []
    for pattern, declared in entries:
        if fnmatch.fnmatchcase(target, pattern):
            owners.extend(declared)
    return sorted(set(owners))


def main() -> int:
    payload = read_hook_input()
    if payload.get("hook_event_name") != "PreToolUse":
        return 0
    if payload.get("tool_name") not in {"Edit", "Write"}:
        return 0

    tool_input = payload.get("tool_input")
    if not isinstance(tool_input, dict):
        return 0
    file_path = tool_input.get("file_path")
    if not isinstance(file_path, str) or not file_path:
        return 0

    cwd = str(payload.get("cwd") or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    root = Path(cwd).resolve()
    target_abs = Path(file_path)
    if not target_abs.is_absolute():
        target_abs = root / target_abs
    try:
        target_rel = target_abs.resolve().relative_to(root).as_posix()
    except ValueError:
        return 0

    try:
        entries = load_registry(root)
    except SystemExit as error:
        return deny(f"UNKNOWN_OWNERSHIP_REFUSED: {error}")

    owners = matching_owners(target_rel, entries)
    if not owners:
        return 0

    changed = changed_paths(root)
    changed_owners = [owner for owner in owners if owner in changed]
    if changed_owners:
        return 0

    rendered_owners = ", ".join(owners)
    return deny(
        "HAND_CODED_GENERATED_OUTPUT: "
        f"{target_rel} is a registered projection owned by {rendered_owners}. "
        "No declared owner is changed in this worktree. Edit the canonical owner, "
        "regenerate every dependent projection, then retry. This guard establishes "
        "owner-first ordering only; exact generator identity and projection digests "
        "must still be validated and receipted."
    )


if __name__ == "__main__":
    raise SystemExit(main())
