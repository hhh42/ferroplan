#!/bin/sh
# Lane 5's read (docs/roadmap-0.30.md): the subset on the 0.30 engine against
# the 0.29.0 board engine, first-attempt estimator, lost cells listed.
C=/Users/harold/ferroplan-0.30/crucible/target/release/crucible
$C --repo /Users/harold/ferroplan compare --set cut29 --a 7fdf185b91e6 --b 9df92e323f0a \
   --board ipc2014-sat --board ipc2018-sat --board ipc2023-sat \
   --only 'city-car|child-snack|maintenance|caldera|organic-synthesis|snake|folding' \
   --lost "$(dirname "$0")/lost.rows" "$@"
