#!/usr/bin/env bash
# The per-rung matrix (0.31 Phase 0.5): five subsets, one rung each, serial.
# Launched 2026-10-29 detached (perl setsid); stop with: kill $(cat matrix.pid)
C=/Users/harold/ferroplan-0.31/crucible/target/release/crucible
ROWS=/Users/harold/ferroplan-0.31/benchmarks/probes-0.31/matrix/rows.txt
FF=/Users/harold/ferroplan-0.31/target/release/ff
for rung in ehc novlight lama driver bfs; do
  echo "=== matrix31-$rung start $(date '+%F %T')"
  $C --repo /Users/harold/ferroplan sweep --set cut30 \
    --board ipc2014-agile --board ipc2014-sat --board ipc2018-sat \
    --board ipc2023-sat --board ipc5-prop --board ipc67-results \
    --rows "$ROWS" --engine "$FF" --env FF_RUNG_ONLY=$rung \
    --name matrix31-$rung --headless
  echo "=== matrix31-$rung exit $? $(date '+%F %T')"
done
echo "=== MATRIX COMPLETE $(date '+%F %T')"
