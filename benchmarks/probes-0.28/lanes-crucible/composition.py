#!/usr/bin/env python3
"""What the six boards' gain is MADE OF: a solved row whose plan has length 0 is the
empty plan on a problem with no hard goal (every goal a preference) -- valid, VAL-accepted,
the published boards' own convention (they carry 27), and floor quality.
  composition.py [--mem-engine HASH12]"""
import json, os, sys, collections
HERE = os.path.dirname(os.path.abspath(__file__))
exec(open(os.path.join(HERE, 'tally.py')).read().split("mem = '--mem'")[0])
tot = collections.Counter()
print(f"{'board':20s} {'pub':>4} {'pub empty':>9} | {'now':>4} {'now empty':>9} {'now unpriced':>12} | {'gained':>6} {'gained empty':>12} {'gained, a real plan':>20}")
for b in BOARDS:
    old = load(f"{PUB}/{b}.jsonl"); new = load(f"{CAND}/{b}.jsonl"); new.update(load(f"{MEMHIGH}/{b}.jsonl")); new.update(load(f"{MEM}/{b}.jsonl"))
    keys = [k for k in new if k in old]; z = lambda r: ok(r) and r.get('length') == 0
    unp = lambda r: ok(r) and r.get('metric') is None and b in ('ipc5-simple-pref', 'ipc5-qual-pref', 'ipc5-complex-pref')
    g = [k for k in keys if ok(new[k]) and not ok(old[k])]; ge = [k for k in g if z(new[k])]
    c = dict(o=sum(ok(old[k]) for k in keys), oe=sum(z(old[k]) for k in keys), n=sum(ok(new[k]) for k in keys), ne=sum(z(new[k]) for k in keys), nu=sum(unp(new[k]) for k in keys), g=len(g), ge=len(ge))
    print(f"{b:20s} {c['o']:4d} {c['oe']:9d} | {c['n']:4d} {c['ne']:9d} {c['nu']:12d} | {c['g']:6d} {c['ge']:12d} {c['g']-c['ge']:20d}")
    tot.update(c)
    for v, n in collections.Counter(k[0] for k in ge).items(): print(f"      empty, gained: {v} x{n}")
print(f"{'six boards':20s} {tot['o']:4d} {tot['oe']:9d} | {tot['n']:4d} {tot['ne']:9d} {tot['nu']:12d} | {tot['g']:6d} {tot['ge']:12d} {tot['g']-tot['ge']:20d}")
