#!/usr/bin/env python3
"""Generate the projections of this plugin's machine-readable sources.

Nothing here is authored by hand. Each output is derived from a source of truth
and regenerated in CI with `--check`, so a projection cannot drift from what it
projects. That drift is the defect this plugin kept reproducing: precise
declarations that no consumer was ever checked against.

Sources and their projections:

    scripts/models.py               -->  schemas/*.json   (JSON Schema per urn)
    ontology/authority-graph.ttl    -->  agents/*.md      (tools, isolation)

The agent projection is the one that changes behaviour. The ontology declared
`ce:allowsTool`/`ce:deniesTool` for all eight agents and an ODRL policy naming
exactly one agent permitted to modify the repository -- while no agent file
declared any tools at all, so every agent inherited every tool. The allocator
ran with Write, Edit and Bash that the ontology explicitly denies it. Generating
the frontmatter is what turns that declaration into a constraint.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Annotated, NamedTuple

import rdflib
import typer

sys.path.insert(0, str(Path(__file__).resolve().parent))
from models import REGISTRY  # noqa: E402
from roots import plugin_root  # noqa: E402

SCHEMA_DIALECT = "https://json-schema.org/draft/2020-12/schema"

CE = rdflib.Namespace("urn:chatman:ecosystem:")
FP = rdflib.Namespace("urn:chatman:ferroplan-domain:v1#")
DCTERMS = rdflib.Namespace("http://purl.org/dc/terms/")

#: Frontmatter keys this generator owns. Everything else in an agent file is
#: left exactly as its author wrote it.
MANAGED_KEYS = ("tools", "isolation")

#: `ce:maxTurns` and `ce:effort` are declared in the ontology but deliberately
#: not projected: they are not part of the agent frontmatter schema, and
#: emitting unrecognised keys would trade one silent non-enforcement for a
#: loader warning. They remain documented-and-unenforced, which is recorded
#: rather than hidden.
UNPROJECTED_PREDICATES = ("maxTurns", "effort")


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


# --------------------------------------------------------------------------
# agent frontmatter, from ontology/authority-graph.ttl
# --------------------------------------------------------------------------


def mcp_tool_prefix(root: Path) -> str:
    """The runtime name prefix for this plugin's MCP tools.

    Derived rather than hardcoded: the plugin name comes from the manifest and
    the server name from the MCP declaration, which together produce the
    `mcp__plugin_<plugin>_<server>__<tool>` names the harness exposes.
    """
    manifest = json.loads(
        (root / ".claude-plugin" / "plugin.json").read_text(encoding="utf-8")
    )
    servers = json.loads((root / ".mcp.json").read_text(encoding="utf-8"))["mcpServers"]
    (server_name,) = servers.keys()
    return f"mcp__plugin_{manifest['name']}_{server_name}__"


def ferroplan_mcp_tools(root: Path) -> list[str]:
    """Every tool the Ferroplan MCP server provides, named as the harness sees it."""
    graph = rdflib.Graph().parse(root / "ontology" / "ferroplan-domain.ttl", format="turtle")
    labels = sorted(
        str(graph.value(tool, rdflib.RDFS.label))
        for tool in graph.subjects(rdflib.RDF.type, FP.McpTool)
        if graph.value(tool, FP.providedBy) == FP.FerroplanMcp
    )
    prefix = mcp_tool_prefix(root)
    return [f"{prefix}{label}" for label in labels]


class AgentGrant(NamedTuple):
    tools: list[str]
    isolation: str | None


def agent_tools(root: Path) -> dict[str, AgentGrant]:
    """Resolve each agent's granted tools from the authority graph.

    `ce:allowsTool` is the grant; `ce:deniesTool` is asserted separately and is
    checked here rather than assumed, because a declaration that both allows and
    denies the same tool is a contradiction in the source, not something to
    silently resolve one way.
    """
    graph = rdflib.Graph().parse(root / "ontology" / "authority-graph.ttl", format="turtle")
    mcp_tools = ferroplan_mcp_tools(root)

    resolved: dict[str, AgentGrant] = {}
    for agent in graph.subjects(rdflib.RDF.type, CE.AgentDefinition):
        path = graph.value(agent, CE.frontmatterPath)
        if path is None:
            continue

        def titles(predicate, subject=agent):
            return {str(graph.value(t, DCTERMS.title)) for t in graph.objects(subject, predicate)}

        allowed, denied = titles(CE.allowsTool), titles(CE.deniesTool)
        contradiction = allowed & denied
        if contradiction:
            raise SystemExit(
                f"{path}: authority graph both allows and denies {sorted(contradiction)}"
            )

        # Harness tools first, then the MCP expansion. Both sorted, so the
        # projection is byte-stable across runs and reviewable as a diff.
        plain = sorted(t for t in allowed if t != "Ferroplan MCP tools")
        tools = [*plain, *(mcp_tools if "Ferroplan MCP tools" in allowed else ())]

        isolation = graph.value(agent, CE.isolatedBy)
        resolved[str(path)] = AgentGrant(
            tools=tools, isolation=str(isolation) if isolation else None
        )
    return resolved


def _split_frontmatter(text: str) -> tuple[list[str], str]:
    """Return (frontmatter lines, body) for a `---` delimited markdown file."""
    if not text.startswith("---\n"):
        raise SystemExit("agent file does not begin with a frontmatter block")
    end = text.index("\n---\n", 3)
    return text[4:end].splitlines(), text[end + 5 :]


def agent_projections(root: Path) -> list[Projection]:
    out: list[Projection] = []
    for relative, grant in sorted(agent_tools(root).items()):
        path = root / relative
        if not path.is_file():
            raise SystemExit(f"authority graph names {relative}, which does not exist")

        existing, body = _split_frontmatter(path.read_text(encoding="utf-8"))

        # Preserve every key the generator does not own, in the author's order.
        preserved = [
            line
            for line in existing
            if not any(line.startswith(f"{key}:") for key in MANAGED_KEYS)
        ]

        managed = [f"tools: {', '.join(grant.tools)}"]
        if grant.isolation:
            managed.append(f"isolation: {grant.isolation}")

        content = "---\n" + "\n".join([*preserved, *managed]) + "\n---\n" + body
        out.append(Projection(path, content))
    return out


def all_projections(root: Path) -> list[Projection]:
    return [*schema_projections(root), *agent_projections(root)]


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
