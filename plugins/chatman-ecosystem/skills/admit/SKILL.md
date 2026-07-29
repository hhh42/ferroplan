---
name: admit
description: Bind allocation, plan, validator, configuration, observation, ownership, actuation, and predecessor commitments into canonical BLAKE3 envelopes and advance only lawful phase dimensions. Use after exact evidence exists.
effort: high
---

Admit `$ARGUMENTS` as data transformation, never as publication.

1. Read the exact pending observation frontier and effective phase.
2. Verify the allocation envelope, including parent allocation receipt when recursive.
3. Require the exact configuration validation record and ownership standing.
4. Require the exact independent validator result containing `valid: true` for the claimed surface.
5. Call `bind_plan_receipt` with:
   - exact `session_think` result;
   - verified allocation receipt;
   - exact observation frontier;
   - exact validator result;
   - configuration and ownership commitments when claimed;
   - predecessor receipt when present.
6. Call `verify_receipt` on the returned envelope.
7. Bind the receipt to the hook frontier:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/loop.py" admit \
     --project "$CLAUDE_PROJECT_DIR" \
     --receipt <receipt> \
     --envelope <path-to-envelope.json> \
     --session <session>
   ```
8. Advance only supported dimensions with `phase.py transition` and the same verified envelope.
9. When protected actuation is requested, bind the exact `ActuationIntent` and later `DerivedExecutionGrant`; do not treat the grant as execution attestation.

Refuse admission when any digest, predecessor, validator standing, ownership relation, event count, authority ceiling, or phase invariant fails. A script-level `verify_receipt` failure is authoritative refusal evidence.
