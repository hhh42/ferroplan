# Ideas adopted from PR #2 (draft, not merged)

PR #2, `agent/v26.7.29-claude-projection`, is a full, self-consistent rewrite
of `plugins/chatman-ecosystem/` (marketplace/plugin manifests, all 8 agents,
all 13 skills, hooks, ontology, profiles, generated-guard, README) pinned to
a single coordinated release, `26.7.29`. It is a draft and was not merged.

This note records what was pulled out of that PR directly into `main`
without merging the PR itself or overwriting anything the PR touches —
per an explicit "merge the ideas, do not overwrite" request.

## Adopted (added as new, standalone files)

These six files have no dependency on the rest of PR #2's rewrite, make no
claims about this repo's current state (no version pins, no "this is
version 26.7.29" assertions), and were verified to parse/compile cleanly
and (for `effective-phase.py`) to run correctly against this project's live
ledger before being added:

- `plugins/chatman-ecosystem/ontology/authority-graph.ttl` — a timeless RDF
  vocabulary for agent tool grants/denials, claim ceilings, and spawn
  authority. Not yet referenced by anything; available for a future SHACL
  pass over the existing agents.
- `plugins/chatman-ecosystem/profiles/actuation-intent.schema.json` — a JSON
  Schema for structured actuation intents / derived execution grants. Not
  yet wired to any hook.
- `plugins/chatman-ecosystem/scripts/effective-phase.py` — projects a
  `canonical_vector` (the receipted phase state) separately from an
  `effective_vector` (canonical + pending-observation frontier), rather than
  conflating them the way `phase.py status` currently does. Genuinely
  useful on its own; run `python3 plugins/chatman-ecosystem/scripts/effective-phase.py
  --project <path>` to try it.
- `plugins/chatman-ecosystem/scripts/actuation-intent.py`,
  `grant-actuation.py` — a two-step "manufacture an intent, then derive a
  bounded grant from a verified receipt" pattern for protected Bash
  actuation. Standalone; not called by `hooks/hooks.json`.
- `plugins/chatman-ecosystem/scripts/event-summary.py` — records bounded
  lifecycle candidates and summarizes parallel tool batches. Standalone;
  not called by `hooks/hooks.json`.

None of these are invoked by any existing hook, skill, or script yet — they
are available capabilities, not activated ones. Wiring them in is a
separate, deliberate decision (see below).

## Deliberately not adopted, and why

Everything else in PR #2 is tightly coupled to the full `26.7.29` rewrite
and could not be pulled in piecemeal without either breaking CI/tests or
making this repo assert things about itself that aren't true:

- `plugins/chatman-ecosystem/scripts/validate-claude-projection.py` and
  `.github/workflows/chatman-ecosystem.yml` — the validator hard-requires
  `plugin.json` version `26.7.29`, `defaultEnabled: false`, no `lspServers`,
  no `.lsp.json`, every agent declaring `effort`/`maxTurns` and denying
  `Write`/`Edit`/`NotebookEdit` (except `source-manufacturer`, isolated in a
  worktree), specific new hook events, and `actuation-intent.py`/
  `event-summary.py` wired into `hooks.json`. Adding the workflow alone
  would ship a CI job that fails on every push.
- `plugins/chatman-ecosystem/tests/test_claude_projection.py` — asserts the
  above validator passes; same problem.
- `plugins/chatman-ecosystem/profiles/claude-projection.json` and
  `profiles/artifact-ownership.json` — both self-declare
  `"release": "26.7.29"` and describe file ownership for the rewritten
  layout. Adding them as-is would have this repo claim a release state that
  isn't real.
- `docs/architecture/claude-projection.md`, `docs/migration/v26.7.29.md`,
  `docs/releases/v26.7.29.md`, `docs/verification/v26.7.29-claude-projection.md`
  — same reason; they describe `26.7.29` as shipped/verified.
- All modifications to existing tracked files — `.claude-plugin/marketplace.json`,
  `plugins/chatman-ecosystem/.claude-plugin/plugin.json`, `README.md`, all 8
  `agents/*.md`, all 13 `skills/*/SKILL.md`, `hooks/hooks.json`,
  `monitors/monitors.json`, the existing `ontology/*.ttl` files,
  `profiles/{config-schema-epoch,phase-space,self-hosting,work-surfaces}.json`,
  and `scripts/generated-guard.py` — left untouched per "do not overwrite."
  The PR's ideas there (source-manufacturer as the sole editor, worktree
  isolation, treating hook events as intent candidates rather than admitted
  truth, recursive CMCA) are real and worth evaluating, but adopting them
  means rewriting files this session's live receipt chain and phase-vector
  work depend on — a deliberate decision for later, not something to fold
  in silently.
- `plugins/chatman-ecosystem/.lsp.json` removal — a deletion, not an
  addition; left in place.

## If the full rewrite is wanted later

PR #2 itself is still open (draft) at
`https://github.com/seanchatmangpt/ferroplan/pull/2` and can be reviewed and
merged as a coordinated whole, which is the only way its coupled pieces
(validator, CI, tests, hooks, agent restrictions) are internally consistent.
