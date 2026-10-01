#!/bin/sh
# Score the lanes29 subset (docs/roadmap-0.29.md), lane by lane, from the
# operator checkout. Usage: score.sh <engine-hash-prefix>
#   17fc6d6f0bd8 = phases A/B (pref boards, loss sides); 4b4d6af75955 = phase C (numeric gain
#   sides, constraints) -- a note-only rebuild of the same code, search behaviour identical.
set -u
B=${1:?engine hash prefix}
REPO=/Users/harold/ferroplan
STAGE="$REPO/benchmarks/probes/lanes29/0.28.0-$B"
CR="$REPO/crucible/target/release/crucible"
cd "$REPO" || exit 1
echo "== Lane 1: IPC points vs SGPlan5 (stage $STAGE)"
python3 /Users/harold/ferroplan-0.29/benchmarks/probes-0.29/lane1/quality.py "$STAGE"
for lane_board in "Lane 3:ipc5-constraints" "Lane W:ipc2018-sat" "Lane W:ipc2023-sat" "Lane 2:ipc2023-numeric" "Lane 2:ipc2026-numeric" "Lane 1 rows:ipc5-simple-pref" "Lane 1 rows:ipc5-qual-pref" "Lane 1 rows:ipc5-complex-pref"; do
  lane=${lane_board%%:*}; board=${lane_board##*:}
  echo; echo "== $lane: rows on $board (compare, promoted 0.28.0 vs $B)"
  "$CR" --repo "$REPO" compare --set cut28 --name lanes29 --a 89cfdc5f06ed --b "$B" --board "$board" 2>&1 | tail -25
done
echo; echo "== Lane 4: storage rows, peak RSS (ru_maxrss) and solved, both engines"
sqlite3 -header -column "file:$HOME/.crucible/db/crucible.db?mode=ro" "
select b.name as board, v.name as variant, i.label as inst, substr(e.blake3,1,12) as engine, r.attempt, r.solved, round(r.max_rss/1048576.0) as rusage_mb, round(r.peak_rss/1048576.0) as sampled_mb, r.time_secs
from run r join engine e on e.id=r.engine_id join instance i on i.id=r.instance_id join variant v on v.id=i.variant_id join board b on b.id=r.board_id
where v.name like 'storage%' and b.name in ('ipc5-qual-pref','ipc5-complex-pref') and (e.blake3 like '$B%' or e.blake3 like '89cfdc5f06ed%')
order by b.name, v.name, cast(i.label as integer), engine, r.attempt" 2>&1 | head -80
