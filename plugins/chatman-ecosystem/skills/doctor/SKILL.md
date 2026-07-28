---
name: doctor
description: Diagnose Chatman Phase Engine installation, configuration, MCP/LSP resolution, Python scripts, Rust binaries, PDDL world projection, phase invariants, and receipt frontier. Use when the plugin fails to load or before claiming operational standing.
effort: high
---

Diagnose `$ARGUMENTS` without repairing automatically.

1. Inspect plugin inventory and loader validation:
   ```sh
   claude plugin details chatman-ecosystem@chatman-ecosystem
   cd "$CLAUDE_PLUGIN_ROOT" && claude plugin validate .
   ```
2. Read `profiles/config-schema-epoch.json` and classify loader/LSP disagreements.
3. Check Python syntax:
   ```sh
   python3 -m py_compile \
     "$CLAUDE_PLUGIN_ROOT/scripts/loop.py" \
     "$CLAUDE_PLUGIN_ROOT/scripts/phase.py" \
     "$CLAUDE_PLUGIN_ROOT/scripts/project-world.py"
   ```
4. Check shell resolver syntax with `sh -n`.
5. Check Rust binaries in the Ferroplan checkout:
   ```sh
   cargo check -p ferroplan-mcp --bin ferroplan-mcp
   ```
6. Generate the live PDDL problem and parse both domain and problem with stateless Ferroplan.
7. Ping the `ferroplan` MCP server (all 16 tools: parse/solve/validate/decompose, persistent Session, Chatman admission).
8. Read phase and hook status; validate every phase invariant.
9. Verify the latest admission envelope when present.

Report each surface as `ALIVE`, `PARTIAL_ALIVE`, `BUILD_BROKEN`, or `UNKNOWN`. Do not repair, publish, or upgrade standing from inspection alone.
