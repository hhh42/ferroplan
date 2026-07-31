#!/usr/bin/env bash
# 0.20 cut sweeps — every touched board against the final 0.20.0 binary.
# The refill loop + novelty-light rung change every budget-armed ladder;
# LM-cut + the optimal ladder change both opt boards; the eps provider
# pin + val=None runner verdicts change the temporal boards; and the
# IPC-2026 numeric corpus gets its FIRST board. Resume-aware: per-board
# .done markers in benchmarks/cut20/.
set -u
cd "$(dirname "$0")/.."
mkdir -p benchmarks/cut20

run_board() { # name track timeout extra-args...
  local name="$1" track="$2" tmo="$3"; shift 3
  if [ -f "benchmarks/cut20/$name.done" ]; then
    echo "SKIP $name (done)"
    return
  fi
  echo "RUN $name ($track ${tmo}s $*)"
  python3 benchmarks/ipc67.py --track "$track" --timeout "$tmo" --jobs 3 "$@" \
    --out "benchmarks/cut20/$name.md" >/dev/null 2>&1
  echo "DONE $name: $(tail -1 "benchmarks/cut20/$name.md")"
  touch "benchmarks/cut20/$name.done"
}

run_board ipc2026-numeric    numeric-2026    60
run_board ipc2018-sat        sat-2018        60
run_board ipc2023-agile      agile-2023      60
run_board ipc2023-numeric    numeric-2023    60
run_board ipc2014-sat        seq-sat-2014    60
run_board ipc2014-agile      seq-agile-2014  60
run_board ipc2014-tempo      tempo-sat-2014  30
run_board ipc67-temporal     tempo-sat       30
run_board ipc-opt-2008-11    seq-opt         60 --mode optimal
run_board ipc2014-opt        seq-opt-2014    60 --mode optimal
run_board ipc67-results      seq-sat         60
run_board ipc2023-agile-300s agile-2023      300
echo "CUT20 SWEEPS ALL DONE"
