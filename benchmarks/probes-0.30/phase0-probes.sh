#!/bin/sh
# The two owed Phase 0 probes, back to back and detached: the losses
# differential (part 2, eleven cells) and the pathways-propositional rows.
# Stop by the pid in THIS file's run.pid (and the child's own run.pid).
echo $$ > "$(dirname "$0")/run.pid"
cd "$(dirname "$0")/losses29" && ./run.sh
cd ../pathways && ./run.sh
echo "PHASE0 PROBES DONE" >> "$(dirname "$0")/../phase0-probes.done"
