---
name: configure
description: Design and validate Claude Code component combinations by federating current loader validation with claude-code-config-lsp and Declare conformance. Use for plugin, marketplace, settings, MCP, LSP, hooks, agents, skills, monitors, user configuration, channels, or dependencies.
context: fork
agent: chatman-ecosystem:config-law-architect
effort: high
paths:
  - "**/.claude/**"
  - "**/.claude-plugin/**"
  - "**/plugin.json"
  - "**/marketplace.json"
  - "**/.mcp.json"
  - "**/.lsp.json"
  - "**/hooks/**"
  - "**/agents/**"
  - "**/skills/**"
  - "**/monitors/**"
---

Design or validate `$ARGUMENTS`.

1. Read `profiles/config-schema-epoch.json`.
2. Run `claude plugin validate` against the plugin root when Claude Code is available.
3. Collect `claude-code-config-lsp` diagnostics, completion/hover facts, virtual health, and Declare traces for modeled surfaces.
4. Classify every disagreement as known epoch delta, unknown delta, loader error, or LSP-modeled error.
5. Compute legal component combinations and the law rejecting each illegal combination.
6. Return the minimum ontology/profile/config change and whether `conformance=conformant` may lawfully be admitted.

The current loader governs loadability. The LSP governs only its modeled epoch. Neither proves semantic correctness. Do not edit in this skill.
