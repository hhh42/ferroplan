#!/bin/sh
# The night's work, in order, one crucible at a time (one crucible per database):
#   1. wait for the v0.27.1 differential over lost.rows to finish;
#   2. re-open the rows the box decided (recheck28.py --reopen);
#   3. sweep --set cut28 again: only the re-opened rows are owed, on the same engine;
#   4. re-promote (idempotent) and re-snapshot 0.28.0.
# Then the release text is written against what the boards say.
set -u
R=/Users/harold/ferroplan
C=$R/crucible/target/release/crucible
L=/Users/harold/ferroplan-0.28/benchmarks/probes-0.28/cut28
stamp() { echo "== $1  $(date '+%F %T')  load:$(uptime | sed 's/.*load averages*://')" >> "$L/run.log"; }
while pgrep -f 'crucible.*backfill.*cut28-regress' > /dev/null; do sleep 60; done
stamp "differential finished"
cd $R
python3 /Users/harold/ferroplan-0.28/benchmarks/recheck28.py --repo $R --list > "$L/recheck28.txt" 2>&1
python3 /Users/harold/ferroplan-0.28/benchmarks/recheck28.py --repo $R --reopen >> "$L/recheck28.txt" 2>&1
stamp "re-opened; recheck sweep"
echo "==== recheck $(date '+%F %T') -- rows re-opened by recheck28.py; same engine" >> benchmarks/cut28-sweep.log
$C --repo $R sweep --set cut28 --headless >> benchmarks/cut28-sweep.log 2>&1
stamp "recheck sweep done"
bash benchmarks/promote-air28.sh > "$L/promote28-recheck.log" 2>&1
python3 scripts/standings-snapshot.py --version 0.28.0 --measured-at 2026-09-26 --released 2026-09-28 --note "cut28 swept by the crucible: 8,444 of 8,444 banked in 18 passes; rows the box decided re-measured (recheck28.py)" >> "$L/promote28-recheck.log" 2>&1
stamp "NIGHT DONE"
