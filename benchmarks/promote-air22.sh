#!/usr/bin/env bash
# Promote the 0.22 cut boards from benchmarks/air22/ into benchmarks/, then
# regenerate the standings. Run ONLY after cut22-sweeps.sh reports ALL DONE —
# a half-promoted set would mix a 0.22 board with a 0.21 one under the same
# name, exactly what the promotion gate exists to prevent (promote-air.sh).
set -eu
cd "$(dirname "$0")/.."
AIR=benchmarks/air22

need=(ipc5-constraints ipc2026-opt ipc67-netben ipc5-prop ipc2023-agile ipc2014-tempo ipc2018-sat ipc2014-sat
      ipc2014-agile ipc2014-opt ipc2026-numeric ipc2023-numeric
      ipc-opt-2008-11 ipc67-temporal ipc67-results ipc2023-agile-300s)

missing=0
for b in "${need[@]}"; do
  [ -f "$AIR/$b.done" ] || { echo "NOT DONE: $b"; missing=1; }
done
[ "$missing" -eq 0 ] || { echo "refusing to promote a partial sweep"; exit 1; }

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
echo "done — review benchmarks/ipc-standings.md and STANDINGS.md, then"
echo "bank the snapshot: python3 scripts/standings-snapshot.py (see RELEASING.md)"
