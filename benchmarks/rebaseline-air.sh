#!/usr/bin/env bash
# THE AIR RE-BASELINE — all twelve canonical boards against the 0.20.0 binary
# on the M5 MacBook Air. Same board list and driver pattern as
# cut20-sweeps.sh, which this supersedes on the new box: the 0.20 cut never
# ran on the cloud container, so the cut sweep and the mandatory re-baseline
# (docs/migration-m5.md step 4) are one and the same pass. Every number these
# boards produce is Air-baselined and MUST NOT be A/B'd against any 0.19
# cloud-box board; from here on, A/B is Air-vs-Air only.
#
# Air discipline (docs/migration-m5.md step 5):
#   --jobs 2, not 3 — the Air is fanless and clocks must stay stable across a
#   multi-hour sweep or coverage-at-timeout stops being comparable within it.
#   --mem-gb 6, not the phys/jobs default of 8 — on macOS RLIMIT_AS cannot be
#   set at all, so ipc67.py enforces the cap with an RSS watchdog instead, and
#   a poll needs headroom the address-space cap did not: 2x6 = 12 GiB against
#   16 GiB physical leaves the OS 4 GiB and absorbs anything a job can
#   allocate inside one 0.25s poll interval.
#
# Resume-aware: per-board .done markers in benchmarks/air/. A laptop sleeps
# and a session ends, so re-running this script is always safe.
set -u
cd "$(dirname "$0")/.."
mkdir -p benchmarks/air

JOBS="${JOBS:-2}"
MEMGB="${MEMGB:-6}"

run_board() { # name track timeout extra-args...
  local name="$1" track="$2" tmo="$3"; shift 3
  if [ -f "benchmarks/air/$name.done" ]; then
    echo "SKIP $name (done)"
    return
  fi
  echo "RUN  $name ($track ${tmo}s $*) $(date '+%H:%M:%S')"
  python3 benchmarks/ipc67.py --track "$track" --timeout "$tmo" \
    --jobs "$JOBS" --mem-gb "$MEMGB" "$@" \
    --out "benchmarks/air/$name.md" >"benchmarks/air/$name.log" 2>&1
  echo "DONE $name: $(tail -1 "benchmarks/air/$name.md") $(date '+%H:%M:%S')"
  touch "benchmarks/air/$name.done"
}

# Cheapest boards first: a driver that dies early should still have banked
# something, and the 300s entry board is deliberately last.
run_board ipc2023-agile      agile-2023      60
run_board ipc2014-tempo      tempo-sat-2014  30
run_board ipc2018-sat        sat-2018        60
run_board ipc2014-sat        seq-sat-2014    60
run_board ipc2014-agile      seq-agile-2014  60
run_board ipc2014-opt        seq-opt-2014    60 --mode optimal
run_board ipc2026-numeric    numeric-2026    60
run_board ipc2023-numeric    numeric-2023    60
run_board ipc-opt-2008-11    seq-opt         60 --mode optimal
run_board ipc67-temporal     tempo-sat       30
run_board ipc67-results      seq-sat         60
run_board ipc2023-agile-300s agile-2023      300
echo "AIR RE-BASELINE ALL DONE $(date '+%Y-%m-%d %H:%M:%S')"
