#!/usr/bin/env python3
"""IPC-5 quality on the PROMOTED boards: 0.27.0 (benchmarks/air27, the cut27 stage) against
0.28.0 (benchmarks/air28) against SGPlan5's `; MetricValue` (IPC5-results.tgz), over the
cells SGPlan5 solved, all metrics minimised.

IPC score per cell = best / own metric (1.0 = the better of the two, 0 = no plan or no
metric). A planner with no metric on a cell scores 0 THERE and the other planner scores 1.0
-- the rule lanes-crucible/quality.py got wrong (it gave SGPlan5 0 wherever ferroplan had
no metric), which is why its SGPlan5 column read low (119.4 / 84.8 / 67.9). Corrected here."""
import json, os, sys, tarfile, tempfile
B = '/Users/harold/ferroplan/benchmarks'
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'sgplan-timing'))
import join
tmp = tempfile.mkdtemp(prefix='ipc5-'); tarfile.open(f'{B}/IPC5-results.tgz').extractall(tmp)
join.R = [os.path.join(d, 'sgplan') for d, sub, _ in os.walk(tmp) if 'sgplan' in sub and os.path.basename(d) == 'RESULTS'][0]
def load(p):
    d = {}
    for l in open(p):
        l = l.strip()
        if l: r = json.loads(l); d[(r['variant'], r['instance'])] = r
    return d
def ok(r): return bool(r and r.get('solved')) and r.get('val') is not False
def metric(r): return r.get('metric') if ok(r) else None
def sgrow(v, i):
    dom, rest = v.split('-', 1); return join.sg(dom, join.TRACK[rest], i) if rest in join.TRACK else None
def score(own, other):
    """own's IPC score against other's metric on one cell."""
    if own is None: return 0.0
    if other is None: return 1.0
    best = min(own, other)
    if best <= 0: return 1.0 if own <= other + 1e-9 else 0.0
    return best / own
print(f"{'board':18s} {'cells':>5} | {'0.27.0':>7} {'0.28.0':>7} {'SGPlan5 (vs 0.28.0)':>20} | 0.28.0 better / equal / worse than SGPlan5, unpriced")
for b in ['ipc5-simple-pref', 'ipc5-qual-pref', 'ipc5-complex-pref']:
    old = load(f'{B}/air27/{b}.jsonl'); new = load(f'{B}/air28/{b}.jsonl')
    so = sn = ss = 0.0; n = 0; bet = eq = wor = unp = 0
    for k in sorted(new):
        s = sgrow(*k)
        if not s or s.get('m') is None: continue
        n += 1; m27 = metric(old.get(k)); m28 = metric(new[k])
        so += score(m27, s['m']); sn += score(m28, s['m']); ss += score(s['m'], m28)
        if ok(new[k]) and m28 is None: unp += 1
        elif m28 is None: pass
        elif m28 < s['m'] - 1e-9: bet += 1
        elif m28 > s['m'] + 1e-9: wor += 1
        else: eq += 1
    print(f"{b:18s} {n:5d} | {so:7.1f} {sn:7.1f} {ss:20.1f} | {bet} / {eq} / {wor}, {unp}")
