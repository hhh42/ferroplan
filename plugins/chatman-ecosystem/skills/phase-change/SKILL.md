---
name: phase-change
description: Inspect and advance the Chatman combinatorial phase vector using exact MCP receipts. Use when repository work changes epistemic, allocation, planning, actuation, drift, or configuration standing.
effort: high
---

Operate the phase engine for `$ARGUMENTS`.

1. Read `profiles/phase-space.json` and run:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/phase.py" status --project "$CLAUDE_PROJECT_DIR"
   ```
2. Read the pending observation ledger:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/loop.py" pending --project "$CLAUDE_PROJECT_DIR"
   ```
3. Invoke only the agents and skills active in the current phase projection.
4. Obtain authoritative MCP evidence for every requested advancement.
5. Audit the target vector against every invariant.
6. Advance dimensions only with a 64-hex admission receipt:
   ```sh
   python3 "$CLAUDE_PLUGIN_ROOT/scripts/phase.py" transition \
     --project "$CLAUDE_PROJECT_DIR" \
     --set <dimension>=<state> \
     --receipt <receipt> \
     --reason <reason>
   ```
7. Never use phase state as execution proof. The phase runtime is a projection over authoritative receipts.

A repository mutation automatically collapses the vector to observed, unallocated, unplanned, sealed, drifted, and conformance-unknown. Re-establish only the dimensions supported by new evidence.
