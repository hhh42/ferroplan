# Low-risk auto-admission daemon

`plugins/chatman-ecosystem/scripts/auto_admit.py` closes routine observation
frontiers without requiring an agent to manually replay the entire CMCA and
Ferroplan ceremony after every small edit.

The daemon is **not** an alternate receipt authority. For each eligible batch
it calls the real `ferroplan-mcp` tools: `cmca_allocate`,
`bind_allocation_receipt`, `session_open`, `session_observe`, `session_think`,
`validate`, `bind_plan_receipt`, and `verify_receipt`. It then pipes the real
plan envelope through the public `loop.py admit --envelope -` broker. The
broker verifies the receipt again and, under the ledger lock, checks both the
canonical project identity and the exact expected event counters before it
advances `admitted_event_count`.

## Admission boundary

The whole pending frontier must satisfy every condition:

- every event is a successful `Write`, `Edit`, or `NotebookEdit` observation;
- the event transport digest recomputes;
- the changed paths exactly match `git status`;
- every path matches `profiles/auto-admit.json`;
- no path hits the hard-coded control-plane exclusions;
- the configured event and byte limits are respected;
- `git diff --check` passes and no conflict markers are present.

A single Bash event, failed edit, protected path, extra dirty path, malformed
ledger record, or concurrent new event refuses the entire batch. There is no
partial frontier admission. Bash is intentionally unsupported because the
bounded ledger stores only its command digest; the daemon cannot prove from
that digest that the original command was non-protected.

The eight CMCA candidates come from the canonical `work-surfaces.json` profile.
The daemon adjusts their named factors with measured event counts, changed
files, line deltas, binary/executable flags, and verification cost. The complete
measurement table and resulting factor vectors are bound into the observation
frontier, so there are no placeholder factors.

## Commands

```bash
python3 plugins/chatman-ecosystem/scripts/auto_admit.py once \
  --project /path/to/ferroplan

python3 plugins/chatman-ecosystem/scripts/auto_admit.py ensure \
  --project /path/to/ferroplan

python3 plugins/chatman-ecosystem/scripts/auto_admit.py stop \
  --project /path/to/ferroplan
```

`ensure` starts one detached watcher per canonical project ledger. Its PID and
log are stored beside `state.json`. The committed SessionStart hook invokes
`ensure`; disabling `profiles/auto-admit.json` leaves manual admission as the
only path.
