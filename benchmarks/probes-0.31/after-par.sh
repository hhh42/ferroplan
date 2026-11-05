#!/usr/bin/env bash
# The box queue behind par31-mco (0.31): waits for the sweep to exit,
# then the owed solo receipts -- ipc2023-sat's five Lane B losses, four
# arms -- and Phase 0.6's grounder receipt. Serial, nothing beside it.
H=/Volumes/Data/Projects/ferroplan-0.31/benchmarks/probes-0.31
export FERROPLAN_IPC_CORPUS=/Volumes/Data/Projects/ferroplan/benchmarks/.ipc-corpus
while kill -0 "$(cat $H/par/par-mco.pid)" 2>/dev/null; do sleep 60; done
echo "=== after-par start $(date '+%F %T')"
python3 $H/laneb/solo-receipt.py \
  ricochet-robots-satisficing/6 ricochet-robots-satisficing/9 ricochet-robots-satisficing/10 \
  slitherlink-satisficing/10 recharging-robots-satisficing/3 > $H/laneb/solo-receipt-2023.log 2>&1
python3 $H/ground/ground-profile.py > $H/ground/ground-profile.log 2>&1
echo "=== AFTER-PAR COMPLETE $(date '+%F %T')"
