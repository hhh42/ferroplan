#!/bin/sh
# Lane 6 (docs/roadmap-0.30.md): every cell the 0.29.0 board left unsolved on
# IPC-5 propositional, solo at 60 s under --mode partition (the SGPlan-style
# partition-and-resolve route), on the 0.30 engine. pathways i17 solved this
# way at 35 s where the auto ladder never does. Record only.
cd "$(dirname "$0")"; echo $$ > run.pid
FF=/Users/harold/ferroplan-0.30/target/release/ff
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2006/domains
printf 'cell\tsolved\tevals\tlen\tsecs\tforeign_pcpu\n' > partition.tsv
while IFS='|' read -r v i; do
  dom=$C/$v/domain.pddl; [ -f "$dom" ] || dom=$C/$v/domains/domain-$i.pddl
  fl=$(ps -Ao pcpu,comm -r | awk 'NR>1 && $2 !~ /crucible|Validate|\/ff$|^ps$|^awk$/ { s += $1 } END { printf "%.0f", s }')
  t0=$(date +%s)
  FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 perl -e 'alarm shift; exec @ARGV' 75 $FF -o "$dom" -f "$C/$v/instances/instance-$i.pddl" --json --threads 1 --mode partition 2>/dev/null | python3 -c '
import sys,json
cell,t0,fl=sys.argv[1:]
import time
try:
    d=json.load(sys.stdin); s=d.get("statistics") or {}; p=d.get("plan") or {}
    print("%s\t%s\t%s\t%s\t%d\t%s" % (cell, d.get("solved"), s.get("evaluated_states"), p.get("length"), time.time()-float(t0), fl))
except Exception:
    print("%s\tno-json\t\t\t%d\t%s" % (cell, time.time()-float(t0), fl))' "$v/$i" "$t0" "$fl" >> partition.tsv
done < unsolved-prop.txt
echo "PROBE DONE" >> partition.tsv
