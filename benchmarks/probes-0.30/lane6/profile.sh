#!/bin/sh
# Lane 6 (docs/roadmap-0.30.md): where an evaluation goes on storage-
# propositional i28 (4.5k evals/s on the 0.29 trucks probe) against trucks-
# propositional i10 (35k/s), solo, one thread, through the repo's headless
# hotspot list (benchmarks/profile.py: the profiling build, samply
# --save-only, atos). Record only; the mechanism is chosen from the lists.
echo $$ > run.pid
cd /Users/harold/ferroplan-0.30
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2006/domains
OUT=benchmarks/probes-0.30/lane6
for cell in storage-propositional/28 trucks-propositional/10 pathways-propositional/18; do
  v=${cell%/*}; i=${cell#*/}
  dom=$C/$v/domain.pddl; [ -f "$dom" ] || dom=$C/$v/domains/domain-$i.pddl
  echo "=== $cell" >> $OUT/hotspots.txt
  FF_TIME_LIMIT=40 FF_MEM_BUDGET_GB=6 python3 benchmarks/profile.py "$dom" "$C/$v/instances/instance-$i.pddl" 2>>$OUT/profile.err | head -40 >> $OUT/hotspots.txt
  # evaluations per second on the ordinary release binary, same cell, 30 s
  s=$(FF_TIME_LIMIT=30 FF_MEM_BUDGET_GB=6 perl -e 'alarm shift; exec @ARGV' 45 target/release/ff -o "$dom" -f "$C/$v/instances/instance-$i.pddl" --json --threads 1 2>/dev/null | python3 -c 'import sys,json
try: d=json.load(sys.stdin); print(d["statistics"]["evaluated_states"])
except Exception: print("?")')
  echo "evals-in-30s $cell $s" >> $OUT/evals.txt
done
echo DONE >> $OUT/hotspots.txt
