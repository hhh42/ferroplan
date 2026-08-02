from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    target = ROOT / path
    text = target.read_text()
    found = text.count(old)
    if found < count:
        raise SystemExit(
            f"EXPERIENCE_PROJECTION_ANCHOR_MISSING {path}: wanted {count}, found {found}: {old[:120]!r}"
        )
    target.write_text(text.replace(old, new, count))


main = ROOT / "crates/ferroplan-mcp/src/main.rs"
text = main.read_text()
if "mod experience;" not in text:
    text = text.replace("mod admission;\n", "mod admission;\nmod experience;\n", 1)
    text = text.replace(
        "                + Self::session_control_router()\n                + Self::admission_router(),",
        "                + Self::session_control_router()\n                + Self::experience_router()\n                + Self::admission_router(),",
        1,
    )
    text = text.replace(
        "            .or_else(|| session_control::ontology_comment(name))\n            .or_else(|| admission::ontology_comment(name))",
        "            .or_else(|| session_control::ontology_comment(name))\n            .or_else(|| experience::ontology_comment(name))\n            .or_else(|| admission::ontology_comment(name))",
        1,
    )
    text = text.replace(
        "        .chain(session_control::RESOURCE_TOOLS)\n        .chain(admission::RESOURCE_TOOLS)",
        "        .chain(session_control::RESOURCE_TOOLS)\n        .chain(experience::RESOURCE_TOOLS)\n        .chain(admission::RESOURCE_TOOLS)",
        1,
    )
    text = text.replace("fn all_tool_names()", "pub(crate) fn all_tool_names()", 1)
    text = text.replace("as 31 MCP tools", "as 42 MCP tools")
    text = text.replace("resource per tool (31", "resource per tool (42")
    text = text.replace("All 31 tool names", "All 42 tool names")
    text = text.replace(
        "/// (stateless planning, then session, then admission).",
        "/// (stateless planning, session, persistent control, operator experience, then admission).",
    )
    text = text.replace(
        "three separate per-module",
        "four separate per-module",
    )
    text = text.replace(
        "sums the three `ToolRouter`s",
        "sums the merged `ToolRouter`s",
    )
    text = text.replace(
        "branch, checkpoint, restore, compare, scope, and drive time through the `session_*` control tools; `cmca_allocate` runs",
        "branch, checkpoint, restore, compare, scope, and drive time through the `session_*` control tools; use `dx_manifest`/`dx_compose`/`vision_lattice` for capability discovery, `doctor_*` for diagnosis, `wizard_*` for guided manufacture, `qol_*` for one-round-trip operation, and `telco_*` for transport-neutral handoff envelopes; `cmca_allocate` runs",
        1,
    )
    main.write_text(text)

merged = ROOT / "crates/ferroplan-mcp/tests/merged_server.rs"
text = merged.read_text()
if "dx_manifest" not in text:
    text = text.replace("31-tool", "42-tool")
    text = text.replace("31-resource", "42-resource")
    text = text.replace("ALL_31_TOOLS", "ALL_42_TOOLS")
    text = text.replace("all_31_tools", "all_42_tools")
    text = text.replace("31 tools", "42 tools")
    text = text.replace("31 resources", "42 resources")
    text = text.replace("exactly 31", "exactly 42")
    text = text.replace("all 31", "all 42")
    text = text.replace("expected 31", "expected 42")
    text = text.replace("        31,", "        42,")
    text = text.replace(
        "    // canonical evidence admission\n",
        "    // Vision 2030 operator experience\n"
        "    \"dx_manifest\",\n"
        "    \"dx_compose\",\n"
        "    \"doctor_scan\",\n"
        "    \"doctor_explain\",\n"
        "    \"wizard_bootstrap\",\n"
        "    \"wizard_recipe\",\n"
        "    \"qol_snapshot\",\n"
        "    \"qol_batch\",\n"
        "    \"telco_envelope\",\n"
        "    \"telco_verify\",\n"
        "    \"vision_lattice\",\n"
        "    // canonical evidence admission\n",
        1,
    )
    merged.write_text(text)

protocol = ROOT / "crates/ferroplan-mcp/tests/protocol.rs"
text = protocol.read_text()
if '"dx_manifest"' not in text:
    text = text.replace(
        '        "decompose",\n',
        '        "decompose",\n'
        '        "doctor_explain",\n'
        '        "doctor_scan",\n'
        '        "dx_compose",\n'
        '        "dx_manifest",\n',
        1,
    )
    text = text.replace(
        '        "parse",\n',
        '        "parse",\n'
        '        "qol_batch",\n'
        '        "qol_snapshot",\n',
        1,
    )
    text = text.replace(
        '        "solve",\n',
        '        "solve",\n'
        '        "telco_envelope",\n'
        '        "telco_verify",\n'
        '        "vision_lattice",\n'
        '        "wizard_bootstrap",\n'
        '        "wizard_recipe",\n',
        1,
    )
    protocol.write_text(text)

print("EXPERIENCE_PLANE_PROJECTED")
