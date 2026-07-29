#!/usr/bin/env python3
"""Project the live Chatman hook and phase state into a Ferroplan PDDL problem."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

import loop  # type: ignore  # noqa: E402  # local plugin script
import phase  # type: ignore  # noqa: E402  # local plugin script

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


def _run_check(cmd: list[str], cwd: str, timeout: float) -> dict[str, Any]:
    """Run a subprocess check and report whether it ran and whether it succeeded.

    Never raises: any failure to even invoke the command (missing binary,
    timeout, not a git repo, etc.) is captured as ran=False with a reason,
    rather than crashing problem-generation.
    """
    start = time.monotonic()
    try:
        result = subprocess.run(
            cmd,
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return {
            "ran": False,
            "ok": False,
            "reason": f"{type(error).__name__}: {error}",
            "duration_seconds": round(time.monotonic() - start, 3),
        }
    return {
        "ran": True,
        "ok": result.returncode == 0,
        "returncode": result.returncode,
        "duration_seconds": round(time.monotonic() - start, 3),
    }


def _git_dirty_check(cwd: str, timeout: float) -> dict[str, Any]:
    """Return {"ran", "dirty", "reason"?, "duration_seconds"} for `git status --porcelain`.

    `dirty` is None when the check could not be run or the directory isn't a
    git work tree — callers must fall back to ledger-only inference in that
    case rather than treating None as False.
    """
    start = time.monotonic()
    try:
        result = subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return {
            "ran": False,
            "dirty": None,
            "reason": f"{type(error).__name__}: {error}",
            "duration_seconds": round(time.monotonic() - start, 3),
        }
    duration = round(time.monotonic() - start, 3)
    if result.returncode != 0:
        return {
            "ran": True,
            "dirty": None,
            "reason": f"git status exited {result.returncode} (not a git repo?): {result.stderr.strip()}",
            "duration_seconds": duration,
        }
    return {"ran": True, "dirty": bool(result.stdout.strip()), "duration_seconds": duration}


def resolve(project: str | None) -> tuple[str, Path, dict[str, Any], dict[str, Any]]:
    cwd = os.path.realpath(project or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    profile = phase.load_profile()
    directory = phase.project_directory(cwd)
    with phase.state_lock(directory):
        phase_state = phase.load_state(directory, cwd, profile)
    with loop.state_lock(directory):
        loop_state = loop.load_state(directory, cwd)
    return cwd, directory, phase_state, loop_state


def problem(
    project: str | None,
    goal_name: str,
    skip_live_checks: bool = False,
) -> tuple[str, dict[str, Any]]:
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

    ledger_dirty = (
        pending > 0 or vector["epistemic"] in {"latent", "observed"} or vector["drift"] == "drifted"
    )

    live_checks: dict[str, Any] = {"skipped": skip_live_checks}
    git_dirty_result: dict[str, Any] = {"ran": False, "dirty": None, "reason": "skipped (--skip-live-checks)"}
    build_result: dict[str, Any] = {"ran": False, "ok": False, "reason": "skipped (--skip-live-checks)"}
    test_result: dict[str, Any] = {"ran": False, "ok": False, "reason": "skipped (--skip-live-checks)"}

    if not skip_live_checks:
        git_dirty_result = _git_dirty_check(cwd, timeout=10.0)
        # cargo check --workspace is used instead of `cargo build --workspace`:
        # it exercises the same type/borrow-check/link-resolution surface that
        # actually breaks most PRs, at a fraction of the wall-clock cost of a
        # full codegen build. This is a deliberate speed/fidelity tradeoff —
        # a `cargo check` pass does not guarantee `cargo build` (or
        # `cargo test`) also succeeds (e.g. codegen-only or test-only
        # failures can still slip through), so it's a proxy for build-green,
        # not an exact match.
        build_result = _run_check(["cargo", "check", "--workspace"], cwd, timeout=180.0)
        # validator-green: "validator" in this ontology is the independent
        # validator agent's verdict, which this script has no access to
        # invoke or query. `cargo test --workspace` is used as an honest,
        # explicitly-labeled PROXY for validator-green — it is not the same
        # guarantee as an actual independent-validator run (it can't see
        # anything the validator agent checks beyond automated test passage,
        # e.g. spec conformance, review judgment).
        test_result = _run_check(["cargo", "test", "--workspace"], cwd, timeout=300.0)

    live_checks["git_dirty"] = git_dirty_result
    live_checks["build_check"] = {**build_result, "command": "cargo check --workspace"}
    live_checks["validator_proxy_test"] = {**test_result, "command": "cargo test --workspace"}

    git_dirty = git_dirty_result.get("dirty")
    dirty = ledger_dirty or bool(git_dirty)
    if dirty:
        facts.add("dirty")
    if vector["allocation"] == "allocated":
        facts.add("allocation-bound")
    if vector["planning"] in {"candidate", "validated"}:
        facts.add("plan-bound")
    # build-green: only asserted when a live `cargo check --workspace` was
    # actually run and passed in this call. Never fabricated from the cached
    # phase vector alone.
    if build_result.get("ran") and build_result.get("ok"):
        facts.add("build-green")
    # validator-green: proxied by a live `cargo test --workspace` pass when
    # live checks are enabled; falls back to the cached-vector condition
    # (vector["planning"] == "validated") when live checks are skipped, so
    # --skip-live-checks callers keep the old fast-path behavior rather than
    # silently losing the fact. Gated on `not drifted`: a drift collapse
    # (phase.py's PostToolUse handler) invalidates any prior validator
    # verdict, so the proxy must not outlive it.
    if (
        (test_result.get("ran") and test_result.get("ok"))
        or (skip_live_checks and vector["planning"] == "validated")
    ) and vector["drift"] != "drifted":
        facts.add("validator-green")
    # receipt-bound: `loop_state["plan_receipt"]` records that a receipt was
    # EVER admitted into this project's ledger and is never reset, so on its
    # own it cannot tell "a receipt was bound once" from "the current phase
    # state is receipted". Require the live `phase_state["receipt"]` (which
    # phase.py nulls on every drift collapse) and `not drifted` as well, so
    # this fact tracks the CURRENT phase state, not ledger history.
    if (
        loop_state.get("plan_receipt")
        and phase_state.get("receipt") is not None
        and vector["drift"] != "drifted"
    ):
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
        "live_checks": live_checks,
        "ledger_dirty": ledger_dirty,
        "git_dirty": git_dirty,
    }
    return text, metadata


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    root.add_argument("--project")
    root.add_argument("--goal", choices=sorted(GOALS), default="receipt")
    root.add_argument("--output")
    root.add_argument("--metadata")
    root.add_argument(
        "--skip-live-checks",
        action="store_true",
        default=False,
        help=(
            "Skip live `git status`/`cargo check`/`cargo test` checks and fall back to the "
            "cached phase-vector-only behavior (fast, but dirty/build-green/validator-green "
            "facts are inferred rather than verified)."
        ),
    )
    return root


def main() -> int:
    args = parser().parse_args()
    text, metadata = problem(args.project, args.goal, skip_live_checks=args.skip_live_checks)
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
