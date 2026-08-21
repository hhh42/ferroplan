#!/usr/bin/env bash
# THE 0.25 ENTRIES SWEEP — the nine NEW boards only (docs/roadmap-0.25.md
# Phase 1: the table grows). Deliberately a SEPARATE sweep from the
# standing twenty-two: entries have no before/after, so the cut sweep
# keeps its like-for-like identity and the cut record carries two
# headlines by design. Staged in benchmarks/air25-entries/; promotion
# rides the 0.25 promote script when it exists.
#
# THE NINE, WITH WHAT EACH ONE IS:
#   ipc2014-mco-t2 / ipc2014-mco-t8 — the two missing 2014 multi-core
#     tiers (corpus was on disk all along; t4 reads 163/280).
#   ipc2018-opt — the 2018 dataset's opt/ half, first proof board there.
#   ipc2023-sat / ipc2023-opt — the REAL 2023 classical tracks (the
#     standing "2023 classical" board is the agile corpus at 60 s,
#     flagged baseline since 0.17).
#   ipc2023-numeric-opt — the numeric corpus under Mode::Optimal: the
#     track whose official field CSV (ipc-2023n/results/opt.csv) was
#     vendored with the corpus and never had a board to referee.
#   ipc2026-opt-full — the official 13-domain/260 Overall Optimal
#     constituency (the standing 3-pair board stays as the slice).
#   ipc5-simple-pref / ipc5-qual-pref — the 2006 preference tracks at
#     full corpus (the curated 8-instance reference boards keep their
#     own files). Coverage = hard-goal solves; the preference metric
#     rides in the raw for the quality sitting.
#
# NOT here: complex-pref-2006 — its engine entry is Phase 2; a canary
# before that lands is expected red and proves nothing.
#
# BEFORE running this: the box must be otherwise QUIET (the standing
# 0.21 conditions), and the binary must be the 0.25 dev head (the build
# step below checks the version string). Entries are first measurements,
# not movements — but a contaminated first column poisons every future
# delta, so the contention discipline is identical to a cut sweep.
#
# Resume-aware: per-board .done markers in benchmarks/air25-entries/.
# Re-running is always safe.
set -u
cd "$(dirname "$0")/.."
mkdir -p benchmarks/air25-entries

JOBS="${JOBS:-2}"
MEMGB="${MEMGB:-6}"

cargo build --release -p ferroplan-cli 2>&1 | tail -1
V="$(./target/release/ff --version 2>/dev/null || true)"
echo "binary: ${V:-unknown}"
case "$V" in
  *0.25*) : ;;
  *) echo "REFUSING: binary does not report 0.25 — build the dev head first"
     exit 1 ;;
esac

# --- measurement hygiene (identical to cut24-sweeps.sh) ----------------------
idle_pct() {
  top -l 2 -n 0 -s 1 2>/dev/null | grep '^CPU usage' | tail -1 \
    | awk -F'[ ,%]+' '{for (i = 1; i <= NF; i++) if ($i == "idle") print int($(i - 1))}'
}
QUIET="${QUIET:-70}"
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
  if [ -f "benchmarks/air25-entries/$name.done" ]; then
    echo "SKIP $name (done)"
    return
  fi
  # The mco wall-clock rule: a board spec carrying --threads runs one
  # instance at a time, whatever $JOBS says.
  local jobs="$JOBS"
  case " $* " in *" --threads "*) jobs=1 ;; esac
  wait_quiet
  echo "RUN  $name ($track ${tmo}s jobs $jobs $*) $(date '+%H:%M:%S')"
  local cond="benchmarks/air25-entries/$name.conditions.json"
  python3 benchmarks/contention.py --out "$cond" --interval 20 &
  local watcher=$!
  python3 benchmarks/ipc67.py --track "$track" --timeout "$tmo" \
    --jobs "$jobs" --mem-gb "$MEMGB" "$@" \
    --out "benchmarks/air25-entries/$name.md" \
    >"benchmarks/air25-entries/$name.log" 2>&1
  kill "$watcher" 2>/dev/null; wait "$watcher" 2>/dev/null
  local verdict med who
  verdict=$(python3 -c "import json;d=json.load(open('$cond'));print(d['verdict'])" 2>/dev/null || echo unknown)
  med=$(python3 -c "import json;d=json.load(open('$cond'));print(d['idle_pct']['median'])" 2>/dev/null || echo '?')
  who=$(python3 -c "
import json
d=json.load(open('$cond'))
c=d.get('competitors_mean_pcpu') or {}
print(', '.join(f'{k} {v:.0f}%' for k,v in list(c.items())[:2]) or 'none')" 2>/dev/null || echo '?')
  echo "DONE $name: $(tail -1 "benchmarks/air25-entries/$name.md") $(date '+%H:%M:%S') [idle ${med}% $verdict]"
  if [ "$verdict" = "clean" ]; then
    touch "benchmarks/air25-entries/$name.done"
  else
    echo "!! $name measured under contention (median idle ${med}%; competing: ${who})"
    echo "   NOT marking done — re-run this driver when the box is free."
  fi
}

# Cheapest first (a driver that dies early banks something); the mco pair
# runs late (one-instance-at-a-time makes them the long boards). One entry
# per board: name track timeout [extra args].
BOARDS=(
  "ipc5-simple-pref    simple-pref-2006 60"
  "ipc5-qual-pref      qual-pref-2006   60"
  "ipc2023-sat         sat-2023         60"
  "ipc2023-opt         opt-2023         60 --mode optimal"
  "ipc2018-opt         opt-2018         60 --mode optimal"
  "ipc2023-numeric-opt numeric-2023     60 --mode optimal"
  "ipc2026-opt-full    opt-2026-full    60 --mode optimal"
  "ipc2014-mco-t2      seq-mco-2014     60 --threads 2"
  "ipc2014-mco-t8      seq-mco-2014     60 --threads 8"
)

remaining() {
  local n out=""
  for spec in "${BOARDS[@]}"; do
    n=${spec%% *}
    [ -f "benchmarks/air25-entries/$n.done" ] || out="$out $n"
  done
  echo "$out"
}

# Self-healing passes, exactly as the cut sweeps do them: a board measured
# under contention is refused, not banked, and the driver keeps making
# passes until the set is complete.
MAX_PASSES="${MAX_PASSES:-8}"
pass=1
while : ; do
  todo=$(remaining)
  [ -z "$todo" ] && break
  [ "$pass" -gt "$MAX_PASSES" ] && {
    echo "GIVING UP after $MAX_PASSES passes; still missing:$todo"
    exit 1
  }
  echo "== pass $pass — remaining:$todo =="
  for spec in "${BOARDS[@]}"; do
    n=${spec%% *}
    case " $todo " in *" $n "*) run_board $spec ;; esac
  done
  pass=$((pass + 1))
done

echo "0.25 ENTRIES SWEEP ALL DONE $(date '+%Y-%m-%d %H:%M:%S') (passes: $((pass - 1)))"
echo "next: write benchmarks/promote-air25.sh (entries promote alongside the"
echo "      cut boards; the SWEEPS registry already knows all nine names),"
echo "      then python3 benchmarks/standings.py"
