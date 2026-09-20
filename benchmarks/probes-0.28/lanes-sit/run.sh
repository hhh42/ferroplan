#!/bin/sh
# The 0.28 S/I/T lanes, measured on the boards they were built for -- and on
# the two temporal boards they were NOT built for (the regression read).
#
# ONE attempt per row, 2-wide, threads 1, 60 s: the board harness, the board
# wall. The comparison column is the PUBLISHED board, which is best-of-N
# (docs/roadmap-0.28.md, THE INSTRUMENT), so a single attempt here is the
# conservative side of every delta. A row the new engine LOSES is re-run on
# the old binary under the same conditions before it is called a loss
# (differential.sh) -- equal-N where it matters, not everywhere.
#
# A MEASUREMENT: writes here, touches no published board. Nothing else should
# be using the CPU while it runs (no cargo, no second sweep).
set -u
cd "$(dirname "$0")/../../.." || exit 1
HERE=benchmarks/probes-0.28/lanes-sit
export FERROPLAN_IPC_CORPUS=/Users/harold/ferroplan/benchmarks/.ipc-corpus
export FERROPLAN_VAL=/Users/harold/ferroplan/benchmarks/.val/VAL/build/bin/Validate
export FERROPLAN_FF="$PWD/target/release/ff"
"$FERROPLAN_FF" --version > "$HERE/engine.txt"
git rev-parse HEAD >> "$HERE/engine.txt"
for track in ${TRACKS:-qual-pref-2006 simple-pref-2006 complex-pref-2006 time-2006 metric-time-2006 constraints-2006 tempo-sat-2014 tempo-sat}; do
  [ -f "$HERE/$track.done" ] && continue
  echo "== $track $(date '+%F %T')" >> "$HERE/run.log"
  python3 benchmarks/ipc67.py --track "$track" --timeout 60 --jobs 2 \
      --out "$HERE/$track.md" >> "$HERE/run.log" 2>&1 && touch "$HERE/$track.done"
done
echo "== finished $(date '+%F %T')" >> "$HERE/run.log"
