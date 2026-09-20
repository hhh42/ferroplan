#!/usr/bin/env python3
"""Read the lanes-sit raws against the PUBLISHED boards (best-of-N) and SGPlan5.
Estimator: first attempt (this sit) vs banked (published) -- the conservative side."""
import json, os, sys, collections
HERE = os.path.dirname(os.path.abspath(__file__))
PUB = '/Users/harold/ferroplan/benchmarks'
sys.path.insert(0, os.path.join(HERE, '..', 'sgplan-timing'))
BOARDS = [('qual-pref-2006', 'ipc5-qual-pref'), ('simple-pref-2006', 'ipc5-simple-pref'),
          ('complex-pref-2006', 'ipc5-complex-pref'), ('time-2006', 'ipc5-time'),
          ('metric-time-2006', 'ipc5-metric-time'), ('constraints-2006', 'ipc5-constraints'),
          ('tempo-sat-2014', 'ipc2014-tempo'), ('tempo-sat', 'ipc67-temporal')]
SG = {'ipc5-qual-pref': 100, 'ipc5-simple-pref': 129, 'ipc5-complex-pref': 105, 'ipc5-time': 80,
      'ipc5-metric-time': 151, 'ipc5-constraints': 47}
def load(p):
    d = {}
    if not os.path.exists(p): return d
    for l in open(p):
        try: r = json.loads(l)
        except Exception: continue
        d[(r['variant'], r['instance'])] = r
    return d
def ok(r): return bool(r.get('solved')) and r.get('val') is not False
tot = collections.Counter(); lost_all = []
print(f"{'board':20s} {'rows':>5} {'published':>9} {'this sit':>8} {'delta':>6} {'gained':>6} {'lost':>5} {'SGPlan5':>8}   done?")
for track, board in BOARDS:
    new = load(f"{HERE}/{track}.jsonl"); old = load(f"{PUB}/{board}.jsonl")
    if not new: continue
    keys = [k for k in new if k in old]
    o = sum(ok(old[k]) for k in keys); n = sum(ok(new[k]) for k in keys)
    gained = [k for k in keys if ok(new[k]) and not ok(old[k])]
    lost = [k for k in keys if ok(old[k]) and not ok(new[k])]
    lost_all += [(track, board, k) for k in lost]
    done = os.path.exists(f"{HERE}/{track}.done")
    print(f"{board:20s} {len(keys):5d} {o:9d} {n:8d} {n-o:+6d} {len(gained):6d} {len(lost):5d} {SG.get(board, ''):>8}   {'yes' if done else f'partial ({len(new)} rows)'}")
    tot.update(rows=len(keys), o=o, n=n, g=len(gained), l=len(lost))
    if '-v' in sys.argv:
        by = collections.Counter(); byl = collections.Counter()
        for v, _ in gained: by[v] += 1
        for v, _ in lost: byl[v] += 1
        for v in sorted(set(by) | set(byl)): print(f"      {v:52s} +{by[v]:<3d} -{byl[v]}")
print(f"{'TOTAL':20s} {tot['rows']:5d} {tot['o']:9d} {tot['n']:8d} {tot['n']-tot['o']:+6d} {tot['g']:6d} {tot['l']:5d}")
json.dump([dict(track=t, board=b, variant=k[0], instance=k[1]) for t, b, k in lost_all], open(f"{HERE}/lost.json", 'w'), indent=1)
print(f"\n{len(lost_all)} lost rows written to lost.json (re-run them on the OLD binary with differential.py before calling any a loss)")
