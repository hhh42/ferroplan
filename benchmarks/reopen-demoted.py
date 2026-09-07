#!/usr/bin/env python3
"""Re-open every UNSOLVED row that was, or may have been, measured in the
background scheduling band.

The 0.27 cut sweep banked timeouts measured on efficiency cores. Darwin's
background band (`PRIO_DARWIN_BG`, what crucible applied under POLITE) is a
factor of ~13 on this box -- rovers-propositional i36 solves in 4.52 s at
normal priority and 59.28 s demoted -- and the referee could not see it,
because `rho` is CPU SHARE and a demoted process has all the share of a slow
core (0.956, comfortably past the 0.95 line).

Rows written before the `demoted` column (schema v8) cannot say whether they
were demoted, so this uses the throttle windows: a row whose measurement
window overlaps a POLITE window was demoted. Solves are KEPT -- a solve is a
solve however slowly it arrived; only unsolved rows are re-opened.

    python3 benchmarks/reopen-demoted.py            # report
    python3 benchmarks/reopen-demoted.py --reopen   # write
"""
import os, sqlite3, sys, collections

DB = os.path.expanduser("~/.crucible/db/crucible.db")
db = sqlite3.connect(DB, timeout=10)
engine = (
    sys.argv[sys.argv.index("--engine") + 1]
    if "--engine" in sys.argv
    else db.execute(
        "select blake3 from engine where ver like 'ff 0.27%' and blake3 is not null"
        " order by id desc limit 1"
    ).fetchone()[0]
)

rows = db.execute(
    """with pol as (select started_at s, coalesce(ended_at, strftime('%s','now')) e
                     from throttle_window where level='polite')
       select r.id, b.name
         from run r join board b on b.id=r.board_id join engine e on e.id=r.engine_id
        where e.blake3 like ? and r.state='done' and r.banked=1 and r.solved=0
          and r.started_at is not null and r.finished_at is not null
          and coalesce(r.demoted, 0) = 0          -- v8 rows that KNOW they were clean stay
          and exists (select 1 from pol
                       where r.started_at < pol.e and r.finished_at > pol.s)""",
    (engine + "%",),
).fetchall()

print(f"engine {engine[:12]}: {len(rows)} banked timeouts measured inside a POLITE window")
for b, n in sorted(collections.Counter(x[1] for x in rows).items(), key=lambda kv: -kv[1]):
    print(f"  {b:<22} {n}")
if "--reopen" in sys.argv and rows:
    db.executemany(
        "update run set banked=0, verdict='demoted' where id=?", [(r[0],) for r in rows]
    )
    db.commit()
    print(f"re-opened {len(rows)} rows as 'demoted'; they re-run under the fixed referee")
