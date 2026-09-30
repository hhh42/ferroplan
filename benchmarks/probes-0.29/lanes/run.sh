#!/bin/sh
# 0.29 lanes, measured THROUGH the crucible as one subset (docs/roadmap-0.29.md):
# the engine-0.29 binary against the promoted 0.28.0 raws, boards in lane
# priority order so a contended morning still leaves the headline scored.
#   Lane 1  ipc5-simple-pref ipc5-qual-pref ipc5-complex-pref   (points vs SGPlan5)
#   Lane 3  ipc5-constraints                                      (40 cells stopped with wall left)
#   Lane 4  the storage rows inside qual/complex (max_rss now in the DB)
#   Lane W  ipc2018-sat ipc2023-sat
#   Lane 2  ipc2023-numeric ipc2026-numeric                       (the coins inlining; coins i18 is wall-edge)
# One crucible per DB dir; nothing else builds or measures while this runs.
set -u
REPO=/Users/harold/ferroplan
ENGINE=/Users/harold/ferroplan-0.29/target/release/ff
LOG=/Users/harold/ferroplan-0.29/benchmarks/probes-0.29/lanes/subset.log
cd "$REPO" || exit 1
echo "== lanes29 subset  $(date '+%Y-%m-%d %H:%M:%S')  engine: $("$ENGINE" --version 2>/dev/null | head -1)" >> "$LOG"
exec ./crucible/target/release/crucible --repo "$REPO" sweep --set cut28 --headless --max-passes 3 \
  --engine "$ENGINE" --name lanes29 \
  --board ipc5-simple-pref --board ipc5-qual-pref --board ipc5-complex-pref \
  --board ipc5-constraints \
  --board ipc2018-sat --board ipc2023-sat \
  --board ipc2023-numeric --board ipc2026-numeric >> "$LOG" 2>&1
