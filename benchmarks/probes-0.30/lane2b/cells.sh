#!/bin/sh
# Lane 2b (docs/roadmap-0.30.md): the five cells the charge lost, solo at
# 60 s, three reps, on the 0.30 engine with the transfer rule and the Eq
# overshoot skip -- charged (the default) against FF_NO_CONSUME=1, plus
# driver-log i6 (the temporal loss that was not the charge's) as a control.
echo $$ > run.pid
FF=/Users/harold/ferroplan-0.30/target/release/ff
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus
OUT=rows.jsonl; : > $OUT
CELLS="ipc-2006|rovers-metric-time|28
ipc-2026n|settlers-snp-numeric-2026|5
ipc-2023n|sugar-numeric-satisficing|17
ipc-2026n|ztalloc-sum-numeric-2026|9
ipc-2026n|ztalloc-sum-numeric-2026|12
ipc-2014|driver-log-temporal-satisficing|6"
foreign() { ps -Ao pcpu,comm -r | awk 'NR>1 && $2 !~ /crucible|Validate|\/ff$|^ps$|^awk$/ { s += $1 } END { printf "%.0f", s }'; }
echo "$CELLS" | while IFS='|' read -r year dom inst; do
  d="$C/$year/domains/$dom"
  for hatch in "" "FF_NO_CONSUME=1"; do
    for rep in 1 2 3; do
      fl=$(foreign); t0=$(date +%s.%N)
      out=$(env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 ${hatch:+$hatch} perl -e 'alarm shift; exec @ARGV' 75 $FF -o "$d/domain.pddl" -f "$d/instances/instance-$inst.pddl" --json --threads 1 2>/dev/null)
      t1=$(date +%s.%N)
      read -r solved evals metric <<EOT
$(printf '%s' "$out" | python3 -c 'import sys,json
try:
    d=json.load(sys.stdin); s=d.get("statistics") or {}; p=d.get("plan") or {}
    print("true" if d.get("solved") else "false", s.get("evaluated_states","null"), p.get("cost", p.get("makespan","null")))
except Exception: print("\"crash\" null null")')
EOT
      printf '{"cell":"%s/%s","hatch":"%s","rep":%d,"solved":%s,"wall":%.1f,"evaluated":%s,"metric":%s,"foreign_pcpu":%s}\n' \
        "$dom" "$inst" "$hatch" "$rep" "$solved" "$(echo "$t1 - $t0" | bc)" "$evals" "$metric" "$fl" >> $OUT
    done
  done
done
echo '{"done":true}' >> $OUT
