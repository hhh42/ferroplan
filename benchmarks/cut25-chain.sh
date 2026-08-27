#!/usr/bin/env bash
# Wait out the certificate gate, then run the 0.25 cut sweep -- but ONLY if the
# gate came back clean.
#
# opt-differential.py --board-budget replays every certified row under BOARD
# conditions (60 s external kill, FF_TIME_LIMIT armed). Its own docstring is
# explicit about what a mismatch means: the named 0.19 conditional-effect
# admissibility rows are the fresh cost being the truth, and anywhere else is
# "a real regression -- stop the phase". So this refuses to sweep on a
# REGRESSION rather than burning three days measuring against a binary whose
# certificates no longer hold.
#
# Same shape as overnight-chain.sh: one detached process, a marker per stage,
# and a log a human can read afterwards.
set -u
cd "$(dirname "$0")/.."

echo "== waiting for the certificate gate $(date '+%Y-%m-%d %H:%M:%S')"
while pgrep -f 'opt-differential\.py' >/dev/null 2>&1; do sleep 30; done

LOG=benchmarks/cut25/opt-differential.log
if ! grep -q "DIFFERENTIAL DONE" "$LOG" 2>/dev/null; then
  echo "REFUSING: the gate did not reach DIFFERENTIAL DONE -- it died or was killed"
  tail -5 "$LOG" 2>/dev/null
  exit 1
fi
# ANCHORED. An unanchored grep matches the summary line's own "0 REGRESSION"
# and refuses a green gate -- which is exactly what happened the first time
# this ran. A verdict line starts with the verdict, left-padded to 12 columns
# (opt-differential.py's print), so ^REGRESSION is the only honest test.
if grep -qE "^REGRESSION" "$LOG"; then
  echo "REFUSING: the gate found REGRESSION(s) -- stop the phase, do not sweep"
  grep -E "^REGRESSION" "$LOG" | head -20
  exit 1
fi

echo "== gate GREEN: $(grep 'DIFFERENTIAL DONE' "$LOG")"
echo "== launching the 0.25 cut sweep $(date '+%Y-%m-%d %H:%M:%S')"
exec bash benchmarks/cut25-sweeps.sh
