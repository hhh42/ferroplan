---
name: validate
description: Independently validate the exact manufactured source, Claude projection, agent authority, build, plan, actuation, and receipt surfaces. Use after manufacturing and before receipted or publishable standing.
context: fork
agent: chatman-ecosystem:independent-validator
effort: max
---

Validate `$ARGUMENTS` without editing or repairing.

Exercise the exact claimed surface using distinct authorities where available:

1. Source-level projection law:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/validate-claude-projection.py" \
     --plugin-root "$CLAUDE_PLUGIN_ROOT"
   ```
2. Python syntax and tests for all plugin scripts.
3. Current Claude loader validation for exact plugin loadability.
4. Explicit bounded config-LSP conformance for modeled surfaces.
5. Agent authority checks proving only the manufacturer can edit and it uses worktree isolation.
6. Cargo format, check, Clippy, tests, feature matrices, and benchmarks required by the changed Rust surface.
7. Ferroplan same-engine plan replay.
8. External VAL or another distinct semantic implementation when independence is required.
9. Canonical digest and receipt verification.
10. Negative fixtures for pending state, tampering, stale grants, ownership drift, and forbidden tools.

Return structured evidence containing `valid`, exact commands or tools, inputs, outputs, executable identities, failures, independence class, limitations, and maximum lawful standing.

A differently named process is not independent. A successful build is not publication. A valid grant is not execution evidence. Do not repair failures inside this skill.
