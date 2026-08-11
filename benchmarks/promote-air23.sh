#!/usr/bin/env bash
# Promote the 0.23 cut boards from benchmarks/air23/ into benchmarks/, then
# regenerate the standings. Run ONLY after cut23-sweeps.sh reports ALL DONE —
# a half-promoted set would mix a 0.23 board with a 0.22 one under the same
# name, exactly what the promotion gate exists to prevent (promote-air.sh).
#
# PROMOTE TIME IS TIER-FLIP TIME (0.23 Phase 3): the two temporal boards
# were swept at 60 s, and standings.py's SWEEPS registry carries their
# budgets at 30 until this moment (see the loud tier-move comment there —
# the committed 30 s raws classify by that registry value). The stamp gate
# below refuses to promote until the registry agrees with what was actually
# measured: every 0.23 raw carries a per-row `budget` stamp, and a
# 60 s-stamped tempo raw against a 30 registry entry is the flip not yet
# done. Flip both fields (ipc67-temporal.jsonl, ipc2014-tempo.jsonl) to 60,
# and update write_summary's "30 s temporal" budget prose in the same edit.
set -eu
cd "$(dirname "$0")/.."
AIR=benchmarks/air23

need=(ipc5-time ipc5-metric-time ipc5-constraints ipc2026-opt ipc67-netben
      ipc5-prop ipc2023-agile ipc2014-tempo ipc2018-sat ipc2014-sat
      ipc2014-agile ipc2014-opt ipc2026-numeric ipc2023-numeric
      ipc-opt-2008-11 ipc67-temporal ipc67-results
      ipc7-mco-t2 ipc7-mco-t4 ipc7-mco-t8 ipc2014-mco-t4
      ipc2023-agile-300s)

missing=0
for b in "${need[@]}"; do
  [ -f "$AIR/$b.done" ] || { echo "NOT DONE: $b"; missing=1; }
done
[ "$missing" -eq 0 ] || { echo "refusing to promote a partial sweep"; exit 1; }

# THE STAMP GATE: every staged raw's per-row budget stamp must match the
# SWEEPS registry entry it will be classified under. One check enforces
# three honesty rules at once: the tier flip above (a 60 s tempo raw
# refuses a 30 registry), a board accidentally swept at the wrong wall
# (the raw says what it ran under — re-sweep it or fix the driver), and a
# raw produced by a pre-stamp runner (no `budget` key: not a 0.23 sweep).
echo "checking budget stamps against the SWEEPS registry..."
python3 - "${need[@]}" <<'EOF'
import json, os, sys
sys.path.insert(0, "benchmarks")
import standings
bad = []
for name in sys.argv[1:]:
    raw = "ipc67-default" if name == "ipc67-results" else name
    reg = standings.SWEEPS.get(raw + ".jsonl")
    if reg is None:
        bad.append(f"{name}: no SWEEPS registry entry for {raw}.jsonl")
        continue
    path = os.path.join("benchmarks/air23", name + ".jsonl")
    with open(path) as f:
        stamp = json.loads(next(f)).get("budget")
    if stamp != reg[2]:
        bad.append(f"{name}: raw stamped {stamp}s, registry says {reg[2]}s")
if bad:
    print("\n".join("  STAMP MISMATCH " + b for b in bad))
    print("refusing to promote: flip the SWEEPS budgets to match what was")
    print("measured (the tier-move comment in standings.py), or re-sweep")
    print("the mismatched board at the registry budget.")
    sys.exit(1)
print(f"  all {len(sys.argv) - 1} raws' budget stamps match the registry")
EOF

for b in "${need[@]}"; do
  # standings.py keys the flagship seq-sat board's RAW as ipc67-default.jsonl
  # while its scoreboard stays ipc67-results.md (see MD_FOR there).
  raw="$b"
  [ "$b" = "ipc67-results" ] && raw="ipc67-default"
  cp "$AIR/$b.md"    "benchmarks/$b.md"
  cp "$AIR/$b.jsonl" "benchmarks/$raw.jsonl"
  printf '  promoted %-22s -> %s.md / %s.jsonl\n' "$b" "$b" "$raw"
done

echo
echo "regenerating standings..."
python3 benchmarks/standings.py
echo "done — review benchmarks/ipc-standings.md and STANDINGS.md (the time/"
echo "metric-time rows should now carry the makespan quality column), then"
echo "bank the snapshot: python3 scripts/standings-snapshot.py (see RELEASING.md)"
