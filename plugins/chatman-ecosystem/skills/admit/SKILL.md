---
name: admit
description: Bind allocation, plan, validator, observation, and predecessor commitments into canonical BLAKE3 envelopes and advance only lawful phase dimensions. Use after evidence exists and before receipt closure.
effort: high
---

Admit `$ARGUMENTS`.

1. Read the exact pending observation frontier.
2. Verify the allocation envelope.
3. Call `bind_plan_receipt` with:
   - exact `session_think` result;
   - verified allocation receipt;
   - exact observation frontier;
   - independent validator result containing `valid: true`;
   - predecessor receipt when present.
4. Call `verify_receipt` on the returned envelope.
5. Bind the plan receipt to the hook frontier with `loop.py admit`.
6. Advance only the phase dimensions supported by the envelope using `phase.py transition`.

Refuse admission when any digest, predecessor, validator standing, event count, or phase invariant fails. Admission is data transformation, not publication.
