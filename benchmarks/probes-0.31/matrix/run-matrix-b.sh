#!/usr/bin/env bash
# Lane B's candidate rungs through the matrix (0.31): same 1,008 rows, same
# engine, one subset per rung, serial. Judged on UNIQUE solves against the
# five-rung matrix of 10-29..10-31. Launched 2026-10-31 detached; stop with
# kill $(cat matrix-b.pid), then pkill -f matrix31-
C=/Users/harold/ferroplan-0.31/crucible/target/release/crucible
ROWS=/Users/harold/ferroplan-0.31/benchmarks/probes-0.31/matrix/rows.txt
FF=/Users/harold/ferroplan-0.31/target/release/ff
for rung in lamanov front driverx; do
  echo "=== matrix31-$rung start $(date '+%F %T')"
  $C --repo /Users/harold/ferroplan sweep --set cut30 \
    --board ipc2014-agile --board ipc2014-sat --board ipc2018-sat \
    --board ipc2023-sat --board ipc5-prop --board ipc67-results \
    --rows "$ROWS" --engine "$FF" --env FF_RUNG_ONLY=$rung \
    --name matrix31-$rung --headless
  echo "=== matrix31-$rung exit $? $(date '+%F %T')"
done
echo "=== MATRIX-B COMPLETE $(date '+%F %T')"
