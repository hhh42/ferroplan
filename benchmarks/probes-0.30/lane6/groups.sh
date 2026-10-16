#!/bin/sh
# Lane 6, the route key (docs/roadmap-0.30.md): what interaction_partition
# sees on every IPC-5 propositional variant -- the group count the partition
# route starts from -- read from `--mode partition`'s narration at a 5 s wall
# on three instances per variant. The question: does pathways (where the
# route solves six cells the ladder cannot) decompose where trucks, storage,
# pipesworld and rovers do not?
echo $$ > groups.pid
FF=/Users/harold/ferroplan-0.30/target/release/ff
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2006/domains
printf 'variant\tinst\tgroups\tremaining\n' > groups.tsv
for v in $(ls $C | grep -- "-propositional"); do
  for i in 10 15 20; do
    dom=$C/$v/domain.pddl; [ -f "$dom" ] || dom=$C/$v/domains/domain-$i.pddl
    [ -f "$C/$v/instances/instance-$i.pddl" ] || continue
    line=$(FF_TIME_LIMIT=5 FF_MEM_BUDGET_GB=6 FF_WALL_DEBUG=1 perl -e 'alarm shift; exec @ARGV' 10 $FF -o "$dom" -f "$C/$v/instances/instance-$i.pddl" --json --threads 1 --mode partition 2>&1 >/dev/null | grep -m1 "entry: .* groups")
    g=$(printf '%s' "$line" | sed -n 's/.*entry: \([0-9]*\) groups.*/\1/p')
    printf '%s\t%s\t%s\t%s\n' "$v" "$i" "${g:-?}" "$(printf '%s' "$line" | sed -n 's/.*remaining \(.*\)$/\1/p')" >> groups.tsv
  done
done
echo "PROBE DONE" >> groups.tsv
