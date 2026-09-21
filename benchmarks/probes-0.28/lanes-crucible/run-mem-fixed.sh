#!/bin/sh
# Lane M AFTER its regression read: the interning checkpoint, the scope latch
# and the armed scorer grounding. Both subsets again, on the fixed engine --
# the 54 cells the lanes candidate banked `mem-cap` (memcap.rows), then the 58
# it solved at >= 3.5 GB peak (memhigh.rows). Same set, cap and referee.
set -u
C=/Users/harold/ferroplan-0.28/crucible/target/release/crucible
E=/Users/harold/ferroplan-0.28/target/release/ff
R=/Users/harold/ferroplan
L=/Users/harold/ferroplan-0.28/benchmarks/probes-0.28/lanes-crucible
stamp() { echo "== $1  $(date '+%F %T')  load:$(uptime | sed 's/.*load averages*://')" >> "$L/run.log"; }
"$E" --version > "$L/engine-mem-fixed.txt"; git -C /Users/harold/ferroplan-0.28 rev-parse HEAD >> "$L/engine-mem-fixed.txt"; git -C /Users/harold/ferroplan-0.28 status --short -- crates >> "$L/engine-mem-fixed.txt"
stamp "6 mem-cap cells, fixed Lane M"
$C --repo $R sweep --set cut27 --headless --max-passes 4 --engine $E --rows "$L/memcap.rows"  --name lanes-0.28-mem     > "$L/6-memcap-fixed.log"  2>&1
stamp "7 solved high-memory cells, fixed Lane M"
$C --repo $R sweep --set cut27 --headless --max-passes 4 --engine $E --rows "$L/memhigh.rows" --name lanes-0.28-memhigh > "$L/7-memhigh-fixed.log" 2>&1
stamp "finished 7"
