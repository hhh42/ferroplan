#!/usr/bin/env python3
"""The six IPC-5 boards through the crucible, against the PUBLISHED boards and SGPlan5.

  tally.py            the lanes candidate (c0893fb8ccc7) alone
  tally.py --mem      the same, with the Lane M engine's rows (69471da363ec) laid over
                      the 54 cells the candidate banked as `mem-cap` (memcap.rows)

Estimator: the published side is best-of-N; the crucible side is what the referee banked
(re-runs fire on a contended or owed row only). `variants SGPlan5 ran` = a variant with at
least one SGPlan5 .soln in IPC5-results.tgz (unpacked at ../sgplan-timing/ipc5/RESULTS)."""
import json, os, sys, collections
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', 'sgplan-timing'))
import join
if not os.path.isdir(join.R):  # the archive is committed; its unpacked tree is not
    import tarfile, tempfile
    tmp = tempfile.mkdtemp(prefix='ipc5-results-')
    tarfile.open(os.path.join(HERE, '..', '..', 'IPC5-results.tgz')).extractall(tmp)
    hit = [os.path.join(d, 'sgplan') for d, sub, _ in os.walk(tmp) if 'sgplan' in sub and os.path.basename(d) == 'RESULTS']
    join.R = hit[0]
PUB = '/Users/harold/ferroplan/benchmarks'
CAND = PUB + '/probes/lanes-0.28/0.27.1-c0893fb8ccc7'
# --mem-engine HASH12 picks which Lane M engine's rows are laid over (default: the first,
# 69471da363ec = 5a102b8; the fixed one is in engine-mem-fixed.txt / 6-memcap-fixed.log)
MEMENG = sys.argv[sys.argv.index('--mem-engine') + 1] if '--mem-engine' in sys.argv else '69471da363ec'
MEM = PUB + '/probes/lanes-0.28-mem/0.27.1-' + MEMENG
MEMHIGH = PUB + '/probes/lanes-0.28-memhigh/0.27.1-' + MEMENG   # the cells the candidate SOLVED at >= 3.5 GB, same engine
BOARDS = ['ipc5-simple-pref', 'ipc5-qual-pref', 'ipc5-complex-pref', 'ipc5-time', 'ipc5-metric-time', 'ipc5-constraints']
def load(p):
    d = {}
    if os.path.exists(p):
        for l in open(p):
            l = l.strip()
            if l:
                r = json.loads(l); d[(r['variant'], r['instance'])] = r
    return d
def ok(r): return bool(r.get('solved')) and r.get('val') is not False
def sgrow(v, i):
    dom, rest = v.split('-', 1)
    return join.sg(dom, join.TRACK[rest], i) if rest in join.TRACK else None
mem = '--mem' in sys.argv
print(f"{'board':20s} {'rows':>5} {'published':>9} {'crucible':>9} {'delta':>6} {'lost':>5}" + (f" {'+ Lane M':>9} {'delta':>6} {'lost':>5}" if mem else ''))
T = collections.Counter(); L = collections.Counter(); lost_names = []
for b in BOARDS:
    old = load(f"{PUB}/{b}.jsonl"); new = load(f"{CAND}/{b}.jsonl"); over = dict(new); (over.update(load(f"{MEMHIGH}/{b}.jsonl")), over.update(load(f"{MEM}/{b}.jsonl"))) if mem else None
    keys = [k for k in new if k in old]
    o = sum(ok(old[k]) for k in keys); n = sum(ok(new[k]) for k in keys); m = sum(ok(over[k]) for k in keys)
    lost = [k for k in keys if ok(old[k]) and not ok(new[k])]; lostm = [k for k in keys if ok(old[k]) and not ok(over[k])]
    lost_names += [f"{b} {k[0]}/{k[1]}" for k in (lostm if mem else lost)]
    print(f"{b:20s} {len(keys):5d} {o:9d} {n:9d} {n-o:+6d} {len(lost):5d}" + (f" {m:9d} {m-o:+6d} {len(lostm):5d}" if mem else ''))
    T.update(rows=len(keys), o=o, n=n, m=m, l=len(lost), lm=len(lostm))
    ran = {v for v, i in keys if (sgrow(v, i) or {}).get('n', 0) > 0}
    lk = [k for k in keys if k[0] in ran]
    sg = sum(1 for k in lk if (sgrow(*k) or {}).get('n', 0) > 0)
    L[b] = (len(lk), sum(ok(old[k]) for k in lk), sum(ok(new[k]) for k in lk), sum(ok(over[k]) for k in lk), sg)
print(f"{'six boards':20s} {T['rows']:5d} {T['o']:9d} {T['n']:9d} {T['n']-T['o']:+6d} {T['l']:5d}" + (f" {T['m']:9d} {T['m']-T['o']:+6d} {T['lm']:5d}" if mem else ''))
print("   lost vs published:", ', '.join(lost_names) or 'none')
print(f"\n{'variants SGPlan5 ran':20s} {'rows':>5} {'0.27 pub':>9} {'crucible':>9}" + (f" {'+ Lane M':>9}" if mem else '') + f" {'SGPlan5':>8} {'gap was':>8} {'gap now':>8}")
S = [0, 0, 0, 0, 0]
for b in BOARDS:
    r, o, n, m, sg = L[b]; now = m if mem else n
    print(f"{b:20s} {r:5d} {o:9d} {n:9d}" + (f" {m:9d}" if mem else '') + f" {sg:8d} {sg-o:8d} {sg-now:8d}")
    S = [a + c for a, c in zip(S, (r, o, n, m, sg))]
r, o, n, m, sg = S; now = m if mem else n
print(f"{'six boards':20s} {r:5d} {o:9d} {n:9d}" + (f" {m:9d}" if mem else '') + f" {sg:8d} {sg-o:8d} {sg-now:8d}")
