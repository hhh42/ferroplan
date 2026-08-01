"""Public admission broker falsifiers for identity, stdin, and frontier races."""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import sys
from pathlib import Path

import loop
import pytest


def args(project: Path, **overrides):
    values = {
        'project': str(project),
        'session': 'session',
        'receipt': 'd' * 64,
        'envelope': '-',
        'plan_digest': None,
        'standing': 'PARTIAL_ALIVE',
        'expected_admitted_event_count': 2,
        'expected_event_count': 3,
    }
    values.update(overrides)
    return argparse.Namespace(**values)


def install_state(monkeypatch, project: Path, state):
    writes = []
    monkeypatch.setattr(loop, 'verify_receipt_envelope', lambda *_args: None)
    monkeypatch.setattr(loop, 'resolve_cli_project', lambda _project: (str(project), project / 'ledger'))
    monkeypatch.setattr(loop, 'state_lock', lambda _directory: contextlib.nullcontext())
    monkeypatch.setattr(loop, 'load_state', lambda _directory, _project: dict(state))
    monkeypatch.setattr(loop, 'atomic_write', lambda path, value: writes.append((path, value)))
    return writes


def test_admit_refuses_stale_project_identity(tmp_path, monkeypatch):
    project = tmp_path / 'ferroplan'
    project.mkdir()
    writes = install_state(
        monkeypatch,
        project,
        {
            'project': str(tmp_path / 'independent-verify-project'),
            'admitted_event_count': 2,
            'event_count': 3,
        },
    )
    with pytest.raises(SystemExit, match='ledger project mismatch'):
        loop.admit(args(project))
    assert writes == []


def test_admit_refuses_frontier_that_moved_after_receipt(tmp_path, monkeypatch):
    project = tmp_path / 'ferroplan'
    project.mkdir()
    writes = install_state(
        monkeypatch,
        project,
        {
            'project': str(project),
            'admitted_event_count': 2,
            'event_count': 4,
        },
    )
    with pytest.raises(SystemExit, match='admission frontier moved'):
        loop.admit(args(project))
    assert writes == []


def test_admit_writes_only_matching_frontier(tmp_path, monkeypatch, capsys):
    project = tmp_path / 'ferroplan'
    project.mkdir()
    writes = install_state(
        monkeypatch,
        project,
        {
            'project': str(project),
            'admitted_event_count': 2,
            'event_count': 3,
        },
    )
    assert loop.admit(args(project)) == 0
    assert len(writes) == 1
    assert writes[0][1]['admitted_event_count'] == 3
    assert writes[0][1]['plan_receipt'] == 'd' * 64
    assert json.loads(capsys.readouterr().out)['admitted_event_count'] == 3


def test_envelope_dash_reads_stdin_and_verifies_live_mcp(monkeypatch):
    receipt = 'a' * 64
    envelope = {'receipt': receipt, 'payload': {'bounded': True}}
    observed = {}

    class Client:
        def __enter__(self):
            return self

        def __exit__(self, *_exc):
            return None

        def call_tool(self, name, arguments):
            observed['name'] = name
            observed['arguments'] = arguments
            return {'valid': True}

    monkeypatch.setattr(sys, 'stdin', io.StringIO(json.dumps(envelope)))
    monkeypatch.setattr(loop, 'McpClient', Client)
    monkeypatch.setattr(loop, 'tool_structured_result', lambda result: result)
    loop.verify_receipt_envelope('-', receipt)
    assert observed == {
        'name': 'verify_receipt',
        'arguments': {'envelope': envelope},
    }
