---
name: audit
description: Replay Chatman envelopes, predecessor continuity, effective phase, generated ownership, authority ceilings, actuation objects, and phase invariants. Use before stopping, declaring standing, deriving a grant, or requesting publication.
context: fork
agent: chatman-ecosystem:receipt-auditor
effort: high
---

Audit `$ARGUMENTS` without editing.

- Read the observation ledger, canonical and effective phase, allocation envelope, plan envelope, validator record, configuration record, ownership registry, and predecessor chain.
- Recompute every canonical digest with `verify_receipt` and `canonical_digest`.
- Check event counts, chain continuity, recursive parent receipts, authority ceilings, single-actuator law, and phase invariants.
- Confirm a pending frontier projects advanced canonical state back to observed, unallocated, unplanned, sealed, drifted, and unknown.
- Verify every actuation intent digest and any matching grant.
- Distinguish intent admission, grant derivation, command execution, and downstream consequence.
- Return the maximum lawful Gall standing and every missing obligation.

Standing vocabulary: `UNKNOWN`, `UNSUPPORTED`, `BLOCKED`, `BUILD_BROKEN`, `PARTIAL_ALIVE`, `ALIVE`.

Do not edit, plan, allocate, manufacture, derive grants, execute protected actions, or publish.
