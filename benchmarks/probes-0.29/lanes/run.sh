#!/bin/sh
# 0.29 lanes, measured THROUGH the crucible as one named subset, `lanes29`
# (docs/roadmap-0.29.md): the engine-0.29 binary against the promoted 0.28.0
# raws. Rows are the record; owed rows are picked up by the next launch under
# the same name. Launched in phases so the night's quiet hours go to the
# headline first and the daytime footprint is the smallest useful piece:
#   A  ipc5-simple-pref ipc5-qual-pref ipc5-complex-pref   Lane 1 (points), Lane 4 (storage rows' memory)
#   B  the six other boards --prior solved                  the loss side of Lanes 2/3/W, packed, fast
#   C  ipc2023-numeric ipc2026-numeric ipc5-constraints      Lane 2's gain side, Lane 3's owed re-runs
#   D  ipc2018-sat ipc2023-sat --prior unsolved             Lane W's gain side (the next quiet night)
# The first launch (03:35) ran the whole set in the crucible's own order:
# ipc5-constraints pass 1 banked (28 solved of 120, 36 owed), ipc2018-sat 2
# rows; it was stopped at 04:45 for this phasing.
# Usage: run.sh <boards and/or --prior ...>      e.g. run.sh --board ipc5-qual-pref
set -u
REPO=/Users/harold/ferroplan
ENGINE=/Users/harold/ferroplan-0.29/target/release/ff
LOG=/Users/harold/ferroplan-0.29/benchmarks/probes-0.29/lanes/subset.log
cd "$REPO" || exit 1
echo "== lanes29 launch $(date '+%Y-%m-%d %H:%M:%S')  args: $*" >> "$LOG"
exec ./crucible/target/release/crucible --repo "$REPO" sweep --set cut28 --headless --max-passes 3 \
  --engine "$ENGINE" --name lanes29 "$@" >> "$LOG" 2>&1
