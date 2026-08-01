#!/usr/bin/env bash
# Promote the Air re-baseline boards from benchmarks/air/ into benchmarks/,
# then regenerate the standings table. Run ONLY after
# benchmarks/rebaseline-air.sh reports ALL DONE — a half-promoted set would
# mix an Air board with a cloud one under the same name, which is the exact
# thing the re-baseline exists to prevent.
#
# The cloud-era boards are NOT deleted, they are overwritten: git history is
# the record of the old box (docs/migration-m5.md step 4).
set -eu
cd "$(dirname "$0")/.."
AIR=benchmarks/air

need=(ipc2023-agile ipc2014-tempo ipc2018-sat ipc2014-sat ipc2014-agile
      ipc2014-opt ipc2026-numeric ipc2023-numeric ipc-opt-2008-11
      ipc67-temporal ipc67-results ipc2023-agile-300s)

missing=0
for b in "${need[@]}"; do
  [ -f "$AIR/$b.done" ] || { echo "NOT DONE: $b"; missing=1; }
done
[ "$missing" -eq 0 ] || { echo "refusing to promote a partial sweep"; exit 1; }

for b in "${need[@]}"; do
  # standings.py keys the flagship seq-sat board's RAW as ipc67-default.jsonl
  # while its scoreboard stays ipc67-results.md (see MD_FOR there). Everything
  # else keeps its name. Get this wrong and the board silently reads as
  # "sweep in flight" forever.
  raw="$b"
  [ "$b" = "ipc67-results" ] && raw="ipc67-default"
  cp "$AIR/$b.md"    "benchmarks/$b.md"
  cp "$AIR/$b.jsonl" "benchmarks/$raw.jsonl"
  printf '  promoted %-22s -> %s.md / %s.jsonl\n' "$b" "$b" "$raw"
done

echo
echo "regenerating standings..."
python3 benchmarks/standings.py
echo "done — review benchmarks/ipc-standings.md before committing"
