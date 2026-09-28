#!/usr/bin/env python3
"""The 0.28 cut's re-check: re-open banked rows the box, not the engine, decided.

The cut28 sweep ran 2026-09-21..26 beside an iOS Simulator renderer pinned at
111 % CPU for 52 hours (found 09-25); the canary read the box 2.4-6.7x slow
through 09-23/24, and the referee's SUSPECT rule -- which re-runs a row that
FAILED under bad conditions -- never looks at a row that SOLVED under them.
Ten pathways-preferences-simple rows solved at 12:03-12:14 on 09-23, rho
0.3-0.75, canary 2.4-6.3x, and were banked: the empty plan, UNPRICED, where
0.27.0 priced them at 22-39. Coverage cannot see that; the IPC quality score
can (94.8 -> 82.0 on that board).

Re-opened here (banked = 0, verdict = 'recheck'), so the next
`crucible sweep --set cut28` pass re-runs each under the referee on a quiet box:

  1. every banked row whose engine result carries "NOT priced" / "NOT scored"
     -- a plan without its number is a wall verdict, and the wall was slow;
  2. every banked SOLVED row measured with the canary above 1.5x that spent at
     least half its budget -- the outcome could depend on the clock;
  3. every cell 0.27.0's promoted boards solved that 0.28.0 banked UNSOLVED
     (the 24 of `compare --lost`) -- the cut27 rule, so the loss is measured
     on the same night as the v0.27.1 differential over the same cells.

Nothing is decided here; the sweep decides, and a row that reads the same
again under a clean canary is what it is and says so.

    python3 benchmarks/recheck28.py --repo /Users/harold/ferroplan            # report only
    python3 benchmarks/recheck28.py --repo /Users/harold/ferroplan --reopen   # write the re-open
"""
import json, os, re, sqlite3, sys, collections

# `--repo`: the OPERATOR's checkout -- the one the crucible is pointed at, where
# the promoted raws and the cut stages live (they are gitignored). Defaults to
# this script's own repository, which in a worktree is the wrong one.
ROOT = sys.argv[sys.argv.index("--repo") + 1] if "--repo" in sys.argv else os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DB = os.path.expanduser("~/.crucible/db/crucible.db")
ENGINE = sys.argv[sys.argv.index("--engine") + 1] if "--engine" in sys.argv else "89cfdc5f06ed"
PRIOR = sys.argv[sys.argv.index("--prior") + 1] if "--prior" in sys.argv else os.path.join(ROOT, "benchmarks", "air27")
if not os.path.isdir(PRIOR):
    sys.exit(f"no 0.27.0 stage at {PRIOR}: pass --repo <the operator's checkout> (or --prior DIR)")
CANARY_SLOW, WALL_FRAC = 1.5, 0.5

man = open(os.path.join(ROOT, "benchmarks/manifest.toml")).read()
raws = {}
for blk in re.split(r"^\[\[board\]\]", man, flags=re.M)[1:]:
    blk = blk.split("\n[", 1)[0]
    g = lambda k: (re.search(r'^%s\s*=\s*"([^"]+)"' % k, blk, re.M) or [None, None])[1]
    raws[g("id")] = g("raw")
prior = {}
for bid, raw in raws.items():
    # The 0.27.0 rows: the cut27 STAGE (benchmarks/air27), not benchmarks/<board>.jsonl,
    # which promote-air28.sh has already overwritten with 0.28.0's.
    p = os.path.join(PRIOR, ("ipc67-results" if raw == "ipc67-default.jsonl" else raw[:-6]) + ".jsonl")
    if os.path.exists(p):
        for line in open(p):
            d = json.loads(line)
            if d.get("solved") and d.get("val") is not False:
                prior.setdefault(bid, set()).add((d["variant"], str(d["instance"])))

db = sqlite3.connect(f"file:{DB}?immutable=1" if "--reopen" not in sys.argv else DB, uri=True, timeout=30)
rows = db.execute(
    """select r.id, b.name, v.name, i.label, r.solved, r.verdict, r.notes_json, r.wall_ms, b.budget_secs,
              (select s.canary_factor from sample s where s.at <= r.started_at order by s.at desc limit 1)
         from run r join board b on b.id=r.board_id join instance i on i.id=r.instance_id
         join variant v on v.id=i.variant_id join engine e on e.id=r.engine_id
        where e.blake3 like ? and r.state='done' and r.banked=1""",
    (ENGINE + "%",),
).fetchall()
why = {}
for rid, board, variant, label, solved, verdict, notes, wall_ms, budget, canary in rows:
    n = notes or ""
    if solved and ("NOT priced" in n or "NOT scored" in n):
        why[rid] = ("unpriced", board, variant, label)
    elif solved and canary is not None and canary > CANARY_SLOW and wall_ms is not None and budget and wall_ms >= WALL_FRAC * budget * 1000:
        why[rid] = ("slow-box, near the wall", board, variant, label)
    elif not solved and (variant, label) in prior.get(board, set()):
        why[rid] = ("0.27.0 solved it", board, variant, label)
print(f"engine {ENGINE}: {len(why)} banked rows to re-open")
for reason, n in collections.Counter(w[0] for w in why.values()).items():
    print(f"  {n:4d}  {reason}")
for board, n in sorted(collections.Counter(w[1] for w in why.values()).items(), key=lambda kv: -kv[1]):
    print(f"        {board:<22} {n}")
if "--list" in sys.argv:
    for rid, (reason, board, variant, label) in sorted(why.items(), key=lambda kv: kv[1]):
        print(f"    {board:<20} {variant}/{label:<6} {reason}")
if "--reopen" in sys.argv and why:
    db.executemany("update run set banked=0, verdict='recheck' where id=?", [(rid,) for rid in why])
    db.commit()
    print(f"re-opened {len(why)} rows; run `crucible sweep --set cut28` on a quiet box")
