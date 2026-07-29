---
name: phase-change
description: Inspect and advance the Chatman combinatorial phase vector using exact MCP receipts while separating canonical snapshots from pending effective state. Use when epistemic, allocation, planning, actuation, drift, or conformance standing changes.
effort: high
---

Operate the phase engine for `$ARGUMENTS`.

1. Read `profiles/phase-space.json`.
2. Project the effective state:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/effective-phase.py" \
     --project "$CLAUDE_PROJECT_DIR"
   ```
3. Inspect the canonical snapshot:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/phase.py" status \
     --project "$CLAUDE_PROJECT_DIR"
   ```
4. Read the pending observation ledger:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/loop.py" pending \
     --project "$CLAUDE_PROJECT_DIR"
   ```
5. Invoke only agents and skills active in the effective projection.
6. Obtain authoritative MCP evidence for every requested advancement.
7. Audit the target vector against every invariant and authority ceiling.
8. Advance dimensions only with a verified 64-hex receipt and exact envelope:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/phase.py" transition \
     --project "$CLAUDE_PROJECT_DIR" \
     --set <dimension>=<state> \
     --receipt <receipt> \
     --envelope <path-to-envelope.json> \
     --reason <reason>
   ```

`phase.py transition` verifies the envelope through the MCP `verify_receipt` tool. Refuse any undeclared transition, invalid invariant, stale frontier, bad predecessor, or unavailable evidence.

Never use phase state as execution proof. The canonical snapshot is a receipt-bound cache. Pending observation candidates project the effective vector to observed, unallocated, unplanned, sealed, drifted, and conformance-unknown until they are admitted.
