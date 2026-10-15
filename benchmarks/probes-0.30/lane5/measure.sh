#!/bin/sh
# Lane 5 day one, re-done (docs/roadmap-0.30.md): where the memory goes, on
# the real paths this time (0.29's probe named folding-sequential-satisficing,
# which does not exist). Each cell solo, one thread, under the BOARD's env
# (FF_TIME_LIMIT=60, FF_MEM_BUDGET_GB=6) with FF_WALL_DEBUG / FF_RES_DEBUG
# narrating which rung holds the memory, plus a cap-1 run for the grounding's
# own footprint. Peak RSS from /usr/bin/time -l; the stderr is the receipt.
echo $$ > run.pid
FF=/Users/harold/ferroplan/target/release/ff
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus
run() { # tag dom prb extra...
  tag=$1; dom=$2; prb=$3; shift 3
  /usr/bin/time -l perl -e 'alarm shift; exec @ARGV' 80 env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 FF_WALL_DEBUG=1 FF_RES_DEBUG=1 \
    $FF -o "$dom" -f "$prb" --json --threads 1 "$@" > "$tag.json" 2> "$tag.err"
  rss=$(grep "maximum resident set size" "$tag.err" | awk '{print $1}')
  printf '%s\tpeak_rss_mb=%d\tsolved=%s\n' "$tag" "$((${rss:-0}/1048576))" "$(python3 -c 'import json,sys
try: print(json.load(open(sys.argv[1])).get("solved"))
except Exception: print("no-json")' "$tag.json")" >> measure.tsv
}
: > measure.tsv
F=$C/ipc-2023/domains/folding-satisficing
CC=$C/ipc-2014/domains/city-car-sequential-satisficing
run folding3-cap1   $F/domain.pddl  $F/instances/instance-3.pddl  --max-evaluated 1
run folding3-board  $F/domain.pddl  $F/instances/instance-3.pddl
run citycar6-cap1   $CC/domain.pddl $CC/instances/instance-6.pddl --max-evaluated 1
run citycar6-board  $CC/domain.pddl $CC/instances/instance-6.pddl
echo DONE >> measure.tsv
