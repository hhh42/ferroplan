#!/bin/sh
# The 0.28 lanes, measured THROUGH the crucible (spec R3): the same referee,
# owed-row cascade and game/foreign-load throttle a cut sweep gets, over the
# cells the question is about. Instrument + promoted raws from the published
# checkout; the engine from the 0.28 worktree's build.
#
#   1. regression read, candidate : the temporal cells the published boards SOLVED
#   2. regression read, baseline  : the same cells on v0.27.1 -- equal-N by construction
#   3. the six IPC-5 boards the lanes were built for, candidate
#
# One crucible per database, so they run in sequence. Nothing else needs to
# stay off the box: that is what the throttle is for.
set -u
C=/Users/harold/ferroplan-0.28/crucible/target/release/crucible
E=/Users/harold/ferroplan-0.28/target/release/ff
R=/Users/harold/ferroplan
L=/Users/harold/ferroplan-0.28/benchmarks/probes-0.28/lanes-crucible
REG="--board ipc67-temporal --board ipc2014-tempo --prior solved --name lanes-0.28-regress"
IPC5="--board ipc5-simple-pref --board ipc5-qual-pref --board ipc5-complex-pref --board ipc5-time --board ipc5-metric-time --board ipc5-constraints --name lanes-0.28"
stamp() { echo "== $1  $(date '+%F %T')  load:$(uptime | sed 's/.*load averages*://')" >> "$L/run.log"; }
"$E" --version > "$L/engine.txt"; git -C /Users/harold/ferroplan-0.28 rev-parse HEAD >> "$L/engine.txt"
stamp "1 regress candidate";  $C --repo $R sweep    --set cut27 --headless --max-passes 4 --engine $E $REG  > "$L/1-regress-candidate.log" 2>&1
stamp "2 regress baseline";   $C --repo $R backfill --set cut27 --tag v0.27.1 --max-passes 4 $REG           > "$L/2-regress-baseline.log"  2>&1
stamp "3 ipc5 candidate";     $C --repo $R sweep    --set cut27 --headless --max-passes 4 --engine $E $IPC5 > "$L/3-ipc5-candidate.log"   2>&1
stamp "finished"
