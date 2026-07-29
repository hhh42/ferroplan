---
name: doctor
description: Diagnose the complete Chatman Phase Engine projection: loader, ownership, agents, hooks, monitors, MCP resolution, scripts, Rust binaries, PDDL world, phase state, actuation objects, and receipts. Use before claiming operational standing.
effort: high
---

Diagnose `$ARGUMENTS` without repairing automatically.

1. Inspect plugin inventory and current loader validation:
   ```sh
   claude plugin details chatman-ecosystem@chatman-ecosystem
   claude plugin validate "$CLAUDE_PLUGIN_ROOT" --strict
   ```
2. Run source-level projection validation:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/validate-claude-projection.py" \
     --plugin-root "$CLAUDE_PLUGIN_ROOT"
   ```
3. Read `profiles/config-schema-epoch.json`, `profiles/claude-projection.json`, and `profiles/artifact-ownership.json`.
4. Confirm the main plugin has no global LSP registration and the standalone validator remains explicit.
5. Check every Python script:
   ```sh
   python3 -m compileall -q "$CLAUDE_PLUGIN_ROOT/scripts"
   ```
6. Check shell resolver syntax with `sh -n`.
7. Check Rust binaries:
   ```sh
   cargo check -p ferroplan-mcp --bin ferroplan-mcp
   ```
8. Initialize the single `ferroplan` MCP process; enumerate resources and all sixteen tools; exercise relevant positive and negative fixtures.
9. Generate the live PDDL problem and parse the exact domain/problem.
10. Read canonical and effective phase, hook status, lifecycle candidates, intents, and grants.
11. Confirm only `source-manufacturer` can edit and it declares worktree isolation.
12. Verify the latest admission envelope and predecessor chain when present.
13. Check monitor activation predicates and report project-scope monitor suppression as an expected loader boundary.

Report each surface as `ALIVE`, `PARTIAL_ALIVE`, `BLOCKED`, `BUILD_BROKEN`, `UNKNOWN`, or `UNSUPPORTED` with exact evidence and limitations.

Do not repair, publish, or upgrade standing from inspection alone.
