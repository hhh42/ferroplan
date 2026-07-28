---
name: audit
description: Replay Chatman admission envelopes, predecessor continuity, hook frontier, claim ceilings, and phase invariants. Use before stopping, declaring standing, or requesting publication.
context: fork
agent: chatman-ecosystem:receipt-auditor
effort: high
---

Audit `$ARGUMENTS`.

- Read the hook ledger, phase vector, allocation envelope, plan envelope, validator result, and predecessor chain.
- Recompute every canonical digest with `verify_receipt` and `canonical_digest`.
- Check event counts, chain continuity, authority claim ceilings, and phase invariants.
- Return the maximum lawful Gall standing and every missing obligation.

Do not edit, plan, allocate, or publish.
