#!/usr/bin/env python3
"""Lane 1's measure, off a crucible SUBSET stage: the IPC quality score of a
board's rows against SGPlan5's `; MetricValue`, the same arithmetic as
standings.py's `pref_quality` (0.29 Phase 0.2b), beside the promoted 0.28.0
boards (benchmarks/air28) for the same cells.

    quality.py <stage-dir> [board ...]        e.g. benchmarks/probes/lane1/0.28.0-<hash>
"""
import json, os, sys, tarfile, tempfile
B = '/Users/harold/ferroplan/benchmarks'
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'probes-0.28', 'sgplan-timing'))
import join
tmp = tempfile.mkdtemp(prefix='ipc5-'); tarfile.open(f'{B}/IPC5-results.tgz').extractall(tmp)
join.R = [os.path.join(d, 'sgplan') for d, sub, _ in os.walk(tmp) if 'sgplan' in sub and os.path.basename(d) == 'RESULTS'][0]
def load(p):
    d = {}
    if os.path.exists(p):
        for l in open(p):
            l = l.strip()
            if l: r = json.loads(l); d[(r['variant'], r['instance'])] = r
    return d
def ok(r): return bool(r and r.get('solved')) and r.get('val') is not False
def metric(r): return r.get('metric') if ok(r) else None
def sgrow(v, i):
    dom, rest = v.split('-', 1); return join.sg(dom, join.TRACK[rest], i) if rest in join.TRACK else None
def score(own, other):
    if own is None: return 0.0
    if other is None: return 1.0
    best = min(own, other)
    if best <= 0: return 1.0 if own <= other + 1e-9 else 0.0
    return best / own
stage = sys.argv[1]
boards = sys.argv[2:] or ['ipc5-simple-pref', 'ipc5-qual-pref', 'ipc5-complex-pref']
print(f"{'board':18s} {'cells':>5} | {'0.28.0':>7} {'stage':>7} {'delta':>7} {'SGPlan5':>8} | rows 0.28.0 / stage | better/worse than 0.28.0 | unpriced 0.28.0 / stage | empty stage")
for b in boards:
    old = load(f'{B}/air28/{b}.jsonl'); new = load(f'{stage}/{b}.jsonl')
    if not new: print(f"{b:18s} (no rows in the stage)"); continue
    so = sn = ss = 0.0; n = 0; up = ups = 0; bet = wor = 0; ro = rn = 0; empty = 0
    for k in sorted(new):
        s = sgrow(*k)
        if not s or s.get('m') is None: continue
        n += 1; m28 = metric(old.get(k)); mst = metric(new[k])
        so += score(m28, s['m']); sn += score(mst, s['m']); ss += score(s['m'], mst)
        ro += ok(old.get(k)); rn += ok(new[k])
        up += ok(old.get(k)) and m28 is None; ups += ok(new[k]) and mst is None
        empty += ok(new[k]) and new[k].get('length') == 0
        if m28 is not None and mst is not None:
            bet += mst < m28 - 1e-9; wor += mst > m28 + 1e-9
    print(f"{b:18s} {n:5d} | {so:7.1f} {sn:7.1f} {sn-so:+7.1f} {ss:8.1f} | {ro:4d} / {rn:<4d}        | {bet:3d} / {wor:<3d}                  | {up:2d} / {ups:<2d}                  | {empty}")
