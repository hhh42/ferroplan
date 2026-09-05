#!/usr/bin/env python3
"""The cut-time regression re-check (docs/roadmap-0.27.md, the cut rule).

Every instance the 0.26 promoted raw solved that the 0.27 sweep banked
UNSOLVED is re-opened in crucible's database (banked = 0, verdict =
'recheck'), so the next `crucible sweep --set cut27` pass re-runs each of
them solo under the referee -- on a quiet box, which is the operator's job.
Nothing is decided here; the sweep decides, and a row that times out again
under a clean canary window is a real loss and says so.

    python3 benchmarks/recheck27.py            # report only
    python3 benchmarks/recheck27.py --reopen   # write the re-open
"""
import json, os, re, sqlite3, sys, collections

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DB = os.path.expanduser("~/.crucible/db/crucible.db")
ENGINE_PREFIX = sys.argv[sys.argv.index("--engine") + 1] if "--engine" in sys.argv else None

man = open(os.path.join(ROOT, "benchmarks/manifest.toml")).read()
raws = {}
for blk in re.split(r"^\[\[board\]\]", man, flags=re.M)[1:]:
    blk = blk.split("\n[", 1)[0]
    g = lambda k: (re.search(r'^%s\s*=\s*"([^"]+)"' % k, blk, re.M) or [None, None])[1]
    raws[g("id")] = g("raw")
prior = {}
for bid, raw in raws.items():
    p = os.path.join(ROOT, "benchmarks", raw)
    if os.path.exists(p):
        for line in open(p):
            d = json.loads(line)
            if d.get("solved"):
                prior.setdefault(bid, set()).add((d["variant"], str(d["instance"])))

db = sqlite3.connect(DB, timeout=10)
if ENGINE_PREFIX is None:
    # The newest engine reporting 0.27.
    ENGINE_PREFIX = db.execute(
        "select blake3 from engine where ver like 'ff 0.27%' and blake3 is not null order by id desc limit 1"
    ).fetchone()[0]
rows = db.execute(
    """select r.id, b.name, v.name, i.label, r.verdict from run r
        join board b on b.id=r.board_id join instance i on i.id=r.instance_id
        join variant v on v.id=i.variant_id join engine e on e.id=r.engine_id
       where e.blake3 like ? and r.state='done' and r.solved=0 and r.banked=1
         and r.attempt=(select max(a.attempt) from run a where a.board_id=r.board_id
                        and a.instance_id=r.instance_id and a.engine_id=r.engine_id and a.state='done')""",
    (ENGINE_PREFIX + "%",),
).fetchall()
lost = [(rid, b, v, l, verdict) for rid, b, v, l, verdict in rows if (v, l) in prior.get(b, set())]
print(f"engine {ENGINE_PREFIX[:12]}: {len(lost)} instances 0.26 solved that 0.27 banked unsolved")
for b, n in sorted(collections.Counter(x[1] for x in lost).items(), key=lambda kv: -kv[1]):
    print(f"  {b:<22} {n}")
if "--reopen" in sys.argv and lost:
    db.executemany("update run set banked=0, verdict='recheck' where id=?", [(x[0],) for x in lost])
    db.commit()
    print(f"re-opened {len(lost)} rows; run `crucible sweep --set cut27` on a quiet box")
