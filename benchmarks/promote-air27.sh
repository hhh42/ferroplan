#!/usr/bin/env bash
# Promote the 0.27 boards into benchmarks/, then regenerate the standings.
# ONE staging dir: benchmarks/air27/ — the 32-board `cut27` set, swept by
# crucible R2's packed scheduler (docs/roadmap-0.27.md). The 0.26 promoted
# raws are the referee; the regression re-check (every instance 0.26 solved
# that 0.27 did not, re-run solo on a quiet box) runs BEFORE this script.
#
# Run ONLY after `benchmarks/cut27-sweep.log` says SWEEP COMPLETE — crucible
# writes a board's .done marker only when nothing on it is still owed, and a
# half-promoted set mixes cycles under the same name, exactly what this gate
# exists to prevent.
#
# No tier move this cycle: the manifest carries no warnings and the 0.25
# registry flips (ipc5-time / ipc5-metric-time at 60 s) already stand. The
# stamp gate below still proves every raw's budget against the registry.
set -eu
cd "$(dirname "$0")/.."
AIR=benchmarks/air27

boards=(ipc5-prop ipc5-time ipc5-metric-time ipc5-constraints
        ipc67-results ipc67-temporal ipc67-netben
        ipc7-mco-t2 ipc7-mco-t4 ipc7-mco-t8
        ipc2014-sat ipc2014-agile ipc2014-tempo ipc2014-mco-t4
        ipc2018-sat ipc2023-agile ipc2023-numeric ipc2026-numeric
        ipc2023-agile-300s ipc-opt-2008-11 ipc2014-opt ipc2026-opt
        ipc2014-mco-t2 ipc2014-mco-t8 ipc2018-opt ipc2023-sat ipc2023-opt
        ipc2023-numeric-opt ipc2026-opt-full
        ipc5-simple-pref ipc5-qual-pref ipc5-complex-pref)

# --accept-owed: promote a sweep that was STOPPED with rows still owed (the
# 0.26 cut did this, by decision, after five days under the R1 referee).
# Under R2 the expected shape is SWEEP COMPLETE; the flag is kept for the
# same situation, and the cut record names every owed row it accepts.
ACCEPT_OWED=0; [ "${1:-}" = "--accept-owed" ] && ACCEPT_OWED=1
missing=0
for b in "${boards[@]}"; do
  [ -f "$AIR/$b.done" ] || { echo "NOT DONE: $b"; missing=1; }
done
if [ "$missing" -ne 0 ] || ! grep -q 'SWEEP COMPLETE' benchmarks/cut27-sweep.log; then
  if [ "$ACCEPT_OWED" -eq 1 ]; then
    echo "promoting a STOPPED sweep with rows owed (--accept-owed)"
  else
    echo "refusing to promote a partial sweep (pass --accept-owed to override)"; exit 1
  fi
fi
for b in "${boards[@]}"; do
  n=$(grep -c '"solved"' "$AIR/$b.jsonl" || true)
  [ "$n" -gt 0 ] || { echo "EMPTY raw: $b"; exit 1; }
done

echo "checking budget stamps against the SWEEPS registry..."
python3 - "${boards[@]}" <<'PY'
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
    path = os.path.join("benchmarks/air27", name + ".jsonl")
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
PY

promote() { # name
  local b="$1" raw="$1"
  [ "$b" = "ipc67-results" ] && raw="ipc67-default"
  cp "$AIR/$b.md"    "benchmarks/$b.md"
  cp "$AIR/$b.jsonl" "benchmarks/$raw.jsonl"
  printf '  promoted %-22s -> %s.md / %s.jsonl\n' "$b" "$b" "$raw"
}
for b in "${boards[@]}"; do promote "$b"; done

echo
echo "regenerating standings (the Python oracle writes; crucible must agree)..."
python3 benchmarks/standings.py
# crucible moved to its own worktree during R2 (ferroplan-r2/crucible), so
# the in-tree path stopped existing and this gate silently degraded: a
# missing binary reported MISMATCH, which reads exactly like the numbers
# disagreeing. Found at the 0.27 cut. Look in both places, and REFUSE to
# cut if neither has one -- an unrunnable check must never pass for having
# nothing to run.
CRUCIBLE=""
for c in ./crucible/target/release/crucible \
         ../ferroplan-r2/crucible/target/release/crucible; do
  [ -x "$c" ] && { CRUCIBLE="$c"; break; }
done
if [ -z "$CRUCIBLE" ]; then
  echo "  no crucible binary: build it, or the parity gate is not a gate"; exit 1
fi
"$CRUCIBLE" --repo . standings --check \
  && echo "  crucible standings --check: parity" \
  || { echo "  crucible standings --check: MISMATCH — do not cut on this"; exit 1; }

echo
echo "done — review benchmarks/ipc-standings.md and STANDINGS.md, then:"
echo "  python3 scripts/standings-snapshot.py --version 0.27.0 --measured-at YYYY-MM-DD"
echo "  CHANGELOG.md [0.27.0] + README 'What's new in 0.27.0' + Status line"
echo "  python3 scripts/release-notes-roll.py   (refuses if the README lags)"
