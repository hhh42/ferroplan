---
name: plan
description: Manufacture or retain a deterministic repository plan through a persistent Ferroplan Session. Use after CMCA allocation, goal changes, or admitted drift.
context: fork
agent: chatman-ecosystem:ferroplan-planner
effort: high
---

Plan `$ARGUMENTS`.

- Require an allocation receipt and admitted observation frontier.
- Parse the self-hosting domain and the *live* problem with stateless
  Ferroplan. The live problem file comes from
  `scripts/project-world.py --project "$CLAUDE_PROJECT_DIR" --goal <goal> --output <problem.pddl> --metadata <metadata.json>`,
  not the static example problem. By default `project-world.py` runs real
  `git status`, `cargo check --workspace`, and `cargo test --workspace`
  checks to derive dirty/build-green/validator-green facts, which can take
  a few minutes; pass `--skip-live-checks` for a fast cached-phase-vector-only
  pass instead.
- Open or inspect one persistent repository session.
- Feed admitted facts and fluents through `session_observe`.
- Retain a valid suffix without search.
- Otherwise call `session_think` with bounded evaluations and prefix-following enabled.
- Return the exact candidate plan, digest, session receipt, cursor, evaluated states, and assumptions.

Do not edit source or claim independent validation.
