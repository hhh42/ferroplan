#!/usr/bin/env python3
"""Project the effective Chatman phase without promoting hook events into truth."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))
try:
    from plugin_data import plugin_data_root as resolve_plugin_data_root
except ImportError:
    resolve_plugin_data_root = None
from roots import project_directory  # noqa: E402

STATE_SCHEMA = "urn:chatman:claude-code-effective-phase:v1"
COLLAPSED_VECTOR = {
    "epistemic": "observed",
    "allocation": "unallocated",
    "planning": "unplanned",
    "actuation": "sealed",
    "drift": "drifted",
    "conformance": "unknown",
}


def plugin_root() -> Path:
    configured = os.environ.get("CLAUDE_PLUGIN_ROOT")
    if configured:
        return Path(configured)
    return Path(__file__).resolve().parent.parent


def plugin_data_root() -> Path:
    if resolve_plugin_data_root is not None:
        return resolve_plugin_data_root()
    configured = os.environ.get("CLAUDE_PLUGIN_DATA")
    if configured:
        return Path(configured)
    return Path.home() / ".claude" / "plugins" / "data" / "chatman-ecosystem"


def load_json(path: Path, default: dict[str, Any]) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return dict(default)
    return value if isinstance(value, dict) else dict(default)


def initial_vector(profile: dict[str, Any]) -> dict[str, str]:
    return {
        name: str(dimension["initial"])
        for name, dimension in profile["dimensions"].items()
    }


def active_projection(profile: dict[str, Any], vector: dict[str, str]) -> dict[str, list[str]]:
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


def project(project: str) -> dict[str, Any]:
    project = os.path.realpath(project)
    profile = json.loads(
        (plugin_root() / "profiles" / "phase-space.json").read_text(encoding="utf-8")
    )
    directory = project_directory(project)
    canonical_default = {
        "vector": initial_vector(profile),
        "transition_count": 0,
        "receipt": None,
        "reason": "effective-phase-default",
    }
    loop_default = {
        "event_count": 0,
        "admitted_event_count": 0,
        "plan_receipt": None,
    }
    phase_state = load_json(directory / "phase-state.json", canonical_default)
    loop_state = load_json(directory / "state.json", loop_default)
    canonical = dict(phase_state.get("vector") or initial_vector(profile))
    event_count = int(loop_state.get("event_count", 0))
    admitted_event_count = int(loop_state.get("admitted_event_count", 0))
    pending = max(0, event_count - admitted_event_count)
    effective = dict(COLLAPSED_VECTOR if pending else canonical)
    return {
        "schema": STATE_SCHEMA,
        "project": project,
        "canonical_vector": canonical,
        "effective_vector": effective,
        "pending_event_count": pending,
        "event_count": event_count,
        "admitted_event_count": admitted_event_count,
        "canonical_receipt": phase_state.get("receipt"),
        "plan_receipt": loop_state.get("plan_receipt"),
        "requires_admission": pending > 0,
        "projection_reason": (
            "pending-observation-frontier" if pending else "canonical-receipt-bound-snapshot"
        ),
        "active": active_projection(profile, effective),
        "replay_limitations": [
            "This command projects pending observations over the canonical snapshot.",
            "It does not independently verify the receipt chain or mutate phase state."
        ]
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project")
    args = parser.parse_args()
    project_path = args.project or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd()
    print(json.dumps(project(project_path), sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
