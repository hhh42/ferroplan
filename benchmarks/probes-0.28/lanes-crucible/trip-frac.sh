#!/bin/sh
# trip-line experiment: at-risk cells x FF_MEM_TRIP_FRAC, true peak RSS by /usr/bin/time -l
S="/private/tmp/claude-501/-Users-harold-ferroplan/493e019b-1c41-419a-9f7b-f0f4bd1cdbf1/scratchpad"
FF=/Users/harold/ferroplan-0.28/target/release/ff
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus
OUT="$S/frac.tsv"; : > "$OUT"
for frac in 0.75 0.85; do
for cell in ipc-2008:elevator-temporal-satisficing-strips:30 ipc-2006:pipesworld-preferences-complex:16 ipc-2006:pipesworld-metric-time:41 ipc-2006:pipesworld-metric-time:26 ipc-2006:storage-preferences-qualitative:16 ipc-2006:storage-preferences-qualitative:18 ipc-2006:storage-preferences-qualitative:20 ipc-2006:storage-preferences-simple:19 ipc-2006:storage-preferences-simple:20 ipc-2006:openstacks-preferences-simple:13 ipc-2006:tpp-preferences-simple:20 ipc-2006:pathways-preferences-simple:29; do
  y=${cell%%:*}; r=${cell#*:}; v=${r%%:*}; i=${r##*:}
  cd "$C/$y/domains/$v" || continue
  env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 FF_MEM_TRIP_FRAC=$frac /usr/bin/time -l perl -e 'alarm shift; exec @ARGV' 75 $FF -o domain.pddl -f instances/instance-$i.pddl --json --threads 1 > "$S/frac.out" 2> "$S/frac.err"
  python3 - "$S/frac.out" "$S/frac.err" "$frac" "$v/$i" >> "$OUT" <<'PY'
import json,sys,re
e=open(sys.argv[2]).read()
try: d=json.load(open(sys.argv[1]))
except Exception: d={}
m=re.search(r'(\d+)\s+maximum resident',e); rss=int(m.group(1))/2**30 if m else -1
t=re.search(r'([\d.]+) real',e); p=d.get('plan') or {}
print(f"{sys.argv[3]}\t{sys.argv[4]}\t{d.get('solved')}\t{p.get('metric')}\t{p.get('makespan')}\t{p.get('length')}\t{t.group(1) if t else '?'}\t{rss:.2f}\t{(d.get('notes') or [''])[0][:60]}")
PY
done; done
echo done >> "$OUT"
