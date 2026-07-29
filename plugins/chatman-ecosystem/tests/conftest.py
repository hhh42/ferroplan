"""Shared fixtures for the chatman-ecosystem plugin tests.

The single most important thing here is `_isolate`: it is autouse, and it
guarantees no test can read or write the live ledger under `~/.claude`. The
plugin's whole job is recording evidence about a repository; a test suite that
could corrupt the thing it measures would be worse than no suite at all.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

PLUGIN_ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = PLUGIN_ROOT / "scripts"

#: Every environment variable that steers where the plugin reads and writes.
#: Cleared for every test so that a developer's real shell can never leak in and
#: so tests reproduce the Bash-tool-call case, where all of these are unset.
STEERING_VARS = (
    "CLAUDE_PLUGIN_DATA",
    "CLAUDE_PLUGIN_ROOT",
    "CLAUDE_PROJECT_DIR",
    "FERROPLAN_ROOT",
    "CARGO_TARGET_DIR",
)


@pytest.fixture(autouse=True)
def _isolate(tmp_path, monkeypatch):
    """Point the plugin at a throwaway data root and clear all steering vars.

    Autouse and unconditional. The assertion at the end is the actual guard --
    if a future edit reorders things such that `CLAUDE_PLUGIN_DATA` escapes
    `tmp_path`, every test fails loudly rather than quietly writing to the real
    ledger.
    """
    for name in STEERING_VARS:
        monkeypatch.delenv(name, raising=False)

    data_root = tmp_path / "plugin-data"
    data_root.mkdir()
    monkeypatch.setenv("CLAUDE_PLUGIN_DATA", str(data_root))

    # Deterministic rendering: no color, stable timezone.
    monkeypatch.setenv("NO_COLOR", "1")
    monkeypatch.setenv("TZ", "UTC")

    resolved = Path(os.environ["CLAUDE_PLUGIN_DATA"]).resolve()
    assert resolved.is_relative_to(tmp_path.resolve()), (
        f"test isolation breached: CLAUDE_PLUGIN_DATA={resolved} is outside {tmp_path}"
    )
    return data_root


def minimal_model(model_type):
    """Construct a model with only its required fields.

    Shared rather than duplicated: both the contract tests and the schema
    validation tests need one valid instance of every registered model, and two
    copies would drift the moment a required field is added.
    """
    from models import BinaryResolution, ChatmanError, LoopState, MonitorTick, RootsReport

    samples = {
        ChatmanError: lambda: ChatmanError(code="C", message="m"),
        # Resolved, because that is the only shape ever emitted: a failed
        # resolution leaves as a ChatmanError, never as this model.
        BinaryResolution: lambda: BinaryResolution(
            binary="b", resolved=True, argv=["/usr/bin/b"], how="PATH"
        ),
        RootsReport: lambda: RootsReport(plugin_root="/p", project_root=None),
        LoopState: lambda: LoopState(project="/x"),
        MonitorTick: lambda: MonitorTick(
            stream="phase-frontier", project="/x", observed_at_unix_ms=0
        ),
    }
    if model_type not in samples:
        raise AssertionError(
            f"{model_type.__name__} is in REGISTRY but has no sample in conftest.minimal_model"
        )
    return samples[model_type]()


@pytest.fixture
def plugin_root() -> Path:
    return PLUGIN_ROOT


@pytest.fixture
def profile() -> dict:
    import phase

    return phase.load_profile()


@pytest.fixture
def git_project(tmp_path) -> Path:
    """A real git repository with a nested subdirectory.

    Used to prove that the ledger key anchors to the repository root rather than
    to whatever directory a command happened to run in -- the defect that split
    one project's ledger across four directories.
    """
    project = tmp_path / "proj"
    (project / "sub" / "deep").mkdir(parents=True)
    (project / "README.md").write_text("fixture\n", encoding="utf-8")
    subprocess.run(["git", "init", "-q"], cwd=project, check=True)
    return project


@pytest.fixture
def run_script(tmp_path):
    """Invoke a plugin script as a subprocess and capture its streams.

    Scripts are run as subprocesses rather than imported so that argparse, exit
    codes, and the stdout/stderr split are all exercised exactly as a hook or a
    terminal would see them.
    """

    def _run(script: str, *args: str, stdin: str | None = None, cwd: Path | None = None):
        proc = subprocess.run(
            [sys.executable, str(SCRIPTS / script), *args],
            input=stdin,
            capture_output=True,
            text=True,
            cwd=str(cwd) if cwd else None,
            env=dict(os.environ),
        )
        return proc

    return _run


@pytest.fixture
def hook_event():
    """Build a hook payload of the shape Claude Code writes to a hook's stdin."""

    def _event(name: str, /, **fields):
        payload = {"hook_event_name": name}
        payload.update(fields)
        return json.dumps(payload)

    return _event
