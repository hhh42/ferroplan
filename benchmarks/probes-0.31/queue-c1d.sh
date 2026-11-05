#!/usr/bin/env bash
# The 0.31 box queue for Lanes D and C.1 on the frozen engine /Volumes/Data/Projects/ferroplan-0.31/target/engines/ff-294959ac4415:
# the red-black rung through the matrix sample, then the iPDB arm on the
# four classical proof boards. Serial; nothing beside it.
H=/Volumes/Data/Projects/ferroplan-0.31/benchmarks/probes-0.31
$H/matrix/run-matrix-rb.sh /Volumes/Data/Projects/ferroplan-0.31/target/engines/ff-294959ac4415 > $H/matrix/matrix-rb.log 2>&1
$H/opt/run-opt-ipdb.sh /Volumes/Data/Projects/ferroplan-0.31/target/engines/ff-294959ac4415 > $H/opt/opt-ipdb.log 2>&1
echo "=== QUEUE-C1D COMPLETE $(date '+%F %T')"
