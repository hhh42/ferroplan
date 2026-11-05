#!/usr/bin/env bash
# Lane C.1's measure (0.31): the iPDB collection under saturated cost
# partitioning (FF_OPT_PDB=ipdb) on the four classical proof boards,
# every row, against the banked 0.30.0 rows (b02061eded16). Judged on
# `proven` (the band: +40-80 proofs; 2008-11 / 2014 / 2018 / 2023), with
# the build receipt (`pdb: iPDB ...` note, seconds) on every cell -- the
# kill is build time eating the median cell's wall, or proofs flat with
# the wall spent in A*. Proof boards run solo (jobs = 1): ~12 h. $1 = a
# frozen copy of the engine. Stop with kill $(cat opt-ipdb.pid), then
# pkill -f opt31-ipdb
C=/Volumes/Data/Projects/ferroplan-0.31/crucible/target/release/crucible
FF=${1:?frozen engine path}
echo "=== opt31-ipdb start $(date '+%F %T') ff=$FF"
$C --repo /Volumes/Data/Projects/ferroplan sweep --set cut30 \
  --board ipc-opt-2008-11 --board ipc2014-opt --board ipc2018-opt --board ipc2023-opt \
  --engine "$FF" --env FF_OPT_PDB=ipdb \
  --name opt31-ipdb --headless
echo "=== opt31-ipdb exit $? $(date '+%F %T')"
echo "=== OPT-IPDB COMPLETE $(date '+%F %T')"
