#!/bin/sh
# The two rows banked unsolved beside my probe chain (16:09-16:10), re-run solo.
echo $$ > "$(dirname "$0")/rerun.pid"
C=/Users/harold/ferroplan-0.30/crucible/target/release/crucible
$C --repo /Users/harold/ferroplan sweep --set cut29 --headless --max-passes 3 \
   --engine /Users/harold/ferroplan-0.30/target/release/ff --name lane5-rerun \
   --board ipc2018-sat --rows "$(dirname "$0")/rerun.rows" --quiet-only "$@"
