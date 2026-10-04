#!/bin/bash
# Lane 2 probe (2026-10-04): the end-side-duration compile fix + the
# consumption charge, solo at 60 s, one thread, VAL-free (engine verdict),
# on the metric-time cells the lane is named for. Two arms: the build as it
# is, and FF_NO_CONSUME=1 (the compile fix alone).
cd "$(dirname "$0")"
FF=../../../target/release/ff
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2006/domains
echo -e "cell\tarm\tsolved\tmakespan\tnote" > probe.tsv
for cell in rovers-metric-time/14 rovers-metric-time/16 rovers-metric-time/18 rovers-metric-time/20 tpp-metric-time/10 tpp-metric-time/20 pathways-metric-time/10 pathways-metric-time/20; do
  v=${cell%/*}; i=${cell#*/}
  for arm in new blind; do
    if [ $arm = blind ]; then export FF_NO_CONSUME=1; else unset FF_NO_CONSUME; fi
    out=$(FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 perl -e 'alarm shift; exec @ARGV' 75 $FF -o $C/$v/domain.pddl -f $C/$v/instances/instance-$i.pddl --threads 1 --json 2>/dev/null)
    echo "$out" | python3 -c '
import sys,json
try:
    d=json.load(sys.stdin)
    print("%s\t%s\t%s\t%s\t%s" % (sys.argv[1], sys.argv[2], d.get("solved"), (d.get("plan") or {}).get("makespan"), (d.get("notes") or [""])[-1][:100]))
except Exception as e:
    print("%s\t%s\tERR\t\t%s" % (sys.argv[1], sys.argv[2], e))' "$cell" "$arm" >> probe.tsv
  done
done
unset FF_NO_CONSUME
# what the ladder does on rovers i20, which still stops at the wall
FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 FF_WALL_DEBUG=1 perl -e 'alarm shift; exec @ARGV' 75 $FF -o $C/rovers-metric-time/domain.pddl -f $C/rovers-metric-time/instances/instance-20.pddl --threads 1 2>&1 >/dev/null | grep -v "^\s*$" | grep -v TREL > rovers20-wall.log
echo PROBE DONE >> probe.tsv
