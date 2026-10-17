#!/bin/sh
# Lane 1b's points read (docs/roadmap-0.30.md): the three IPC-5 preference
# boards in full on the engine with the anytime polish. BUILD THE CLI FIRST
# (`cargo build --release -p ferroplan-cli` in the 0.30 worktree) and read
# `crucible engine --repo /Users/harold/ferroplan-0.30` -- cargo test never
# builds target/release/ff. Points are read from ipc-standings.md's quality
# column on this ONE engine against SGPlan5; rows by compare.
echo $$ > "$(dirname "$0")/sweep.pid"
C=/Users/harold/ferroplan-0.30/crucible/target/release/crucible
FF=/Users/harold/ferroplan-0.30/target/release/ff
$C --repo /Users/harold/ferroplan sweep --set cut29 --headless --max-passes 4 \
   --engine $FF --name lane1b-rounds \
   --board ipc5-simple-pref --board ipc5-qual-pref --board ipc5-complex-pref "$@"
echo "LANE1B SWEEP DONE" >> "$(dirname "$0")/sweep.done"
