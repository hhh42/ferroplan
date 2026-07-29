#!/usr/bin/env python3
"""Resolve the plugin root, the project root, and executable binaries.

Every path guess in this plugin used to be made independently, and each one was
calibrated for a layout that is not the one that actually runs.

The concrete failure this module exists to prevent: `mcp_client` derived the
project directory by walking four parents up from the launcher script. Under the
repository layout (`<repo>/plugins/chatman-ecosystem/scripts/`) that lands on the
repository root and works. Under the *installed* layout
(`~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/scripts/`) -- the only
layout a user ever runs -- it lands on `.../cache/<marketplace>`, which has no
`crates/`, so the launcher fell through to `exit 69` while a perfectly good
binary sat in `target/debug/`. A depth-counted walk cannot be load-bearing
across two layouts.

So: locate the plugin by its own marker file rather than by counting parents,
locate the project by asking git rather than by assuming, and never skip a
binary that is already built.

The other half of the job is the failure message. The previous one named
neither what it looked for nor what it saw, and the caller then relabelled it
("receipt verification failed") so the user was told about receipts when the
real problem was a missing binary. A resolver that cannot explain itself just
moves the confusion.
"""

from __future__ import annotations

import hashlib
import os
import shlex
import shutil
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Annotated, NamedTuple

import typer

sys.path.insert(0, str(Path(__file__).resolve().parent))
from emit import Format, emit, emit_error, renders  # noqa: E402
from models import (  # noqa: E402
    BinaryResolution,
    ChatmanError,
    ResolutionAttempt,
    RootCandidate,
    RootsReport,
)

try:
    from plugin_data import plugin_data_root as _resolve_plugin_data_root
except ImportError:
    _resolve_plugin_data_root = None

#: Identifies a plugin checkout. Present in both the repo and the install cache.
PLUGIN_MARKER = Path(".claude-plugin") / "plugin.json"

#: Identifies a Ferroplan checkout able to build the MCP server.
FERROPLAN_MARKER = Path("crates") / "ferroplan-mcp" / "Cargo.toml"

#: Reported verbatim in failures so the reader sees what the process saw.
REPORTED_ENV = (
    "CLAUDE_PLUGIN_ROOT",
    "CLAUDE_PROJECT_DIR",
    "CLAUDE_PLUGIN_DATA",
    "FERROPLAN_ROOT",
    "CARGO_TARGET_DIR",
)


def reported_environment() -> dict[str, str | None]:
    """The steering variables as seen.

    `None` and `""` are kept distinct on purpose: an unset variable and an
    empty one fail differently, because `setdefault`-style derivation fires for
    one and not the other. Collapsing them is how a resolution bug hides.
    """
    return {name: os.environ.get(name) for name in REPORTED_ENV}


class ResolutionFailure(RuntimeError):
    """A resolution failure, carrying the data needed to explain itself.

    The message a human reads is rendered from `as_error()`, not stored as
    prose, so a caller can branch on the code and inspect the attempts without
    parsing English.
    """

    def __init__(self, target: str, tried: list[tuple[str, str]], fix: str) -> None:
        self.target = target
        self.tried = tried
        self.fix = fix
        super().__init__(f"cannot resolve `{target}`")

    def as_error(self) -> ChatmanError:
        return ChatmanError(
            code="BINARY_UNRESOLVED",
            message=f"cannot resolve `{self.target}`",
            context={
                "binary": self.target,
                "tried": [f"{candidate} -> {why}" for candidate, why in self.tried],
                "cwd": os.getcwd(),
                **reported_environment(),
            },
            remedy=self.fix,
        )


class ResolvedBinary(NamedTuple):
    argv: list[str]
    how: str
    root: Path | None


def _has(root: Path, marker: Path) -> bool:
    try:
        return (root / marker).is_file()
    except OSError:
        return False


def _env_path(name: str) -> Path | None:
    """Read an environment variable as a path, treating empty as absent."""
    raw = os.environ.get(name)
    return Path(raw) if raw else None


def _fallback_plugin_data_root() -> Path:
    if _resolve_plugin_data_root is not None:
        return _resolve_plugin_data_root()
    configured = os.environ.get("CLAUDE_PLUGIN_DATA")
    if configured:
        return Path(configured)
    return Path.home() / ".claude" / "plugins" / "data" / "chatman-ecosystem"


def project_key(project: str) -> str:
    """Stable, filesystem-safe identifier for a project checkout.

    Realpaths internally, so a caller may pass a bare cwd string, an
    already-resolved path, or `CLAUDE_PROJECT_DIR` -- the digest is the same.

    Anchors at `project_root()` when the realpath resolves to one *and* that
    root is `project` itself or a real ancestor of it, so callers from any
    subdirectory of a checkout hash to the same key as the repo root. An
    env-provided root (`FERROPLAN_ROOT`/`CLAUDE_PROJECT_DIR`) that is
    unrelated to `project` -- e.g. a stale env var pointing at a different
    checkout while a throwaway path is being keyed -- is rejected as an
    anchor rather than causing an unrelated project's key to collide. Falls
    back to the raw realpath when no related project root resolves (e.g.
    outside any recognized checkout), matching prior behavior exactly.
    """
    real = Path(os.path.realpath(project))
    root = project_root(real)
    if root is not None:
        try:
            resolved_root = root.resolve()
        except OSError:
            resolved_root = root
        if resolved_root != real and resolved_root not in real.parents:
            root = None
    anchor = root if root is not None else real
    return hashlib.sha256(str(anchor).encode("utf-8")).hexdigest()[:24]


def project_directory(project: str) -> Path:
    """Per-project state directory under the plugin's data root."""
    return _fallback_plugin_data_root() / "projects" / project_key(project)


def plugin_root(start: Path | None = None) -> Path:
    """Locate the plugin checkout by its marker, never by parent counting."""
    candidate = _env_path("CLAUDE_PLUGIN_ROOT")
    if candidate and _has(candidate, PLUGIN_MARKER):
        return candidate.resolve()

    here = (start or Path(__file__)).resolve()
    for parent in [here, *here.parents]:
        if _has(parent, PLUGIN_MARKER):
            return parent
    # `scripts/` sits directly under the plugin root in every known layout.
    return Path(__file__).resolve().parent.parent


def git_toplevel(start: Path) -> Path | None:
    try:
        proc = subprocess.run(
            ["git", "-C", str(start), "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if proc.returncode != 0:
        return None
    top = proc.stdout.strip()
    return Path(top) if top else None


def project_candidates(
    start: Path | None = None,
    *,
    marker: Path = FERROPLAN_MARKER,
    siblings: Sequence[str] = (),
) -> list[tuple[str, Path]]:
    """Ordered (provenance, path) pairs to try as the project root.

    `marker` identifies what a usable root must contain, so the same resolution
    order serves both the in-workspace `ferroplan-mcp` and a sibling checkout
    like `claude-code-config-lsp`. `siblings` names directories to try beside
    the resolved repository, which is how a neighbouring checkout is found
    without hardcoding anyone's directory layout.
    """
    begin = (start or Path.cwd()).resolve()
    out: list[tuple[str, Path]] = []

    for name, provenance in (
        ("FERROPLAN_ROOT", "env:FERROPLAN_ROOT"),
        ("CLAUDE_PROJECT_DIR", "env:CLAUDE_PROJECT_DIR"),
    ):
        value = _env_path(name)
        if value:
            out.append((provenance, value))

    top = git_toplevel(begin)
    if top:
        out.append(("git toplevel", top))

    for parent in [begin, *begin.parents]:
        if _has(parent, marker):
            out.append(("upward walk from cwd", parent))
            break

    # The plugin is vendored at <repo>/plugins/<name>/ in a source checkout.
    vendored = plugin_root().parent.parent
    out.append(("plugin root ../..", vendored))

    for sibling in siblings:
        out.append((f"sibling checkout ../{sibling}", vendored.parent / sibling))
    return out


def project_root(
    start: Path | None = None,
    *,
    marker: Path = FERROPLAN_MARKER,
    siblings: Sequence[str] = (),
) -> Path | None:
    for _, candidate in project_candidates(start, marker=marker, siblings=siblings):
        if _has(candidate, marker):
            return candidate.resolve()
    return None


def cargo_target_dirs(root: Path) -> list[Path]:
    """Built-binary directories, release first.

    This branch did not exist before. A `cargo run` that rebuilds the world was
    preferred over a binary already sitting on disk, and when cargo was missing
    the resolution failed outright with the binary right there.
    """
    bases: list[Path] = []
    override = _env_path("CARGO_TARGET_DIR")
    if override:
        bases.append(override)
    bases.append(root / "target")
    return [base / profile for base in bases for profile in ("release", "debug")]


def resolve_binary(
    name: str,
    *,
    crate: str | None = None,
    start: Path | None = None,
    extra_roots: Sequence[tuple[str, Path]] = (),
    marker: Path = FERROPLAN_MARKER,
    siblings: Sequence[str] = (),
) -> ResolvedBinary:
    tried: list[tuple[str, str]] = []

    found = shutil.which(name)
    if found:
        return ResolvedBinary([found], "PATH", None)
    tried.append((f"{name} (PATH lookup)", "not found on PATH"))

    candidates = [*extra_roots, *project_candidates(start, marker=marker, siblings=siblings)]
    seen: set[Path] = set()
    root: Path | None = None
    for provenance, candidate in candidates:
        try:
            resolved = candidate.resolve()
        except OSError:
            continue
        if resolved in seen:
            continue
        seen.add(resolved)
        if not _has(resolved, marker):
            tried.append((f"{resolved} (via {provenance})", f"no {marker}"))
            continue
        root = resolved
        break

    if root is None:
        raise ResolutionFailure(
            name,
            tried,
            f"set FERROPLAN_ROOT to a checkout containing {marker}, "
            f"or install `{name}` onto PATH.",
        )

    for candidate in cargo_target_dirs(root):
        binary = candidate / name
        if binary.is_file() and os.access(binary, os.X_OK):
            return ResolvedBinary([str(binary)], f"built binary ({candidate.name})", root)
        if binary.is_file():
            tried.append((str(binary), "present but not executable"))
        else:
            tried.append((str(binary), "no such file"))

    cargo = shutil.which("cargo")
    if cargo and crate:
        argv = [
            cargo, "run", "--locked", "--quiet",
            "--manifest-path", str(root / "Cargo.toml"),
            "-p", crate, "--bin", name, "--",
        ]
        return ResolvedBinary(argv, "cargo run", root)
    tried.append((
        f"cargo run -p {crate or name}",
        "cargo not found on PATH" if not cargo else "no crate name given",
    ))

    raise ResolutionFailure(
        name,
        tried,
        f"run `cargo build -p {crate or name}` in {root}, or install `{name}` onto PATH.",
    )


app = typer.Typer(
    add_completion=False,
    no_args_is_help=True,
    help="Resolve the plugin root, the project root, and executable binaries.",
)


@renders(BinaryResolution)
def _render_resolution(resolution: BinaryResolution) -> str:
    """The shell-consumable projection: an exec-ready argv, nothing else.

    This is what the launcher scripts eval, which is precisely why it is a
    *projection* and not the payload -- a shell string cannot carry provenance,
    and the provenance is what makes a slow `cargo run` distinguishable from a
    resolved binary.

    An unresolved payload has no argv, and rendering it as the empty string
    would hand a launcher `exec ""`. That can only be reached by rendering a
    failure that should have exited 69, so it is an internal error rather than
    an output.
    """
    if not resolution.resolved or not resolution.argv:
        raise ValueError(
            "refusing to render an unresolved binary as a shell argv; "
            "the failure path must emit a ChatmanError and exit 69"
        )
    return shlex.join(resolution.argv)


@app.command()
def resolve(
    binary: Annotated[str, typer.Option(help="Executable to locate.")],
    crate: Annotated[str | None, typer.Option(help="Cargo package providing --binary.")] = None,
    env_root: Annotated[
        str | None, typer.Option(help="Extra environment variable naming a checkout.")
    ] = None,
    marker: Annotated[
        str | None,
        typer.Option(help="Relative file a usable checkout must contain."),
    ] = None,
    sibling: Annotated[
        list[str] | None,
        typer.Option(help="Directory name to try beside the repository. Repeatable."),
    ] = None,
    fmt: Annotated[
        Format, typer.Option("--format", help="Output format.")
    ] = Format.JSON,
) -> None:
    """Locate an executable and report how it was found.

    Exits 69 (EX_UNAVAILABLE) when nothing resolves, matching the launcher
    contract. Launchers pass `--format human` to get a bare argv to eval.
    """
    extra: list[tuple[str, Path]] = []
    if env_root:
        value = _env_path(env_root)
        if value:
            extra.append((f"env:{env_root}", value))

    try:
        resolved = resolve_binary(
            binary,
            crate=crate,
            extra_roots=extra,
            marker=Path(marker) if marker else FERROPLAN_MARKER,
            siblings=tuple(sibling or ()),
        )
    except ResolutionFailure as failure:
        emit_error(failure.as_error(), fmt, exit_code=69)
        raise typer.Exit(code=69) from None

    emit(
        BinaryResolution(
            binary=binary,
            resolved=True,
            argv=resolved.argv,
            how=resolved.how,
            project_root=str(resolved.root) if resolved.root else None,
            attempts=[ResolutionAttempt(candidate=resolved.argv[0], outcome="accepted")],
            environment=reported_environment(),
        ),
        fmt,
    )


@app.command()
def show(
    fmt: Annotated[Format, typer.Option("--format", help="Output format.")] = Format.JSON,
) -> None:
    """Report how each root resolves, and by which rule."""
    root = project_root()
    emit(
        RootsReport(
            plugin_root=str(plugin_root()),
            project_root=str(root) if root else None,
            project_candidates=[
                RootCandidate(provenance=p, path=str(c), usable=_has(c, FERROPLAN_MARKER))
                for p, c in project_candidates()
            ],
            target_dirs=[str(d) for d in cargo_target_dirs(root)] if root else [],
            environment=reported_environment(),
        ),
        fmt,
    )


if __name__ == "__main__":
    app()
