#!/bin/sh
# Phase 0.1 (docs/roadmap-0.30.md): the twelve cells 0.28.0 solved and 0.29.0
# never did (cut29's three-engine table, `probes-0.29/cut29/lost-cells-three-
# engines.txt`), each run solo at its board's wall, three reps, on three arms:
# 0.29.0 as shipped, 0.29.0 under FF_NO_CONSUME=1 (the consumption charge off),
# and the rebuilt 0.28.0 (bb07d50d2015). A probe, not a board number: the same
# env the crucible exports (FF_TIME_LIMIT, FF_MEM_BUDGET_GB), the board's
# thread count, 60 s. Pre-registered fork in the roadmap: the hatch restores a
# cell -> the charge has a defect on that shape (a Lane 2b fixture); it does
# not and 0.28.0 reproduces -> the duration compile / achiever index are next;
# nothing restores it -> the box, recorded.
#
# The foreign load at each run's start is recorded beside the row (`ps`
# named-competitor total, the crucible's sample), since the box is not
# guaranteed idle while the operator is away.
set -u
echo $$ > "$(dirname "$0")/run.pid"   # stop it by THIS pid, never by a pattern
NEW=/Users/harold/ferroplan/target/release/ff                      # ff 0.29.0 [7fdf185b91e6]
OLD=/Users/harold/.crucible/worktrees/v0.28.0/target/release/ff    # ff 0.28.0 [bb07d50d2015], rebuilt
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus
OUT=/Users/harold/ferroplan-0.30/benchmarks/probes-0.30/losses29/rows.jsonl
: > "$OUT"
# cell: ipc-dir | domain-dir | instance | threads
# rovers-metric-time i28 was read in the first launch (rows-part1.jsonl, 12:49):
# 0.29.0 0/3, FF_NO_CONSUME=1 3/3 at 24.4 s, 0.28.0 3/3 at 25.8 s.
CELLS="ipc-2006|pipesworld-metric-time|36|1
ipc-2026n|settlers-snp-numeric-2026|5|1
ipc-2026n|ztalloc-sum-numeric-2026|9|1
ipc-2026n|ztalloc-sum-numeric-2026|12|1
ipc-2023n|sugar-numeric-satisficing|17|1
ipc-2014|driver-log-temporal-satisficing|6|1
ipc-2011|elevator-sequential-multi-core|19|2
ipc-2011|elevator-sequential-multi-core|20|2
ipc-2011|transport-sequential-multi-core|18|2
ipc-2011|elevator-sequential-multi-core|19|4
ipc-2011|transport-sequential-multi-core|18|4"
ARMS="0.29.0|
0.29.0|FF_NO_CONSUME=1
0.28.0|"
foreign() {
  ps -Ao pcpu,comm -r | awk 'NR>1 && $2 !~ /crucible|Validate|\/ff$|^ps$|^awk$/ { s += $1 } END { printf "%.0f", s }'
}
echo "$CELLS" | while IFS='|' read -r year dom inst thr; do
  d="$C/$year/domains/$dom"; dfile="$d/domain.pddl"
  echo "$ARMS" | while IFS='|' read -r ver hatch; do
    bin=$NEW; [ "$ver" = "0.28.0" ] && bin=$OLD
    for rep in 1 2 3; do
      fl=$(foreign)
      t0=$(date +%s.%N)
      out=$(env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 ${hatch:+$hatch} perl -e 'alarm shift; exec @ARGV' 75 $bin -o "$dfile" -f "$d/instances/instance-$inst.pddl" --json --threads "$thr" 2>/dev/null)
      t1=$(date +%s.%N)
      read -r solved evals metric <<EOF
$(printf '%s' "$out" | python3 -c 'import sys,json
try:
    d=json.load(sys.stdin); s=d.get("statistics") or {}
    p=d.get("plan") or {}; print("true" if d.get("solved") else "false", s.get("evaluated_states","null"), p.get("cost", p.get("makespan","null")))
except Exception: print("\"crash\" null null")')
EOF
      printf '{"cell":"%s/%s","threads":%s,"arm":"%s","hatch":"%s","rep":%d,"solved":%s,"wall":%.1f,"evaluated":%s,"metric":%s,"foreign_pcpu":%s}\n' \
        "$dom" "$inst" "$thr" "$ver" "$hatch" "$rep" "$solved" "$(echo "$t1 - $t0" | bc)" "$evals" "$metric" "$fl" >> "$OUT"
    done
  done
done
echo '{"done":true}' >> "$OUT"
