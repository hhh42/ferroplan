#!/usr/bin/env python3
"""Combinatorial phase-state runtime for the Chatman Claude Code plugin.

This process is a transport/state projection, not an authority. Authoritative
allocation, plan, validator, and admission claims come from MCP receipts. The
runtime stores the current product-state vector, validates declared transition
laws, computes the active capability/agent/skill union, and injects that state
into Claude Code lifecycle events.
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import itertools
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
except ImportError:  # pragma: no cover
    fcntl = None

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bash_classify import is_mutation as bash_is_mutation  # noqa: E402
from mcp_client import McpClient, McpToolError, tool_structured_result  # noqa: E402
from plugin_data import plugin_data_root as resolve_plugin_data_root  # noqa: E402
from roots import project_directory  # noqa: E402

STATE_SCHEMA = "urn:chatman:claude-code-phase-state:v1"
EVENT_SCHEMA = "urn:chatman:claude-code-phase-event:v1"
RECEIPT_RE = re.compile(r"^[0-9a-fA-F]{64}$")


def plugin_root() -> Path:
    configured = os.environ.get("CLAUDE_PLUGIN_ROOT")
    if configured:
        return Path(configured)
    return Path(__file__).resolve().parent.parent


def profile_path() -> Path:
    return plugin_root() / "profiles" / "phase-space.json"


def load_profile() -> dict[str, Any]:
    value = json.loads(profile_path().read_text(encoding="utf-8"))
    if value.get("schema") != "urn:chatman:claude-code-phase-space:v1":
        raise SystemExit("unsupported phase-space profile")
    return value


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode(
        "utf-8"
    )


def transport_digest(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def plugin_data_root() -> Path:
    return resolve_plugin_data_root()


@contextlib.contextmanager
def state_lock(directory: Path) -> Iterator[None]:
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / "phase.lock").open("a+b") as handle:
        if fcntl is not None:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        try:
            yield
        finally:
            if fcntl is not None:
                fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


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


def initial_vector(profile: dict[str, Any]) -> dict[str, str]:
    return {
        name: str(dimension["initial"])
        for name, dimension in profile["dimensions"].items()
    }


def default_state(cwd: str, profile: dict[str, Any]) -> dict[str, Any]:
    vector = initial_vector(profile)
    return {
        "schema": STATE_SCHEMA,
        "project": os.path.realpath(cwd),
        "vector": vector,
        "phase_digest": transport_digest(vector),
        "transition_count": 0,
        "receipt": None,
        "reason": "initialized",
        "updated_at_unix_ms": int(time.time() * 1000),
    }


def load_state(directory: Path, cwd: str, profile: dict[str, Any]) -> dict[str, Any]:
    path = directory / "phase-state.json"
    if not path.exists():
        return default_state(cwd, profile)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return default_state(cwd, profile)
    if value.get("schema") != STATE_SCHEMA:
        return default_state(cwd, profile)
    return value


def validate_vector(profile: dict[str, Any], vector: dict[str, str]) -> list[str]:
    violations: list[str] = []
    dimensions = profile["dimensions"]
    for name, dimension in dimensions.items():
        value = vector.get(name)
        if value not in dimension["states"]:
            violations.append(f"unknown {name} state `{value}`")

    for invariant in profile.get("invariants", []):
        when = invariant.get("when", {})
        if not all(vector.get(key) == value for key, value in when.items()):
            continue
        required = invariant.get("requires", {})
        for key, value in required.items():
            if vector.get(key) != value:
                violations.append(
                    f"{invariant['id']}: requires {key}={value}, got {vector.get(key)}"
                )
        alternatives = invariant.get("requires_any", [])
        if alternatives and not any(
            all(vector.get(key) == value for key, value in candidate.items())
            for candidate in alternatives
        ):
            rendered = " or ".join(
                "+".join(f"{key}={value}" for key, value in candidate.items())
                for candidate in alternatives
            )
            violations.append(f"{invariant['id']}: requires one of {rendered}")
        forbidden = invariant.get("forbids", {})
        for key, value in forbidden.items():
            if vector.get(key) == value:
                violations.append(f"{invariant['id']}: forbids {key}={value}")
    return violations


def active_projection(profile: dict[str, Any], vector: dict[str, str]) -> dict[str, Any]:
    capabilities: set[str] = set()
    agents: set[str] = set()
    skills: set[str] = set()
    for dimension_name, state_name in vector.items():
        state = profile["dimensions"][dimension_name]["states"][state_name]
        capabilities.update(state.get("capabilities", []))
        agents.update(state.get("agents", []))
        skills.update(state.get("skills", []))
    return {
        "capabilities": sorted(capabilities),
        "agents": sorted(agents),
        "skills": sorted(skills),
    }


def allowed_transition(
    profile: dict[str, Any], dimension: str, source: str, target: str
) -> bool:
    if source == target:
        return True
    transitions = profile["dimensions"][dimension].get("transitions", [])
    return [source, target] in transitions


def parse_assignments(values: list[str]) -> dict[str, str]:
    assignments: dict[str, str] = {}
    for value in values:
        if "=" not in value:
            raise SystemExit(f"phase assignment must be dimension=value: `{value}`")
        dimension, state = value.split("=", 1)
        if not dimension or not state:
            raise SystemExit(f"invalid phase assignment: `{value}`")
        assignments[dimension] = state
    return assignments


def append_event(directory: Path, event: dict[str, Any]) -> None:
    with (directory / "phase-events.jsonl").open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(event, sort_keys=True, separators=(",", ":")) + "\n")
        handle.flush()
        os.fsync(handle.fileno())


def resolve_project(project: str | None) -> tuple[str, Path]:
    cwd = os.path.realpath(project or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    return cwd, project_directory(cwd)


def combination_census(profile: dict[str, Any]) -> dict[str, Any]:
    """Classify every point in the raw product space against the invariants.

    Both counts are derived. `raw` is the product of the dimension sizes, not
    the profile's `raw_combination_count` literal -- that literal is reported
    separately as `declared_raw` so a drift between the two is visible instead
    of being silently believed.

    The lawful count is the number that actually describes the system, and it
    was written down nowhere: an invariant carrying an unread predicate key
    enforced nothing for as long as nobody could see that the count had not
    moved.
    """
    names = list(profile["dimensions"])
    state_lists = [list(profile["dimensions"][name]["states"]) for name in names]
    raw = 0
    lawful = 0
    per_state: dict[str, dict[str, int]] = {
        name: dict.fromkeys(profile["dimensions"][name]["states"], 0) for name in names
    }
    for combo in itertools.product(*state_lists):
        vector = dict(zip(names, combo, strict=True))
        raw += 1
        if validate_vector(profile, vector):
            continue
        lawful += 1
        for name, value in vector.items():
            per_state[name][value] += 1
    declared = profile.get("raw_combination_count")
    return {
        "raw": raw,
        "lawful": lawful,
        "ratio": round(lawful / raw, 4) if raw else 0.0,
        "declared_raw": declared,
        "declared_raw_matches": declared == raw,
        "lawful_per_state": per_state,
    }


def status(project: str | None) -> int:
    profile = load_profile()
    cwd, directory = resolve_project(project)
    with state_lock(directory):
        state = load_state(directory, cwd, profile)
        violations = validate_vector(profile, state["vector"])
        result = {
            **state,
            "valid": not violations,
            "violations": violations,
            "active": active_projection(profile, state["vector"]),
            "raw_combination_count": profile["raw_combination_count"],
            "census": combination_census(profile),
        }
        if not (directory / "phase-state.json").exists():
            atomic_write(directory / "phase-state.json", state)
    print(json.dumps(result, sort_keys=True, indent=2))
    return 0 if not violations else 2


def verify_receipt_envelope(envelope_path: str, receipt: str) -> None:
    """Verify `receipt` against a real MCP envelope via `verify_receipt`.

    Raises SystemExit (same message style as the existing hex-format check)
    if the envelope cannot be read/parsed, the declared receipt does not
    match the envelope's own `receipt` field, the MCP call errors, or the
    server reports the receipt invalid. A format-valid but unverified receipt
    is never treated as success.
    """
    try:
        envelope = json.loads(Path(envelope_path).read_text(encoding="utf-8"))
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


def transition(args: argparse.Namespace) -> int:
    profile = load_profile()
    assignments = parse_assignments(args.set)
    if not assignments:
        raise SystemExit("at least one --set dimension=value is required")
    if not RECEIPT_RE.fullmatch(args.receipt):
        raise SystemExit("receipt must be a 64-character hexadecimal BLAKE3 digest")
    verify_receipt_envelope(args.envelope, args.receipt)

    cwd, directory = resolve_project(args.project)
    with state_lock(directory):
        state = load_state(directory, cwd, profile)
        before = dict(state["vector"])
        target = dict(before)
        for dimension, value in assignments.items():
            if dimension not in profile["dimensions"]:
                raise SystemExit(f"unknown phase dimension `{dimension}`")
            if value not in profile["dimensions"][dimension]["states"]:
                raise SystemExit(f"unknown {dimension} state `{value}`")
            if not allowed_transition(profile, dimension, before[dimension], value):
                raise SystemExit(
                    f"transition refused: {dimension}={before[dimension]} -> {value} is not declared"
                )
            target[dimension] = value

        violations = validate_vector(profile, target)
        if violations:
            raise SystemExit("phase invariant refusal: " + "; ".join(violations))

        sequence = int(state.get("transition_count", 0)) + 1
        event = {
            "schema": EVENT_SCHEMA,
            "sequence": sequence,
            "project": cwd,
            "before": before,
            "after": target,
            "receipt": args.receipt.lower(),
            "reason": args.reason,
            "observed_at_unix_ms": int(time.time() * 1000),
        }
        event["transport_digest"] = transport_digest(event)
        append_event(directory, event)
        state.update(
            {
                "vector": target,
                "phase_digest": transport_digest(target),
                "transition_count": sequence,
                "receipt": args.receipt.lower(),
                "reason": args.reason,
                "updated_at_unix_ms": int(time.time() * 1000),
            }
        )
        atomic_write(directory / "phase-state.json", state)
        result = {
            **state,
            "valid": True,
            "active": active_projection(profile, target),
            "transition_event": event,
        }
    print(json.dumps(result, sort_keys=True, indent=2))
    return 0


def is_mutation(payload: dict[str, Any]) -> bool:
    tool = payload.get("tool_name")
    if tool in {"Write", "Edit", "NotebookEdit"}:
        return True
    if tool != "Bash":
        return False
    tool_input = payload.get("tool_input")
    if not isinstance(tool_input, dict):
        return False
    # No self-exemption here, by design: a phase vector collapses on every
    # observed mutation, including the ledger's own.
    command = tool_input.get("command")
    return isinstance(command, str) and bash_is_mutation(command)


def invalidate_from_mutation(payload: dict[str, Any]) -> None:
    profile = load_profile()
    cwd = str(payload.get("cwd") or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    cwd = os.path.realpath(cwd)
    directory = project_directory(cwd)
    with state_lock(directory):
        state = load_state(directory, cwd, profile)
        before = dict(state["vector"])
        target = dict(before)
        target.update(
            {
                "epistemic": "observed",
                "allocation": "unallocated",
                "planning": "unplanned",
                "actuation": "sealed",
                "drift": "drifted",
                "conformance": "unknown",
            }
        )
        sequence = int(state.get("transition_count", 0)) + 1
        event = {
            "schema": EVENT_SCHEMA,
            "sequence": sequence,
            "project": cwd,
            "before": before,
            "after": target,
            "receipt": None,
            "reason": "hook-observed-repository-mutation",
            "tool": payload.get("tool_name"),
            "tool_use_id": payload.get("tool_use_id"),
            "observed_at_unix_ms": int(time.time() * 1000),
        }
        event["transport_digest"] = transport_digest(event)
        append_event(directory, event)
        state.update(
            {
                "vector": target,
                "phase_digest": transport_digest(target),
                "transition_count": sequence,
                "receipt": None,
                "reason": event["reason"],
                "updated_at_unix_ms": event["observed_at_unix_ms"],
            }
        )
        atomic_write(directory / "phase-state.json", state)


def hook() -> int:
    raw = sys.stdin.read()
    payload = json.loads(raw) if raw.strip() else {}
    event = str(payload.get("hook_event_name", ""))
    if event in {"PostToolUse", "PostToolUseFailure"} and is_mutation(payload):
        invalidate_from_mutation(payload)
        return 0
    if event == "SessionStart":
        profile = load_profile()
        cwd = str(payload.get("cwd") or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
        directory = project_directory(cwd)
        with state_lock(directory):
            state = load_state(directory, cwd, profile)
        projection = active_projection(profile, state["vector"])
        context = (
            "Chatman phase engine active. Product-state vector: "
            + json.dumps(state["vector"], sort_keys=True)
            + ". Active agents: "
            + ", ".join(projection["agents"])
            + ". Active skills: "
            + ", ".join(projection["skills"])
            + ". Configuration law is supplied by claude-code-config-lsp. "
            "Never advance a phase from prose; bind an MCP receipt and use phase.py transition."
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


def monitor(project: str | None) -> int:
    profile = load_profile()
    cwd, directory = resolve_project(project)
    previous: str | None = None
    while True:
        with state_lock(directory):
            state = load_state(directory, cwd, profile)
        current = state["phase_digest"]
        if current != previous:
            print(
                json.dumps(
                    {
                        "project": cwd,
                        "vector": state["vector"],
                        "phase_digest": current,
                        "active": active_projection(profile, state["vector"]),
                        "valid": not validate_vector(profile, state["vector"]),
                    },
                    sort_keys=True,
                ),
                flush=True,
            )
            previous = current
        time.sleep(1.0)


def matrix() -> int:
    profile = load_profile()
    print(json.dumps(profile, sort_keys=True, indent=2))
    return 0


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    sub = root.add_subparsers(dest="command", required=True)
    sub.add_parser("hook")
    for name in ("status", "monitor"):
        command = sub.add_parser(name)
        command.add_argument("--project")
    sub.add_parser("matrix")
    command = sub.add_parser("transition")
    command.add_argument("--project")
    command.add_argument("--set", action="append", default=[])
    command.add_argument("--receipt", required=True)
    command.add_argument(
        "--envelope",
        required=True,
        help="Path to the JSON admission envelope returned by bind_plan_receipt/"
        "bind_allocation_receipt, whose `receipt` field must equal --receipt. "
        "Verified against the ferroplan-mcp verify_receipt tool before transition.",
    )
    command.add_argument("--reason", required=True)
    return root


def main() -> int:
    args = parser().parse_args()
    if args.command == "hook":
        return hook()
    if args.command == "status":
        return status(args.project)
    if args.command == "transition":
        return transition(args)
    if args.command == "monitor":
        return monitor(args.project)
    if args.command == "matrix":
        return matrix()
    raise SystemExit(f"unsupported command: {args.command}")


if __name__ == "__main__":
    raise SystemExit(main())
