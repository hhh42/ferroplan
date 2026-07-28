---
name: validate
description: Independently validate the exact manufactured source, Claude configuration, build, plan, and receipt surfaces. Use after manufacturing and before receipted or publishable standing.
context: fork
agent: chatman-ecosystem:independent-validator
effort: max
---

Validate `$ARGUMENTS` without editing.

Exercise the exact claimed surfaces using distinct authorities where available:

- claude-code-config-lsp conformance;
- Cargo format/check/Clippy/tests;
- Ferroplan plan replay;
- external VAL or another independent semantic implementation when required;
- canonical digest and receipt verification.

Return structured JSON containing `valid`, exact commands/tools, inputs, outputs, executable identity when available, failures, independence class, and maximum lawful standing.

Do not repair failures inside this skill.
