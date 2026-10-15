#!/bin/sh
# Lane 5, city-car i6 per RUNG: which rung holds 5.8 GB? Three arms, each solo
# under the board's env, with an RSS timeline (ps every second) beside the
# FF_WALL_DEBUG narration so the growth can be read against the rung.
echo $$ > run2.pid
FF=/Users/harold/ferroplan/target/release/ff
CC=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2014/domains/city-car-sequential-satisficing
arm() { # tag env...
  tag=$1; shift
  env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 FF_WALL_DEBUG=1 FF_RES_DEBUG=1 "$@" \
    $FF -o $CC/domain.pddl -f $CC/instances/instance-6.pddl --json --threads 1 > "$tag.json" 2> "$tag.err" &
  pid=$!; t0=$(date +%s); : > "$tag.rss"
  while kill -0 $pid 2>/dev/null; do
    printf '%s\t%s\n' "$(( $(date +%s) - t0 ))" "$(ps -o rss= -p $pid 2>/dev/null | tr -d ' ')" >> "$tag.rss"
    sleep 1
  done
  wait $pid
  printf '%s\tpeak_rss_mb=%s\tevals=%s\n' "$tag" "$(awk -F'\t' 'BEGIN{m=0} $2>m{m=$2} END{printf "%d", m/1024}' "$tag.rss")" \
    "$(python3 -c 'import json,sys
try: print(json.load(open(sys.argv[1]))["statistics"]["evaluated_states"])
except Exception: print("?")' "$tag.json")" >> measure2.tsv
}
: > measure2.tsv
arm citycar6-bfs-only   FF_NO_NOVLIGHT=1 FF_NO_LAMA=1 FF_NO_NOVELTY=1
arm citycar6-lama-held  FF_NO_NOVLIGHT=1 FF_LAMA_WALL_FRAC=0.9
arm citycar6-nov-only   FF_NOVELTY_ONLY=1
echo DONE >> measure2.tsv
