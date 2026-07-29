#!/usr/bin/env python3
"""Generate the projections of this plugin's machine-readable sources.

Nothing here is authored by hand. Each output is derived from a source of truth
and regenerated in CI with `--check`, so a projection cannot drift from what it
projects. That drift is the defect this plugin kept reproducing: precise
declarations that no consumer was ever checked against.

Sources and their projections:

    scripts/models.py   -->  schemas/*.json     (JSON Schema per output urn)

Further projections (agent frontmatter and the C4 diagrams, both from
`ontology/authority-graph.ttl`) attach here as additional generators.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Annotated, NamedTuple

import typer

sys.path.insert(0, str(Path(__file__).resolve().parent))
from models import REGISTRY  # noqa: E402
from roots import plugin_root  # noqa: E402

SCHEMA_DIALECT = "https://json-schema.org/draft/2020-12/schema"


class Projection(NamedTuple):
    """One generated file: where it goes and what it should contain."""

    path: Path
    content: str


def urn_to_filename(urn: str) -> str:
    """`urn:chatman:claude-code-loop-state:v1` -> `claude-code-loop-state.v1.json`.

    The version stays in the filename so two versions of one payload can be
    published side by side rather than one silently replacing the other.
    """
    body = urn.removeprefix("urn:chatman:")
    name, _, version = body.rpartition(":")
    return f"{name}.{version}.json"


def schema_projections(root: Path) -> list[Projection]:
    out: list[Projection] = []
    for model in REGISTRY:
        schema = model.json_schema()
        schema["$schema"] = SCHEMA_DIALECT
        schema["$id"] = model.SCHEMA
        content = json.dumps(schema, indent=2, sort_keys=True) + "\n"
        out.append(Projection(root / "schemas" / urn_to_filename(model.SCHEMA), content))
    return out


def all_projections(root: Path) -> list[Projection]:
    return schema_projections(root)


app = typer.Typer(
    add_completion=False,
    no_args_is_help=True,
    help="Generate machine-readable projections; --check proves they are current.",
)


@app.callback()
def _root() -> None:
    """Present so the app keeps named subcommands as further generators land."""


@app.command()
def build(
    check: Annotated[
        bool,
        typer.Option(
            "--check",
            help="Do not write. Exit 1 if any projection is missing or stale.",
        ),
    ] = False,
) -> None:
    """Write every projection, or verify that the committed ones are current."""
    root = plugin_root()
    projections = all_projections(root)

    if check:
        stale: list[str] = []
        for projection in projections:
            relative = projection.path.relative_to(root)
            if not projection.path.is_file():
                stale.append(f"missing: {relative}")
            elif projection.path.read_text(encoding="utf-8") != projection.content:
                stale.append(f"stale:   {relative}")
        if stale:
            print("generated files are not current:", file=sys.stderr)
            for line in stale:
                print(f"  {line}", file=sys.stderr)
            print("", file=sys.stderr)
            print("fix: python3 scripts/generate.py build", file=sys.stderr)
            raise typer.Exit(code=1)
        typer.echo(json.dumps({"checked": len(projections), "stale": 0}))
        return

    written = 0
    for projection in projections:
        projection.path.parent.mkdir(parents=True, exist_ok=True)
        if (
            not projection.path.is_file()
            or projection.path.read_text(encoding="utf-8") != projection.content
        ):
            projection.path.write_text(projection.content, encoding="utf-8")
            written += 1
    typer.echo(json.dumps({"total": len(projections), "written": written}))


if __name__ == "__main__":
    app()
