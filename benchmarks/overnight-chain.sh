#!/usr/bin/env bash
# Wait for the box to go quiet, then run the measurement queue unattended.
#
# WHY THE GATE. Near-wall results on this machine are not reproducible under
# load, and that is not a small effect: elevator-2011 i10 proves cost 63 in
# 27 s on an overnight-quiet box and takes 122 s SOLO while a game and an
# editor hold the 4 Super cores — macOS schedules `ff` onto an Efficiency core
# and it does ~a quarter of the work per second. A differential run under those
# conditions reported 0/13 where the boards recorded 13/13, with a
# byte-identical binary. So: measure when quiet, or do not measure.
#
#   bash benchmarks/overnight-chain.sh                 # gate, differentials, v0.19.0
#   bash benchmarks/overnight-chain.sh v0.18.0 v0.17.0  # gate, then those tags in order
#   NO_WAIT=1 bash benchmarks/overnight-chain.sh ...    # skip the gate (box already quiet)
#
# Resume-aware throughout: every stage skips its own completed work, so
# re-running after an interruption costs nothing.
set -u
cd "$(dirname "$0")/.."
mkdir -p benchmarks/air-chain
LOG=benchmarks/air-chain/chain.log
exec > >(tee -a "$LOG") 2>&1

echo "=== chain start $(date '+%F %H:%M:%S') ==="

wait_for_quiet() { # want N consecutive quiet samples
  local need="${1:-3}" got=0 idle
  echo "==> waiting for the box to go quiet (${need} consecutive quiet samples)"
  while : ; do
    # `top -l 2` because the first sample is cumulative-since-boot and useless.
    # Find the field LABELLED idle rather than counting from the end: the
    # line has a trailing space, so $(NF-1) is the literal word "idle" and
    # int() of it is 0 — a gate that would have waited forever.
    idle=$(top -l 2 -n 0 -s 2 2>/dev/null | grep '^CPU usage' | tail -1 \
           | awk -F'[ ,%]+' '{for (i = 1; i <= NF; i++) if ($i == "idle") print int($(i - 1))}')
    [ -z "$idle" ] && idle=0
    if [ "$idle" -ge 70 ]; then
      got=$((got + 1))
      echo "    idle ${idle}%  (${got}/${need})"
    else
      [ "$got" -gt 0 ] && echo "    idle ${idle}% — resetting"
      got=0
    fi
    [ "$got" -ge "$need" ] && { echo "==> box is quiet $(date '+%H:%M:%S')"; return 0; }
    sleep 60
  done
}

[ "${NO_WAIT:-0}" = "1" ] || wait_for_quiet 3

# --- stage 1: the phase differentials (short, and they gate the cut record) ---
for spec in lmcut novlight refill; do
  marker="benchmarks/air-chain/diff-$spec.done"
  if [ -f "$marker" ]; then echo "SKIP differential $spec (done)"; continue; fi
  echo "==> differential: $spec $(date '+%H:%M:%S')"
  if python3 benchmarks/hatch-differential.py --spec "$spec" --timeout 60 \
       --jobs 1 --repeat 2 > "benchmarks/air-chain/diff-$spec.log" 2>&1; then
    tail -4 "benchmarks/air-chain/diff-$spec.log"
    touch "$marker"
  else
    echo "!! differential $spec FAILED — see benchmarks/air-chain/diff-$spec.log"
  fi
done

# --- stage 2: the backfills (the long pole, ~17-20h each) ---
TAGS=("$@"); [ ${#TAGS[@]} -eq 0 ] && TAGS=(v0.19.0)
for tag in "${TAGS[@]}"; do
  ver="${tag#v}"
  if [ -f "benchmarks/air-chain/backfill-$ver.done" ]; then
    echo "SKIP backfill $tag (done)"; continue
  fi
  echo "==> backfill $tag $(date '+%F %H:%M:%S')"
  if bash benchmarks/backfill-air.sh "$tag"; then
    touch "benchmarks/air-chain/backfill-$ver.done"
  else
    echo "!! backfill $tag FAILED (resume by re-running this script)"
  fi
done

echo "=== chain done $(date '+%F %H:%M:%S') ==="
