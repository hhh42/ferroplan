#!/bin/bash
# Lane 5 day one (2026-10-04): where the memory goes on the classical
# mem-cap rows. Each cell solo, one thread, under a NODE cap instead of a
# memory cap (a solo run with no runner watchdog would take the box), with
# the peak RSS from `/usr/bin/time -l` at two caps: the grounding's own
# footprint (cap 1) and the search's growth (cap 200k) -> bytes per node.
cd "$(dirname "$0")"
FF=../../../target/release/ff
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus
echo -e "cell\tcap\tevaluated\tpeak_rss_mb\tsecs\tnote" > measure.tsv
run() { # cell dir domain problem cap
  local cell=$1 dom=$2 prb=$3 cap=$4
  local t0=$(date +%s.%N)
  /usr/bin/time -l perl -e 'alarm shift; exec @ARGV' 120 $FF -o $dom -f $prb --threads 1 --max-evaluated $cap --json > out.json 2> time.err
  local t1=$(date +%s.%N)
  local rss=$(grep "maximum resident set size" time.err | awk '{print $1}')
  python3 - "$cell" "$cap" "$rss" "$t0" "$t1" <<'PY' >> measure.tsv
import sys,json
cell,cap,rss,t0,t1=sys.argv[1:]
try:
    d=json.load(open("out.json")); ev=d.get("evaluated"); note=(d.get("notes") or [""])[-1][:70]
except Exception as e:
    ev=None; note="no json"
print("%s\t%s\t%s\t%.0f\t%.1f\t%s" % (cell,cap,ev,int(rss or 0)/1048576,float(t1)-float(t0),note))
PY
}
cells="ipc-2014/city-car-sequential-satisficing/6 ipc-2014/child-snack-sequential-satisficing/10 ipc-2014/maintenance-sequential-satisficing/4 ipc-2018/caldera-sequential-satisficing/4 ipc-2018/organic-synthesis-sequential-satisficing/8 ipc-2023/folding-sequential-satisficing/3"
for c in $cells; do
  ipc=${c%%/*}; rest=${c#*/}; v=${rest%/*}; i=${rest#*/}
  dir=$C/$ipc/domains/$v
  dom=$dir/domain.pddl; [ -f $dom ] || dom=$dir/domains/domain-$i.pddl
  prb=$dir/instances/instance-$i.pddl
  for cap in 1 200000; do run "$v/$i" $dom $prb $cap; done
done
rm -f out.json time.err
echo MEASURE DONE >> measure.tsv
