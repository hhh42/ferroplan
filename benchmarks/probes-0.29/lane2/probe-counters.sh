#!/bin/sh
# lanes29 phase B banked three losses of one shape: counters-numeric i12/i13/
# i16 (0.28.0 solved at 39/25/36 s) now end "mem-cap (self-inflicted: node
# byte target raised)" at 16-25 s. Hypothesis: the inlined `widen` reaches the
# best-first refill round sooner, and the round's doubled node byte target
# trips the memory cap. Solo, one thread, the crucible's env, on a quiet box.
set -u
NEW=/Users/harold/ferroplan-0.29/target/release/ff
OLD=/Users/harold/ferroplan/target/release/ff
D=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2023n/domains/counters-numeric-satisficing
OUT=/Users/harold/ferroplan-0.29/benchmarks/probes-0.29/lane2/probe-counters.out
: > "$OUT"
for inst in 12 13; do
  dfile="$D/domain.pddl"; [ -f "$D/domains/instance-$inst-domain.pddl" ] && dfile="$D/domains/instance-$inst-domain.pddl"
  for arm in "0.29:$NEW:FF_PROBE=1" "0.29-noraise:$NEW:FF_NO_NODECAP_REFILL=1" "0.29-norefill:$NEW:FF_NO_REFILL=1" "0.28.0:$OLD:FF_PROBE=1"; do
    tag=${arm%%:*}; rest=${arm#*:}; bin=${rest%%:*}; env=${rest#*:}
    /usr/bin/time -l env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 FF_WALL_DEBUG=1 $env \
      perl -e 'alarm shift; exec @ARGV' 90 "$bin" -o "$dfile" -f "$D/instances/instance-$inst.pddl" --json --threads 1 \
      > /tmp/counters.json 2> /tmp/counters.err
    printf "== i%s %-14s " "$inst" "$tag" >> "$OUT"
    python3 -c "import json; d=json.load(open('/tmp/counters.json')); st=d.get('statistics',{}); print('solved', d.get('solved'), 'evals', st.get('evaluated_states'), '|', ' | '.join(n[:70] for n in d.get('notes',[])))" >> "$OUT" 2>&1
    grep -E "maximum resident" /tmp/counters.err | awk '{printf "   peak %.2f GB\n", $1/1073741824}' >> "$OUT"
    grep -E "real " /tmp/counters.err | awk '{printf "   real %s s\n", $1}' >> "$OUT"
    grep -E "refill round|node byte target|MEMORY checkpoint|checkpoint expired" /tmp/counters.err | sed 's/^/   /' | cut -c1-130 >> "$OUT"
  done
done
cat "$OUT"
