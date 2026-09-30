#!/bin/sh
# Lane W's one banked loss in lanes29 phase B: spider-sat i17, which 0.28.0
# solved at 60.0 s (the wall) and 0.29 did not on a clean third attempt.
# Solo, one thread, the crucible's env, three reps per arm, on a QUIET box:
# 0.29 default, 0.29 FF_NO_EHC_EXTEND=1, 0.28.0.
set -u
NEW=/Users/harold/ferroplan-0.29/target/release/ff
OLD=/Users/harold/ferroplan/target/release/ff
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus
OUT=/Users/harold/ferroplan-0.29/benchmarks/probes-0.29/laneW/probe-spider.out
: > "$OUT"
# cell: corpus-year domain-dir instance   (labyrinth-sat i9 joined it: 0.28.0's
# second-attempt 58.5 s rescue, unsolved three times under 0.29)
for cell in "ipc-2018 spider-sequential-satisficing 17" "ipc-2023 labyrinth-satisficing 9"; do
set -- $cell; year=$1; dom=$2; inst=$3
D="$C/$year/domains/$dom"
dfile="$D/domain.pddl"; [ -f "$D/domains/instance-$inst-domain.pddl" ] && dfile="$D/domains/instance-$inst-domain.pddl"
for arm in "0.29:$NEW:FF_PROBE=1" "0.29-noextend:$NEW:FF_NO_EHC_EXTEND=1" "0.28.0:$OLD:FF_PROBE=1"; do
  tag=${arm%%:*}; rest=${arm#*:}; bin=${rest%%:*}; env=${rest#*:}
  for rep in 1 2 3; do
    /usr/bin/time -p env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 FF_WALL_DEBUG=1 $env \
      perl -e 'alarm shift; exec @ARGV' 90 "$bin" -o "$dfile" -f "$D/instances/instance-$inst.pddl" --json --threads 1 \
      > /tmp/spider.json 2> /tmp/spider.err
    printf "%s/%s %-14s rep %s  " "$dom" "$inst" "$tag" "$rep" >> "$OUT"
    python3 -c "import json; d=json.load(open('/tmp/spider.json')); print('solved', d.get('solved'), end='  ')" >> "$OUT" 2>&1
    grep -E "^real" /tmp/spider.err | tr '\n' ' ' >> "$OUT"; grep -oE "solved by [a-zA-Z-]+|EHC slice exhausted" /tmp/spider.err | head -2 | tr '\n' ' ' >> "$OUT"; echo >> "$OUT"
  done
done
done
cat "$OUT"
