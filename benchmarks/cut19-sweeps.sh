#!/usr/bin/env bash
# 0.19 cut sweeps — every classical board re-swept against the final
# 0.19.0 binary (the novelty-default-under-budget rider changes every
# runner sweep's ladder), plus the ONE official-budget 300 s 2023-agile
# ENTRY (locked at scoping). Resume-aware: per-board .done markers in
# benchmarks/cut19/; re-running skips finished boards.
set -u
cd "$(dirname "$0")/.."
mkdir -p benchmarks/cut19

run_board() { # name track timeout extra-args...
  local name="$1" track="$2" tmo="$3"; shift 3
  if [ -f "benchmarks/cut19/$name.done" ]; then
    echo "SKIP $name (done)"
    return
  fi
  echo "RUN $name ($track ${tmo}s $*)"
  python3 benchmarks/ipc67.py --track "$track" --timeout "$tmo" --jobs 3 "$@" \
    --out "benchmarks/cut19/$name.md" >/dev/null 2>&1
  echo "DONE $name: $(tail -1 "benchmarks/cut19/$name.md")"
  touch "benchmarks/cut19/$name.done"
}

run_board ipc2018-sat        sat-2018        60
run_board ipc2023-agile      agile-2023      60
run_board ipc2023-numeric    numeric-2023    60
run_board ipc67-results      seq-sat         60
run_board ipc2014-sat        seq-sat-2014    60
run_board ipc2014-agile      seq-agile-2014  60
run_board ipc2023-agile-300s agile-2023      300
echo "CUT19 SWEEPS ALL DONE"
