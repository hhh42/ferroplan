#!/usr/bin/env python3
"""Generate the projections of this plugin's machine-readable sources.

Nothing here is authored by hand. Each output is derived from a source of truth
and regenerated in CI with `--check`, so a projection cannot drift from what it
projects.

Sources and projections:

    scripts/models.py               --> schemas/*.json
    ontology/authority-graph.ttl    --> agents/*.md
    ontology/chatman-ecosystem.ttl  --> scripts/_standing.py
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

# These fields are generated from authority-graph.ttl. All other frontmatter
# remains author-owned and is preserved byte-for-byte in its existing order.
MANAGED_KEYS = (
    "effort",
    "maxTurns",
    "tools",
    "disallowedTools",
    "isolation",
)


class Projection(NamedTuple):
    path: Path
    content: str


def urn_to_filename(urn: str) -> str:
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
# standing vocabulary, from ontology/chatman-ecosystem.ttl
# --------------------------------------------------------------------------


def standing_vocabulary(root: Path) -> tuple[list[tuple[str, int, str]], list[tuple[str, str]]]:
    graph = rdflib.Graph().parse(root / "ontology" / "chatman-ecosystem.ttl", format="turtle")
    standings = sorted(
        (
            str(graph.value(node, DCTERMS.title)),
            int(graph.value(node, CE.standingRank)),
            str(graph.value(node, rdflib.RDFS.comment)),
        )
        for node in graph.subjects(rdflib.RDF.type, CE.Standing)
    )
    reasons = sorted(
        (str(graph.value(node, DCTERMS.title)), str(graph.value(node, rdflib.RDFS.comment)))
        for node in graph.subjects(rdflib.RDF.type, CE.StandingReason)
    )
    if not standings:
        raise SystemExit("ontology declares no ce:Standing individuals")
    return sorted(standings, key=lambda item: -item[1]), reasons


def standing_projection(root: Path) -> Projection:
    standings, reasons = standing_vocabulary(root)
    lines = [
        '"""Standing vocabulary. GENERATED from ontology/chatman-ecosystem.ttl.',
        "",
        "Do not edit. Run `python3 scripts/generate.py build` instead.",
        "",
        "Three unreconciled vocabularies existed before this file: loop.py accepted",
        "four values, docs/gall-checkpoints.md listed seven, and the canonical set has",
        "six. A standing that cannot be recorded in the ledger is not a standing.",
        '"""',
        "",
        "from __future__ import annotations",
        "",
        "from enum import StrEnum",
        "",
        "",
        "class Standing(StrEnum):",
        '    """The six canonical standings, strongest first."""',
        "",
    ]
    for title, _rank, comment in standings:
        lines.append(f"    #: {comment}")
        lines.append(f'    {title} = "{title}"')
    lines += [
        "",
        "",
        "class StandingReason(StrEnum):",
        '    """Why a standing is capped. Never a standing in its own right."""',
        "",
    ]
    for title, comment in reasons:
        lines.append(f"    #: {comment}")
        lines.append(f'    {title} = "{title}"')
    lines += [
        "",
        "",
        "#: Advisory ordering for the rule that a checkpoint preserves the standing of",
        "#: its predecessors. Not a lattice: UNKNOWN and UNSUPPORTED are both 'no",
        "#: positive claim' and differ in why, not in strength.",
        "RANK: dict[Standing, int] = {",
    ]
    lines += [f"    Standing.{title}: {rank}," for title, rank, _comment in standings]
    lines += [
        "}",
        "",
        "#: Default for a surface that has done work but cannot be promoted.",
        "DEFAULT = Standing.PARTIAL_ALIVE",
        "",
    ]
    return Projection(root / "scripts" / "_standing.py", "\n".join(lines))


# --------------------------------------------------------------------------
# agent frontmatter, from ontology/authority-graph.ttl
# --------------------------------------------------------------------------


def plugin_name(root: Path) -> str:
    manifest = json.loads(
        (root / ".claude-plugin" / "plugin.json").read_text(encoding="utf-8")
    )
    return str(manifest["name"])


def mcp_tool_prefix(root: Path) -> str:
    manifest_name = plugin_name(root)
    servers = json.loads((root / ".mcp.json").read_text(encoding="utf-8"))["mcpServers"]
    (server_name,) = servers.keys()
    return f"mcp__plugin_{manifest_name}_{server_name}__"


def ferroplan_mcp_tools(root: Path) -> list[str]:
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
    denied_tools: list[str]
    isolation: str | None
    effort: str
    max_turns: int


def _title_set(graph: rdflib.Graph, subject, predicate) -> set[str]:
    return {
        str(graph.value(tool, DCTERMS.title))
        for tool in graph.objects(subject, predicate)
    }


def _bounded_agent_tool(graph: rdflib.Graph, agent, root: Path) -> str:
    children: list[str] = []
    for child in graph.objects(agent, CE.maySpawn):
        relative = graph.value(child, CE.frontmatterPath)
        if relative is None:
            raise SystemExit(f"spawn target {child} has no ce:frontmatterPath")
        children.append(f"{plugin_name(root)}:{Path(str(relative)).stem}")
    if not children:
        raise SystemExit(f"{agent} allows Agent but declares no ce:maySpawn targets")
    return f"Agent({', '.join(sorted(children))})"


def agent_tools(root: Path) -> dict[str, AgentGrant]:
    graph = rdflib.Graph().parse(root / "ontology" / "authority-graph.ttl", format="turtle")
    mcp_tools = ferroplan_mcp_tools(root)
    resolved: dict[str, AgentGrant] = {}

    for agent in graph.subjects(rdflib.RDF.type, CE.AgentDefinition):
        path = graph.value(agent, CE.frontmatterPath)
        if path is None:
            continue

        allowed = _title_set(graph, agent, CE.allowsTool)
        denied = _title_set(graph, agent, CE.deniesTool)
        contradiction = allowed & denied
        if contradiction:
            raise SystemExit(
                f"{path}: authority graph both allows and denies {sorted(contradiction)}"
            )

        tools = sorted(
            tool for tool in allowed if tool not in {"Agent", "Ferroplan MCP tools"}
        )
        if "Agent" in allowed:
            tools.append(_bounded_agent_tool(graph, agent, root))
        if "Ferroplan MCP tools" in allowed:
            tools.extend(mcp_tools)

        effort = graph.value(agent, CE.effort)
        max_turns = graph.value(agent, CE.maxTurns)
        if effort is None or max_turns is None:
            raise SystemExit(f"{path}: authority graph must declare ce:effort and ce:maxTurns")

        isolation = graph.value(agent, CE.isolatedBy)
        resolved[str(path)] = AgentGrant(
            tools=tools,
            denied_tools=sorted(denied),
            isolation=str(isolation) if isolation else None,
            effort=str(effort),
            max_turns=int(max_turns),
        )
    return resolved


def parse_tool_list(value: str) -> list[str]:
    """Split a Claude tool grant while preserving commas inside `Agent(...)`."""
    result: list[str] = []
    current: list[str] = []
    depth = 0
    for character in value:
        if character == "(":
            depth += 1
        elif character == ")":
            depth = max(0, depth - 1)
        if character == "," and depth == 0:
            token = "".join(current).strip()
            if token:
                result.append(token)
            current = []
        else:
            current.append(character)
    token = "".join(current).strip()
    if token:
        result.append(token)
    return result


def _split_frontmatter(text: str) -> tuple[list[str], str]:
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
        preserved = [
            line
            for line in existing
            if not any(line.startswith(f"{key}:") for key in MANAGED_KEYS)
        ]
        managed = [
            f"effort: {grant.effort}",
            f"maxTurns: {grant.max_turns}",
            f"tools: {', '.join(grant.tools)}",
        ]
        if grant.denied_tools:
            managed.append(f"disallowedTools: {', '.join(grant.denied_tools)}")
        if grant.isolation:
            managed.append(f"isolation: {grant.isolation}")

        content = "---\n" + "\n".join([*preserved, *managed]) + "\n---\n" + body
        out.append(Projection(path, content))
    return out


def all_projections(root: Path) -> list[Projection]:
    return [standing_projection(root), *schema_projections(root), *agent_projections(root)]


app = typer.Typer(
    add_completion=False,
    no_args_is_help=True,
    help="Generate machine-readable projections; --check proves they are current.",
)


@app.callback()
def _root() -> None:
    """Keep named subcommands as additional generators are added."""


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
            print("\nfix: python3 scripts/generate.py build", file=sys.stderr)
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
