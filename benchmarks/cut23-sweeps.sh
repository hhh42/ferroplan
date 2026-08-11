#!/usr/bin/env bash
# THE 0.23 CUT SWEEP — all TWENTY-TWO boards — the sixteen 0.22 boards
# plus the Phase 3 sitting's six re-entries (ipc5-time, ipc5-metric-time,
# and the four mco boards: the LAST cloud-era ghosts, 1,450 instances
# leaving the not-re-baselined list) against the final 0.23.0 binary,
# staged in benchmarks/air23/ and promoted by promote-air23.sh. Same
# driver pattern and Air discipline as cut22-sweeps.sh (--jobs 2 /
# --mem-gb 6 — see rebaseline-air.sh and docs/migration-m5.md step 5).
#
# WHAT IS NEW THIS CYCLE, ON THE RECORD BEFORE THE SWEEP RUNS:
#
# 1. THE TEMPORAL TIER MOVE, 30 -> 60 s (docs/roadmap-0.23.md Phase 3).
#    ipc67-temporal and ipc2014-tempo run at 60 s below. The 0.21
#    methodological rule applies: A BUDGET CHANGE IS REFEREED BY
#    OLD-BINARY-AT-NEW-BUDGET, NEVER BY HATCHES — so the budget/engine
#    split is provable per instance. The sweep session runs the v0.22.0
#    binary at the SAME 60 s budget (current harness, old engine — the
#    backfill-air.sh instrument rule), staged beside the boards but
#    NEVER promoted:
#
#      git worktree add --detach ../ferroplan-backfill-0.22.0 v0.22.0
#      ( cd ../ferroplan-backfill-0.22.0 && cargo build --release -q -p ferroplan-cli )
#      OLD="$(cd ../ferroplan-backfill-0.22.0 && pwd)/target/release/ff"
#      FERROPLAN_FF="$OLD" python3 benchmarks/ipc67.py \
#        --track tempo-sat --timeout 60 --jobs 2 --mem-gb 6 \
#        --out benchmarks/air23/ipc67-temporal-v0.22.0-60s.md
#      FERROPLAN_FF="$OLD" python3 benchmarks/ipc67.py \
#        --track tempo-sat-2014 --timeout 60 --jobs 2 --mem-gb 6 \
#        --out benchmarks/air23/ipc2014-tempo-v0.22.0-60s.md
#
#    (Same quiet-box discipline as the boards; run them through a quiet
#    window, not alongside. The 0.22 record's open v0.19/v0.21.0
#    backfill column is a SEPARATE debt — backfill-air.sh — and is not
#    discharged by these two passes.)
#
# 2. THE IPC-5 TIME / METRIC-TIME RE-BASELINE (330 instances, 30 s —
#    the standing temporal tier for these boards; both tracks verified
#    to enumerate: time-2006 = 5 variants/130, metric-time-2006 =
#    6 variants/200). Every solved row carries makespan (recorded since
#    0.22), so standings.py scores the tracks' quality currency against
#    the vendored IPC5-results.tgz archive the day these boards land.
#
# 3. THE MCO METHODOLOGY, WRITTEN DOWN BEFORE THE SWEEP (the 0.16 rule,
#    re-affirmed for the 4P+6E Air): the competition scores WALL-CLOCK
#    on a fixed box, however many cores a planner burns — so each mco
#    board runs ONE INSTANCE AT A TIME with the full thread count
#    (--threads N --jobs 1; the driver forces jobs=1 on any board spec
#    carrying --threads). The 60 s budget is wall-clock. t2 and t4 fit
#    the four performance cores; t8 is OVERSUBSCRIBED BY CONSTRUCTION
#    on this box (8 worker threads vs 4 P-cores, spillover onto
#    E-cores) and its board is read with that stamp, not excused by it.
#    --mem-gb stays 6 per job for cross-board consistency (mco runs one
#    job, so the box has headroom to spare).
#
# BEFORE running this:
#   1. The box must be otherwise QUIET (no backfill chain, no differentials —
#      docs/roadmap-0.21.md Phase 1 conditions; the 0.20 measurement-gate
#      finding). overnight-chain.sh's idle gate is the model: three quiet
#      samples before anything is measured.
#   2. benchmarks/opt-differential.py must be GREEN on this binary (all 373
#      certificates re-certify) — it is the proof boards' gate, and a cut
#      sweep on a binary that bleeds certificates is a wasted day.
#   3. The binary must be the cut candidate: cargo build --release, version
#      0.23.0 (the build step below checks the version string).
#
# Resume-aware: per-board .done markers in benchmarks/air23/. Re-running is
# always safe.
set -u
cd "$(dirname "$0")/.."
mkdir -p benchmarks/air23

JOBS="${JOBS:-2}"
MEMGB="${MEMGB:-6}"

cargo build --release -p ferroplan-cli 2>&1 | tail -1
V="$(./target/release/ff --version 2>/dev/null || true)"
echo "binary: ${V:-unknown}"
case "$V" in
  *0.23*) : ;;
  *) echo "REFUSING: binary does not report 0.23 — build the cut candidate first"
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
  if [ -f "benchmarks/air23/$name.done" ]; then
    echo "SKIP $name (done)"
    return
  fi
  # The mco wall-clock rule (header point 3): a board spec carrying
  # --threads runs one instance at a time, whatever $JOBS says.
  local jobs="$JOBS"
  case " $* " in *" --threads "*) jobs=1 ;; esac
  wait_quiet
  echo "RUN  $name ($track ${tmo}s jobs $jobs $*) $(date '+%H:%M:%S')"
  # Record what ELSE the machine is doing for the whole board, attributed by
  # process. This box is a laptop: a browser, Spotlight, a Docker VM running
  # CI, Time Machine. Contention only ever DEPRESSES coverage, so it invents
  # regressions and hides gains — a board must carry its own conditions.
  local cond="benchmarks/air23/$name.conditions.json"
  python3 benchmarks/contention.py --out "$cond" --interval 20 &
  local watcher=$!
  python3 benchmarks/ipc67.py --track "$track" --timeout "$tmo" \
    --jobs "$jobs" --mem-gb "$MEMGB" "$@" \
    --out "benchmarks/air23/$name.md" >"benchmarks/air23/$name.log" 2>&1
  kill "$watcher" 2>/dev/null; wait "$watcher" 2>/dev/null
  local verdict med who
  verdict=$(python3 -c "import json;d=json.load(open('$cond'));print(d['verdict'])" 2>/dev/null || echo unknown)
  med=$(python3 -c "import json;d=json.load(open('$cond'));print(d['idle_pct']['median'])" 2>/dev/null || echo '?')
  who=$(python3 -c "
import json
d=json.load(open('$cond'))
c=d.get('competitors_mean_pcpu') or {}
print(', '.join(f'{k} {v:.0f}%' for k,v in list(c.items())[:2]) or 'none')" 2>/dev/null || echo '?')
  echo "DONE $name: $(tail -1 "benchmarks/air23/$name.md") $(date '+%H:%M:%S') [idle ${med}% $verdict]"
  if [ "$verdict" = "clean" ]; then
    touch "benchmarks/air23/$name.done"
  else
    echo "!! $name measured under contention (median idle ${med}%; competing: ${who})"
    echo "   NOT marking done — re-run this driver when the box is free."
  fi
}

# Cheapest first (a driver that dies early banks something); the cycle's NEW
# boards lead so the sitting's re-entries exist even on a truncated day; the
# mco quartet runs late (one-instance-at-a-time makes them the long boards);
# 300s entry last. The two temporal boards carry the 60 s tier (header
# point 1). One entry per board: name track timeout [extra args].
BOARDS=(
  "ipc5-time          time-2006        30"
  "ipc5-metric-time   metric-time-2006 30"
  "ipc5-constraints   constraints-2006 60"
  "ipc2026-opt        opt-2026        60 --mode optimal"
  "ipc67-netben       net-benefit     60"
  "ipc5-prop          prop-2006       60"
  "ipc2023-agile      agile-2023      60"
  "ipc2014-tempo      tempo-sat-2014  60"
  "ipc2018-sat        sat-2018        60"
  "ipc2014-sat        seq-sat-2014    60"
  "ipc2014-agile      seq-agile-2014  60"
  "ipc2014-opt        seq-opt-2014    60 --mode optimal"
  "ipc2026-numeric    numeric-2026    60"
  "ipc2023-numeric    numeric-2023    60"
  "ipc-opt-2008-11    seq-opt         60 --mode optimal"
  "ipc67-temporal     tempo-sat       60"
  "ipc67-results      seq-sat         60"
  "ipc7-mco-t2        seq-mco         60 --threads 2"
  "ipc7-mco-t4        seq-mco         60 --threads 4"
  "ipc7-mco-t8        seq-mco         60 --threads 8"
  "ipc2014-mco-t4     seq-mco-2014    60 --threads 4"
  "ipc2023-agile-300s agile-2023      300"
)

remaining() {  # names of boards not yet banked clean
  local n out=""
  for spec in "${BOARDS[@]}"; do
    n=${spec%% *}
    [ -f "benchmarks/air23/$n.done" ] || out="$out $n"
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

echo "0.23 CUT SWEEP ALL DONE $(date '+%Y-%m-%d %H:%M:%S') (passes: $((pass - 1)))"
echo "next: the two old-binary-at-new-budget passes (header point 1) if not"
echo "      yet run, then benchmarks/promote-air23.sh"
