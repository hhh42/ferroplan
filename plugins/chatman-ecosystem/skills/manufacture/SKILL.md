---
name: manufacture
description: Implement one admitted Ferroplan plan step as a reversible source change in an isolated worktree. Use only when the phase vector permits manufacturing.
context: fork
agent: chatman-ecosystem:source-manufacturer
effort: max
---

Manufacture `$ARGUMENTS`.

Require:

- epistemic admission;
- an allocation receipt;
- a current candidate plan;
- `actuation=manufacturing` in the phase vector.

Implement only the selected plan step or tightly coupled reversible batch. Preserve ontology/template ownership, deterministic projection, typed refusals, and source boundaries. Do not publish.

Return exact changed paths, commands attempted, outputs, failures, and remaining obligations. Hooks will mark the resulting repository state as drifted and unadmitted; that is expected.
