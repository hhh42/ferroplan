#!/usr/bin/env bash
# Promote the 0.24 cut boards from benchmarks/air24/ into benchmarks/, then
# regenerate the standings. Run ONLY after cut24-sweeps.sh reports ALL DONE —
# a half-promoted set would mix a 0.24 board with a 0.23 one under the same
# name, exactly what the promotion gate exists to prevent (promote-air.sh).
#
# No tier moves this cycle (unlike 0.23's temporal 30->60 flip) — the
# SWEEPS registry in standings.py already carries every board's 0.24
# budget correctly, since nothing changed from 0.23's tiers. The stamp
# gate below still runs: it is a general "what was measured matches what
# the registry will classify it as" check, not just a one-time flip
# mechanism, and it is what would catch a board accidentally swept at the
# wrong wall or a raw from a pre-stamp runner.
set -eu
cd "$(dirname "$0")/.."
AIR=benchmarks/air24

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
    path = os.path.join("benchmarks/air24", name + ".jsonl")
    with open(path) as f:
        stamp = json.loads(next(f)).get("budget")
    if stamp != reg[2]:
        bad.append(f"{name}: raw stamped {stamp}s, registry says {reg[2]}s")
if bad:
    print("\n".join("  STAMP MISMATCH " + b for b in bad))
    print("refusing to promote: fix the SWEEPS budget or re-sweep the")
    print("mismatched board at the registry budget.")
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
echo "done — review benchmarks/ipc-standings.md and STANDINGS.md, then bank"
echo "the snapshot: python3 scripts/standings-snapshot.py (see RELEASING.md)"
