#!/bin/sh
# Lane M, confirmed THROUGH the crucible: every cell the lanes candidate
# (c0893fb8ccc7) banked as `mem-cap` on the six IPC-5 boards -- 54 of them,
# memcap.rows -- measured again on the engine that carries the memory wall.
# Same set, same manifest cap (mem_gb = 6, exported to the cell as
# FF_MEM_BUDGET_GB), same referee. A new engine hash, so nothing is read back.
set -u
C=/Users/harold/ferroplan-0.28/crucible/target/release/crucible
E=/Users/harold/ferroplan-0.28/target/release/ff
R=/Users/harold/ferroplan
L=/Users/harold/ferroplan-0.28/benchmarks/probes-0.28/lanes-crucible
stamp() { echo "== $1  $(date '+%F %T')  load:$(uptime | sed 's/.*load averages*://')" >> "$L/run.log"; }
"$E" --version > "$L/engine-mem.txt"; git -C /Users/harold/ferroplan-0.28 rev-parse HEAD >> "$L/engine-mem.txt"
stamp "4 mem-cap cells, Lane M engine"
$C --repo $R sweep --set cut27 --headless --max-passes 4 --engine $E --rows "$L/memcap.rows" --name lanes-0.28-mem > "$L/4-memcap-lane-m.log" 2>&1
stamp "finished 4"
