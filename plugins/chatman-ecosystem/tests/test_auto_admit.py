"""Falsifiers for bounded low-risk auto-admission."""

from __future__ import annotations

import contextlib
import json
import subprocess
from pathlib import Path

import auto_admit
import auto_admit_git
import auto_admit_model
import pytest


def policy(allow=('docs/**/*.md',)):
    return auto_admit.Policy(True, 2, 30, 4, 1024, allow)


def event(project: Path, tool='Edit', path='docs/note.md', failed=False):
    value = {'schema': 'urn:chatman:claude-code-observation:v1', 'sequence': 1, 'session_id': 's', 'tool_use_id': 't', 'hook_event': 'PostToolUseFailure' if failed else 'PostToolUse', 'tool': tool, 'surface': {'path': str(project / path), 'new_string_bytes': 12}, 'duration_ms': 1, 'failed': failed, 'observed_at_unix_ms': 1, 'transport_digest_algorithm': 'sha256'}
    value['transport_digest'] = auto_admit.digest(value)
    return value


def test_doc_edit_is_eligible(tmp_path):
    project = tmp_path / 'repo'
    (project / 'docs').mkdir(parents=True)
    assert auto_admit.validate_events(project, [event(project)], policy()) == (('docs/note.md',), 12)


@pytest.mark.parametrize('tool', ['Bash', 'Read', 'mcp__ferroplan'])
def test_non_editor_requires_manual_admission(tmp_path, tool):
    project = tmp_path / 'repo'
    project.mkdir()
    with pytest.raises(auto_admit.Refused) as caught:
        auto_admit.validate_events(project, [event(project, tool=tool)], policy())
    assert caught.value.code == 'TOOL_NOT_AUTO_ADMISSIBLE'


def test_failed_event_refuses(tmp_path):
    project = tmp_path / 'repo'
    project.mkdir()
    with pytest.raises(auto_admit.Refused) as caught:
        auto_admit.validate_events(project, [event(project, failed=True)], policy())
    assert caught.value.code == 'FAILED_EVENT_NOT_AUTO_ADMISSIBLE'


@pytest.mark.parametrize('path', ['CLAUDE.md', '.github/workflows/ci.yml', 'plugins/chatman-ecosystem/scripts/loop.py', 'plugins/chatman-ecosystem/hooks/hooks.json', 'crates/ferroplan-mcp/src/admission.rs'])
def test_hard_deny_beats_permissive_profile(tmp_path, path):
    project = tmp_path / 'repo'
    target = project / path
    target.parent.mkdir(parents=True)
    target.write_text('x')
    with pytest.raises(auto_admit.Refused) as caught:
        auto_admit.validate_events(project, [event(project, path=path)], policy(('**',)))
    assert caught.value.code == 'PROTECTED_PATH'


def test_digest_tampering_refuses(tmp_path):
    project = tmp_path / 'repo'
    project.mkdir()
    value = event(project)
    value['surface']['new_string_bytes'] = 99
    with pytest.raises(auto_admit.Refused) as caught:
        auto_admit.validate_events(project, [value], policy())
    assert caught.value.code == 'EVENT_DIGEST_MISMATCH'


def test_path_escape_refuses(tmp_path):
    project = tmp_path / 'repo'
    project.mkdir()
    value = event(project)
    value['surface']['path'] = str(tmp_path / 'outside.md')
    unsigned = dict(value)
    unsigned.pop('transport_digest')
    value['transport_digest'] = auto_admit.digest(unsigned)
    with pytest.raises(auto_admit.Refused) as caught:
        auto_admit.validate_events(project, [value], policy())
    assert caught.value.code == 'EVENT_PATH_ESCAPES_PROJECT'


def test_ledger_project_mismatch_refuses(tmp_path, monkeypatch):
    project = tmp_path / 'repo'
    project.mkdir()

    class Ledger:
        @staticmethod
        def state_lock(_directory):
            return contextlib.nullcontext()

        @staticmethod
        def load_state(_directory, _project):
            return {
                'project': '/tmp/independent-verify-project',
                'event_count': 0,
                'admitted_event_count': 0,
            }

    monkeypatch.setattr(
        auto_admit_model,
        'runtime',
        lambda: (Ledger, None, None, None, lambda _project: tmp_path / 'ledger', str),
    )
    with pytest.raises(auto_admit.Refused) as caught:
        auto_admit_model.read_snapshot(str(project), policy())
    assert caught.value.code == 'LEDGER_PROJECT_MISMATCH'


def test_commit_invokes_public_loop_admit_stdin(tmp_path, monkeypatch):
    snapshot_value = auto_admit.Snapshot(
        str(tmp_path),
        tmp_path / 'ledger',
        2,
        3,
        (),
        (),
        0,
    )
    observed = {}

    def run(command, **kwargs):
        observed['command'] = command
        observed.update(kwargs)
        return subprocess.CompletedProcess(
            command,
            0,
            stdout='{"admitted_event_count": 3}',
            stderr='',
        )

    monkeypatch.setattr(auto_admit, 'root', lambda: tmp_path / 'plugin')
    monkeypatch.setattr(auto_admit.subprocess, 'run', run)
    state = auto_admit.commit(
        snapshot_value,
        {'receipt': 'd' * 64, 'payload_digest': 'e' * 64},
        'session',
    )
    assert state['admitted_event_count'] == 3
    assert observed['command'][1:4] == [
        str(tmp_path / 'plugin/scripts/loop.py'),
        'admit',
        '--project',
    ]
    assert '--envelope' in observed['command']
    assert observed['command'][observed['command'].index('--envelope') + 1] == '-'
    assert observed['command'][observed['command'].index('--expected-admitted-event-count') + 1] == '2'
    assert observed['command'][observed['command'].index('--expected-event-count') + 1] == '3'
    assert json.loads(observed['input'])['receipt'] == 'd' * 64


def test_commit_maps_broker_frontier_refusal(tmp_path, monkeypatch):
    snapshot_value = auto_admit.Snapshot(
        str(tmp_path),
        tmp_path / 'ledger',
        0,
        1,
        (),
        (),
        0,
    )

    def run(command, **_kwargs):
        return subprocess.CompletedProcess(
            command,
            2,
            stdout='',
            stderr='admission frontier moved: expected=(0,1), actual=(0,2)',
        )

    monkeypatch.setattr(auto_admit.subprocess, 'run', run)
    with pytest.raises(auto_admit.Refused) as caught:
        auto_admit.commit(snapshot_value, {'receipt': 'd' * 64}, 'session')
    assert caught.value.code == 'FRONTIER_MOVED'


def git(project, *args):
    subprocess.run(['git', '-C', str(project), *args], check=True, capture_output=True)


def repo(tmp_path):
    project = tmp_path / 'repo'
    (project / 'docs').mkdir(parents=True)
    for name in ('one.md', 'two.md'):
        (project / 'docs' / name).write_text(name)
    git(project, 'init', '-q')
    git(project, 'config', 'user.email', 't@example.com')
    git(project, 'config', 'user.name', 'T')
    git(project, 'add', '.')
    git(project, 'commit', '-qm', 'base')
    return project


def snapshot(project, paths=('docs/one.md',)):
    return auto_admit.Snapshot(str(project), project / '.ledger', 0, 1, (event(project, path=paths[0]),), paths, 12)


def test_measure_requires_exact_dirty_frontier(tmp_path):
    project = repo(tmp_path)
    (project / 'docs/one.md').write_text('changed one')
    (project / 'docs/two.md').write_text('changed two')
    with pytest.raises(auto_admit.Refused) as caught:
        auto_admit.measure(snapshot(project))
    assert caught.value.code == 'DIFF_FRONTIER_MISMATCH'


def test_measure_refuses_conflict_markers(tmp_path):
    project = repo(tmp_path)
    (project / 'docs/one.md').write_text('<<<<<<< a\nx\n=======\ny\n>>>>>>> b\n')
    with pytest.raises(auto_admit.Refused) as caught:
        auto_admit.measure(snapshot(project))
    assert caught.value.code == 'CONFLICT_MARKERS_PRESENT'


class Client:
    def __init__(self):
        self.calls = []

    def call_tool(self, name, arguments):
        self.calls.append((name, arguments))
        return {'cmca_allocate': {'payload': {}, 'payload_digest': 'a' * 64}, 'bind_allocation_receipt': {'receipt': 'b' * 64}, 'session_open': {'session_id': arguments.get('session_id')}, 'session_observe': {'epoch': 1}, 'session_think': {'decision': 'replan', 'solution': {'plan': {'steps': ['close']}}}, 'validate': {'valid': True}, 'bind_plan_receipt': {'receipt': 'd' * 64}, 'verify_receipt': {'valid': True}, 'session_close': {'closed': True}}[name]


def test_ceremony_calls_live_tool_chain_shape(tmp_path, monkeypatch):
    plugin = tmp_path / 'plugin'
    (plugin / 'profiles').mkdir(parents=True)
    source = Path(__file__).resolve().parent.parent / 'profiles/work-surfaces.json'
    (plugin / 'profiles/work-surfaces.json').write_bytes(source.read_bytes())
    monkeypatch.setattr(auto_admit_git, 'root', lambda: plugin)
    project = tmp_path / 'repo'
    (project / 'docs').mkdir(parents=True)
    snap = snapshot(project)
    client = Client()
    envelope, session = auto_admit.ceremony(snap, [auto_admit.Measure('docs/one.md', 3, 1, 20, False, False)], client, lambda x: x)
    assert [name for name, _ in client.calls] == ['cmca_allocate', 'bind_allocation_receipt', 'session_open', 'session_observe', 'session_think', 'validate', 'bind_plan_receipt', 'verify_receipt', 'session_close']
    assert envelope['receipt'] == 'd' * 64 and session.endswith('-1')
    assert len(client.calls[1][1]['candidates']) == 8
    assert all(
        len(candidate['factors']) == 10
        for candidate in client.calls[1][1]['candidates']
    )
