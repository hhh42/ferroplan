#!/usr/bin/env bash
# Lane B's ladder claim (0.31): BOTH knobs as the ladder's defaults --
# novelty-first LAMA in LAMA's slot (FF_LAMA_NOV=1) and R refresh +
# forgetting in the driver's (FF_NOV_REFRESH=1 FF_NOV_FORGET=1) -- on the
# six matrix boards, every row, against the banked 0.30.0 rows
# (b02061eded16). Read with benchmarks/laneb-read.py. Launched detached;
# stop with kill $(cat laneb-ladder.pid), then pkill -f laneb31-
C=/Users/harold/ferroplan-0.31/crucible/target/release/crucible
FF=/Users/harold/ferroplan-0.31/target/release/ff
echo "=== laneb31-ladder start $(date '+%F %T') ff=$(stat -f %Sm "$FF")"
$C --repo /Users/harold/ferroplan sweep --set cut30 \
  --board ipc2014-agile --board ipc2014-sat --board ipc2018-sat \
  --board ipc2023-sat --board ipc5-prop --board ipc67-results \
  --engine "$FF" --env FF_LAMA_NOV=1 --env FF_NOV_REFRESH=1 --env FF_NOV_FORGET=1 \
  --name laneb31-ladder --headless
echo "=== laneb31-ladder exit $? $(date '+%F %T')"
echo "=== LANEB-LADDER COMPLETE $(date '+%F %T')"
