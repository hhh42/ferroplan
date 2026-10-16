#!/bin/sh
# Lane 5 mechanism 1, the first read: city-car i6 solo under the board's env
# on the engine with the rung memory wall (engine-0.30 after arm_rung), RSS
# timeline beside the narration. The question: does LAMA hand down at the
# trip line, and what does the best-first do with the wall it is handed?
echo $$ > run3.pid
FF=/Users/harold/ferroplan-0.30/target/release/ff
CC=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2014/domains/city-car-sequential-satisficing
tag=citycar6-key128
env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 FF_WALL_DEBUG=1 FF_RES_DEBUG=1 \
  $FF -o $CC/domain.pddl -f $CC/instances/instance-6.pddl --json --threads 1 > "$tag.json" 2> "$tag.err" &
pid=$!; t0=$(date +%s); : > "$tag.rss"
while kill -0 $pid 2>/dev/null; do
  printf '%s\t%s\n' "$(( $(date +%s) - t0 ))" "$(ps -o rss= -p $pid 2>/dev/null | tr -d ' ')" >> "$tag.rss"
  sleep 1
done
wait $pid
printf '%s\tpeak_rss_mb=%s\tsolved=%s\n' "$tag" "$(awk -F'\t' 'BEGIN{m=0} $2>m{m=$2} END{printf "%d", m/1024}' "$tag.rss")" \
  "$(python3 -c 'import json,sys
try: d=json.load(open(sys.argv[1])); print(d.get("solved"), d["statistics"]["evaluated_states"])
except Exception: print("?")' "$tag.json")" >> measure3.tsv
echo DONE >> measure3.tsv
