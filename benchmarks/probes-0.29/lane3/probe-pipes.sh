#!/bin/sh
# Lane 3 side effect seen in lanes29 (docs/roadmap-0.29.md, Lane 1's complex
# row): pipesworld-preferences-complex i12/i14/i15/i16 solved at 6-21 s and
# came back "NOT scored" -- peak RSS across the memory wall's 4.5 GB trip.
# Solo, one thread, the crucible's env, with and without the refill hatch.
set -u
FF=/Users/harold/ferroplan-0.29/target/release/ff
D=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2006/domains/pipesworld-preferences-complex
OUT=/Users/harold/ferroplan-0.29/benchmarks/probes-0.29/lane3/probe-pipes.out
: > "$OUT"
for inst in 14 12; do
  for arm in default FF_NO_TNODE_REFILL=1; do
    printf "== i%s  %s\n" "$inst" "$arm" >> "$OUT"
    if [ "$arm" = default ]; then X=FF_PROBE=1; else X=$arm; fi
    dfile="$D/domain.pddl"; [ -f "$D/domains/instance-$inst-domain.pddl" ] && dfile="$D/domains/instance-$inst-domain.pddl"
    /usr/bin/time -l env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 FF_WALL_DEBUG=1 FF_RES_DEBUG=1 $X \
      perl -e 'alarm shift; exec @ARGV' 90 "$FF" -o "$dfile" -f "$D/instances/instance-$inst.pddl" --json --threads 1 \
      > "/tmp/probe-pipes-$inst-$arm.json" 2> "/tmp/probe-pipes-$inst-$arm.err"
    grep -E "refilled|NOT scored|scored post-hoc|MEMORY checkpoint|maximum resident|real " "/tmp/probe-pipes-$inst-$arm.err" | sed 's/^ *//' | cut -c1-140 >> "$OUT"
    python3 -c "import json,sys; d=json.load(open('/tmp/probe-pipes-$inst-$arm.json')); print('   solved',d.get('solved'),'|',' | '.join(n[:90] for n in d.get('notes',[])))" >> "$OUT" 2>&1
  done
done
cat "$OUT"
