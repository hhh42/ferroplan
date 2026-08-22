#!/usr/bin/env bash
# THE 0.25 FINISH-OUT RUNNER — everything that needed the box and had to
# queue behind the entries sweep (docs/roadmap-0.25.md Phases 0/2/4).
# Run AFTER entries25-sweeps.sh reports ALL DONE (it refuses to start
# early). Each item writes its receipts under benchmarks/air25-entries/;
# nothing here mutates tracked files — read the receipts, then record.
#
#   1. The complex-preferences board (ipc5-complex-pref, 108 rows) — the
#      Phase 2 entry's first real measurement, staged exactly like the
#      other entries (contention watcher, refuse-not-bank).
#   2. The onlycraft contended re-check (Phase 0): the cut's
#      unattributed +34, re-run SOLO and then under DELIBERATE load
#      (4 spinner processes) on the six docket rows (sat i4-i6,
#      opt i4-i6). If the solves survive load, the gain is real; if
#      they die, the fragility goes on the record.
#   3. The model-train encoder probe (Phase 4): the anti-pot's own exit
#      clause — Mode::Sat on i1/i2, capture the encoder's named decline
#      (or its price). Probe, don't build.
#   4. The tground_wall idle re-verify: the 1 s-wall child that reads as
#      a contention phantom under sweep load must pass on the idle box
#      before any cut.
set -u
cd "$(dirname "$0")/.."
ENT=benchmarks/air25-entries
mkdir -p "$ENT"

for b in ipc2014-mco-t2 ipc2014-mco-t8 ipc2018-opt ipc2023-sat ipc2023-opt \
         ipc2023-numeric-opt ipc2026-opt-full ipc5-simple-pref ipc5-qual-pref; do
  [ -f "$ENT/$b.done" ] || {
    echo "REFUSING: entries sweep not complete ($b not banked) — run entries25-sweeps.sh first"
    exit 1
  }
done

cargo build --release -p ferroplan-cli 2>&1 | tail -1
V="$(./target/release/ff --version 2>/dev/null || true)"
case "$V" in *0.25*) : ;; *) echo "REFUSING: binary is not 0.25"; exit 1 ;; esac

idle_pct() {
  top -l 2 -n 0 -s 1 2>/dev/null | grep '^CPU usage' | tail -1 \
    | awk -F'[ ,%]+' '{for (i = 1; i <= NF; i++) if ($i == "idle") print int($(i - 1))}'
}
wait_quiet() {
  local got=0 idle
  while [ "$got" -lt 2 ]; do
    idle=$(idle_pct); [ -z "$idle" ] && idle=0
    if [ "$idle" -ge 70 ]; then got=$((got + 1)); else got=0; sleep 60; fi
  done
}

# ---- 1. the complex-preferences board --------------------------------------
if [ ! -f "$ENT/ipc5-complex-pref.done" ]; then
  wait_quiet
  echo "RUN  ipc5-complex-pref (complex-pref-2006 60s) $(date '+%H:%M:%S')"
  cond="$ENT/ipc5-complex-pref.conditions.json"
  python3 benchmarks/contention.py --out "$cond" --interval 20 &
  watcher=$!
  python3 benchmarks/ipc67.py --track complex-pref-2006 --timeout 60 \
    --jobs 2 --mem-gb 6 --out "$ENT/ipc5-complex-pref.md" \
    >"$ENT/ipc5-complex-pref.log" 2>&1
  kill "$watcher" 2>/dev/null; wait "$watcher" 2>/dev/null
  verdict=$(python3 -c "import json;print(json.load(open('$cond'))['verdict'])" 2>/dev/null || echo unknown)
  echo "DONE ipc5-complex-pref: $(tail -1 "$ENT/ipc5-complex-pref.md") [$verdict]"
  [ "$verdict" = "clean" ] && touch "$ENT/ipc5-complex-pref.done" \
    || echo "!! contended — re-run this script when the box is free"
fi

# ---- 2. the onlycraft contended re-check -----------------------------------
OC=benchmarks/.ipc-corpus/ipc-2026n/domains
RC=$ENT/onlycraft-recheck
mkdir -p "$RC"
run_oc() { # variant instance mode tag
  local d="$OC/onlycraft-$1-numeric-2026"
  local extra=""
  [ "$3" = "optimal" ] && extra="--mode optimal"
  # shellcheck disable=SC2086
  FF_TIME_LIMIT=60 /usr/bin/time -o "$RC/$4.time" \
    ./target/release/ff -o "$d/domain.pddl" -f "$d/instances/instance-$2.pddl" \
    --json $extra >"$RC/$4.json" 2>/dev/null
  python3 -c "import json;d=json.load(open('$RC/$4.json'));print('  $4: solved', d['solved'])"
}
echo "onlycraft re-check, SOLO leg $(date '+%H:%M:%S')"
wait_quiet
for i in 4 5 6; do run_oc sat "$i" auto "solo-sat-i$i"; run_oc opt "$i" optimal "solo-opt-i$i"; done
echo "onlycraft re-check, CONTENDED leg (4 spinners)"
spinners=()
for _ in 1 2 3 4; do yes >/dev/null & spinners+=($!); done
for i in 4 5 6; do run_oc sat "$i" auto "load-sat-i$i"; run_oc opt "$i" optimal "load-opt-i$i"; done
kill "${spinners[@]}" 2>/dev/null
echo "receipts in $RC/ — solo vs load, six rows each"

# ---- 3. the model-train encoder probe --------------------------------------
MT=benchmarks/.ipc-corpus/ipc-2008/domains/model-train-temporal-satisficing-numeric-fluents
if [ -d "$MT" ]; then
  for i in 1 2; do
    FF_TIME_LIMIT=60 FF_WALL_DEBUG=1 ./target/release/ff -o "$MT/domain.pddl" \
      -f "$MT/instances/instance-$i.pddl" --mode sat --json \
      >"$ENT/model-train-probe-i$i.json" 2>"$ENT/model-train-probe-i$i.log" || true
    echo "model-train i$i probe: $(grep -o 'declined[^\"]*' "$ENT/model-train-probe-i$i.json" | head -1 || echo 'see json/log')"
  done
else
  echo "model-train dir not found at $MT — check the corpus layout"
fi

# ---- 3b. the Phase 4 sitting's named probes --------------------------------
# Transport L3 (rung budget re-slice): if 2011-sat i4 drops well under its
# 59.6 s solve with novelty-light and LAMA hatched off, their wall slices
# buy nothing on this family and the reclaimed ~20 s prices the re-slice.
TR=benchmarks/.ipc-corpus/ipc-2011/domains/transport-sequential-satisficing
for i in 4 6; do
  FF_TIME_LIMIT=60 /usr/bin/time -o "$ENT/transport-L3-i$i.time" \
    env FF_NO_NOVLIGHT=1 FF_NO_LAMA=1 ./target/release/ff \
    -o "$TR/domain.pddl" -f "$TR/instances/instance-$i.pddl" --json \
    >"$ENT/transport-L3-i$i.json" 2>/dev/null || true
  echo "  transport-L3 i$i: solved=$(python3 -c "import json;print(json.load(open('$ENT/transport-L3-i$i.json'))['solved'])" 2>/dev/null) $(head -1 "$ENT/transport-L3-i$i.time" | awk '{print $1}')s"
done
# Metric-time P4 (the empty-constraints riddle): tpp-metric-time-constraints
# i1 carries an EMPTY (:constraints (and)) block yet its variant scores
# 0/30 beside tpp-metric-time's 3/40 — does the empty block disarm
# something on its way through the PDDL3 wing?
TC=benchmarks/.ipc-corpus/ipc-2006/domains/tpp-metric-time-constraints
FF_TIME_LIMIT=60 /usr/bin/time -o "$ENT/tpp-emptyc.time" ./target/release/ff \
  -o "$TC/domain.pddl" -f "$TC/instances/instance-1.pddl" --json \
  >"$ENT/tpp-emptyc.json" 2>/dev/null || true
echo "  tpp-empty-constraints i1: solved=$(python3 -c "import json;print(json.load(open('$ENT/tpp-emptyc.json'))['solved'])" 2>/dev/null) $(head -1 "$ENT/tpp-emptyc.time" | awk '{print $1}')s"
# Pathways at the board budget with the dur-0 fix + sum-goal mask: how many
# of the thirty false instant failures become real solves.
PW=benchmarks/.ipc-corpus/ipc-2006/domains/pathways-metric-time
pwn=0
for i in $(seq 1 30); do
  FF_TIME_LIMIT=60 ./target/release/ff -o "$PW/domain.pddl" \
    -f "$PW/instances/instance-$i.pddl" --json >"$ENT/pw-i$i.json" 2>/dev/null || true
  s=$(python3 -c "import json;print(json.load(open('$ENT/pw-i$i.json'))['solved'])" 2>/dev/null)
  [ "$s" = "True" ] && pwn=$((pwn+1))
done
echo "  pathways-metric-time with the dur-0 fix: $pwn/30"

# Parking counted-case baselines (the 0.24 read's body was never
# committed — the re-derivation starts from receipts): parking-opt
# i2-i4 solo at the board budget, expansion counts in the JSON notes.
PK=benchmarks/.ipc-corpus/ipc-2014/domains/parking-sequential-optimal
for i in 2 3 4; do
  FF_TIME_LIMIT=60 /usr/bin/time -o "$ENT/parking-opt-i$i.time" \
    ./target/release/ff -o "$PK/domain.pddl" -f "$PK/instances/instance-$i.pddl" \
    --mode optimal --json >"$ENT/parking-opt-i$i.json" 2>/dev/null || true
  echo "  parking-opt i$i: solved=$(python3 -c "import json;print(json.load(open('$ENT/parking-opt-i$i.json'))['solved'])" 2>/dev/null) $(head -1 "$ENT/parking-opt-i$i.time" | awk '{print $1}')s"
done

# ---- 4. tground_wall at idle -----------------------------------------------
wait_quiet
echo "tground_wall idle re-verify..."
cargo test -q -p ferroplan --release --test tground_wall 2>&1 | tail -2

echo
echo "0.25 FINISH-OUT RUNNER DONE $(date '+%Y-%m-%d %H:%M:%S')"
echo "next: read the receipts, record the verdicts in roadmap-0.25.md"
echo "      (onlycraft docket, model-train anti-pot, complex-prefs board),"
echo "      then benchmarks/cut25-sweeps.sh when the cycle's engine work"
echo "      is complete, then promote-air25.sh."
