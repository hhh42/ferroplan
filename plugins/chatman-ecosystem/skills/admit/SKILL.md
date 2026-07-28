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
5. Bind the plan receipt to the hook frontier with `loop.py admit --receipt <receipt> --envelope <path-to-envelope.json> --session <session>`.
6. Advance only the phase dimensions supported by the envelope using `phase.py transition --receipt <receipt> --envelope <path-to-envelope.json> ...`.

Both `loop.py admit` and `phase.py transition` now require `--envelope`
alongside `--receipt`, and each calls `verify_receipt` on that envelope
before accepting it — so refusal on a bad digest, predecessor, or standing
is partially enforced by the script itself now, not solely a human/agent
judgment call. Refuse admission when any digest, predecessor, validator
standing, event count, or phase invariant fails, and treat a script-level
`verify_receipt` failure as an authoritative refusal signal in addition to
manual checks. Admission is data transformation, not publication.
