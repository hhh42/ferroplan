#!/usr/bin/env bash
# Promote the 0.25 boards into benchmarks/, then regenerate the standings.
# TWO staging dirs this cycle, by design (docs/roadmap-0.25.md Phase 1):
#   benchmarks/air25/          — the standing 22, from cut25-sweeps.sh
#   benchmarks/air25-entries/  — the 10 NEW boards, from entries25-sweeps.sh
#     (+ ipc5-complex-pref, swept solo after the Phase 2 entry landed)
# Run ONLY after BOTH drivers report ALL DONE — a half-promoted set mixes
# cycles under the same name, exactly what this gate exists to prevent.
#
# THE TIER MOVE (0.25 Phase 5): ipc5-time and ipc5-metric-time swept at
# 60 s this cycle; their SWEEPS registry fields still read 30 from the
# 0.23-era raws. This script flips them (idempotently) BEFORE the stamp
# gate, so the gate then proves the flip against the raws' own stamps —
# the 0.23 mechanism, finished. The cut record must carry the budget
# change in those two boards' movement column: +rows at 60 s is budget
# plus engine, never engine alone.
set -eu
cd "$(dirname "$0")/.."
AIR=benchmarks/air25
ENT=benchmarks/air25-entries

cut=(ipc5-time ipc5-metric-time ipc5-constraints ipc2026-opt ipc67-netben
     ipc5-prop ipc2023-agile ipc2014-tempo ipc2018-sat ipc2014-sat
     ipc2014-agile ipc2014-opt ipc2026-numeric ipc2023-numeric
     ipc-opt-2008-11 ipc67-temporal ipc67-results
     ipc7-mco-t2 ipc7-mco-t4 ipc7-mco-t8 ipc2014-mco-t4
     ipc2023-agile-300s)
entries=(ipc2014-mco-t2 ipc2014-mco-t8 ipc2018-opt ipc2023-sat ipc2023-opt
         ipc2023-numeric-opt ipc2026-opt-full ipc5-simple-pref
         ipc5-qual-pref ipc5-complex-pref)

missing=0
for b in "${cut[@]}"; do
  [ -f "$AIR/$b.done" ] || { echo "NOT DONE (cut): $b"; missing=1; }
done
for b in "${entries[@]}"; do
  [ -f "$ENT/$b.done" ] || { echo "NOT DONE (entry): $b"; missing=1; }
done
[ "$missing" -eq 0 ] || { echo "refusing to promote a partial sweep"; exit 1; }

echo "tier move: flipping ipc5-time / ipc5-metric-time to 60 in the SWEEPS registry (idempotent)..."
python3 - <<'EOF'
import re
p = "benchmarks/standings.py"
s = open(p).read()
s = s.replace('"ipc5-time.jsonl": ("time", "ipc5", 30),',
              '"ipc5-time.jsonl": ("time", "ipc5", 60),')
s = s.replace('"ipc5-metric-time.jsonl": ("metric-time", "ipc5", 30),',
              '"ipc5-metric-time.jsonl": ("metric-time", "ipc5", 60),')
open(p, "w").write(s)
EOF

echo "checking budget stamps against the SWEEPS registry..."
python3 - "${cut[@]}" "${entries[@]}" <<'EOF'
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
    for d in ("benchmarks/air25", "benchmarks/air25-entries"):
        path = os.path.join(d, name + ".jsonl")
        if os.path.exists(path):
            break
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

promote() { # dir name
  local dir="$1" b="$2" raw="$2"
  [ "$b" = "ipc67-results" ] && raw="ipc67-default"
  cp "$dir/$b.md"    "benchmarks/$b.md"
  cp "$dir/$b.jsonl" "benchmarks/$raw.jsonl"
  printf '  promoted %-22s -> %s.md / %s.jsonl\n' "$b" "$b" "$raw"
}
for b in "${cut[@]}"; do promote "$AIR" "$b"; done
for b in "${entries[@]}"; do promote "$ENT" "$b"; done

echo
echo "regenerating standings..."
python3 benchmarks/standings.py
echo "done — review benchmarks/ipc-standings.md and STANDINGS.md. The cut"
echo "record carries TWO headlines: the like-for-like 22 (tier move named"
echo "on ipc5-time/metric-time) AND the grown table with the ten entries."
echo "Then bank the snapshot: python3 scripts/standings-snapshot.py"
