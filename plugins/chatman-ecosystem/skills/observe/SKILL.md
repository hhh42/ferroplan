---
name: observe
description: Convert the current repository and lifecycle frontier into a bounded RDF-shaped observation with explicit uncertainty and exactly eight CMCA work surfaces. Use after mutation, failure, resume, configuration change, worktree event, or external drift.
context: fork
agent: chatman-ecosystem:rdf-observer
effort: high
---

Observe `$ARGUMENTS` without editing.

1. Read the effective and canonical phase separately:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/effective-phase.py" \
     --project "$CLAUDE_PROJECT_DIR"
   ```
2. Read the exact pending hook ledger and bounded lifecycle candidates.
3. Inspect repository state, changed paths, manifests, dependencies, canonical owners, generated projections, commands, failures, and available evidence.
4. Project PROV-O, OCEL, QUDT, SHACL, ODRL, DCAT, and DCTERMS-shaped claims using stable identifiers.
5. Produce exactly eight CMCA candidates in the canonical ten-factor order.
6. When observing a recursive subtree, bind its parent allocation receipt and identify the consequence expected to return upward.
7. Separate observed facts, projection laws, uncertainty, refusals, unsupported rails, and unavailable evidence.
8. Return machine-usable data plus the maximum lawful epistemic standing.

A hook event is an observation candidate, not admitted truth. Do not call CMCA, plan, edit, validate, or authorize actuation.
