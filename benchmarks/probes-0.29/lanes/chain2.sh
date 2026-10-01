#!/bin/sh
# Phase D (Lane W's gain side) behind phase C' on the fixed engine, for the
# quiet night: wait for the running crucible to exit, then launch it. Each
# phase suspends itself while the operator is active.
set -u
LOG=/Users/harold/ferroplan-0.29/benchmarks/probes-0.29/lanes/chain.log
RUN=/Users/harold/ferroplan-0.29/benchmarks/probes-0.29/lanes/run.sh
wait_idle() { while pgrep -f "[c]rucible.*sweep" > /dev/null 2>&1; do sleep 30; done; }
echo "chain2: waiting for phase C'  $(date '+%H:%M:%S')" >> "$LOG"
sleep 60; wait_idle
echo "chain2: phase D (sat gain side, --prior unsolved)  $(date '+%H:%M:%S')" >> "$LOG"
"$RUN" --prior unsolved --board ipc2018-sat --board ipc2023-sat
echo "chain2: phase D exit $?  $(date '+%H:%M:%S')" >> "$LOG"
