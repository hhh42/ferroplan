#!/usr/bin/env python3
"""Coverage is not the IPC-5 preference score: quality is. For every cell of the four
metric boards that SGPlan5 solved, compare our metric to the MetricValue in its .soln
header (minimised on every one of these boards). IPC-style score per cell = best/ours
(1.0 = as good as the better of the two, 0 = unsolved); an unpriced plan scores 0 here.
Ours = lanes candidate c0893fb8ccc7 with the Lane M rows (69471da363ec) laid over."""
import json, os, sys, collections
HERE = os.path.dirname(os.path.abspath(__file__))
ARGV = sys.argv[:]; sys.argv = sys.argv[:1] + ([a for a in ARGV if a == '--mem-engine' or (ARGV.index(a) > 0 and ARGV[ARGV.index(a) - 1] == '--mem-engine')])
exec(open(os.path.join(HERE, 'tally.py')).read().split("mem = '--mem'")[0])   # load(), ok(), sgrow(), join, paths
def q(a, b):
    if a is None or b is None: return 0.0
    if a <= 0 and b <= 0: return 1.0 if abs(a - b) < 1e-9 else (1.0 if a < b else 0.0)
    best = min(a, b); return 1.0 if a <= best + 1e-9 else max(best, 0.0) / a
print(f"{'board':18s} {'variant':38s} {'sg':>3} {'both':>4} {'empty':>5} {'ours<':>5} {'=':>3} {'sg<':>4} {'unpr':>4} {'0.27 score':>10} {'our score':>9} {'sg score':>8}")
for b in ['ipc5-simple-pref', 'ipc5-qual-pref', 'ipc5-complex-pref']:
    new = load(f"{CAND}/{b}.jsonl"); new.update(load(f"{MEMHIGH}/{b}.jsonl")); new.update(load(f"{MEM}/{b}.jsonl")); old = load(f"{PUB}/{b}.jsonl")
    byv = collections.OrderedDict()
    for k in sorted(new, key=lambda k: (k[0], k[1])): byv.setdefault(k[0], []).append(k)
    T = collections.Counter(); TS = [0.0, 0.0, 0.0]
    for v, ks in byv.items():
        c = collections.Counter(); so = ss = sp = 0.0
        for k in ks:
            s = sgrow(*k)
            if not s or s.get('n', 0) <= 0 and s.get('m') is None: continue
            c['sg'] += 1; r = new[k]; m = r.get('metric') if ok(r) else None
            if ok(r):
                c['both'] += 1
                if r.get('length') == 0: c['empty'] += 1
                if m is None: c['unpr'] += 1
                elif s['m'] is None: pass
                elif m < s['m'] - 1e-9: c['ours'] += 1
                elif m > s['m'] + 1e-9: c['sgb'] += 1
                else: c['eq'] += 1
            o = old.get(k); sp += q(o.get('metric'), s['m']) if o and ok(o) else 0.0
            so += q(m, s['m']) if ok(r) else 0.0; ss += q(s['m'], m if ok(r) else None) if s['m'] is not None else 0.0
        if not c['sg']: continue
        print(f"{b:18s} {v:38s} {c['sg']:3d} {c['both']:4d} {c['empty']:5d} {c['ours']:5d} {c['eq']:3d} {c['sgb']:4d} {c['unpr']:4d} {sp:10.1f} {so:9.1f} {ss:8.1f}")
        T.update(c); TS[0] += so; TS[1] += ss; TS[2] += sp
    print(f"{b:18s} {'TOTAL':38s} {T['sg']:3d} {T['both']:4d} {T['empty']:5d} {T['ours']:5d} {T['eq']:3d} {T['sgb']:4d} {T['unpr']:4d} {TS[2]:10.1f} {TS[0]:9.1f} {TS[1]:8.1f}\n")
