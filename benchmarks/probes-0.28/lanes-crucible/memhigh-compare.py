#!/usr/bin/env python3
"""Lane M's regression read: the cells the lanes candidate SOLVED at >= 3.5 GB peak
(memhigh.rows), candidate c0893fb8ccc7 vs the Lane M engine 69471da363ec. Equal-N: one
banked row per cell per engine. Metric and makespan are minimised on every board here."""
import json, glob, os
P = '/Users/harold/ferroplan/benchmarks/probes'
def load(*dirs):
    d = {}
    for dd in dirs:
        for f in glob.glob(f"{P}/{dd}/*.jsonl"):
            for l in open(f):
                l = l.strip()
                if l:
                    r = json.loads(l); r['board'] = os.path.basename(f)[:-6]; d[(r['variant'], r['instance'])] = r
    return d
A = load('lanes-0.28/0.27.1-c0893fb8ccc7', 'lanes-0.28-regress/0.27.1-c0893fb8ccc7')
import sys
BENG = sys.argv[1] if len(sys.argv) > 1 else '69471da363ec'   # the Lane M engine to read; default the first (5a102b8)
B = load('lanes-0.28-memhigh/0.27.1-' + BENG)
print(f'A  lanes candidate [c0893fb8ccc7]\nB  Lane M engine  [{BENG}]')
def ok(r): return bool(r.get('solved')) and r.get('val') is not False
def notes(r):
    n = r.get('notes'); return [n] if isinstance(n, str) else (n or [])
keys = sorted(k for k in B if k in A)
both = [k for k in keys if ok(A[k]) and ok(B[k])]; lost = [k for k in keys if ok(A[k]) and not ok(B[k])]
# LIKE WITH LIKE: a metric against a metric, else a makespan against a makespan. A cell
# whose metric B no longer reports is its own class -- never scored against a makespan.
def field(k):
    a, b = A[k], B[k]
    if a.get('metric') is not None or b.get('metric') is not None: return 'metric'
    return 'makespan'
def q(r, f): return r.get(f)
unpriced = [k for k in both if field(k) == 'metric' and q(A[k], 'metric') is not None and q(B[k], 'metric') is None]
newly = [k for k in both if field(k) == 'metric' and q(A[k], 'metric') is None and q(B[k], 'metric') is not None]
cmpk = [k for k in both if k not in unpriced and k not in newly and q(A[k], field(k)) is not None and q(B[k], field(k)) is not None]
better = [k for k in cmpk if q(B[k], field(k)) < q(A[k], field(k)) - 1e-9]
worse = [k for k in cmpk if q(B[k], field(k)) > q(A[k], field(k)) + 1e-9]
ta = sum(float(A[k]['time']) for k in both); tb = sum(float(B[k]['time']) for k in both)
print(f"cells {len(keys)}   A solved {sum(ok(A[k]) for k in keys)}   B solved {sum(ok(B[k]) for k in keys)}   lost {len(lost)}")
print(f"quality, like with like (lower is better): same {len(cmpk)-len(better)-len(worse)}  better {len(better)}  worse {len(worse)}  |  B lost its metric (unpriced / not scored) {len(unpriced)}  B newly priced {len(newly)}")
print(f"summed solve time over cells both solved: A {ta:.1f} s   B {tb:.1f} s   B/A {tb/ta:.2f}x")
for name, ks in (('LOST', lost), ('WORSE', worse), ('UNPRICED', unpriced), ('BETTER', better)):
    for k in ks:
        a, b = A[k], B[k]
        f = field(k)
        print(f"  {name:8s} {k[0]}/{k[1]:<4} {f} {q(a, f)} -> {q(b, f)}   t {a['time']} -> {b['time']}   {[str(x)[:70] for x in notes(b)]}")
