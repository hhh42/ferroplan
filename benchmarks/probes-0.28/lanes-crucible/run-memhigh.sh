#!/bin/sh
# Lane M's REGRESSION read, through the crucible: the 58 cells the lanes
# candidate (c0893fb8ccc7) SOLVED while peaking at or above 3.5 GB -- the only
# cells where a wall that trips at 0.75 x 6 GB can change an outcome. Equal-N:
# one banked row per cell per engine. Read solved AND metric/makespan.
set -u
C=/Users/harold/ferroplan-0.28/crucible/target/release/crucible
E=/Users/harold/ferroplan-0.28/target/release/ff
R=/Users/harold/ferroplan
L=/Users/harold/ferroplan-0.28/benchmarks/probes-0.28/lanes-crucible
stamp() { echo "== $1  $(date '+%F %T')  load:$(uptime | sed 's/.*load averages*://')" >> "$L/run.log"; }
stamp "5 solved high-memory cells, Lane M engine"
$C --repo $R sweep --set cut27 --headless --max-passes 4 --engine $E --rows "$L/memhigh.rows" --name lanes-0.28-memhigh > "$L/5-memhigh-lane-m.log" 2>&1
stamp "finished 5"
