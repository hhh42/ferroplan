"""Resolution must work from anywhere, with nothing set.

The failure these guard against: a depth-counted parent walk that is correct
under the repository layout and silently wrong under the installed-cache
layout, so a built binary on disk was skipped and the error blamed receipts.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest
import roots
from models import ChatmanError


def test_plugin_root_is_found_with_no_environment(monkeypatch):
    """`dirname $0` is the one locator correct under both layouts."""
    monkeypatch.delenv("CLAUDE_PLUGIN_ROOT", raising=False)
    found = roots.plugin_root()
    assert (found / roots.PLUGIN_MARKER).is_file()


def test_plugin_root_ignores_an_env_var_that_does_not_look_like_a_plugin(monkeypatch, tmp_path):
    """A pointer to the wrong place must lose to a marker that actually exists.

    The installed-cache bug in miniature: trusting a variable without checking
    what it points at is what produced a root with no crates/ in it.
    """
    monkeypatch.setenv("CLAUDE_PLUGIN_ROOT", str(tmp_path))
    assert (roots.plugin_root() / roots.PLUGIN_MARKER).is_file()


def test_empty_env_var_is_treated_as_absent(monkeypatch):
    monkeypatch.setenv("FERROPLAN_ROOT", "")
    assert roots._env_path("FERROPLAN_ROOT") is None


def test_target_dirs_prefer_release_over_debug(tmp_path):
    """A rebuild must never be chosen over an already-built binary."""
    dirs = [p.name for p in roots.cargo_target_dirs(tmp_path)]
    assert dirs.index("release") < dirs.index("debug")


def test_cargo_target_dir_override_is_honoured(monkeypatch, tmp_path):
    monkeypatch.setenv("CARGO_TARGET_DIR", str(tmp_path / "elsewhere"))
    dirs = roots.cargo_target_dirs(tmp_path)
    assert any("elsewhere" in str(d) for d in dirs)


def test_built_binary_is_preferred_over_cargo_run(monkeypatch, tmp_path):
    """The regression that mattered: a binary on disk was being ignored."""
    root = tmp_path / "checkout"
    (root / "crates" / "ferroplan-mcp").mkdir(parents=True)
    (root / "crates" / "ferroplan-mcp" / "Cargo.toml").write_text("", encoding="utf-8")
    target = root / "target" / "debug"
    target.mkdir(parents=True)
    binary = target / "faux-mcp"
    binary.write_text("#!/bin/sh\n", encoding="utf-8")
    binary.chmod(0o755)

    monkeypatch.setenv("FERROPLAN_ROOT", str(root))
    resolved = roots.resolve_binary("faux-mcp", crate="faux", start=root)
    assert resolved.argv == [str(binary)]
    assert "built binary" in resolved.how


def test_a_present_but_non_executable_binary_is_reported_not_used(monkeypatch, tmp_path):
    root = tmp_path / "checkout"
    (root / "crates" / "ferroplan-mcp").mkdir(parents=True)
    (root / "crates" / "ferroplan-mcp" / "Cargo.toml").write_text("", encoding="utf-8")
    target = root / "target" / "debug"
    target.mkdir(parents=True)
    (target / "faux-mcp").write_text("", encoding="utf-8")  # mode 644
    (target / "faux-mcp").chmod(0o644)

    monkeypatch.setenv("FERROPLAN_ROOT", str(root))
    monkeypatch.setattr(roots.shutil, "which", lambda _name: None)
    with pytest.raises(roots.ResolutionFailure) as caught:
        roots.resolve_binary("faux-mcp", crate=None, start=root)
    outcomes = " ".join(why for _c, why in caught.value.tried)
    assert "not executable" in outcomes


def test_failure_is_a_structured_error_naming_what_was_tried(monkeypatch, tmp_path):
    """A resolver that cannot explain itself just moves the confusion."""
    monkeypatch.setattr(roots.shutil, "which", lambda _name: None)
    monkeypatch.setattr(roots, "project_candidates", lambda *a, **k: [])
    with pytest.raises(roots.ResolutionFailure) as caught:
        roots.resolve_binary("nonexistent-binary", crate="nope", start=tmp_path)

    error = caught.value.as_error()
    assert isinstance(error, ChatmanError)
    assert error.code == "BINARY_UNRESOLVED"
    assert error.remedy, "a failure must say what would fix it"
    assert any("PATH" in line for line in error.context["tried"])
    # Unset and empty are distinguishable, because they fail differently.
    assert "FERROPLAN_ROOT" in error.context


def test_reported_environment_distinguishes_unset_from_empty(monkeypatch):
    monkeypatch.delenv("FERROPLAN_ROOT", raising=False)
    monkeypatch.setenv("CARGO_TARGET_DIR", "")
    env = roots.reported_environment()
    assert env["FERROPLAN_ROOT"] is None
    assert env["CARGO_TARGET_DIR"] == ""


def test_siblings_are_offered_as_candidates():
    candidates = roots.project_candidates(marker=Path("Cargo.toml"), siblings=("neighbour",))
    assert any(p.name == "neighbour" for _prov, p in candidates)


def test_every_candidate_carries_its_provenance():
    """Knowing *why* a root was chosen is what makes a wrong one debuggable."""
    for provenance, _path in roots.project_candidates():
        assert provenance and isinstance(provenance, str)


# --------------------------------------------------------------------------
# CLI contract -- exercised as a subprocess, the way launchers use it
# --------------------------------------------------------------------------


def _run_cli(*args: str, cwd: Path | None = None):
    return subprocess.run(
        [sys.executable, str(roots.Path(roots.__file__).parent / "roots.py"), *args],
        capture_output=True,
        text=True,
        cwd=str(cwd) if cwd else None,
    )


def test_cli_default_output_is_schema_tagged_json(tmp_path):
    proc = _run_cli("resolve", "--binary", "ferroplan-mcp", "--crate", "ferroplan-mcp", cwd=tmp_path)
    assert proc.returncode == 0, proc.stderr
    payload = json.loads(proc.stdout)
    assert payload["schema"] == "urn:chatman:binary-resolution:v1"
    assert payload["resolved"] is True


def test_cli_human_projection_is_a_bare_argv(tmp_path):
    """This is what the launcher scripts eval; extra output would break exec."""
    proc = _run_cli(
        "resolve", "--binary", "ferroplan-mcp", "--crate", "ferroplan-mcp",
        "--format", "human", cwd=tmp_path,
    )
    assert proc.returncode == 0, proc.stderr
    assert len(proc.stdout.strip().splitlines()) == 1


def test_cli_failure_exits_69_with_json_on_stderr(tmp_path):
    """Exit 69 is the launcher contract; stdout must stay a clean channel."""
    proc = _run_cli("resolve", "--binary", "definitely-not-a-real-binary-xyz", cwd=tmp_path)
    assert proc.returncode == 69
    assert proc.stdout.strip() == "", "diagnostics must not pollute stdout"
    assert json.loads(proc.stderr)["code"] == "BINARY_UNRESOLVED"


def test_unresolved_binary_is_never_rendered_as_a_shell_argv():
    """`exec ""` is the failure mode this guards.

    A launcher evals the human projection, so an empty rendering would exec
    nothing and look like success. The only safe rendering of an unresolved
    binary is no rendering at all.
    """
    from emit import Format, serialize
    from models import BinaryResolution

    unresolved = BinaryResolution(binary="b", resolved=False)
    with pytest.raises(ValueError, match="unresolved"):
        serialize(unresolved, Format.HUMAN)


def test_cli_show_is_schema_tagged(tmp_path):
    proc = _run_cli("show", cwd=tmp_path)
    assert proc.returncode == 0, proc.stderr
    assert json.loads(proc.stdout)["schema"] == "urn:chatman:roots-report:v1"


# --------------------------------------------------------------------------
# CE-GALL-32 -- ledger anchoring: a subdirectory must key the same ledger
# --------------------------------------------------------------------------


def test_project_key_is_identical_for_cwd_and_its_subdirectory():
    """The CE-GALL-32 defect: a `cd` into a subdirectory used to fork the ledger.

    `project_key` (and therefore `project_directory`) must anchor at
    `project_root()` rather than the raw realpath, so a command run from the
    repository root and the same command run from a subdirectory of that
    checkout (e.g. `plugins/chatman-ecosystem`) hash to the same project key
    and read/write the same ledger directory.
    """
    repo_root = roots.project_root()
    assert repo_root is not None, "this checkout must resolve a project root"

    subdir = repo_root / "plugins" / "chatman-ecosystem"
    assert subdir.is_dir()

    root_key = roots.project_key(str(repo_root))
    subdir_key = roots.project_key(str(subdir))
    assert root_key == subdir_key

    assert roots.project_directory(str(repo_root)) == roots.project_directory(str(subdir))


def test_project_key_does_not_collide_with_an_unrelated_env_provided_root(monkeypatch, tmp_path):
    """Regression: an env-provided root unrelated to the target path must not anchor it.

    The original CE-GALL-32 fix accepted whatever `project_root()` returned
    without checking that the resolved root actually contains the path being
    keyed. Since `project_root()` treats `CLAUDE_PROJECT_DIR`/`FERROPLAN_ROOT`
    as valid candidates regardless of the `start` argument, that meant ANY
    path -- including one entirely unrelated to the real checkout -- hashed
    to the SAME key as the checkout whenever those env vars happened to be
    set to it (which they normally are, during hook execution). This
    collapsed two unrelated projects' ledgers into one and, in practice,
    caused a live incident: a throwaway /tmp test directory silently wrote
    into and truncated the real repository's event ledger.

    `project_key` must reject an anchor that is not the target path itself
    or a real ancestor of it.
    """
    repo_root = roots.project_root()
    assert repo_root is not None, "this checkout must resolve a project root"

    unrelated = tmp_path / "unrelated-project"
    unrelated.mkdir()

    monkeypatch.setenv("CLAUDE_PROJECT_DIR", str(repo_root))
    monkeypatch.delenv("FERROPLAN_ROOT", raising=False)

    repo_key = roots.project_key(str(repo_root))
    unrelated_key = roots.project_key(str(unrelated))

    assert repo_key != unrelated_key, (
        "an unrelated directory must not collide with the real checkout's "
        "ledger key just because CLAUDE_PROJECT_DIR points at the checkout"
    )
