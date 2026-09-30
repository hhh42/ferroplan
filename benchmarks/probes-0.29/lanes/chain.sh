#!/bin/sh
# Phases B and C of lanes29 behind phase A (see run.sh): wait for the running
# crucible to exit, then launch the next phase. Each phase suspends itself
# while the operator is active (game detection / foreign load), so this is
# safe to leave chained through a working day; phase D (Lane W's gain side)
# is deliberately NOT chained -- it wants a quiet night.
set -u
LOG=/Users/harold/ferroplan-0.29/benchmarks/probes-0.29/lanes/chain.log
RUN=/Users/harold/ferroplan-0.29/benchmarks/probes-0.29/lanes/run.sh
wait_idle() { while pgrep -f "[c]rucible.*sweep" > /dev/null 2>&1; do sleep 30; done; }
echo "chain: waiting for phase A  $(date '+%H:%M:%S')" >> "$LOG"
wait_idle
echo "chain: phase B (loss side, --prior solved)  $(date '+%H:%M:%S')" >> "$LOG"
"$RUN" --prior solved --board ipc5-constraints --board ipc2018-sat --board ipc2023-sat --board ipc2023-numeric --board ipc2026-numeric
echo "chain: phase B exit $?  $(date '+%H:%M:%S')" >> "$LOG"
sleep 20; wait_idle
echo "chain: phase C (numeric gain side + constraints owed)  $(date '+%H:%M:%S')" >> "$LOG"
"$RUN" --board ipc2023-numeric --board ipc2026-numeric --board ipc5-constraints
echo "chain: phase C exit $?  $(date '+%H:%M:%S')" >> "$LOG"
echo "chain: done  $(date '+%H:%M:%S')" >> "$LOG"
