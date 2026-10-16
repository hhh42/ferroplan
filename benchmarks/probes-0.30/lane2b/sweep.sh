#!/bin/sh
# Lane 2b's board read: the two numeric boards in full on the 0.30 engine --
# the charge's +27 on 2023 numeric must stay, and the five cells must come
# back. The crucible, the main checkout as --repo.
echo $$ > sweep.pid
C=/Users/harold/ferroplan-0.30/crucible/target/release/crucible
$C --repo /Users/harold/ferroplan sweep --set cut29 --headless --max-passes 4 \
   --engine /Users/harold/ferroplan-0.30/target/release/ff --name lane2b-numeric \
   --board ipc2023-numeric --board ipc2026-numeric --board ipc5-metric-time "$@"
