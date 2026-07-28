#!/usr/bin/env python3
"""Project the live Chatman hook and phase state into a Ferroplan PDDL problem."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

import loop  # type: ignore  # local plugin script
import phase  # type: ignore  # local plugin script

PREDICATES = {
    "epistemic": {
        "latent": "epistemic-latent",
        "observed": "epistemic-observed",
        "admitted": "epistemic-admitted",
    },
    "allocation": {
        "unallocated": "unallocated",
        "allocated": "allocated",
    },
    "planning": {
        "unplanned": "unplanned",
        "candidate": "candidate-plan",
        "validated": "validated-plan",
    },
    "actuation": {
        "sealed": "actuation-sealed",
        "manufacturing": "manufacturing",
        "receipted": "receipted",
        "publishable": "publishable",
    },
    "drift": {
        "stable": "stable",
        "drifted": "drifted",
        "refused": "refused",
    },
    "conformance": {
        "unknown": "config-unknown",
        "nonconformant": "config-nonconformant",
        "conformant": "config-conformant",
    },
}

GOALS = {
    "plan": ["candidate-plan"],
    "validate": ["validated-plan", "validator-green"],
    "receipt": ["receipt-bound", "validator-green"],
    "publish": ["draft-pr-open"],
}


def resolve(project: str | None) -> tuple[str, Path, dict[str, Any], dict[str, Any]]:
    cwd = os.path.realpath(project or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    profile = phase.load_profile()
    directory = phase.project_directory(cwd)
    with phase.state_lock(directory):
        phase_state = phase.load_state(directory, cwd, profile)
    with loop.state_lock(directory):
        loop_state = loop.load_state(directory, cwd)
    return cwd, directory, phase_state, loop_state


def problem(project: str | None, goal_name: str) -> tuple[str, dict[str, Any]]:
    cwd, _, phase_state, loop_state = resolve(project)
    vector = phase_state["vector"]
    violations = phase.validate_vector(phase.load_profile(), vector)
    if violations:
        raise SystemExit("cannot project invalid phase vector: " + "; ".join(violations))

    facts: set[str] = set()
    for dimension, value in vector.items():
        try:
            facts.add(PREDICATES[dimension][value])
        except KeyError as error:
            raise SystemExit(f"no PDDL projection for {dimension}={value}") from error

    event_count = int(loop_state.get("event_count", 0))
    admitted_count = int(loop_state.get("admitted_event_count", 0))
    pending = max(0, event_count - admitted_count)
    standing = str(loop_state.get("standing", "UNKNOWN"))

    if pending > 0 or vector["epistemic"] in {"latent", "observed"} or vector["drift"] == "drifted":
        facts.add("dirty")
    if vector["allocation"] == "allocated":
        facts.add("allocation-bound")
    if vector["planning"] in {"candidate", "validated"}:
        facts.add("plan-bound")
    if vector["planning"] == "validated":
        facts.add("validator-green")
        facts.add("build-green")
        facts.add("bcinr-green")
    if loop_state.get("plan_receipt"):
        facts.add("receipt-bound")
    if vector["drift"] == "refused" or standing == "BUILD_BROKEN":
        facts.add("blocked")

    risk = pending
    if standing == "UNKNOWN":
        risk += 2
    elif standing == "BUILD_BROKEN":
        risk += 8
    elif standing == "PARTIAL_ALIVE":
        risk += 1

    init_lines = [f"    ({name} ferroplan)" for name in sorted(facts)]
    init_lines.extend(
        [
            f"    (= (pending-events ferroplan) {pending})",
            f"    (= (risk ferroplan) {risk})",
            "    (= (available-capacity ferroplan) 8)",
        ]
    )
    goal_lines = [f"      ({name} ferroplan)" for name in GOALS[goal_name]]

    text = "\n".join(
        [
            "(define (problem ferroplan-self-host-live)",
            "  (:domain ferroplan-self-host)",
            "  (:objects ferroplan - repository)",
            "  (:init",
            *init_lines,
            "  )",
            "  (:goal",
            "    (and",
            *goal_lines,
            "    )",
            "  )",
            ")",
            "",
        ]
    )
    metadata = {
        "schema": "urn:chatman:ferroplan-live-world:v1",
        "project": cwd,
        "goal": goal_name,
        "phase_vector": vector,
        "phase_digest": phase_state["phase_digest"],
        "event_count": event_count,
        "admitted_event_count": admitted_count,
        "pending_events": pending,
        "standing": standing,
        "facts": sorted(facts),
        "problem_transport_digest": phase.transport_digest(text),
    }
    return text, metadata


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    root.add_argument("--project")
    root.add_argument("--goal", choices=sorted(GOALS), default="receipt")
    root.add_argument("--output")
    root.add_argument("--metadata")
    return root


def main() -> int:
    args = parser().parse_args()
    text, metadata = problem(args.project, args.goal)
    if args.output:
        path = Path(args.output)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)
    if args.metadata:
        path = Path(args.metadata)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(metadata, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
