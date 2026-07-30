#!/usr/bin/env python3
"""Fail-closed wrapper for hook entry points. Standard library only.

The hooks are this plugin's only mechanical authority: `loop.py` is what denies
a protected command and what blocks a turn on an unadmitted frontier. Once the
control plane depends on third-party packages, a missing or broken dependency
becomes a question about the *fence*, not about a command -- and the dangerous
answer is the quiet one.

Three behaviours a hook must never have:

* a traceback, because the harness sees an unparseable stdout and the operator
  sees noise with no decision in it;
* a silent exit 0 on the deny path, which reads as "allowed" and fails **open**;
* a refusal that does not say what broke, which is unactionable.

So every hook entry runs through `guarded`, and any exception before the real
handler produces a refusal shaped for the event that was actually being handled,
naming the failure. This module imports nothing outside the standard library on
purpose: it is the last thing that still has to work when the rest cannot load.
"""

from __future__ import annotations

import json
import sys
import traceback
from collections.abc import Callable
from typing import Any

#: Emitted when the control plane itself cannot start.
DEGRADED_CODE = "CONTROL_PLANE_UNAVAILABLE"


def read_event() -> dict[str, Any]:
    """Parse the hook payload, tolerating a malformed one.

    A hook that cannot read its own input still has to decide, and the safe
    decision needs the event name -- so a parse failure yields an empty payload
    rather than raising, and the caller refuses on the strength of that.
    """
    try:
        raw = sys.stdin.read()
    except OSError:
        return {}
    try:
        payload = json.loads(raw)
    except (ValueError, TypeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def refusal(event: str, reason: str) -> dict[str, Any]:
    """Shape a refusal for the given hook event.

    The three events differ structurally, and getting this wrong is what turns
    a refusal into a no-op: `Stop` takes a top-level decision, `PreToolUse`
    takes a nested permissionDecision, and `SessionStart` has no refusal form
    at all -- the most it can do is say so in context.
    """
    if event == "Stop":
        return {"decision": "block", "reason": reason}
    if event == "PreToolUse":
        return {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason,
            }
        }
    if event == "SessionStart":
        return {
            "hookSpecificOutput": {
                "hookEventName": "SessionStart",
                "additionalContext": reason,
            }
        }
    # PostToolUse and friends cannot refuse; they observe. Say nothing rather
    # than emit a shape the harness will reject.
    return {}


def degraded_reason(event: str, error: BaseException) -> str:
    kind = type(error).__name__
    detail = str(error).strip() or kind
    lines = [
        f"{DEGRADED_CODE}: the Chatman control plane could not start ({kind}: {detail}).",
        "",
        "This hook refuses rather than allowing an unobserved mutation.",
        "",
        "  Most likely a missing dependency. From the plugin directory:",
        "    python3 -m pip install pydantic typer",
        "",
        "  Verify with:",
        "    python3 -m compileall -q scripts",
    ]
    if event == "PreToolUse":
        lines.append("")
        lines.append("Re-run the command once the control plane imports cleanly.")
    return "\n".join(lines)


def guarded(handler: Callable[[dict[str, Any]], int]) -> int:
    """Run `handler`, converting any failure into a refusal.

    Always returns 0: the decision travels in stdout JSON, and a non-zero exit
    is reported to the operator as a broken hook rather than as a refusal.
    """
    event_payload = read_event()
    event = str(event_payload.get("hook_event_name", ""))
    try:
        return handler(event_payload)
    except SystemExit:
        raise
    except BaseException as error:  # noqa: BLE001 - a hook must not propagate
        print(traceback.format_exc(), file=sys.stderr)
        output = refusal(event, degraded_reason(event, error))
        if output:
            print(json.dumps(output))
        return 0
