#!/bin/sh
# Lane 6's board read (docs/roadmap-0.30.md): the partition rung measured on
# the whole IPC-5 propositional board, then on the SOLVED side of the other
# classical satisficing boards (the rung spends half the wall before the
# ladder: a cell the ladder solved late is the risk). Two invocations, in
# sequence, the crucible from the 0.30 worktree, the main checkout as --repo.
echo $$ > "$(dirname "$0")/sweep.pid"
C=/Users/harold/ferroplan-0.30/crucible/target/release/crucible
FF=/Users/harold/ferroplan-0.30/target/release/ff
$C --repo /Users/harold/ferroplan sweep --set cut29 --headless --max-passes 4 \
   --engine $FF --name lane6-rung --board ipc5-prop "$@"
$C --repo /Users/harold/ferroplan sweep --set cut29 --headless --max-passes 4 \
   --engine $FF --name lane6-rung-solved --prior solved \
   --board ipc2014-sat --board ipc2018-sat --board ipc2023-sat \
   --board ipc2014-agile --board ipc2023-agile --board ipc67-results "$@"
echo "LANE6 SWEEP DONE" >> "$(dirname "$0")/sweep.done"
