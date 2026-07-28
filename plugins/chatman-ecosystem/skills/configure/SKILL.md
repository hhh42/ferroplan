---
name: configure
description: Design and validate Claude Code plugin combinations through claude-code-config-lsp and Declare conformance. Use for plugin.json, marketplace.json, settings, MCP, LSP, hooks, agents, skills, monitors, or dependency topology.
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

Design or validate the configuration request `$ARGUMENTS`.

Use `claude-code-config-lsp` diagnostics, completion, hover, semantic tokens, virtual health documents, and Declare constraints. Treat the ontology as the configuration source of truth.

Return:

- existing surfaces and dependency graph;
- exact diagnostics and conformance score;
- legal component combinations;
- illegal combinations and the law rejecting each;
- minimal ontology/profile/config change;
- whether the conformance phase may advance to `conformant`.

Do not manufacture source changes in this skill.
