#!/bin/sh
# Lane 2b: the two cells the numeric sweep lost on the 0.30 engine, solo at
# 60 s -- petri-net 9_1 (0.29.0 at 17 s) under each rule's hatch, gear-car i9
# (0.29.0 at 57 s, a wall-edge shape) three reps. Arms: default, the transfer
# rule off, the Eq rule off, the whole charge off, and the 0.29.0 board binary.
echo $$ > lost.pid
FF=/Users/harold/ferroplan-0.30/target/release/ff
FF29=/Users/harold/ferroplan/target/release/ff
C=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2026n/domains
C23=/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2023n/domains
printf 'cell\tarm\trep\tsolved\tsecs\n' > lost.tsv
run() { # cell dom inst arm env... 
  cell=$1; dom=$2; inst=$3; arm=$4; shift 4
  for rep in 1 2; do
    t0=$(date +%s)
    s=$(env FF_TIME_LIMIT=60 FF_MEM_BUDGET_GB=6 "$@" perl -e 'alarm shift; exec @ARGV' 75 ${BIN:-$FF} -o ${ROOT:-$C}/$dom/domain.pddl -f ${ROOT:-$C}/$dom/instances/instance-$inst.pddl --json --threads 1 2>/dev/null | python3 -c 'import sys,json
try: print(json.load(sys.stdin)["solved"])
except Exception: print("no-json")')
    printf '%s\t%s\t%d\t%s\t%d\n' "$cell" "$arm" "$rep" "$s" "$(( $(date +%s)-t0 ))" >> lost.tsv
  done
}
run petri-net/9_1 petri-net-numeric-2026 9_1 default FF_X=0
run petri-net/9_1 petri-net-numeric-2026 9_1 no-transfer FF_NO_TRANSFER=1
run petri-net/9_1 petri-net-numeric-2026 9_1 no-eq-exact FF_NO_EQ_EXACT=1
run petri-net/9_1 petri-net-numeric-2026 9_1 no-consume FF_NO_CONSUME=1
BIN=$FF29 run petri-net/9_1 petri-net-numeric-2026 9_1 0.29.0 FF_X=0
run gear-car/9 gear-car-numeric-2026 9 default FF_X=0
BIN=$FF29 run gear-car/9 gear-car-numeric-2026 9 0.29.0 FF_X=0
ROOT=$C23 run delivery/20 delivery-numeric-satisficing 20 default FF_X=0
ROOT=$C23 run delivery/20 delivery-numeric-satisficing 20 no-transfer FF_NO_TRANSFER=1
ROOT=$C23 run delivery/20 delivery-numeric-satisficing 20 no-eq-exact FF_NO_EQ_EXACT=1
ROOT=$C23 run delivery/20 delivery-numeric-satisficing 20 no-consume FF_NO_CONSUME=1
ROOT=$C23 BIN=$FF29 run delivery/20 delivery-numeric-satisficing 20 0.29.0 FF_X=0
ROOT=$C23 run sugar/16 sugar-numeric-satisficing 16 default FF_X=0
ROOT=$C23 run sugar/16 sugar-numeric-satisficing 16 no-transfer FF_NO_TRANSFER=1
ROOT=$C23 run sugar/16 sugar-numeric-satisficing 16 no-eq-exact FF_NO_EQ_EXACT=1
ROOT=$C23 run sugar/16 sugar-numeric-satisficing 16 no-consume FF_NO_CONSUME=1
ROOT=$C23 BIN=$FF29 run sugar/16 sugar-numeric-satisficing 16 0.29.0 FF_X=0
ROOT=$C23 run sugar/3 sugar-numeric-satisficing 3 default FF_X=0
ROOT=$C23 run sugar/3 sugar-numeric-satisficing 3 no-transfer FF_NO_TRANSFER=1
ROOT=$C23 run sugar/3 sugar-numeric-satisficing 3 no-eq-exact FF_NO_EQ_EXACT=1
ROOT=$C23 run hydropower/11 hydropower-numeric-satisficing 11 default FF_X=0
ROOT=$C23 run hydropower/11 hydropower-numeric-satisficing 11 no-transfer FF_NO_TRANSFER=1
ROOT=$C23 run hydropower/11 hydropower-numeric-satisficing 11 no-eq-exact FF_NO_EQ_EXACT=1
ROOT=$C23 run hydropower/11 hydropower-numeric-satisficing 11 no-consume FF_NO_CONSUME=1
ROOT=$C23 BIN=$FF29 run hydropower/11 hydropower-numeric-satisficing 11 0.29.0 FF_X=0
ROOT=$C23 run markettrader/1 markettrader-numeric-satisficing 1 default FF_X=0
ROOT=$C23 run markettrader/1 markettrader-numeric-satisficing 1 no-transfer FF_NO_TRANSFER=1
ROOT=$C23 run markettrader/1 markettrader-numeric-satisficing 1 no-eq-exact FF_NO_EQ_EXACT=1
ROOT=$C23 run markettrader/1 markettrader-numeric-satisficing 1 no-consume FF_NO_CONSUME=1
ROOT=$C23 BIN=$FF29 run markettrader/1 markettrader-numeric-satisficing 1 0.29.0 FF_X=0
echo "PROBE DONE" >> lost.tsv
