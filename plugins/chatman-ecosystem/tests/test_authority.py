"""The authority model must reach the runtime, not just the ontology."""

from __future__ import annotations

import pyshacl
import pytest
import rdflib

import generate
from roots import plugin_root

ODRL = rdflib.Namespace("http://www.w3.org/ns/odrl/2/")


@pytest.fixture(scope="module")
def authority() -> rdflib.Graph:
    return rdflib.Graph().parse(
        plugin_root() / "ontology" / "authority-graph.ttl", format="turtle"
    )


@pytest.fixture(scope="module")
def shapes() -> rdflib.Graph:
    return rdflib.Graph().parse(
        plugin_root() / "ontology" / "chatman-shapes.ttl", format="turtle"
    )


def frontmatter_values(relative: str) -> dict[str, str]:
    lines, _body = generate._split_frontmatter(
        (plugin_root() / relative).read_text(encoding="utf-8")
    )
    return {
        key.strip(): value.strip()
        for line in lines
        if ":" in line
        for key, value in [line.split(":", 1)]
    }


def test_authority_graph_conforms_to_its_shapes(authority, shapes):
    conforms, _graph, text = pyshacl.validate(authority, shacl_graph=shapes, advanced=True)
    assert conforms, text


def test_every_agent_file_named_by_the_ontology_exists(authority):
    root = plugin_root()
    CE = generate.CE
    for agent in authority.subjects(rdflib.RDF.type, CE.AgentDefinition):
        path = authority.value(agent, CE.frontmatterPath)
        assert path is not None
        assert (root / str(path)).is_file(), f"ontology names {path}, which is missing"


def test_every_agent_file_is_covered_by_the_ontology():
    root = plugin_root()
    declared = set(generate.agent_tools(root))
    on_disk = {f"agents/{path.name}" for path in (root / "agents").glob("*.md")}
    assert on_disk == declared, f"undeclared agents: {sorted(on_disk - declared)}"


def test_frontmatter_grants_exactly_what_the_ontology_allows():
    for relative, grant in generate.agent_tools(plugin_root()).items():
        values = frontmatter_values(relative)
        assert "tools" in values, f"{relative} inherits ambient tools"
        assert generate.parse_tool_list(values["tools"]) == grant.tools, relative


def test_denials_effort_and_turn_caps_are_projected():
    for relative, grant in generate.agent_tools(plugin_root()).items():
        values = frontmatter_values(relative)
        denied = generate.parse_tool_list(values.get("disallowedTools", ""))
        assert denied == grant.denied_tools, relative
        assert values.get("effort") == grant.effort, relative
        assert values.get("maxTurns") == str(grant.max_turns), relative


def test_denied_tools_are_absent_from_frontmatter():
    for relative, grant in generate.agent_tools(plugin_root()).items():
        values = frontmatter_values(relative)
        granted = set(generate.parse_tool_list(values["tools"]))
        denied = set(grant.denied_tools)
        overlap = granted & denied
        assert not overlap, f"{relative} grants denied tool(s): {sorted(overlap)}"


def test_controller_agent_grant_is_bounded_by_may_spawn():
    grants = generate.agent_tools(plugin_root())
    controller = grants["agents/ecosystem-controller.md"]
    agent_tools = [tool for tool in controller.tools if tool.startswith("Agent(")]
    assert len(agent_tools) == 1
    bounded = agent_tools[0]
    assert "source-manufacturer" in bounded
    assert "ecosystem-controller" not in bounded
    assert "Agent" not in set(controller.tools)


def test_single_actuator_policy_is_enforced(authority):
    root = plugin_root()
    CE = generate.CE
    write_tools = {"Write", "Edit", "NotebookEdit"}

    permitted = {
        str(authority.value(assignee, CE.frontmatterPath))
        for permission in authority.objects(None, ODRL.permission)
        for assignee in authority.objects(permission, ODRL.assignee)
    }
    assert permitted, "no ODRL permission found; the policy would be vacuous"

    writers = set()
    for agent in authority.subjects(rdflib.RDF.type, CE.AgentDefinition):
        relative = str(authority.value(agent, CE.frontmatterPath))
        values = frontmatter_values(relative)
        granted = set(generate.parse_tool_list(values["tools"]))
        if granted & write_tools:
            writers.add(relative)

    assert writers == permitted, (
        f"agents able to modify: {sorted(writers)}; ODRL permits: {sorted(permitted)}"
    )


def test_only_the_actuator_is_isolated():
    isolated = {
        relative
        for relative, grant in generate.agent_tools(plugin_root()).items()
        if grant.isolation
    }
    assert isolated == {"agents/source-manufacturer.md"}


def test_mcp_tool_prefix_is_derived_not_hardcoded():
    prefix = generate.mcp_tool_prefix(plugin_root())
    assert prefix.startswith("mcp__plugin_")
    assert prefix.endswith("__")
    assert "chatman-ecosystem" in prefix


def test_mcp_tool_expansion_matches_the_server_ontology():
    tools = generate.ferroplan_mcp_tools(plugin_root())
    assert len(tools) == 17, len(tools)
    assert all(tool.startswith("mcp__plugin_") for tool in tools)
    assert any(tool.endswith("__cmca_allocate") for tool in tools)
    assert tools == sorted(tools), "expansion must be deterministic"
