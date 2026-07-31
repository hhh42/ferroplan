#!/usr/bin/env python3
"""Git measurements and evidence-derived CMCA factors for auto-admission."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from collections.abc import Sequence
from dataclasses import asdict
from pathlib import Path

from auto_admit_model import Measure, Refused, Snapshot, event_path, root


def git(project: Path, *args: str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(['git', '-C', str(project), *args], capture_output=True)


def dirty_paths(project: Path) -> set[str]:
    result = git(project, 'status', '--porcelain=v1', '-z', '--untracked-files=all')
    if result.returncode:
        raise Refused('GIT_STATUS_FAILED', result.stderr.decode(errors='replace'))
    records, paths, i = result.stdout.decode(errors='surrogateescape').split('\x00'), set(), 0
    while i < len(records):
        record, i = records[i], i + 1
        if not record:
            continue
        if len(record) < 4:
            raise Refused('GIT_STATUS_INVALID', repr(record))
        status, path = record[:2], record[3:]
        paths.add(path)
        if status[0] in 'RC' and i < len(records):
            paths.add(records[i])
            i += 1
    return paths


def measure(snapshot: Snapshot) -> tuple[Measure, ...]:
    project, observed = Path(snapshot.project), set(snapshot.paths)
    dirty = dirty_paths(project)
    if dirty != observed:
        raise Refused('DIFF_FRONTIER_MISMATCH', 'dirty paths differ', observed_paths=sorted(observed), dirty_paths=sorted(dirty))
    measured = []
    for path in snapshot.paths:
        target = project / path
        result = git(project, 'diff', '--numstat', '--no-renames', 'HEAD', '--', path)
        if result.returncode:
            raise Refused('GIT_DIFF_FAILED', result.stderr.decode(errors='replace'))
        text, added, deleted, binary = result.stdout.decode(errors='replace').strip(), 0, 0, False
        if text:
            left, right, *_ = text.splitlines()[-1].split('\t')
            binary = left == '-' or right == '-'
            if not binary:
                added, deleted = int(left), int(right)
        data = target.read_bytes() if target.is_file() else b''
        if not text and data:
            added, binary = data.count(b'\n') + int(not data.endswith(b'\n')), b'\x00' in data
        if any(marker in data for marker in (b'<<<<<<<', b'=======', b'>>>>>>>')):
            raise Refused('CONFLICT_MARKERS_PRESENT', path)
        measured.append(Measure(path, added, deleted, len(data), binary, target.is_file() and os.access(target, os.X_OK)))
    check = git(project, 'diff', '--check', 'HEAD', '--', *snapshot.paths)
    if check.returncode:
        raise Refused('DIFF_CHECK_FAILED', check.stdout.decode(errors='replace'))
    return tuple(measured)


def covered(path: str, declared: str) -> bool:
    declared = declared.rstrip('/')
    return path == declared or path.startswith(declared + '/')


def frontier(snapshot: Snapshot, measured: Sequence[Measure]):
    source = root() / 'profiles' / 'work-surfaces.json'
    profile = json.loads(source.read_text())
    order, raw, surface_paths = profile.get('factor_order', []), profile.get('candidates', []), profile.get('surface_paths', {})
    if len(order) != 10 or len(raw) != 8:
        raise Refused('CMCA_PROFILE_INVALID', 'requires 8 candidates x 10 factors')
    index = {name: i for i, name in enumerate(order)}
    candidates, evidence = [], {}
    event_paths = [event_path(Path(snapshot.project), e['surface']['path']) for e in snapshot.events]
    all_declared = [p for paths in surface_paths.values() for p in paths]
    for item in raw:
        ident, declared = str(item['id']), list(surface_paths.get(item['id'], []))
        selected = [m for m in measured if any(covered(m.path, p) for p in declared)]
        event_count = sum(
            any(covered(path, declared_path) for declared_path in declared)
            for path in event_paths
        )
        if ident == 'evidence':
            selected += [m for m in measured if not any(covered(m.path, p) for p in all_declared) and m not in selected]
            event_count += sum(not any(covered(path, p) for p in all_declared) for path in event_paths)
        files, changed = len({m.path for m in selected}), sum(m.added + m.deleted for m in selected)
        binary, executable = sum(m.binary for m in selected), sum(m.executable for m in selected)
        factors = [float(x) for x in item['factors']]
        factors[index['accessFrequency']] += event_count
        factors[index['recomputationCost']] += changed / 100
        factors[index['retrievalDemand']] += files / 10
        factors[index['schedulingDemand']] += event_count / 10
        factors[index['searchDemand']] += files / 10
        factors[index['standing']] = factors[index['validity']] = 1.0
        factors[index['verificationCost']] += changed / 50 + binary
        factors[index['downstreamConsequence']] += changed + executable * 100
        candidates.append({'id': ident, 'parent': item.get('parent'), 'factors': factors, 'cost': float(item.get('cost', 1))})
        evidence[ident] = {'paths': sorted({m.path for m in selected}), 'events': event_count, 'files': files, 'changed_lines': changed, 'binary': binary, 'executable': executable, 'factors': factors}
    observation = {'schema': 'urn:chatman:auto-admit-observation-frontier:v1', 'project': snapshot.project, 'project_key': snapshot.directory.name, 'admitted_event_count': snapshot.admitted, 'event_count': snapshot.count, 'events': list(snapshot.events), 'paths': list(snapshot.paths), 'observed_bytes': snapshot.observed_bytes, 'measurements': [asdict(m) for m in measured], 'factor_evidence': {'profile_sha256': hashlib.sha256(source.read_bytes()).hexdigest(), 'factor_order': order, 'surfaces': evidence}}
    return candidates, observation
