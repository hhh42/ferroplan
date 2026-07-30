#!/usr/bin/env python3
"""Derive a bounded execution grant from an exact actuation intent and verified receipt."""

from __future__ import annotations

import argparse
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
from mcp_client import McpClient, McpToolError, tool_structured_result

try:
    from plugin_data import plugin_data_root as resolve_plugin_data_root
except ImportError:
    resolve_plugin_data_root = None
from roots import project_directory  # noqa: E402

INTENT_SCHEMA = "urn:chatman:actuation-intent:v1"
GRANT_SCHEMA = "urn:chatman:derived-execution-grant:v1"
RECEIPT_RE = re.compile(r"^[0-9a-f]{64}$")
REQUIRED_VECTOR = {
    "epistemic": "admitted",
    "allocation": "allocated",
    "planning": "validated",
    "actuation": "publishable",
    "drift": "stable",
    "conformance": "conformant",
}


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
    except OSError as error:
        raise SystemExit(f"cannot read {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise SystemExit(f"invalid JSON in {path}: {error}") from error
    if not isinstance(value, dict):
        raise SystemExit(f"{path} must contain a JSON object")
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


def verify_receipt(envelope_path: Path, receipt: str) -> dict[str, Any]:
    envelope = load_json(envelope_path)
    declared = envelope.get("receipt")
    if not isinstance(declared, str) or declared.lower() != receipt:
        raise SystemExit("receipt does not match the envelope's receipt field")
    try:
        with McpClient() as client:
            result = client.call_tool("verify_receipt", {"envelope": envelope})
    except McpToolError as error:
        raise SystemExit(f"receipt verification failed: {error}") from error
    verification = tool_structured_result(result)
    if not isinstance(verification, dict) or verification.get("valid") is not True:
        raise SystemExit(f"receipt verification failed: {json.dumps(verification)}")
    return verification


def current_state(project: str) -> tuple[dict[str, Any], dict[str, Any], int]:
    directory = project_directory(project)
    phase = load_json(directory / "phase-state.json")
    loop = load_json(directory / "state.json")
    vector = phase.get("vector")
    if not isinstance(vector, dict) or vector != REQUIRED_VECTOR:
        raise SystemExit("effective phase is not publishable, stable, conformant, admitted, allocated, and validated")
    event_count = int(loop.get("event_count", 0))
    admitted = int(loop.get("admitted_event_count", 0))
    pending = max(0, event_count - admitted)
    if pending:
        raise SystemExit(f"cannot grant actuation with {pending} pending observation event(s)")
    return phase, loop, pending


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project")
    parser.add_argument("--intent", required=True)
    parser.add_argument("--receipt", required=True)
    parser.add_argument("--envelope", required=True)
    parser.add_argument("--scope", required=True)
    parser.add_argument("--ttl-seconds", type=int, default=900)
    args = parser.parse_args()

    receipt = args.receipt.lower()
    if not RECEIPT_RE.fullmatch(receipt):
        raise SystemExit("receipt must be a 64-character lowercase hexadecimal digest")
    if args.ttl_seconds < 1 or args.ttl_seconds > 3600:
        raise SystemExit("--ttl-seconds must be between 1 and 3600")

    project = os.path.realpath(args.project or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    intent_path = Path(args.intent)
    intent = load_json(intent_path)
    if intent.get("schema") != INTENT_SCHEMA:
        raise SystemExit("unsupported actuation intent schema")
    expected_intent_digest = intent.get("intent_digest")
    if not isinstance(expected_intent_digest, str) or not RECEIPT_RE.fullmatch(expected_intent_digest):
        raise SystemExit("actuation intent has no valid intent_digest")
    unsigned_intent = dict(intent)
    unsigned_intent.pop("intent_digest", None)
    if digest(unsigned_intent) != expected_intent_digest:
        raise SystemExit("actuation intent digest mismatch")
    if os.path.realpath(str(intent.get("project", ""))) != project:
        raise SystemExit("actuation intent project does not match --project")

    verification = verify_receipt(Path(args.envelope), receipt)
    phase, loop, _ = current_state(project)
    if receipt not in {phase.get("receipt"), loop.get("plan_receipt")}:
        raise SystemExit("verified receipt is not the active phase or plan receipt")

    created = int(time.time() * 1000)
    grant: dict[str, Any] = {
        "schema": GRANT_SCHEMA,
        "intent_digest": expected_intent_digest,
        "receipt": receipt,
        "granted": True,
        "scope": args.scope,
        "expires_at_unix_ms": created + args.ttl_seconds * 1000,
        "created_at_unix_ms": created,
        "verification": verification,
    }
    grant["grant_digest"] = digest(grant)
    output = project_directory(project) / "grants" / f"{expected_intent_digest}.json"
    atomic_write(output, grant)
    print(json.dumps({"grant": grant, "path": str(output)}, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
