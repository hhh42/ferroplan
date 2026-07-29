---
name: publish
description: Publish a validated, receipted Ferroplan change through an exact structured actuation intent and derived execution grant. Use only when the user explicitly requests commit, push, or draft pull-request publication.
disable-model-invocation: true
effort: max
---

Publish `$ARGUMENTS` only after explicit user instruction.

Require:

- zero pending observation events;
- effective phase `epistemic=admitted`, `allocation=allocated`, `planning=validated`, `actuation=publishable`, `drift=stable`, `conformance=conformant`;
- verified allocation and plan envelopes;
- exact configuration and ownership validation;
- independent validator result with `valid: true`;
- receipt audit establishing grant eligibility;
- exact admitted publication scope.

## Intent and grant sequence

1. Construct the exact protected command that would publish only the admitted scope. Prefer a draft pull request.
2. Attempt that exact command once. The PreToolUse hook must record its `ActuationIntent` and deny it when no grant exists.
3. Read the refusal reason and exact intent path. Do not alter the protected command after the intent is created.
4. Derive the grant:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/grant-actuation.py" \
     --project "$CLAUDE_PROJECT_DIR" \
     --intent <intent.json> \
     --receipt <active-receipt> \
     --envelope <verified-envelope.json> \
     --scope <exact-publication-scope>
   ```
5. Audit the grant digest, intent digest, active receipt, expiration, and scope.
6. Retry the exact same protected command. Any command change requires a new intent and grant.
7. Record the resulting tool event as execution evidence. Do not treat the grant itself as proof that publication occurred.
8. Observe and bind the resulting GitHub object, commit, push, or pull request as an `ExecutionAttestation` when that rail is available.

The compatibility receipt fence remains active in parallel. If either hook refuses, stop and report the missing frontier. Never bypass, weaken, disable, or race the hooks.
