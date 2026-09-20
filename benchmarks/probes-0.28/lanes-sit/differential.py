#!/usr/bin/env python3
"""A row the new engine lost is re-run on BOTH binaries, same harness conditions
(threads 1, 60 s, FF_TIME_LIMIT=60, 2-wide), before it is called a loss: the
published board is best-of-N, and a banked solve that was a lucky third attempt
fails a single shot on ANY engine. usage: differential.py [N attempts, default 1]"""
import json, os, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
HERE = os.path.dirname(os.path.abspath(__file__))
C = '/Users/harold/ferroplan/benchmarks/.ipc-corpus'
BIN = {'old-0.27.1': '/Users/harold/.crucible/worktrees/v0.27.1/target/release/ff',
       'new': '/Users/harold/ferroplan-0.28/target/release/ff'}
IPC = {'tempo-sat': ['ipc-2008', 'ipc-2011'], 'tempo-sat-2014': ['ipc-2014']}
N = int(sys.argv[1]) if len(sys.argv) > 1 else 1
def paths(track, variant, i):
    for ipc in IPC.get(track, ['ipc-2006']):
        base = f"{C}/{ipc}/domains/{variant}"
        p = f"{base}/instances/instance-{i}.pddl"
        if os.path.exists(p):
            d = f"{base}/domain.pddl"
            return (d if os.path.exists(d) else f"{base}/domains/domain-{i}.pddl"), p
    return None, None
def run(job):
    row, arm = job
    d, p = paths(row['track'], row['variant'], row['instance'])
    if not d: return row, arm, None, None
    env = dict(os.environ, FF_TIME_LIMIT='60', FF_MEM_BUDGET_GB='8')
    t = time.time()
    try:
        r = subprocess.run([BIN[arm], '-o', d, '-f', p, '--json', '--threads', '1'], capture_output=True, text=True, timeout=60, env=env)
        solved = bool(json.loads(r.stdout).get('solved'))
    except Exception:
        solved = False
    return row, arm, solved, round(time.time() - t, 1)
lost = json.load(open(f"{HERE}/lost.json"))
jobs = [(r, a) for r in lost for a in BIN for _ in range(N)]
res = {}
with ThreadPoolExecutor(2) as ex:
    for row, arm, solved, secs in ex.map(run, jobs):
        k = (row['variant'], row['instance']); res.setdefault(k, {}).setdefault(arm, []).append((solved, secs))
        print(row['board'], *k, arm, solved, secs, flush=True)
real = [k for k, v in res.items() if any(s for s, _ in v.get('old-0.27.1', [])) and not any(s for s, _ in v.get('new', []))]
print(f"\n{len(lost)} rows lost against the published (best-of-N) board;")
print(f"{len(real)} of them solve on 0.27.1 and not on the new engine under identical single-shot conditions:")
for k in real: print("   ", *k, res[k])
json.dump({f"{k[0]}:{k[1]}": v for k, v in res.items()}, open(f"{HERE}/differential.json", 'w'), indent=1)
