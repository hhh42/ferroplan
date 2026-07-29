---
name: manufacture
description: Implement one admitted Ferroplan plan step as a reversible source change through the isolated source manufacturer. Use only when the effective phase permits manufacturing.
context: fork
agent: chatman-ecosystem:source-manufacturer
effort: max
---

Manufacture `$ARGUMENTS`.

Require:

- effective `epistemic=admitted`;
- zero pending observation events;
- a verified allocation receipt;
- a current candidate plan and exact selected step;
- `actuation=manufacturing`;
- canonical ownership for every affected projection.

Before editing, read `profiles/artifact-ownership.json`. Change canonical ontology, profile, template, or generator sources before their projected artifacts. Prefer ggen when the admitted pack exists. Refuse hand-coded generated output and unknown ownership.

Implement only the selected plan step or tightly coupled reversible batch in the agent's isolated worktree. Preserve deterministic ordering, canonical serialization, typed refusals, and source boundaries. Do not push, merge, publish, or claim validation.

Return:

- worktree identity;
- canonical owners changed;
- projected artifacts regenerated;
- exact changed paths;
- commands attempted;
- outputs and failures;
- remaining plan suffix;
- validation obligations.

Hooks will record the resulting mutation frontier. The effective phase will collapse until that frontier is admitted; this is expected.
