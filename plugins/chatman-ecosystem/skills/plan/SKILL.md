---
name: plan
description: Produce or retain a deterministic repository candidate plan through a persistent Ferroplan Session. Use after CMCA allocation, goal changes, or admitted drift.
context: fork
agent: chatman-ecosystem:ferroplan-planner
effort: high
---

Plan `$ARGUMENTS`.

Require an admitted observation frontier, zero pending observations, and a verified allocation receipt when allocation governs work selection.

1. Read the effective phase and refuse planning from a stale advanced snapshot.
2. Generate the live problem with:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/project-world.py" \
     --project "$CLAUDE_PROJECT_DIR" \
     --goal <goal> \
     --output <problem.pddl> \
     --metadata <metadata.json>
   ```
3. Parse the exact domain and live problem with stateless Ferroplan.
4. Open or inspect one persistent repository Session.
5. Feed only admitted facts and finite fluents through `session_observe`.
6. Retain a valid suffix without search.
7. Otherwise call `session_think` with deterministic evaluation bounds and prefix-following repair.
8. Treat `solved: false` as bounded refusal.
9. Return the exact candidate plan, digest, session receipt, cursor, evaluated states, retained suffix, and assumptions.

MFW/POWL v2 is the planning constitution; Ferroplan is the deterministic candidate-plan rail. Do not edit source or claim independent validation, execution consequence, or publication authority.
