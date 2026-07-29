---
name: configure
description: Design and validate Claude Code component combinations by federating current loader validation, generated-artifact ownership, bounded config-LSP analysis, and schema-epoch law. Use for plugin, marketplace, settings, MCP, hooks, agents, skills, monitors, userConfig, channels, or dependencies.
context: fork
agent: chatman-ecosystem:config-law-architect
effort: high
---

Design or validate `$ARGUMENTS` without editing.

1. Read:
   - `profiles/claude-projection.json`;
   - `profiles/config-schema-epoch.json`;
   - `profiles/artifact-ownership.json`;
   - `ontology/chatman-shapes.ttl`.
2. Run the source-level projection validator:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/validate-claude-projection.py" \
     --plugin-root "$CLAUDE_PLUGIN_ROOT"
   ```
3. Run current loader validation when Claude Code is available:
   ```sh
   claude plugin validate "$CLAUDE_PLUGIN_ROOT" --strict
   ```
4. Invoke `claude-code-config-lsp` only as a standalone or explicit bounded validator over modeled configuration surfaces. The main plugin must not claim broad repository extensions.
5. Classify every finding as loader error, modeled-conformance error, known epoch delta, unknown delta, ownership drift, documentation drift, or unavailable executor.
6. Compute legal component combinations and the law rejecting each illegal combination.
7. Confirm every agent tool grant matches `ontology/authority-graph.ttl` and only `source-manufacturer` can edit.
8. Confirm monitors are skill-triggered and note that project-scope plugins do not load them.
9. Return the minimum canonical owner change, all dependent projections, exact evidence, and whether `conformance=conformant` may lawfully be admitted.

The current loader governs loadability. The LSP governs only its modeled epoch. Ownership law governs generated drift. None of these independently proves semantic correctness or runtime consequence.
