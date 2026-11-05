#!/usr/bin/env bash
# Lane D's rung through the matrix (0.31): the red-black GBFS alone
# (FF_RUNG_ONLY=rb) on the same 1,008 rows as the eight rungs before it,
# judged on its UNIQUE solves against them and its evals/s against h^FF's
# (the kill: > 3x the per-evaluation cost without rows). $1 = a frozen
# copy of the engine. Stop with kill $(cat matrix-rb.pid), then
# pkill -f matrix31-rb
C=/Volumes/Data/Projects/ferroplan-0.31/crucible/target/release/crucible
ROWS=/Volumes/Data/Projects/ferroplan-0.31/benchmarks/probes-0.31/matrix/rows.txt
FF=${1:?frozen engine path}
echo "=== matrix31-rb start $(date '+%F %T') ff=$FF"
$C --repo /Volumes/Data/Projects/ferroplan sweep --set cut30 \
  --board ipc2014-agile --board ipc2014-sat --board ipc2018-sat \
  --board ipc2023-sat --board ipc5-prop --board ipc67-results \
  --rows "$ROWS" --engine "$FF" --env FF_RUNG_ONLY=rb \
  --name matrix31-rb --headless
echo "=== matrix31-rb exit $? $(date '+%F %T')"
echo "=== MATRIX-RB COMPLETE $(date '+%F %T')"
