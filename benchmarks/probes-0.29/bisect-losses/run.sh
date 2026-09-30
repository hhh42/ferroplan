#!/bin/sh
# Phase 0.3 (docs/roadmap-0.29.md): the five REAL 0.28.0 losses, each run solo
# at its board's wall on 0.28.0 under every lane hatch in turn, and on v0.27.1,
# three reps each. A probe, not a board number: solo, quiet box, the same env
# the crucible exports (FF_TIME_LIMIT, FF_MEM_BUDGET_GB), one thread, 60 s.
# Pre-registered fork in the roadmap: one hatch restores the numeric four ->
# that lane has a numeric defect; none does -> Lane 2 inherits them as
# fixtures; the box restores them -> recorded, not a loss.
set -u
NEW=/Users/harold/ferroplan/target/release/ff          # ff 0.28.0 [89cfdc5f06ed], the cut's binary
OLD=/Users/harold/.crucible/worktrees/v0.27.1/target/release/ff
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus
OUT=/Users/harold/ferroplan-0.29/benchmarks/probes-0.29/bisect-losses/rows.jsonl
: > "$OUT"
# cell: year domain-dir instance-file extra-args
CELLS="ipc-2014|tetris-sequential-satisficing|instance-15.pddl|
ipc-2023n|sugar-numeric-satisficing|instance-5.pddl|
ipc-2026n|coins-numeric-2026|instance-18.pddl|
ipc-2026n|line-exchange-snp-numeric-2026|instance-3_5_90_100.pddl|
ipc-2026n|sailing-wind-opt-numeric-2026|instance-10.pddl|--mode optimal"
ARMS="0.28.0|
0.28.0|FF_NO_TCOMPRESS=1
0.28.0|FF_PREF_NO_SEED=1
0.28.0|FF_NO_NEED_DIRS=1
0.28.0|FF_NO_MEM_WALL=1
0.28.0|FF_NO_TSUCC=1
0.27.1|"
echo "$CELLS" | while IFS='|' read -r year dom inst args; do
  d="$C/$year/domains/$dom"; dfile="$d/domain.pddl"; [ -f "$d/domains/${inst%.pddl}-domain.pddl" ] && dfile="$d/domains/${inst%.pddl}-domain.pddl"
  echo "$ARMS" | while IFS='|' read -r ver hatch; do
    bin=$NEW; [ "$ver" = "0.27.1" ] && bin=$OLD
    for rep in 1 2 3; do
      t0=$(date +%s.%N)
      out=$(env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 ${hatch:+$hatch} perl -e 'alarm shift; exec @ARGV' 75 $bin -o "$dfile" -f "$d/instances/$inst" --json --threads 1 $args 2>/dev/null)
      t1=$(date +%s.%N)
      solved=$(printf '%s' "$out" | python3 -c 'import sys,json
try: d=json.load(sys.stdin); print("true" if d.get("solved") else "false")
except Exception: print("crash")')
      printf '{"cell":"%s/%s","arm":"%s","hatch":"%s","rep":%d,"solved":%s,"wall":%.1f}\n' "$dom" "${inst#instance-}" "$ver" "$hatch" "$rep" "$solved" "$(echo "$t1 - $t0" | bc)" >> "$OUT"
    done
  done
done
echo done >> "$OUT"
