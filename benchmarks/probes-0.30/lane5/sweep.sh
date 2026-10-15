#!/bin/sh
# Lane 5 subset sweep (docs/roadmap-0.30.md, Lane 5 "Measured"): the memory
# cells through the crucible, the 0.30 engine named by --engine, the main
# checkout as --repo (promoted raws, corpus and VAL live only there).
echo $$ > "$(dirname "$0")/sweep.pid"
C=/Users/harold/ferroplan-0.30/crucible/target/release/crucible
FF=/Users/harold/ferroplan-0.30/target/release/ff
$C --repo /Users/harold/ferroplan sweep --set cut29 --headless --max-passes 4 \
   --engine $FF --name lane5-rungs \
   --board ipc2014-sat --board ipc2018-sat --board ipc2023-sat \
   --only 'city-car|child-snack|maintenance|caldera|organic-synthesis|snake|folding' \
   "$@"
