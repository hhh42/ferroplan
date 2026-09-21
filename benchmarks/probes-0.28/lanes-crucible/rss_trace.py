#!/usr/bin/env python3
"""rss_trace.py DOMAIN PROBLEM [ENV=VAL ...] -- run ff, stamp each stderr line with t and RSS."""
import subprocess, sys, time, threading, os
FF='/Users/harold/ferroplan-0.28/target/release/ff'
dom, prob = sys.argv[1], sys.argv[2]
env = dict(os.environ, FF_TIME_LIMIT='60', FF_MEM_BUDGET_GB='6', FF_WALL_DEBUG='1', FF_GROUND_PHASES='1')
for kv in sys.argv[3:]:
    k, v = kv.split('=', 1); env[k] = v
t0 = time.time()
p = subprocess.Popen([FF, '-o', dom, '-f', prob, '--json', '--threads', '1'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
lastp=[time.time()]; cur = [0.0]; peak = [0.0, 0.0]; done = [False]
def poll():
    while not done[0]:
        try:
            r = int(subprocess.run(['ps', '-o', 'rss=', '-p', str(p.pid)], capture_output=True, text=True).stdout.strip() or 0) / 1048576
        except Exception: r = 0
        cur[0] = r
        if r > peak[0]: peak[0], peak[1] = r, time.time() - t0
        if time.time()-lastp[0] > 0.5:
            print(f"{time.time()-t0:7.2f}s {r:5.2f}GB  .", flush=True); lastp[0]=time.time()
        time.sleep(0.05)
th = threading.Thread(target=poll); th.start()
last = 0
for line in p.stderr:
    print(f"{time.time()-t0:7.2f}s {cur[0]:5.2f}GB  {line.rstrip()[:150]}", flush=True); lastp[0]=time.time()
p.wait(); done[0] = True; th.join()
print(f"exit {p.returncode} at {time.time()-t0:.2f}s; sampled peak {peak[0]:.2f} GB at {peak[1]:.2f}s")
