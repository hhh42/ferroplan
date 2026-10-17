#!/bin/sh
# Lane 2b: the cells the numeric sweep lost on the 0.30 engine, solo at 60 s,
# two reps, under each rule's hatch and on the 0.29.0 board binary. The
# binary is an explicit argument: the first version set it with an env
# prefix on a shell function and the assignment leaked, so every arm after
# the first 0.29.0 arm ran on 0.29.0 (lost-leaked.tsv, 2026-10-16 22:10).
echo $$ > lost.pid
FF30=/Users/harold/ferroplan-0.30/target/release/ff
FF29=/Users/harold/ferroplan/target/release/ff
C26=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2026n/domains
C23=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2023n/domains
printf 'cell\tarm\trep\tsolved\tsecs\tnote\n' > lost.tsv
run() { # bin root cell dom inst arm env
  bin=$1; root=$2; cell=$3; dom=$4; inst=$5; arm=$6; var=$7
  for rep in 1 2; do
    t0=$(date +%s)
    out=$(env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 "$var" perl -e 'alarm shift; exec @ARGV' 75 "$bin" -o "$root/$dom/domain.pddl" -f "$root/$dom/instances/instance-$inst.pddl" --json --threads 1 2>/dev/null)
    s=$(printf '%s' "$out" | python3 -c 'import sys,json
try:
    d=json.load(sys.stdin); print("%s\t%s" % (d["solved"], (d.get("notes") or [""])[-1][:60]))
except Exception: print("no-json\t")')
    printf '%s\t%s\t%d\t%s\t%d\n' "$cell" "$arm" "$rep" "$(printf '%s' "$s" | cut -f1)" "$(( $(date +%s)-t0 ))" >> lost.tsv
  done
}
run_both() { # root cell dom inst -- both rules off
  bin=$FF30; root=$1; cell=$2; dom=$3; inst=$4
  for rep in 1 2; do
    t0=$(date +%s)
    s=$(env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 FF_NO_TRANSFER=1 FF_NO_EQ_EXACT=1 perl -e 'alarm shift; exec @ARGV' 75 "$bin" -o "$root/$dom/domain.pddl" -f "$root/$dom/instances/instance-$inst.pddl" --json --threads 1 2>/dev/null | python3 -c 'import sys,json
try: print(json.load(sys.stdin)["solved"])
except Exception: print("no-json")')
    printf '%s\t%s\t%d\t%s\t%d\n' "$cell" both-off "$rep" "$s" "$(( $(date +%s)-t0 ))" >> lost.tsv
  done
}
cells() { # root dom inst cell
  run $FF30 $1 $4 $2 $3 default FF_X=0
  run $FF30 $1 $4 $2 $3 no-transfer FF_NO_TRANSFER=1
  run $FF30 $1 $4 $2 $3 no-eq-exact FF_NO_EQ_EXACT=1
  run_both $1 $4 $2 $3
  run $FF30 $1 $4 $2 $3 no-consume FF_NO_CONSUME=1
  run $FF29 $1 $4 $2 $3 0.29.0 FF_X=0
}
cells $C23 sugar-numeric-satisficing 16 sugar/16
cells $C23 hydropower-numeric-satisficing 11 hydropower/11
cells $C23 markettrader-numeric-satisficing 1 markettrader/1
cells $C23 delivery-numeric-satisficing 20 delivery/20
cells $C26 gear-car-numeric-2026 9 gear-car/9
echo "PROBE DONE" >> lost.tsv
