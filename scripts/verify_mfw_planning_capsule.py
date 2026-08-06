#!/usr/bin/env python3
"""Verify the bounded MFW planning/autonomic source capsule."""
from __future__ import annotations
import ast
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
CAPSULE = ROOT / "vendor" / "mfw-planning-autonomic"
SOURCE = CAPSULE / "SOURCE.json"
EXPECTED_COMMIT = "df511b2dd6aec591d49bf25f652d46a2d03fc3d1"
EXPECTED_TYPES = {
    "classical", "cost_optimal", "numeric", "temporal", "preferences",
    "probabilistic", "fond", "conformant", "contingent", "hierarchical",
    "partial_order", "workflow", "flow_constrained", "resolution_adaptive",
    "multi_agent", "rdf_derived", "a2a_delegated", "mcp_bound",
}

def refuse(code: str, **details: object) -> None:
    print(json.dumps({"standing": "REFUSED", "code": code, "details": details}, sort_keys=True))
    raise SystemExit(2)

def digest(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()

def main() -> int:
    try:
        source = json.loads(SOURCE.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError) as error:
        refuse("SOURCE_MANIFEST_INVALID", reason=str(error))
    if source.get("schema") != "urn:mfw:planning-autonomic-source-capsule:v1":
        refuse("SOURCE_SCHEMA_UNSUPPORTED")
    if source.get("producer", {}).get("commit") != EXPECTED_COMMIT:
        refuse("SOURCE_COMMIT_DRIFT", observed=source.get("producer", {}).get("commit"))
    projection = source.get("projection", {})
    if projection.get("generated") is not True or projection.get("hand_edit_refused") is not True:
        refuse("PROJECTION_AUTHORITY_INVALID")
    authority = source.get("authority", {})
    for capability in ("actuation", "merge", "release", "network", "shell_interpretation"):
        if authority.get(capability) is not False:
            refuse("AUTHORITY_EXPANSION", capability=capability)
    if authority.get("lifecycle_wip_limit") != 1:
        refuse("WIP_LIMIT_DRIFT")
    files = []
    for relative in source.get("consumer_paths", []):
        path = ROOT / relative
        if not path.is_file():
            refuse("CAPSULE_FILE_MISSING", path=relative)
        try:
            ast.parse(path.read_text(encoding="utf-8"), filename=relative)
        except SyntaxError as error:
            refuse("CAPSULE_SYNTAX_INVALID", path=relative, line=error.lineno, reason=error.msg)
        files.append({"path": relative, "sha256": digest(path)})
    oracle_root = CAPSULE / "oracle"
    lifecycle_root = CAPSULE / "lifecycle"
    env = dict(os.environ)
    env["PYTHONPATH"] = os.pathsep.join([str(oracle_root), str(lifecycle_root)])
    process = subprocess.run(
        [sys.executable, "-m", "mfw_planner_oracle", "capabilities"],
        cwd=ROOT, env=env, text=True, capture_output=True, check=False,
    )
    if process.returncode != 0:
        refuse("ORACLE_CAPABILITY_PROCESS_FAILED", stderr=process.stderr)
    try:
        capabilities = json.loads(process.stdout)
    except json.JSONDecodeError as error:
        refuse("ORACLE_CAPABILITY_JSON_INVALID", reason=str(error))
    if capabilities.get("oracle") != "mfw-python-v1" or set(capabilities.get("planning_types", [])) != EXPECTED_TYPES:
        refuse("ORACLE_CAPABILITY_DRIFT", observed=capabilities)
    receipt = {
        "schema": "urn:ferroplan:mfw-planning-capsule-verification:v1",
        "standing": "ALIVE",
        "producer_commit": EXPECTED_COMMIT,
        "claim_ceiling": projection.get("claim_ceiling"),
        "planning_types": sorted(EXPECTED_TYPES),
        "files": files,
    }
    receipt["receipt_digest"] = "sha256:" + hashlib.sha256(
        json.dumps(receipt, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    print(json.dumps(receipt, sort_keys=True, separators=(",", ":")))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
