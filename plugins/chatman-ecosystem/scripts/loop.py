#!/usr/bin/env python3
"""Public ledger broker with atomic identity and frontier admission guards.

The implementation remains in :mod:`loop_impl`; this wrapper narrows the
public ``loop.py admit`` morphism without duplicating the hook/event runtime.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time

import loop_impl as _impl
from roots import project_key


def admit(args: argparse.Namespace) -> int:
    """Verify a real envelope, then atomically admit exactly its observed frontier."""
    if not re.fullmatch(r"[0-9a-fA-F]{64}", args.receipt):
        raise SystemExit("receipt must be a 64-character hexadecimal BLAKE3 digest")
    if args.plan_digest and not re.fullmatch(r"[0-9a-fA-F]{64}", args.plan_digest):
        raise SystemExit("plan digest must be a 64-character hexadecimal digest")
    _impl.verify_receipt_envelope(args.envelope, args.receipt)
    cwd, directory = _impl.resolve_cli_project(args.project)
    with _impl.state_lock(directory):
        state = _impl.load_state(directory, cwd)
        stored_project = state.get("project")
        if not isinstance(stored_project, str) or project_key(stored_project) != project_key(cwd):
            raise SystemExit(
                "ledger project mismatch: "
                f"requested={cwd!r}, stored={stored_project!r}, directory={str(directory)!r}"
            )

        admitted = int(state.get("admitted_event_count", 0))
        events = int(state.get("event_count", 0))
        expected_admitted = args.expected_admitted_event_count
        expected_events = args.expected_event_count
        if (
            expected_admitted is not None
            and admitted != expected_admitted
            or expected_events is not None
            and events != expected_events
        ):
            raise SystemExit(
                "admission frontier moved: "
                f"expected=({expected_admitted},{expected_events}), "
                f"actual=({admitted},{events})"
            )

        state["admitted_event_count"] = events
        state["plan_receipt"] = args.receipt.lower()
        state["plan_digest"] = args.plan_digest.lower() if args.plan_digest else None
        state["session_id"] = args.session
        state["standing"] = args.standing
        state["updated_at_unix_ms"] = int(time.time() * 1000)
        _impl.atomic_write(directory / "state.json", state)
        print(json.dumps(state, sort_keys=True, indent=2))
    return 0


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=_impl.__doc__)
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
        help=(
            "Path to the JSON admission envelope returned by bind_plan_receipt/"
            "bind_allocation_receipt, whose `receipt` field must equal --receipt. "
            "Pass `-` to read the envelope JSON from stdin instead of a file. "
            "Verified against the ferroplan-mcp verify_receipt tool before admission."
        ),
    )
    command.add_argument("--plan-digest")
    command.add_argument("--expected-admitted-event-count", type=int)
    command.add_argument("--expected-event-count", type=int)
    command.add_argument(
        "--standing",
        choices=tuple(_impl.Standing),
        default=str(_impl.STANDING_DEFAULT),
    )
    return root


def main() -> int:
    args = parser().parse_args()
    if args.command == "hook":
        return _impl.hook()
    if args.command == "pending":
        return _impl.pending(args.project)
    if args.command == "admit":
        return admit(args)
    if args.command == "status":
        return _impl.status(args.project)
    if args.command == "monitor":
        return _impl.monitor(args.project)
    raise SystemExit(f"unsupported command: {args.command}")


# Imports of ``loop`` receive the compatibility implementation with the public
# broker overrides installed, preserving monkeypatch and existing import paths.
_impl.admit = admit
_impl.parser = parser
_impl.main = main

if __name__ == "__main__":
    raise SystemExit(main())

sys.modules[__name__] = _impl
