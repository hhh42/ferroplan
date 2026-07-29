"""The fence must fail closed.

Putting third-party imports on the hook path makes a missing dependency a
question about the actuation fence rather than about a command. These tests pin
the only acceptable answer: refuse, say why, and never exit non-zero.
"""

from __future__ import annotations

import json

import hookguard
import pytest


def test_guard_uses_only_the_standard_library():
    """The last line of defence cannot itself depend on what may be missing."""
    source = (hookguard.__file__ or "").replace(".pyc", ".py")
    text = open(source, encoding="utf-8").read()
    for forbidden in ("import pydantic", "import typer", "from pydantic", "from typer"):
        assert forbidden not in text


def test_stop_refusal_is_top_level_not_nested():
    """Stop takes a top-level decision; nesting it silently does nothing."""
    out = hookguard.refusal("Stop", "because")
    assert out == {"decision": "block", "reason": "because"}


def test_pretooluse_refusal_is_nested():
    out = hookguard.refusal("PreToolUse", "because")
    inner = out["hookSpecificOutput"]
    assert inner["hookEventName"] == "PreToolUse"
    assert inner["permissionDecision"] == "deny"
    assert inner["permissionDecisionReason"] == "because"


def test_observation_events_emit_nothing():
    """PostToolUse cannot refuse. Emitting a refusal shape there is a bug."""
    assert hookguard.refusal("PostToolUse", "because") == {}
    assert hookguard.refusal("", "because") == {}


@pytest.mark.parametrize("event", ["Stop", "PreToolUse"])
def test_import_failure_produces_a_refusal(event, capsys, monkeypatch):
    """The core property: a broken control plane denies, it does not allow."""
    monkeypatch.setattr(hookguard, "read_event", lambda: {"hook_event_name": event})

    def handler(_payload):
        raise ImportError("No module named 'pydantic'")

    code = hookguard.guarded(handler)
    assert code == 0, "a hook signals its decision in stdout, never via exit code"

    captured = capsys.readouterr()
    payload = json.loads(captured.out)
    reason = (
        payload["reason"]
        if event == "Stop"
        else payload["hookSpecificOutput"]["permissionDecisionReason"]
    )
    assert hookguard.DEGRADED_CODE in reason
    assert "pydantic" in reason, "the refusal must name what actually broke"
    assert "pip install" in reason, "and must be actionable"
    assert captured.err, "the traceback belongs on stderr for the operator"


def test_import_failure_on_an_observation_event_stays_silent(capsys, monkeypatch):
    """No refusal shape exists for PostToolUse; emitting one would be rejected."""
    monkeypatch.setattr(hookguard, "read_event", lambda: {"hook_event_name": "PostToolUse"})

    def handler(_payload):
        raise ImportError("boom")

    assert hookguard.guarded(handler) == 0
    assert capsys.readouterr().out == ""


def test_malformed_stdin_does_not_raise(monkeypatch):
    """A hook that cannot read its input must still be able to decide."""
    monkeypatch.setattr("sys.stdin", _FakeStdin("not json at all"))
    assert hookguard.read_event() == {}


def test_non_object_stdin_does_not_raise(monkeypatch):
    monkeypatch.setattr("sys.stdin", _FakeStdin("[1, 2, 3]"))
    assert hookguard.read_event() == {}


def test_successful_handler_is_passed_through(monkeypatch):
    monkeypatch.setattr(hookguard, "read_event", lambda: {"hook_event_name": "Stop"})
    assert hookguard.guarded(lambda payload: 0) == 0


def test_systemexit_is_not_swallowed(monkeypatch):
    """argparse and explicit refusals use SystemExit; guarding it would mask them."""
    monkeypatch.setattr(hookguard, "read_event", lambda: {"hook_event_name": "Stop"})

    def handler(_payload):
        raise SystemExit(2)

    with pytest.raises(SystemExit):
        hookguard.guarded(handler)


class _FakeStdin:
    def __init__(self, text: str) -> None:
        self._text = text

    def read(self) -> str:
        return self._text
