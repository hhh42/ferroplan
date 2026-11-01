#!/usr/bin/env bash
# Lane A' (0.31): the parallel rung portfolio (FF_PARALLEL_RUNGS=1) on the
# six mco boards (t2/t4/t8 x ipc2014/ipc7), every row, against the banked
# 0.30.0 rows (b02061eded16). Band +40-80 rows; kill < +20 or any t2 board
# losing more than it gains. Launched detached; stop with
# kill $(cat par-mco.pid), then pkill -f par31-
C=/Users/harold/ferroplan-0.31/crucible/target/release/crucible
FF=/Users/harold/ferroplan-0.31/target/release/ff
echo "=== par31-mco start $(date '+%F %T') ff=$(stat -f %Sm "$FF")"
$C --repo /Users/harold/ferroplan sweep --set cut30 \
  --board ipc2014-mco-t2 --board ipc2014-mco-t4 --board ipc2014-mco-t8 \
  --board ipc7-mco-t2 --board ipc7-mco-t4 --board ipc7-mco-t8 \
  --engine "$FF" --env FF_PARALLEL_RUNGS=1 \
  --name par31-mco --headless
echo "=== par31-mco exit $? $(date '+%F %T')"
echo "=== PAR-MCO COMPLETE $(date '+%F %T')"
