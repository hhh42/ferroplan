#!/bin/sh
# Phase 0.5 (docs/roadmap-0.30.md): the pathways-propositional rows the 0.29
# trucks probe could not run (its script asked for one shared domain file and
# pathways ships one per instance, `domains/domain-N.pddl`). Solo, one thread,
# three reps at 60 s and one at 300 s, the board's env; the question is the
# trucks probe's: wall-edge speed or a lost search. Record only; feeds Lane 6.
echo $$ > "$(dirname "$0")/run.pid"   # stop it by THIS pid, never by a pattern
FF=/Users/harold/ferroplan/target/release/ff
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2006/domains/pathways-propositional
OUT="$(dirname "$0")/probe.tsv"
printf 'cell\twall\trep\tsolved\tevals\tsecs\tforeign_pcpu\tnote\n' > "$OUT"
foreign() {
  ps -Ao pcpu,comm -r | awk 'NR>1 && $2 !~ /crucible|Validate|\/ff$|^ps$|^awk$/ { s += $1 } END { printf "%.0f", s }'
}
for i in 17 18 19; do
  rep=0
  for wall in 60 60 60 300; do
    rep=$((rep+1)); fl=$(foreign); t0=$(date +%s)
    out=$(env FF_TIME_LIMIT=$wall FF_MEM_BUDGET_GB=6 perl -e 'alarm shift; exec @ARGV' $((wall+20)) $FF -o "$C/domains/domain-$i.pddl" -f "$C/instances/instance-$i.pddl" --threads 1 --json 2>/dev/null)
    t1=$(date +%s)
    printf '%s' "$out" | python3 -c '
import sys,json
cell,wall,rep,secs,fl=sys.argv[1:]
try:
    d=json.load(sys.stdin); s=d.get("statistics") or {}
    print("%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s" % (cell, wall, rep, d.get("solved"), s.get("evaluated_states"), secs, fl, (d.get("notes") or [""])[-1][:80]))
except Exception as e:
    print("%s\t%s\t%s\tno-json\t\t%s\t%s\t%s" % (cell, wall, rep, secs, fl, e))' "pathways-propositional/$i" "$wall" "$rep" "$((t1-t0))" "$fl" >> "$OUT"
  done
done
echo "PROBE DONE" >> "$OUT"
