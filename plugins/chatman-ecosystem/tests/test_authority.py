"""The authority model must reach the runtime, not just the ontology.

`authority-graph.ttl` declared tools, isolation, turn caps, and an ODRL policy
naming exactly one agent permitted to modify the repository. No agent file
declared any tools at all, so all eight inherited every tool -- the allocator
ran with Write, Edit and Bash that the ontology explicitly denies it.

A declaration nothing checks is a wish. These tests are the check.
"""

from __future__ import annotations

import generate
import pyshacl
import pytest
import rdflib
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


def test_authority_graph_conforms_to_its_shapes(authority, shapes):
    """SHACL caught that ce:maxTurns values did not satisfy their own declared
    range. Shapes that nobody runs are documentation; this runs them."""
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
    """The reverse direction: an agent the ontology forgot has no constraints."""
    root = plugin_root()
    declared = set(generate.agent_tools(root))
    on_disk = {f"agents/{p.name}" for p in (root / "agents").glob("*.md")}
    assert on_disk == declared, f"undeclared agents: {sorted(on_disk - declared)}"


def test_frontmatter_grants_exactly_what_the_ontology_allows():
    """The projection must match its source, agent by agent."""
    root = plugin_root()
    for relative, grant in generate.agent_tools(root).items():
        lines, _body = generate._split_frontmatter(
            (root / relative).read_text(encoding="utf-8")
        )
        tools_line = next((line for line in lines if line.startswith("tools:")), None)
        assert tools_line, f"{relative} declares no tools, so it inherits everything"
        granted = [t.strip() for t in tools_line.removeprefix("tools:").split(",")]
        assert granted == grant.tools, relative


def test_denied_tools_are_absent_from_frontmatter(authority):
    """The property that actually matters for safety."""
    root = plugin_root()
    CE = generate.CE
    DCTERMS = generate.DCTERMS
    for agent in authority.subjects(rdflib.RDF.type, CE.AgentDefinition):
        relative = str(authority.value(agent, CE.frontmatterPath))
        denied = {
            str(authority.value(t, DCTERMS.title))
            for t in authority.objects(agent, CE.deniesTool)
        }
        lines, _ = generate._split_frontmatter((root / relative).read_text(encoding="utf-8"))
        tools_line = next(line for line in lines if line.startswith("tools:"))
        granted = {t.strip() for t in tools_line.removeprefix("tools:").split(",")}
        overlap = granted & denied
        assert not overlap, f"{relative} grants denied tool(s): {sorted(overlap)}"


def test_single_actuator_policy_is_enforced(authority):
    """ODRL declares one agent may modify. Verify no other agent can.

    This is the safety property the whole authority graph exists to state, and
    until the frontmatter was generated it was true of the ontology and false
    of the running system.
    """
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
        lines, _ = generate._split_frontmatter((root / relative).read_text(encoding="utf-8"))
        tools_line = next(line for line in lines if line.startswith("tools:"))
        granted = {t.strip() for t in tools_line.removeprefix("tools:").split(",")}
        if granted & write_tools:
            writers.add(relative)

    assert writers == permitted, (
        f"agents able to modify: {sorted(writers)}; ODRL permits: {sorted(permitted)}"
    )


def test_only_the_actuator_is_isolated():
    """Isolation is expensive; it belongs exactly where mutation happens."""
    root = plugin_root()
    isolated = {rel for rel, grant in generate.agent_tools(root).items() if grant.isolation}
    assert isolated == {"agents/source-manufacturer.md"}


def test_mcp_tool_prefix_is_derived_not_hardcoded():
    """The prefix follows plugin and server names; hardcoding it would rot."""
    root = plugin_root()
    prefix = generate.mcp_tool_prefix(root)
    assert prefix.startswith("mcp__plugin_")
    assert prefix.endswith("__")
    assert "chatman-ecosystem" in prefix


def test_mcp_tool_expansion_matches_the_server_ontology():
    """All 17 tools, named as the harness exposes them."""
    tools = generate.ferroplan_mcp_tools(plugin_root())
    assert len(tools) == 17, len(tools)
    assert all(t.startswith("mcp__plugin_") for t in tools)
    assert any(t.endswith("__cmca_allocate") for t in tools)
    assert tools == sorted(tools), "expansion must be deterministic"


def test_unprojected_predicates_are_recorded_as_such():
    """maxTurns and effort are declared but not enforced.

    Pinned deliberately: this is a known unenforced declaration, and naming it
    is the difference between a recorded negative and the silent gap that this
    whole test module exists because of.
    """
    assert generate.UNPROJECTED_PREDICATES == ("maxTurns", "effort")
