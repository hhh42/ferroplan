#!/usr/bin/env bash
# 0.18 cut sweeps — every gate-touched classical board re-swept under the
# wired runner (FF_TIME_LIMIT armed; the Phase 4 referee) against the
# final 0.18.0 binary. Resume-aware: each board writes to benchmarks/cut18/
# and stamps a .done marker, so a container restart re-runs only what's
# missing. Delete benchmarks/cut18/ to force a full re-run.
set -u
cd "$(dirname "$0")/.."
mkdir -p benchmarks/cut18

run_board() { # name track extra-env...
  local name="$1" track="$2"; shift 2
  if [ -f "benchmarks/cut18/$name.done" ]; then
    echo "SKIP $name (done)"
    return
  fi
  echo "RUN $name ($track $*)"
  env "$@" python3 benchmarks/ipc67.py --track "$track" --timeout 60 --jobs 3 \
    --out "benchmarks/cut18/$name.md" >/dev/null 2>&1
  echo "DONE $name: $(tail -1 "benchmarks/cut18/$name.md")"
  touch "benchmarks/cut18/$name.done"
}

run_board ipc2018-sat       sat-2018
run_board ipc2023-agile     agile-2023
run_board ipc2023-numeric   numeric-2023
run_board ipc2018-sat-nov   sat-2018     FF_NOVELTY=1
run_board ipc2023-agile-nov agile-2023   FF_NOVELTY=1
run_board ipc67-results     seq-sat
run_board ipc2014-sat       seq-sat-2014
run_board ipc2014-agile     seq-agile-2014
echo "CUT18 SWEEPS ALL DONE"
