#!/usr/bin/env python3
"""Auto-admit bounded editor frontiers through the real Ferroplan MCP chain."""

from __future__ import annotations

import argparse
import contextlib
import json
import os
import signal
import subprocess
import sys
import time
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

from auto_admit_git import frontier as frontier
from auto_admit_git import measure as measure
from auto_admit_model import (
    REPORT_SCHEMA as REPORT_SCHEMA,
)
from auto_admit_model import (
    Measure as Measure,
)
from auto_admit_model import (
    Policy as Policy,
)
from auto_admit_model import (
    Refused as Refused,
)
from auto_admit_model import (
    Snapshot as Snapshot,
)
from auto_admit_model import (
    digest as digest,
)
from auto_admit_model import (
    load_policy as load_policy,
)
from auto_admit_model import (
    read_snapshot as read_snapshot,
)
from auto_admit_model import (
    root as root,
)
from auto_admit_model import (
    runtime as _runtime,
)
from auto_admit_model import (
    validate_events as validate_events,
)

try:
    import fcntl
except ImportError:  # pragma: no cover - Windows fallback
    fcntl = None


DOMAIN = '(define (domain ferroplan-auto-admit)\n(:requirements :strips)\n(:predicates (pending) (low-risk) (allocated) (admitted))\n(:action close-low-risk-frontier\n :precondition (and (pending) (low-risk) (allocated))\n :effect (and (admitted) (not (pending)))))'


PROBLEM = '(define (problem ferroplan-auto-admit-frontier)\n(:domain ferroplan-auto-admit)\n(:init (pending) (low-risk) (allocated))\n(:goal (admitted)))'


PLAN = 'step 1: (close-low-risk-frontier)'


def structured(result: Mapping[str, Any], helper) -> dict[str, Any]:
    value = helper(dict(result))
    if not isinstance(value, dict):
        raise Refused('MCP_RESULT_INVALID', repr(value))
    return value


def ceremony(snapshot: Snapshot, measured: Sequence[Measure], client: Any, helper):
    candidates, observation = frontier(snapshot, measured)
    allocation = structured(client.call_tool('cmca_allocate', {'candidates': candidates}), helper)
    bound = structured(client.call_tool('bind_allocation_receipt', {'candidates': candidates, 'allocation_result': allocation, 'observation_frontier': observation}), helper)
    allocation_receipt = bound.get('receipt')
    if not isinstance(allocation_receipt, str):
        raise Refused('ALLOCATION_RECEIPT_MISSING', 'no allocation receipt')
    session = f'auto-admit-{snapshot.directory.name}-{snapshot.count}'
    opened = False
    try:
        structured(client.call_tool('session_open', {'session_id': session, 'domain': DOMAIN, 'problem': PROBLEM}), helper)
        opened = True
        structured(client.call_tool('session_observe', {'session_id': session, 'facts': [{'fact': '(pending)', 'value': True}, {'fact': '(low-risk)', 'value': True}, {'fact': '(allocated)', 'value': True}], 'fluents': []}), helper)
        think = structured(client.call_tool('session_think', {'session_id': session, 'max_evaluated': 50000}), helper)
        plan = think.get('plan') or (think.get('solution') or {}).get('plan')
        if think.get('decision') not in {'follow', 'replan'} or not isinstance(plan, Mapping) or len(plan.get('steps', [])) != 1:
            raise Refused('PLANNER_REFUSED', repr(think))
        validator = structured(client.call_tool('validate', {'domain': DOMAIN, 'problem': PROBLEM, 'plan': PLAN}), helper)
        if validator.get('valid') is not True:
            raise Refused('PLAN_VALIDATION_FAILED', repr(validator))
        envelope = structured(client.call_tool('bind_plan_receipt', {'session_think': think, 'allocation_receipt': allocation_receipt, 'observation_frontier': observation, 'validator_result': validator}), helper)
        verification = structured(client.call_tool('verify_receipt', {'envelope': envelope}), helper)
        if verification.get('valid') is not True:
            raise Refused('RECEIPT_VERIFICATION_FAILED', repr(verification))
        return envelope, session
    finally:
        if opened:
            with contextlib.suppress(Exception):
                client.call_tool('session_close', {'session_id': session})


def commit(snapshot: Snapshot, envelope: Mapping[str, Any], session: str):
    """Advance the ledger only through the public ``loop.py admit`` broker."""
    receipt = envelope.get('receipt')
    if not isinstance(receipt, str) or len(receipt) != 64 or any(
        c not in '0123456789abcdefABCDEF' for c in receipt
    ):
        raise Refused('PLAN_RECEIPT_INVALID', repr(receipt))

    command = [
        sys.executable,
        str(root() / 'scripts' / 'loop.py'),
        'admit',
        '--project',
        snapshot.project,
        '--session',
        session,
        '--receipt',
        receipt,
        '--envelope',
        '-',
        '--standing',
        'PARTIAL_ALIVE',
        '--expected-admitted-event-count',
        str(snapshot.admitted),
        '--expected-event-count',
        str(snapshot.count),
    ]
    plan_digest = envelope.get('payload_digest')
    if isinstance(plan_digest, str):
        command.extend(('--plan-digest', plan_digest))

    result = subprocess.run(
        command,
        input=json.dumps(envelope, sort_keys=True, separators=(',', ':')),
        capture_output=True,
        text=True,
    )
    if result.returncode:
        diagnostic = result.stderr.strip() or result.stdout.strip()
        code = 'LOOP_ADMIT_FAILED'
        if 'admission frontier moved' in diagnostic:
            code = 'FRONTIER_MOVED'
        elif 'ledger project mismatch' in diagnostic:
            code = 'LEDGER_PROJECT_MISMATCH'
        raise Refused(code, diagnostic or f'loop.py admit exited {result.returncode}')
    try:
        state = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise Refused('LOOP_ADMIT_OUTPUT_INVALID', result.stdout) from error
    if int(state.get('admitted_event_count', -1)) != snapshot.count:
        raise Refused('LOOP_ADMIT_FRONTIER_INVALID', repr(state))
    return state


def once(project: str, policy: Policy, client: Any, helper):
    snapshot = read_snapshot(project, policy)
    envelope, session = ceremony(snapshot, measure(snapshot), client, helper)
    state = commit(snapshot, envelope, session)
    return {'schema': REPORT_SCHEMA, 'project': snapshot.project, 'status': 'admitted', 'event_count': snapshot.count, 'admitted_event_count': state['admitted_event_count'], 'receipt': envelope['receipt'], 'session_id': session, 'paths': list(snapshot.paths)}


def pid_path(project: str) -> Path:
    *_, directory_for, _ = _runtime()
    return directory_for(os.path.realpath(project)) / 'auto-admit.pid'


def live_pid(path: Path) -> int | None:
    try:
        pid = int(path.read_text().strip())
        os.kill(pid, 0)
        return pid
    except (OSError, ValueError):
        return None


def ensure(project: str, policy_path: Path | None) -> int:
    policy = load_policy(policy_path)
    if not policy.enabled:
        return 0
    path = pid_path(project)
    path.parent.mkdir(parents=True, exist_ok=True)
    if live_pid(path):
        return 0
    log = path.with_name('auto-admit.log')
    with log.open('ab', buffering=0) as output:
        argv = [sys.executable, str(Path(__file__).resolve()), 'watch', '--project', project]
        if policy_path:
            argv += ['--policy', str(policy_path)]
        subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=output, stderr=output, start_new_session=True, close_fds=True)
    return 0


def watch(project: str, policy_path: Path | None) -> int:
    policy = load_policy(policy_path)
    if not policy.enabled:
        return 0
    path = pid_path(project)
    lock = path.with_name('auto-admit-daemon.lock')
    lock.parent.mkdir(parents=True, exist_ok=True)
    handle = lock.open('a+b')
    if fcntl is not None:
        try:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return 0
    path.write_text(f'{os.getpid()}\n')
    _, Client, ToolError, helper, *_ = _runtime()
    last_activity, last_report = time.monotonic(), None

    def emit(report):
        nonlocal last_activity, last_report
        current = digest(report)
        if current != last_report:
            print(json.dumps(report, sort_keys=True), flush=True)
            last_report = current
            last_activity = time.monotonic()

    try:
        with Client() as client:
            while True:
                try:
                    emit(once(project, policy, client, helper))
                except Refused as error:
                    if error.code != 'NO_PENDING_EVENTS':
                        emit(error.report(project))
                except ToolError as error:
                    emit(Refused('MCP_TOOL_FAILED', str(error)).report(project))
                if time.monotonic() - last_activity >= policy.idle:
                    return 0
                time.sleep(policy.poll)
    finally:
        with contextlib.suppress(OSError):
            if live_pid(path) == os.getpid():
                path.unlink()
        handle.close()


def stop(project: str) -> int:
    pid = live_pid(pid_path(project))
    if pid:
        os.kill(pid, signal.SIGTERM)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest='command', required=True)
    for name in ('once', 'watch', 'ensure'):
        command = sub.add_parser(name)
        command.add_argument('--project', required=True)
        command.add_argument('--policy', type=Path)
    command = sub.add_parser('stop')
    command.add_argument('--project', required=True)
    args = parser.parse_args()
    project = os.path.realpath(args.project)
    if args.command == 'ensure':
        return ensure(project, args.policy)
    if args.command == 'watch':
        return watch(project, args.policy)
    if args.command == 'stop':
        return stop(project)
    policy = load_policy(args.policy)
    if not policy.enabled:
        return 0
    _, Client, ToolError, helper, *_ = _runtime()
    try:
        with Client() as client:
            print(json.dumps(once(project, policy, client, helper), indent=2))
        return 0
    except Refused as error:
        print(json.dumps(error.report(project), indent=2), file=sys.stderr)
        return 2
    except ToolError as error:
        print(json.dumps(Refused('MCP_TOOL_FAILED', str(error)).report(project), indent=2), file=sys.stderr)
        return 3


if __name__ == '__main__':
    raise SystemExit(main())
