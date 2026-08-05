#!/usr/bin/env python3
"""Policy, ledger identity, and event-frontier admission for auto-admission."""

from __future__ import annotations

import fnmatch
import hashlib
import json
import os
import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

POLICY_SCHEMA = 'urn:chatman:auto-admit-policy:v1'


REPORT_SCHEMA = 'urn:chatman:auto-admit-report:v1'


TOOLS = {'Write', 'Edit', 'NotebookEdit'}


DENY = ('.git/**', '.github/**', '.claude/**', '.claude-plugin/**', 'CLAUDE.md', 'RELEASING.md', 'Cargo.toml', 'Cargo.lock', 'pyproject.toml', 'plugins/chatman-ecosystem/hooks/**', 'plugins/chatman-ecosystem/scripts/**', 'plugins/chatman-ecosystem/agents/**', 'plugins/chatman-ecosystem/ontology/**', 'plugins/chatman-ecosystem/profiles/**', 'plugins/chatman-ecosystem/receipts/**', 'crates/ferroplan-mcp/src/**', 'scripts/**', '**/*.sh')


class Refused(RuntimeError):
    def __init__(self, code: str, message: str, **context: Any):
        super().__init__(message)
        self.code, self.context = code, context

    def report(self, project: str) -> dict[str, Any]:
        return {'schema': REPORT_SCHEMA, 'project': project, 'status': 'refused', 'code': self.code, 'message': str(self), 'context': self.context}


@dataclass(frozen=True)
class Policy:
    enabled: bool
    poll: float
    idle: float
    max_events: int
    max_bytes: int
    allow: tuple[str, ...]


@dataclass(frozen=True)
class Snapshot:
    project: str
    directory: Path
    admitted: int
    count: int
    events: tuple[dict[str, Any], ...]
    paths: tuple[str, ...]
    observed_bytes: int


@dataclass(frozen=True)
class Measure:
    path: str
    added: int
    deleted: int
    bytes_on_disk: int
    binary: bool
    executable: bool


def root() -> Path:
    return Path(__file__).resolve().parent.parent


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def digest(value: Any) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def runtime():
    scripts = str(Path(__file__).resolve().parent)
    if scripts not in sys.path:
        sys.path.insert(0, scripts)
    import loop
    from mcp_client import McpClient, McpToolError, tool_structured_result
    from roots import project_directory, project_key
    return loop, McpClient, McpToolError, tool_structured_result, project_directory, project_key


def load_policy(path: Path | None = None) -> Policy:
    source = path or root() / 'profiles' / 'auto-admit.json'
    try:
        value = json.loads(source.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise Refused('POLICY_INVALID', str(error)) from error
    if value.get('schema') != POLICY_SCHEMA:
        raise Refused('POLICY_SCHEMA_UNSUPPORTED', repr(value.get('schema')))
    allow = value.get('allow_globs')
    if not isinstance(allow, list) or not allow or not all(isinstance(x, str) for x in allow):
        raise Refused('POLICY_INVALID', 'allow_globs must be a non-empty string array')
    return Policy(bool(value.get('enabled', False)), max(0.5, float(value.get('poll_interval_seconds', 2))), max(10.0, float(value.get('idle_exit_seconds', 1800))), max(1, int(value.get('max_pending_events', 16))), max(1, int(value.get('max_total_bytes', 32768))), tuple(allow))


def matches(path: str, patterns: Sequence[str]) -> bool:
    return any(
        fnmatch.fnmatchcase(path, pattern)
        or (
            '/**/' in pattern
            and fnmatch.fnmatchcase(path, pattern.replace('/**/', '/'))
        )
        for pattern in patterns
    )


def event_path(project: Path, raw: Any) -> str:
    if not isinstance(raw, str) or not raw:
        raise Refused('EVENT_PATH_MISSING', 'editor event has no path')
    candidate = Path(raw) if Path(raw).is_absolute() else project / raw
    resolved, base = candidate.resolve(strict=False), project.resolve(strict=True)
    if resolved != base and base not in resolved.parents:
        raise Refused('EVENT_PATH_ESCAPES_PROJECT', raw, resolved=str(resolved))
    return resolved.relative_to(base).as_posix()


def validate_events(project: Path, events: Sequence[Mapping[str, Any]], policy: Policy):
    if not events:
        raise Refused('NO_PENDING_EVENTS', 'no pending frontier')
    if len(events) > policy.max_events:
        raise Refused('FRONTIER_TOO_LARGE', str(len(events)))
    paths, total = [], 0
    for event in events:
        unsigned = dict(event)
        declared = unsigned.pop('transport_digest', None)
        if not isinstance(declared, str) or declared.lower() != digest(unsigned):
            raise Refused('EVENT_DIGEST_MISMATCH', str(event.get('sequence')))
        if event.get('tool') not in TOOLS:
            raise Refused('TOOL_NOT_AUTO_ADMISSIBLE', 'Bash/non-editor requires manual admission')
        if event.get('failed') or event.get('hook_event') != 'PostToolUse':
            raise Refused('FAILED_EVENT_NOT_AUTO_ADMISSIBLE', str(event.get('sequence')))
        surface = event.get('surface') if isinstance(event.get('surface'), Mapping) else {}
        path = event_path(project, surface.get('path'))
        if matches(path, DENY):
            raise Refused('PROTECTED_PATH', path)
        if not matches(path, policy.allow):
            raise Refused('PATH_NOT_ALLOWLISTED', path)
        paths.append(path)
        total += sum(v for k, v in surface.items() if k.endswith('_bytes') and isinstance(v, int) and v >= 0)
    if total > policy.max_bytes:
        raise Refused('FRONTIER_BYTES_EXCEEDED', str(total))
    return tuple(sorted(set(paths))), total


def read_snapshot(project: str, policy: Policy) -> Snapshot:
    loop, _, _, _, directory_for, project_key = runtime()
    project = os.path.realpath(project)
    directory = directory_for(project)
    with loop.state_lock(directory):
        state = loop.load_state(directory, project)
        stored = state.get('project')
        if not isinstance(stored, str) or project_key(stored) != project_key(project):
            raise Refused('LEDGER_PROJECT_MISMATCH', 'state belongs to another project', requested=project, stored=stored, directory=str(directory))
        admitted, count = int(state.get('admitted_event_count', 0)), int(state.get('event_count', 0))
        events = []
        event_file = directory / 'events.jsonl'
        if event_file.exists():
            for number, line in enumerate(event_file.read_text().splitlines(), 1):
                try:
                    value = json.loads(line)
                except json.JSONDecodeError as error:
                    raise Refused('EVENT_LOG_INVALID', str(error), line=number) from error
                if admitted < int(value.get('sequence', 0)) <= count:
                    events.append(value)
        actual = [int(e.get('sequence', 0)) for e in events]
        expected = list(range(admitted + 1, count + 1))
        if actual != expected:
            raise Refused('EVENT_FRONTIER_GAPPED', 'non-contiguous events', expected=expected, actual=actual)
    paths, total = validate_events(Path(project), events, policy)
    return Snapshot(project, directory, admitted, count, tuple(events), paths, total)
