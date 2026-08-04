#!/usr/bin/env bash
# THE 0.21 CUT SWEEP — all THIRTEEN boards (the twelve canonical ones plus
# the new ipc2026-opt proof board) against the final 0.21.0 binary, staged in
# benchmarks/air21/ and promoted by promote-air21.sh. Same driver pattern and
# Air discipline as rebaseline-air.sh (--jobs 2 / --mem-gb 6 — see the
# rationale there and in docs/migration-m5.md step 5).
#
# BEFORE running this:
#   1. The box must be otherwise QUIET (no backfill chain, no differentials —
#      docs/roadmap-0.21.md Phase 1 conditions; the 0.20 measurement-gate
#      finding). overnight-chain.sh's idle gate is the model: three quiet
#      samples before anything is measured.
#   2. benchmarks/opt-differential.py must be GREEN on this binary (all 306
#      certificates re-certify) — it is Phase 4's gate, and a cut sweep on a
#      binary that bleeds certificates is a wasted day.
#   3. The binary must be the cut candidate: cargo build --release, version
#      0.21.0 (the build step below checks the version string).
#
# Resume-aware: per-board .done markers in benchmarks/air21/. Re-running is
# always safe.
set -u
cd "$(dirname "$0")/.."
mkdir -p benchmarks/air21

JOBS="${JOBS:-2}"
MEMGB="${MEMGB:-6}"

cargo build --release -p ferroplan-cli 2>&1 | tail -1
V="$(./target/release/ff --version 2>/dev/null || true)"
echo "binary: ${V:-unknown}"
case "$V" in
  *0.21*) : ;;
  *) echo "REFUSING: binary does not report 0.21 — build the cut candidate first"
     exit 1 ;;
esac

# --- measurement hygiene (0.21: learned the hard way) ------------------------
# A dev thread compiling on this box starves a sweep: four parallel `cargo test
# --all --release` jobs took elevator-2011 i10 from 22s to 122s. A board measured
# under that is not a slow board, it is a WRONG board, and the v0.18 backfill lost
# one to exactly this. So: never START a board under load, and never MARK DONE a
# board that was contaminated while it ran — leave it for the resume pass.
idle_pct() {
  top -l 2 -n 0 -s 1 2>/dev/null | grep '^CPU usage' | tail -1 \
    | awk -F'[ ,%]+' '{for (i = 1; i <= NF; i++) if ($i == "idle") print int($(i - 1))}'
}
QUIET="${QUIET:-70}"     # percent idle that counts as quiet
wait_quiet() {
  local got=0 idle
  while [ "$got" -lt 2 ]; do
    idle=$(idle_pct); [ -z "$idle" ] && idle=0
    if [ "$idle" -ge "$QUIET" ]; then got=$((got + 1)); else
      [ "$got" -gt 0 ] && echo "    (load returned: ${idle}% idle — waiting)"
      got=0; sleep 60
    fi
  done
}

run_board() { # name track timeout extra-args...
  local name="$1" track="$2" tmo="$3"; shift 3
  if [ -f "benchmarks/air21/$name.done" ]; then
    echo "SKIP $name (done)"
    return
  fi
  wait_quiet
  echo "RUN  $name ($track ${tmo}s $*) $(date '+%H:%M:%S')"
  # Record what ELSE the machine is doing for the whole board, attributed by
  # process. This box is a laptop: a browser, Spotlight, a Docker VM running
  # CI, Time Machine. Contention only ever DEPRESSES coverage, so it invents
  # regressions and hides gains — a board must carry its own conditions.
  local cond="benchmarks/air21/$name.conditions.json"
  python3 benchmarks/contention.py --out "$cond" --interval 20 &
  local watcher=$!
  python3 benchmarks/ipc67.py --track "$track" --timeout "$tmo" \
    --jobs "$JOBS" --mem-gb "$MEMGB" "$@" \
    --out "benchmarks/air21/$name.md" >"benchmarks/air21/$name.log" 2>&1
  kill "$watcher" 2>/dev/null; wait "$watcher" 2>/dev/null
  local verdict med who
  verdict=$(python3 -c "import json;d=json.load(open('$cond'));print(d['verdict'])" 2>/dev/null || echo unknown)
  med=$(python3 -c "import json;d=json.load(open('$cond'));print(d['idle_pct']['median'])" 2>/dev/null || echo '?')
  who=$(python3 -c "
import json
d=json.load(open('$cond'))
c=d.get('competitors_mean_pcpu') or {}
print(', '.join(f'{k} {v:.0f}%' for k,v in list(c.items())[:2]) or 'none')" 2>/dev/null || echo '?')
  echo "DONE $name: $(tail -1 "benchmarks/air21/$name.md") $(date '+%H:%M:%S') [idle ${med}% $verdict]"
  if [ "$verdict" = "clean" ]; then
    touch "benchmarks/air21/$name.done"
  else
    echo "!! $name measured under contention (median idle ${med}%; competing: ${who})"
    echo "   NOT marking done — re-run this driver when the box is free."
  fi
}

# Cheapest first (a driver that dies early banks something); the NEW board
# leads so the cycle's entry exists even on a truncated day; 300s entry last.
# One entry per board: name track timeout [extra args].
BOARDS=(
  "ipc2026-opt        opt-2026        60 --mode optimal"
  "ipc2023-agile      agile-2023      60"
  "ipc2014-tempo      tempo-sat-2014  30"
  "ipc2018-sat        sat-2018        60"
  "ipc2014-sat        seq-sat-2014    60"
  "ipc2014-agile      seq-agile-2014  60"
  "ipc2014-opt        seq-opt-2014    60 --mode optimal"
  "ipc2026-numeric    numeric-2026    60"
  "ipc2023-numeric    numeric-2023    60"
  "ipc-opt-2008-11    seq-opt         60 --mode optimal"
  "ipc67-temporal     tempo-sat       30"
  "ipc67-results      seq-sat         60"
  "ipc2023-agile-300s agile-2023      300"
)

remaining() {  # names of boards not yet banked clean
  local n out=""
  for spec in "${BOARDS[@]}"; do
    n=${spec%% *}
    [ -f "benchmarks/air21/$n.done" ] || out="$out $n"
  done
  echo "$out"
}

# SELF-HEALING PASSES. A board measured under contention is refused, not
# banked — so without this the driver would finish "done" with holes in it and
# wait for a human to notice. Instead it keeps making passes: each pass waits
# for a quiet window, retries only what is still missing, and stops when the
# set is complete. On a machine that is busy by day and free overnight this
# converges on its own.
MAX_PASSES="${MAX_PASSES:-8}"
pass=1
while : ; do
  todo=$(remaining)
  [ -z "$todo" ] && break
  if [ "$pass" -gt "$MAX_PASSES" ]; then
    echo "!! gave up after $MAX_PASSES passes; still contended:$todo"
    echo "   (raise MAX_PASSES=, or free the box and re-run this driver)"
    exit 1
  fi
  [ "$pass" -gt 1 ] && echo "== pass $pass — retrying contended boards:$todo"
  for spec in "${BOARDS[@]}"; do
    # shellcheck disable=SC2086
    run_board $spec
  done
  pass=$((pass + 1))
done

echo "0.21 CUT SWEEP ALL DONE $(date '+%Y-%m-%d %H:%M:%S') (passes: $((pass - 1)))"
echo "next: benchmarks/promote-air21.sh"
