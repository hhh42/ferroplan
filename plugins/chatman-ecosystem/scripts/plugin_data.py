#!/usr/bin/env python3
"""Resolve the plugin data root the *Claude Code runtime* actually writes to.

`loop.py` and `phase.py` are invoked two ways, and they must agree on where the
ledger lives:

* From a hook, where the runtime sets `CLAUDE_PLUGIN_DATA` to
  `~/.claude/plugins/data/<plugin>-<marketplace>`.
* From a terminal, where that variable is unset.

The original fallback silently used a *different* directory
(`~/.claude/chatman-ecosystem-data`). A CLI reader therefore opened an empty
ledger and reported `event_count: 0` while the hook ledger held real pending
observations -- a wrong answer that looks exactly like a correct "nothing
pending", which is the failure mode that makes a refusal impossible to debug
from a terminal. Worse, a CLI *write* landed in the legacy root, splitting the
ledger in two.

So resolution is: the environment if set, otherwise the runtime directory if it
exists, otherwise the legacy directory -- and every step past the first says so
on stderr, because guessing quietly is what caused the problem.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

#: What the Claude Code runtime passes as `CLAUDE_PLUGIN_DATA` for this plugin.
#: `<plugin>-<marketplace>`; both are `chatman-ecosystem` here.
RUNTIME_DIR_NAME = "chatman-ecosystem-chatman-ecosystem"

#: Pre-runtime location. Read only when nothing better exists.
LEGACY_DIR_NAME = "chatman-ecosystem-data"


def runtime_root() -> Path:
    return Path.home() / ".claude" / "plugins" / "data" / RUNTIME_DIR_NAME


def legacy_root() -> Path:
    return Path.home() / ".claude" / LEGACY_DIR_NAME


def plugin_data_root(*, warn: bool = True) -> Path:
    """Return the plugin data root, preferring what the runtime actually uses.

    Set `warn=False` only for callers that must not write to stderr (a hook
    emitting strict JSON on stdout is unaffected -- warnings go to stderr).
    """
    configured = os.environ.get("CLAUDE_PLUGIN_DATA")
    if configured:
        return Path(configured)

    runtime = runtime_root()
    if runtime.exists():
        if warn:
            print(
                f"note: CLAUDE_PLUGIN_DATA is unset; using the runtime ledger at {runtime}. "
                "Export CLAUDE_PLUGIN_DATA to silence this.",
                file=sys.stderr,
            )
        return runtime

    legacy = legacy_root()
    if warn:
        print(
            f"warning: CLAUDE_PLUGIN_DATA is unset and no runtime ledger exists at "
            f"{runtime}; falling back to {legacy}. If a hook has reported pending "
            "events that this command does not show, they are in a different ledger "
            "and this reading is not authoritative.",
            file=sys.stderr,
        )
    return legacy
