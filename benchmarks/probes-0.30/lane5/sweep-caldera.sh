#!/bin/sh
echo $$ > "$(dirname "$0")/sweep-caldera.pid"
C=/Users/harold/ferroplan-0.30/crucible/target/release/crucible
$C --repo /Users/harold/ferroplan sweep --set cut29 --headless --max-passes 4 \
   --engine /Users/harold/ferroplan-0.30/target/release/ff --name lane5-caldera \
   --board ipc2018-sat --only 'caldera-sequential' "$@"
